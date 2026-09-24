import * as lz4 from 'lz4js';
import { initWasmLz4Sync, base64ToUint8Array } from './wasm';

const UTF8_DECODER = new TextDecoder();

function decodeUtf8(bytes: Uint8Array): string {
    const len = bytes.length;
    // Fast path for short ASCII strings (keys and identifiers)
    if (len < 32) {
        let str = '';
        for (let i = 0; i < len; i++) {
            const b = bytes[i];
            if (b >= 128) {
                return UTF8_DECODER.decode(bytes);
            }
            str += String.fromCharCode(b);
        }
        return str;
    }
    return UTF8_DECODER.decode(bytes);
}

interface BFastHeader {
    magic: number;
    flags: number;
    version: number;
    stringTableCount: number;
    stringTable: string[];
}

class BFastParser {
    private view: DataView;
    private buffer: Uint8Array;
    private offset: number = 0;
    private header: BFastHeader;
    private typedArrays: boolean;

    constructor(view: DataView, typedArrays: boolean = false) {
        this.view = view;
        this.buffer = new Uint8Array(view.buffer, view.byteOffset, view.byteLength);
        this.typedArrays = typedArrays;
        this.header = this.parseHeader();
    }

    private parseHeader(): BFastHeader {
        if (this.buffer.length < 6) {
            throw new BFastError('Buffer too small for B-FAST header');
        }

        const magic = this.view.getUint16(0, false);
        if (magic !== 0x4246) { // 'BF'
            throw new BFastError('Invalid B-FAST magic number');
        }

        const flags = this.buffer[2];
        const version = this.buffer[3];
        const stringTableCount = this.view.getUint16(4, true);

        this.offset = 6;
        const stringTable: string[] = new Array(stringTableCount);

        // Parse string table
        for (let i = 0; i < stringTableCount; i++) {
            if (this.offset >= this.buffer.length) {
                throw new BFastError('Unexpected end of buffer in string table');
            }
            
            const length = this.buffer[this.offset++];
            if (this.offset + length > this.buffer.length) {
                throw new BFastError('String extends beyond buffer');
            }
            
            const bytes = this.buffer.subarray(this.offset, this.offset + length);
            stringTable[i] = decodeUtf8(bytes);
            this.offset += length;
        }

        return { magic, flags, version, stringTableCount, stringTable };
    }

    parse(): any {
        return this.parseValue();
    }

    private checkBounds(bytes: number): void {
        if (this.offset + bytes > this.buffer.length) {
            throw new BFastError('Unexpected end of buffer');
        }
    }

    private parseValue(): any {
        if (this.offset >= this.buffer.length) {
            throw new BFastError('Unexpected end of buffer');
        }
        const tag = this.buffer[this.offset++];
        
        // Null
        if (tag === 0x10) return null;
        
        // Booleans
        if (tag === 0x20) return false;
        if (tag === 0x21) return true;
        
        // Int64 (check BEFORE small integers to avoid 0x38 being caught by 0x3X pattern)
        if (tag === 0x38) {
            if (this.offset + 8 > this.buffer.length) throw new BFastError('Unexpected end of buffer');
            const value = this.view.getBigInt64(this.offset, true);
            this.offset += 8;
            return Number(value);
        }
        
        // Small integers (bit-packed)
        if ((tag & 0xF0) === 0x30) return tag & 0x0F;
        
        // Float64
        if (tag === 0x40) {
            if (this.offset + 8 > this.buffer.length) throw new BFastError('Unexpected end of buffer');
            const value = this.view.getFloat64(this.offset, true);
            this.offset += 8;
            return value;
        }
        
        // Raw string
        if (tag === 0x50) {
            if (this.offset + 4 > this.buffer.length) throw new BFastError('Unexpected end of buffer');
            const length = this.view.getUint32(this.offset, true);
            this.offset += 4;
            if (this.offset + length > this.buffer.length) throw new BFastError('Unexpected end of buffer');
            const bytes = this.buffer.subarray(this.offset, this.offset + length);
            this.offset += length;
            return decodeUtf8(bytes);
        }
        
        // List/Array
        if (tag === 0x60) {
            if (this.offset + 4 > this.buffer.length) throw new BFastError('Unexpected end of buffer');
            const length = this.view.getUint32(this.offset, true);
            this.offset += 4;
            const array: any[] = new Array(length);
            for (let i = 0; i < length; i++) {
                array[i] = this.parseValue();
            }
            return array;
        }
        
        // Object start
        if (tag === 0x70) {
            const obj: any = {};
            const stringTable = this.header.stringTable;
            const strTableLen = stringTable.length;
            const buf = this.buffer;
            const view = this.view;
            while (this.offset < buf.length && buf[this.offset] !== 0x7F) {
                if (this.offset + 4 > buf.length) throw new BFastError('Unexpected end of buffer');
                const keyId = view.getUint32(this.offset, true);
                this.offset += 4;
                
                if (keyId >= strTableLen) {
                    throw new BFastError(`Invalid string table index: ${keyId}`);
                }
                
                const key = stringTable[keyId];
                obj[key] = this.parseValue();
            }
            
            if (this.offset >= buf.length) {
                throw new BFastError('Object not properly terminated');
            }
            
            this.offset++; // Skip 0x7F
            return obj;
        }
        
        // Bytes
        if (tag === 0x80) {
            if (this.offset + 4 > this.buffer.length) throw new BFastError('Unexpected end of buffer');
            const length = this.view.getUint32(this.offset, true);
            this.offset += 4;
            if (this.offset + length > this.buffer.length) throw new BFastError('Unexpected end of buffer');
            const bytes = this.buffer.slice(this.offset, this.offset + length);
            this.offset += length;
            return bytes;
        }
        
        // NumPy Array (f64)
        if (tag === 0x90) {
            if (this.offset + 4 > this.buffer.length) throw new BFastError('Unexpected end of buffer');
            const length = this.view.getUint32(this.offset, true);
            this.offset += 4;
            const byteLen = length * 8;
            if (this.offset + byteLen > this.buffer.length) throw new BFastError('Unexpected end of buffer');
            
            const totalByteOffset = this.buffer.byteOffset + this.offset;
            let floatArray: Float64Array;
            if ((totalByteOffset & 7) === 0) {
                // Zero-copy: 8-byte aligned direct view on underlying ArrayBuffer
                floatArray = new Float64Array(this.buffer.buffer, totalByteOffset, length);
            } else {
                // Unaligned fallback: create aligned copy
                const slice = this.buffer.slice(this.offset, this.offset + byteLen);
                floatArray = new Float64Array(slice.buffer, slice.byteOffset, length);
            }
            this.offset += byteLen;
            return this.typedArrays ? floatArray : Array.from(floatArray);
        }
        
        // DateTime (0xD1) - ISO 8601 string
        if (tag === 0xD1) {
            this.checkBounds(4);
            const length = this.view.getUint32(this.offset, true);
            this.offset += 4;
            this.checkBounds(length);
            const bytes = this.buffer.subarray(this.offset, this.offset + length);
            this.offset += length;
            const isoString = decodeUtf8(bytes);
            return new Date(isoString);
        }
        
        // Date (0xD2) - ISO 8601 date string
        if (tag === 0xD2) {
            this.checkBounds(4);
            const length = this.view.getUint32(this.offset, true);
            this.offset += 4;
            this.checkBounds(length);
            const bytes = this.buffer.subarray(this.offset, this.offset + length);
            this.offset += length;
            const isoString = decodeUtf8(bytes);
            return new Date(isoString);
        }
        
        // Time (0xD3) - ISO 8601 time string
        if (tag === 0xD3) {
            this.checkBounds(4);
            const length = this.view.getUint32(this.offset, true);
            this.offset += 4;
            this.checkBounds(length);
            const bytes = this.buffer.subarray(this.offset, this.offset + length);
            this.offset += length;
            return decodeUtf8(bytes);
        }
        
        // UUID (0xD4) - hex string
        if (tag === 0xD4) {
            this.checkBounds(4);
            const length = this.view.getUint32(this.offset, true);
            this.offset += 4;
            this.checkBounds(length);
            const bytes = this.buffer.subarray(this.offset, this.offset + length);
            this.offset += length;
            const hex = decodeUtf8(bytes);
            // Format as UUID: xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx
            return `${hex.slice(0, 8)}-${hex.slice(8, 12)}-${hex.slice(12, 16)}-${hex.slice(16, 20)}-${hex.slice(20)}`;
        }
        
        // Decimal (0xD5) - decimal string
        if (tag === 0xD5) {
            this.checkBounds(4);
            const length = this.view.getUint32(this.offset, true);
            this.offset += 4;
            this.checkBounds(length);
            const bytes = this.buffer.subarray(this.offset, this.offset + length);
            this.offset += length;
            const decimalString = decodeUtf8(bytes);
            return parseFloat(decimalString);
        }
        
        throw new BFastError(`Unknown tag: 0x${tag.toString(16).padStart(2, '0')}`);
    }
}

export type DecompressorFn = (compressedData: Uint8Array, uncompressedSize: number) => Uint8Array;

let wasmAutoAttempted = false;

function ensureWasmAutoInit(): void {
    if (wasmAutoAttempted) return;
    wasmAutoAttempted = true;
    try {
        if (typeof WebAssembly !== 'undefined' && !BFastDecoder.getDecompressor()) {
            initWasmLz4Sync();
        }
    } catch {
        // Silently fall back to lz4js if WebAssembly is unavailable or restricted by CSP
    }
}

function decompressBlockLz4(compressedData: Uint8Array): Uint8Array {
    if (compressedData.length < 4) {
        throw new BFastError('Compressed block too small');
    }
    const view = new DataView(compressedData.buffer, compressedData.byteOffset, compressedData.byteLength);
    const uncompressedSize = view.getUint32(0, true);

    // Auto-initialize WebAssembly on first compressed block if no custom decompressor is registered
    if (!BFastDecoder.getDecompressor()) {
        ensureWasmAutoInit();
    }

    const customDecompressor = BFastDecoder.getDecompressor();
    if (customDecompressor) {
        try {
            return customDecompressor(compressedData.subarray(4), uncompressedSize);
        } catch {
            // Fall back to pure JS decompressor if custom decompressor fails
        }
    }

    const dst = new Uint8Array(uncompressedSize);
    const decompressedSize = lz4.decompressBlock(
        compressedData,
        dst,
        4,
        compressedData.length - 4,
        0
    );
    if (decompressedSize !== uncompressedSize) {
        throw new BFastError(`LZ4 decompression size mismatch: expected ${uncompressedSize}, got ${decompressedSize}`);
    }
    return dst;
}

export interface DecodeOptions<T = any> {
    isCompressed?: boolean;
    typedArrays?: boolean;
    schema?: SchemaValidator<T>;
}

export class BFastDecoder {
    private static _decompressor?: DecompressorFn;

    /**
     * Register a custom high-performance decompressor (such as WebAssembly).
     */
    static setDecompressor(fn: DecompressorFn | undefined): void {
        this._decompressor = fn;
    }

    /**
     * Get the currently registered custom decompressor.
     */
    static getDecompressor(): DecompressorFn | undefined {
        return this._decompressor;
    }

    /**
     * Decode B-FAST binary data to JavaScript objects
     * @param buffer - ArrayBuffer or Uint8Array containing B-FAST data
     * @param optionsOrCompressed - Optional boolean indicating if the data is compressed, or DecodeOptions object
     * @returns Decoded JavaScript object
     */
    static decode<T = any>(
        buffer: ArrayBuffer | Uint8Array,
        optionsOrCompressed?: boolean | DecodeOptions<T>
    ): T {
        let isCompressed: boolean | undefined;
        let typedArrays = false;
        let schema: SchemaValidator<T> | undefined;

        if (typeof optionsOrCompressed === 'boolean') {
            isCompressed = optionsOrCompressed;
        } else if (optionsOrCompressed) {
            isCompressed = optionsOrCompressed.isCompressed;
            typedArrays = !!optionsOrCompressed.typedArrays;
            schema = optionsOrCompressed.schema;
        }

        let data = buffer instanceof Uint8Array ? buffer : new Uint8Array(buffer);

        // Decompress with LZ4 if explicitly compressed or auto-detected (doesn't start with 'BF' magic)
        const shouldDecompress =
            isCompressed !== undefined
                ? isCompressed
                : data.length >= 2 && (data[0] !== 0x42 || data[1] !== 0x46);

        if (shouldDecompress) {
            try {
                // Try single-chunk decompression first
                data = decompressBlockLz4(data);
            } catch (error) {
                // Fall back to parallel chunk decompression
                try {
                    const view = new DataView(data.buffer, data.byteOffset, data.byteLength);
                    const uncompressedSize = view.getUint32(0, true);
                    const chunksCount = view.getUint32(4, true);

                    let offset = 8;
                    const decompressedChunks: Uint8Array[] = [];
                    for (let i = 0; i < chunksCount; i++) {
                        if (offset + 4 > data.length) {
                            throw new BFastError('Unexpected end of data in parallel compression chunk headers');
                        }
                        const chunkLen = view.getUint32(offset, true);
                        offset += 4;
                        if (offset + chunkLen > data.length) {
                            throw new BFastError('Unexpected end of data in parallel compression chunk data');
                        }
                        const chunkData = new Uint8Array(
                            data.buffer.slice(data.byteOffset + offset, data.byteOffset + offset + chunkLen)
                        );
                        offset += chunkLen;

                        decompressedChunks.push(decompressBlockLz4(chunkData));
                    }

                    // Concatenate chunks
                    const result = new Uint8Array(uncompressedSize);
                    let writeOffset = 0;
                    for (const chunk of decompressedChunks) {
                        result.set(chunk, writeOffset);
                        writeOffset += chunk.length;
                    }
                    data = result;
                } catch (parallelError) {
                    throw new BFastError(`LZ4 decompression failed (single: ${error}, parallel: ${parallelError})`);
                }
            }
        }

        const view = new DataView(data.buffer, data.byteOffset, data.byteLength);
        const parsed = new BFastParser(view, typedArrays).parse();
        if (schema) {
            return validateWithSchema(schema, parsed);
        }
        return parsed as T;
    }
}

export class BFastError extends Error {
    constructor(message: string) {
        super(message);
        this.name = 'BFastError';
    }
}

export class BFastValidationError extends BFastError {
    public readonly issues: readonly unknown[];

    constructor(message: string, issues: readonly unknown[] = []) {
        super(message);
        this.name = 'BFastValidationError';
        this.issues = issues;
    }
}

/**
 * Universal schema validator supporting Standard Schema (~standard specification),
 * Zod (v3.x / v4.x), Valibot (v1.0+), ArkType, and custom validator functions.
 */
export type SchemaValidator<T> =
    | {
          '~standard': {
              version: 1;
              vendor: string;
              validate: (
                  value: unknown
              ) =>
                  | { value: T; issues?: undefined }
                  | { issues: readonly unknown[]; value?: unknown }
                  | Promise<
                        | { value: T; issues?: undefined }
                        | { issues: readonly unknown[]; value?: unknown }
                    >;
          };
      }
    | { safeParse: (data: unknown) => { success: true; data: T } | { success: false; error: any } }
    | { parse: (data: unknown) => T }
    | ((data: unknown) => T);

/**
 * Validates data against a provided schema (Standard Schema, Zod, Valibot, ArkType, or function).
 * Throws BFastValidationError on failure.
 */
export function validateWithSchema<T>(schema: SchemaValidator<T> | undefined, data: unknown): T {
    if (!schema) return data as T;

    // 1. Standard Schema (~standard specification - Zod 3.24+, Valibot 1.0+, ArkType 2.0+)
    if (typeof schema === 'object' && schema !== null && '~standard' in schema) {
        const standard = (schema as any)['~standard'];
        const result = standard.validate(data);
        if (result && typeof result.then === 'function') {
            throw new BFastValidationError(
                'Async schema validation is not supported in synchronous BFastDecoder.decode().'
            );
        }
        if (result.issues && result.issues.length > 0) {
            const msg = result.issues
                .map((i: any) => i.message || JSON.stringify(i))
                .join('; ');
            throw new BFastValidationError(`Schema validation failed: ${msg}`, result.issues);
        }
        return result.value as T;
    }

    // 2. Classic Zod safeParse
    if (typeof (schema as any).safeParse === 'function') {
        const result = (schema as any).safeParse(data);
        if (!result.success) {
            const issues = result.error?.issues || [result.error];
            const msg = issues.map((i: any) => i.message || String(i)).join('; ');
            throw new BFastValidationError(`Schema validation failed: ${msg}`, issues);
        }
        return result.data as T;
    }

    // 3. Classic parse
    if (typeof (schema as any).parse === 'function') {
        try {
            return (schema as any).parse(data) as T;
        } catch (err: any) {
            const issues = err?.issues || [err];
            throw new BFastValidationError(err.message || 'Schema validation failed', issues);
        }
    }

    // 4. Custom validator function
    if (typeof schema === 'function') {
        return (schema as (val: unknown) => T)(data);
    }

    return data as T;
}

export const STREAM_MAGIC = 0x4253; // 'BS'
export const STREAM_VERSION = 0x01;
export const FRAME_EOS = 0x00;
export const FRAME_DATA = 0x01;
export const FRAME_DICT_DELTA = 0x02;
export const DEFAULT_MAX_FRAME_SIZE = 64 * 1024 * 1024; // 64 MB

export interface StreamDecodeOptions<T = any> {
    maxFrameSize?: number;
    expectHandshake?: boolean;
    typedArrays?: boolean;
    schema?: SchemaValidator<T>;
}

export class BFastStreamDecoder<T = any> {
    private buffer: Uint8Array;
    private readPos: number = 0;
    private writePos: number = 0;
    private _handshakeReceived: boolean = false;
    private maxFrameSize: number;
    private expectHandshake?: boolean;
    private typedArrays: boolean;
    private schema?: SchemaValidator<T>;
    private _isEos: boolean = false;

    constructor(
        optionsOrMaxFrameSize?: number | StreamDecodeOptions<T>,
        expectHandshake?: boolean,
        typedArrays?: boolean,
        schema?: SchemaValidator<T>
    ) {
        this.buffer = new Uint8Array(8192);
        if (typeof optionsOrMaxFrameSize === 'number') {
            this.maxFrameSize = optionsOrMaxFrameSize;
            this.expectHandshake = expectHandshake;
            this._handshakeReceived = expectHandshake === false;
            this.typedArrays = !!typedArrays;
            this.schema = schema;
        } else if (optionsOrMaxFrameSize && typeof optionsOrMaxFrameSize === 'object') {
            this.maxFrameSize = optionsOrMaxFrameSize.maxFrameSize ?? DEFAULT_MAX_FRAME_SIZE;
            this.expectHandshake = optionsOrMaxFrameSize.expectHandshake;
            this._handshakeReceived = optionsOrMaxFrameSize.expectHandshake === false;
            this.typedArrays = !!optionsOrMaxFrameSize.typedArrays;
            this.schema = optionsOrMaxFrameSize.schema;
        } else {
            this.maxFrameSize = DEFAULT_MAX_FRAME_SIZE;
            this.expectHandshake = expectHandshake;
            this._handshakeReceived = expectHandshake === false;
            this.typedArrays = !!typedArrays;
            this.schema = schema;
        }
    }

    get isEos(): boolean {
        return this._isEos;
    }

    get pendingBytes(): number {
        return this.writePos - this.readPos;
    }

    get handshakeReceived(): boolean {
        return this._handshakeReceived;
    }

    clear(): void {
        this.buffer = new Uint8Array(8192);
        this.readPos = 0;
        this.writePos = 0;
        this._handshakeReceived = this.expectHandshake === false;
        this._isEos = false;
    }

    reset(): void {
        this.clear();
    }

    feed<T = any>(chunk: Uint8Array): T[] {
        if (!chunk || chunk.length === 0) {
            return [];
        }

        // Amortized compaction
        if (this.readPos > 0) {
            if (this.readPos === this.writePos) {
                this.readPos = 0;
                this.writePos = 0;
            } else if (this.readPos >= 32768 && this.readPos >= this.writePos / 2) {
                this.buffer.copyWithin(0, this.readPos, this.writePos);
                this.writePos -= this.readPos;
                this.readPos = 0;
            }
        }

        // Buffer growth
        if (this.writePos + chunk.length > this.buffer.length) {
            const newCap = Math.max(this.buffer.length * 2, this.writePos + chunk.length + 8192);
            const newBuf = new Uint8Array(newCap);
            newBuf.set(this.buffer.subarray(0, this.writePos), 0);
            this.buffer = newBuf;
        }

        this.buffer.set(chunk, this.writePos);
        this.writePos += chunk.length;

        const results: T[] = [];

        // 1. Process Stream Handshake (if not already handled)
        if (!this._handshakeReceived) {
            const available = this.writePos - this.readPos;
            if (available >= 2) {
                const view = new DataView(this.buffer.buffer, this.buffer.byteOffset + this.readPos, available);
                const magic = view.getUint16(0, false); // big-endian
                if (magic === STREAM_MAGIC) {
                    if (available < 4) {
                        return results; // wait for remaining handshake bytes
                    }
                    const version = this.buffer[this.readPos + 2];
                    if (version !== STREAM_VERSION) {
                        throw new BFastError(`Unsupported stream version: ${version} (expected ${STREAM_VERSION})`);
                    }
                    this.readPos += 4;
                    this._handshakeReceived = true;
                } else if (this.expectHandshake === true) {
                    throw new BFastError("Expected stream handshake 'BS' not found");
                } else {
                    this._handshakeReceived = true; // raw stream without 'BS'
                }
            }
        }

        // 2. Process Length-Prefixed Frames
        while (!this._isEos) {
            const available = this.writePos - this.readPos;
            if (available < 6) {
                break; // incomplete frame header
            }

            const view = new DataView(this.buffer.buffer, this.buffer.byteOffset + this.readPos, available);
            const payloadLen = view.getUint32(0, true); // little-endian
            const frameType = this.buffer[this.readPos + 4];
            const flags = this.buffer[this.readPos + 5];

            if (payloadLen > this.maxFrameSize) {
                const err = new BFastError(`Frame size ${payloadLen} exceeds maximum allowed ${this.maxFrameSize}`);
                this.clear();
                throw err;
            }

            if (frameType === FRAME_EOS) {
                this.readPos += 6 + payloadLen;
                this._isEos = true;
                break;
            }

            if (available < 6 + payloadLen) {
                break; // incomplete frame payload
            }

            const frameStart = this.readPos + 6;
            const frameEnd = frameStart + payloadLen;
            const frameBytes = this.buffer.subarray(frameStart, frameEnd);

            if (frameType === FRAME_DATA) {
                const isCompressed = (flags & 0x01) !== 0;
                const decoded = BFastDecoder.decode<T>(frameBytes, {
                    isCompressed,
                    typedArrays: this.typedArrays,
                    schema: this.schema as any,
                });
                results.push(decoded);
            } else if (frameType === FRAME_DICT_DELTA) {
                // Reserved for dictionary delta updates
            } else {
                const err = new BFastError(`Unknown frame type: 0x${frameType.toString(16)}`);
                this.clear();
                throw err;
            }

            this.readPos = frameEnd;
        }

        return results;
    }
}

function compressLz4Payload(uncompressed: Uint8Array): Uint8Array {
    if (uncompressed.length < 13) {
        return uncompressed; // Too small for block compression to be worthwhile
    }
    const blockBuf = new Uint8Array(lz4.compressBound(uncompressed.length));
    const hashTable = new Uint32Array(65536);
    const compLen = lz4.compressBlock(uncompressed, blockBuf, 0, uncompressed.length, hashTable);
    if (compLen === 0 || compLen >= uncompressed.length) {
        return uncompressed; // Compression did not yield size savings
    }
    const result = new Uint8Array(4 + compLen);
    const view = new DataView(result.buffer);
    view.setUint32(0, uncompressed.length, true); // uncompressed size (4B LE)
    result.set(blockBuf.subarray(0, compLen), 4);
    return result;
}

export interface EncodeOptions {
    compress?: boolean;
}

export class BFastEncoder {
    private stringTable: Map<string, number> = new Map();
    private strings: string[] = [];
    private buffer: Uint8Array = new Uint8Array(4096);
    private offset: number = 0;
    private textEncoder: TextEncoder = new TextEncoder();

    private ensureCapacity(needed: number): void {
        if (this.offset + needed > this.buffer.length) {
            const newCap = Math.max(this.buffer.length * 2, this.offset + needed + 4096);
            const newBuf = new Uint8Array(newCap);
            newBuf.set(this.buffer.subarray(0, this.offset));
            this.buffer = newBuf;
        }
    }

    private getStringId(key: string): number {
        let id = this.stringTable.get(key);
        if (id === undefined) {
            id = this.strings.length;
            this.stringTable.set(key, id);
            this.strings.push(key);
        }
        return id;
    }

    private writeUint8(val: number): void {
        this.ensureCapacity(1);
        this.buffer[this.offset++] = val;
    }

    private writeUint32LE(val: number): void {
        this.ensureCapacity(4);
        new DataView(this.buffer.buffer, this.buffer.byteOffset).setUint32(this.offset, val, true);
        this.offset += 4;
    }

    private writeInt64LE(val: number | bigint): void {
        this.ensureCapacity(8);
        new DataView(this.buffer.buffer, this.buffer.byteOffset).setBigInt64(this.offset, BigInt(val), true);
        this.offset += 8;
    }

    private writeFloat64LE(val: number): void {
        this.ensureCapacity(8);
        new DataView(this.buffer.buffer, this.buffer.byteOffset).setFloat64(this.offset, val, true);
        this.offset += 8;
    }

    private writeBytes(bytes: Uint8Array): void {
        this.ensureCapacity(bytes.length);
        this.buffer.set(bytes, this.offset);
        this.offset += bytes.length;
    }

    private serializeValue(val: any): void {
        if (val === null || val === undefined) {
            this.writeUint8(0x10);
            return;
        }
        if (typeof val === 'boolean') {
            this.writeUint8(val ? 0x21 : 0x20);
            return;
        }
        if (typeof val === 'number') {
            if (Number.isInteger(val)) {
                if (val >= 0 && val <= 7) {
                    this.writeUint8(0x30 | val);
                } else {
                    this.writeUint8(0x38);
                    this.writeInt64LE(val);
                }
            } else {
                this.writeUint8(0x40);
                this.writeFloat64LE(val);
            }
            return;
        }
        if (typeof val === 'bigint') {
            this.writeUint8(0x38);
            this.writeInt64LE(val);
            return;
        }
        if (typeof val === 'string') {
            const bytes = this.textEncoder.encode(val);
            this.writeUint8(0x50);
            this.writeUint32LE(bytes.length);
            this.writeBytes(bytes);
            return;
        }
        if (val instanceof Date) {
            const iso = val.toISOString();
            const bytes = this.textEncoder.encode(iso);
            this.writeUint8(0xD1);
            this.writeUint32LE(bytes.length);
            this.writeBytes(bytes);
            return;
        }
        if (val instanceof Float64Array) {
            this.writeUint8(0x90);
            this.writeUint32LE(val.length);
            const byteSlice = new Uint8Array(val.buffer, val.byteOffset, val.byteLength);
            this.writeBytes(byteSlice);
            return;
        }
        if (val instanceof Uint8Array) {
            this.writeUint8(0x80);
            this.writeUint32LE(val.length);
            this.writeBytes(val);
            return;
        }
        if (Array.isArray(val)) {
            this.writeUint8(0x60);
            this.writeUint32LE(val.length);
            for (let i = 0; i < val.length; i++) {
                this.serializeValue(val[i]);
            }
            return;
        }
        if (typeof val === 'object') {
            this.writeUint8(0x70);
            const entries = Object.entries(val);
            for (const [k, v] of entries) {
                if (v === undefined) continue;
                const id = this.getStringId(k);
                this.writeUint32LE(id);
                this.serializeValue(v);
            }
            this.writeUint8(0x7F);
            return;
        }
        throw new BFastError('Unsupported value type for serialization: ' + typeof val);
    }

    /**
     * Encode a JavaScript value into B-FAST binary format.
     * @param data - Any serializable JavaScript value (objects, arrays, primitives, Dates, TypedArrays)
     * @param options - Optional encoding options (e.g. compress: true)
     */
    static encode(data: any, options?: EncodeOptions): Uint8Array {
        const encoder = new BFastEncoder();
        encoder.serializeValue(data);
        const payloadBytes = encoder.buffer.subarray(0, encoder.offset);

        // Build header with string table
        let headerLen = 6;
        const encodedStrings: Uint8Array[] = [];
        for (const str of encoder.strings) {
            const bytes = encoder.textEncoder.encode(str);
            encodedStrings.push(bytes);
            headerLen += 1 + bytes.length;
        }

        const totalLen = headerLen + payloadBytes.length;
        const uncompressedBuf = new Uint8Array(totalLen);
        const view = new DataView(uncompressedBuf.buffer);

        // Header: Magic 'BF' (0x42, 0x46)
        uncompressedBuf[0] = 0x42;
        uncompressedBuf[1] = 0x46;
        uncompressedBuf[2] = 0x00; // flags (uncompressed)
        uncompressedBuf[3] = 0x01; // version
        view.setUint16(4, encoder.strings.length, true);

        let hOffset = 6;
        for (const strBytes of encodedStrings) {
            uncompressedBuf[hOffset++] = strBytes.length;
            uncompressedBuf.set(strBytes, hOffset);
            hOffset += strBytes.length;
        }

        uncompressedBuf.set(payloadBytes, hOffset);

        if (options?.compress) {
            return compressLz4Payload(uncompressedBuf);
        }

        return uncompressedBuf;
    }
}

export class BFastStreamEncoder {
    static getHandshake(): Uint8Array {
        return new Uint8Array([0x42, 0x53, STREAM_VERSION, 0x00]);
    }

    static getEosFrame(): Uint8Array {
        return new Uint8Array([0x00, 0x00, 0x00, 0x00, FRAME_EOS, 0x00]);
    }

    /**
     * Encode a frame for streaming. Accepts either raw B-FAST binary bytes (Uint8Array)
     * or any JavaScript object, which will be serialized on the fly.
     */
    static encodeFrame(objOrBytes: any, compress: boolean = false): Uint8Array {
        const packetData =
            objOrBytes instanceof Uint8Array
                ? objOrBytes
                : BFastEncoder.encode(objOrBytes, { compress });

        const isCompressed =
            packetData.length >= 2 && packetData[0] === 0x42 && packetData[1] === 0x46 ? 0 : 1;
        const frame = new Uint8Array(6 + packetData.length);
        const view = new DataView(frame.buffer, frame.byteOffset, frame.byteLength);
        view.setUint32(0, packetData.length, true);
        frame[4] = FRAME_DATA;
        frame[5] = isCompressed;
        frame.set(packetData, 6);
        return frame;
    }
}

export interface BFastFetchOptions<T = any> extends Omit<RequestInit, 'body'> {
    body?: any;
    compress?: boolean;
    typedArrays?: boolean;
    schema?: SchemaValidator<T>;
}

/**
 * High-level fetch wrapper for B-FAST endpoints.
 * Automatically manages Accept/Content-Type headers, serializes request bodies,
 * and decodes binary B-FAST responses into strongly-typed objects.
 */
export async function bfastFetch<T = any>(
    input: RequestInfo | URL,
    init?: BFastFetchOptions<T>
): Promise<T> {
    const headers = new Headers(init?.headers);
    if (!headers.has('Accept')) {
        headers.set('Accept', 'application/x-bfast, application/octet-stream, */*');
    }

    let body: BodyInit | null | undefined = undefined;
    if (init && 'body' in init && init.body !== undefined && init.body !== null) {
        if (
            typeof init.body === 'string' ||
            init.body instanceof ArrayBuffer ||
            init.body instanceof Uint8Array ||
            (typeof Blob !== 'undefined' && init.body instanceof Blob) ||
            (typeof FormData !== 'undefined' && init.body instanceof FormData) ||
            (typeof URLSearchParams !== 'undefined' && init.body instanceof URLSearchParams)
        ) {
            body = init.body as BodyInit;
        } else {
            body = BFastEncoder.encode(init.body, { compress: init.compress }) as unknown as BodyInit;
            if (!headers.has('Content-Type')) {
                headers.set('Content-Type', 'application/x-bfast');
            }
        }
    }

    const response = await fetch(input, {
        ...init,
        headers,
        body,
    });

    if (!response.ok) {
        throw new BFastError(`HTTP error ${response.status}: ${response.statusText}`);
    }

    const buffer = await response.arrayBuffer();
    return BFastDecoder.decode<T>(buffer, {
        typedArrays: init?.typedArrays,
        schema: init?.schema,
    });
}

export async function* decodeReadableStream<T = any>(
    stream: ReadableStream<Uint8Array>,
    options?: StreamDecodeOptions<T>
): AsyncGenerator<T, void, unknown> {
    const decoder = new BFastStreamDecoder<T>(options);
    const reader = stream.getReader();
    try {
        while (true) {
            const { done, value } = await reader.read();
            if (done) break;
            if (value && value.length > 0) {
                const items = decoder.feed<T>(value);
                for (const item of items) {
                    yield item;
                }
                if (decoder.isEos) break;
            }
        }
    } finally {
        reader.releaseLock();
    }
}

export async function* decodeNodeStream<T = any>(
    stream: any,
    options?: StreamDecodeOptions<T>
): AsyncGenerator<T, void, unknown> {
    const decoder = new BFastStreamDecoder<T>(options);
    for await (const chunk of stream) {
        const bytes = chunk instanceof Uint8Array ? chunk : new Uint8Array(chunk);
        const items = decoder.feed<T>(bytes);
        for (const item of items) {
            yield item;
        }
        if (decoder.isEos) break;
    }
}

export async function* decodeStream<T = any>(
    stream: any,
    options?: StreamDecodeOptions<T>
): AsyncGenerator<T, void, unknown> {
    if (stream && typeof stream.getReader === 'function') {
        yield* decodeReadableStream<T>(stream, options);
    } else if (stream && Symbol.asyncIterator in Object(stream)) {
        yield* decodeNodeStream<T>(stream, options);
    } else {
        throw new BFastError('Input must be a ReadableStream or an async iterable Node.js stream');
    }
}

/**
 * Decode B-FAST binary payload from an MCP (Model Context Protocol) tool result,
 * EmbeddedResource, BlobResourceContents, or base64 string.
 */
export function decodeMcpResource<T = any>(
    resourceOrResult: any,
    options?: DecodeOptions
): T {
    if (!resourceOrResult) {
        throw new BFastError('Invalid MCP resource: null or undefined');
    }

    // 1. Direct Uint8Array or ArrayBuffer
    if (resourceOrResult instanceof Uint8Array || resourceOrResult instanceof ArrayBuffer) {
        return BFastDecoder.decode<T>(resourceOrResult, options);
    }

    // 2. Base64 string
    if (typeof resourceOrResult === 'string') {
        const bytes = base64ToUint8Array(resourceOrResult);
        return BFastDecoder.decode<T>(bytes, options);
    }

    // 3. Object with content or contents array (CallToolResult)
    const contentList = resourceOrResult.content || resourceOrResult.contents;
    if (Array.isArray(contentList)) {
        return decodeMcpResource<T>(contentList, options);
    }

    // 4. Array of content items
    if (Array.isArray(resourceOrResult)) {
        for (const item of resourceOrResult) {
            if (item && item.resource) {
                const res = item.resource;
                const mime = res.mimeType || res.mime_type || '';
                const uri = res.uri || '';
                if (mime.includes('bfast') || uri.includes('bfast') || uri.startsWith('bfast://')) {
                    if (res.blob) {
                        return decodeMcpResource<T>(res.blob, options);
                    }
                }
            } else if (item && item.content) {
                return decodeMcpResource<T>(item.content, options);
            }
        }
        // Fallback: check any item with resource.blob
        for (const item of resourceOrResult) {
            if (item && item.resource && item.resource.blob) {
                return decodeMcpResource<T>(item.resource.blob, options);
            }
        }
        throw new BFastError('No B-FAST resource found in MCP content array');
    }

    // 5. EmbeddedResource
    if (resourceOrResult.resource && resourceOrResult.resource.blob) {
        return decodeMcpResource<T>(resourceOrResult.resource.blob, options);
    }

    // 6. BlobResourceContents
    if (resourceOrResult.blob) {
        return decodeMcpResource<T>(resourceOrResult.blob, options);
    }

    throw new BFastError('Unsupported MCP resource format for B-FAST decoding');
}

/**
 * Configuration options for TanStack Query (React Query, Vue Query, Svelte Query, Solid Query).
 */
export interface BFastQueryOptionsInput<TData, TError = Error> {
    queryKey: readonly unknown[];
    url: string | URL;
    fetchOptions?: BFastFetchOptions<TData>;
    schema?: SchemaValidator<TData>;
    staleTime?: number;
    gcTime?: number;
    enabled?: boolean;
    retry?: boolean | number | ((failureCount: number, error: TError) => boolean);
    select?: (data: TData) => any;
    [key: string]: any;
}

/**
 * Creates query options compatible with TanStack Query (React Query v4/v5, Vue Query, etc.).
 *
 * Usage with TanStack Query:
 * ```typescript
 * import { useQuery } from '@tanstack/react-query';
 * import { bfastQueryOptions } from 'bfast-client';
 * import { z } from 'zod';
 *
 * const UserSchema = z.object({ id: z.number(), name: z.string() });
 *
 * function MyComponent() {
 *   const { data, isLoading } = useQuery(
 *     bfastQueryOptions({
 *       queryKey: ['user', 1],
 *       url: '/api/users/1',
 *       schema: UserSchema,
 *     })
 *   );
 * }
 * ```
 */
export function bfastQueryOptions<TData, TError = Error>(
    options: BFastQueryOptionsInput<TData, TError>
) {
    const { url, fetchOptions, schema, queryKey, ...rest } = options;
    return {
        queryKey,
        queryFn: async ({ signal }: { signal?: AbortSignal }) => {
            return bfastFetch<TData>(url, {
                ...fetchOptions,
                signal: signal || fetchOptions?.signal,
                schema: schema || fetchOptions?.schema,
            });
        },
        ...rest,
    };
}

/**
 * Configuration options for TanStack Infinite Query.
 */
export interface BFastInfiniteQueryOptionsInput<TData, TPageParam = unknown, TError = Error> {
    queryKey: readonly unknown[];
    getUrl: (pageParam: TPageParam) => string | URL;
    initialPageParam: TPageParam;
    getNextPageParam: (
        lastPage: TData,
        allPages: TData[],
        lastPageParam: TPageParam
    ) => TPageParam | undefined | null;
    getPreviousPageParam?: (
        firstPage: TData,
        allPages: TData[],
        firstPageParam: TPageParam
    ) => TPageParam | undefined | null;
    fetchOptions?: BFastFetchOptions<TData>;
    schema?: SchemaValidator<TData>;
    staleTime?: number;
    gcTime?: number;
    enabled?: boolean;
    retry?: boolean | number | ((failureCount: number, error: TError) => boolean);
    [key: string]: any;
}

/**
 * Creates infinite query options compatible with TanStack Query.
 */
export function bfastInfiniteQueryOptions<TData, TPageParam = unknown, TError = Error>(
    options: BFastInfiniteQueryOptionsInput<TData, TPageParam, TError>
) {
    const {
        getUrl,
        initialPageParam,
        getNextPageParam,
        getPreviousPageParam,
        fetchOptions,
        schema,
        queryKey,
        ...rest
    } = options;
    return {
        queryKey,
        initialPageParam,
        getNextPageParam,
        getPreviousPageParam,
        queryFn: async ({
            pageParam,
            signal,
        }: {
            pageParam: TPageParam;
            signal?: AbortSignal;
        }) => {
            const url = getUrl(pageParam);
            return bfastFetch<TData>(url, {
                ...fetchOptions,
                signal: signal || fetchOptions?.signal,
                schema: schema || fetchOptions?.schema,
            });
        },
        ...rest,
    };
}

export { initWasmLz4, initWasmLz4Sync, isWasmEnabled, wasmDecompress, base64ToUint8Array } from './wasm';