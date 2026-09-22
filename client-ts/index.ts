import * as lz4 from 'lz4js';

interface BFastHeader {
    magic: number;
    flags: number;
    version: number;
    stringTableCount: number;
    stringTable: string[];
}

class BFastParser {
    private view: DataView;
    private offset: number = 0;
    private header: BFastHeader;

    constructor(view: DataView) {
        this.view = view;
        this.header = this.parseHeader();
    }

    private parseHeader(): BFastHeader {
        if (this.view.byteLength < 6) {
            throw new BFastError('Buffer too small for B-FAST header');
        }

        const magic = this.view.getUint16(0, false);
        if (magic !== 0x4246) { // 'BF'
            throw new BFastError('Invalid B-FAST magic number');
        }

        const flags = this.view.getUint8(2);
        const version = this.view.getUint8(3);
        const stringTableCount = this.view.getUint16(4, true);

        this.offset = 6;
        const stringTable: string[] = [];

        // Parse string table
        for (let i = 0; i < stringTableCount; i++) {
            if (this.offset >= this.view.byteLength) {
                throw new BFastError('Unexpected end of buffer in string table');
            }
            
            const length = this.view.getUint8(this.offset++);
            if (this.offset + length > this.view.byteLength) {
                throw new BFastError('String extends beyond buffer');
            }
            
            const bytes = new Uint8Array(this.view.buffer, this.view.byteOffset + this.offset, length);
            stringTable.push(new TextDecoder().decode(bytes));
            this.offset += length;
        }

        return { magic, flags, version, stringTableCount, stringTable };
    }

    parse(): any {
        return this.parseValue();
    }

    private checkBounds(bytes: number): void {
        if (this.offset + bytes > this.view.byteLength) {
            throw new BFastError('Unexpected end of buffer');
        }
    }

    private parseValue(): any {
        this.checkBounds(1);
        const tag = this.view.getUint8(this.offset++);
        
        // Null
        if (tag === 0x10) return null;
        
        // Booleans
        if (tag === 0x20) return false;
        if (tag === 0x21) return true;
        
        // Int64 (check BEFORE small integers to avoid 0x38 being caught by 0x3X pattern)
        if (tag === 0x38) {
            this.checkBounds(8);
            const value = this.view.getBigInt64(this.offset, true);
            this.offset += 8;
            return Number(value);
        }
        
        // Small integers (bit-packed)
        if ((tag & 0xF0) === 0x30) return tag & 0x0F;
        
        // Float64
        if (tag === 0x40) {
            this.checkBounds(8);
            const value = this.view.getFloat64(this.offset, true);
            this.offset += 8;
            return value;
        }
        
        // Raw string
        if (tag === 0x50) {
            this.checkBounds(4);
            const length = this.view.getUint32(this.offset, true);
            this.offset += 4;
            this.checkBounds(length);
            const bytes = new Uint8Array(this.view.buffer, this.view.byteOffset + this.offset, length);
            this.offset += length;
            return new TextDecoder().decode(bytes);
        }
        
        // List/Array
        if (tag === 0x60) {
            this.checkBounds(4);
            const length = this.view.getUint32(this.offset, true);
            this.offset += 4;
            const array: any[] = [];
            for (let i = 0; i < length; i++) {
                array.push(this.parseValue());
            }
            return array;
        }
        
        // Object start
        if (tag === 0x70) {
            const obj: any = {};
            while (this.offset < this.view.byteLength && this.view.getUint8(this.offset) !== 0x7F) {
                this.checkBounds(4);
                const keyId = this.view.getUint32(this.offset, true);
                this.offset += 4;
                
                if (keyId >= this.header.stringTable.length) {
                    throw new BFastError(`Invalid string table index: ${keyId}`);
                }
                
                const key = this.header.stringTable[keyId];
                const value = this.parseValue();
                obj[key] = value;
            }
            
            if (this.offset >= this.view.byteLength) {
                throw new BFastError('Object not properly terminated');
            }
            
            this.offset++; // Skip 0x7F
            return obj;
        }
        
        // Bytes
        if (tag === 0x80) {
            this.checkBounds(4);
            const length = this.view.getUint32(this.offset, true);
            this.offset += 4;
            this.checkBounds(length);
            const bytes = new Uint8Array(this.view.buffer, this.view.byteOffset + this.offset, length);
            this.offset += length;
            return bytes;
        }
        
        // NumPy Array (f64)
        if (tag === 0x90) {
            this.checkBounds(4);
            const length = this.view.getUint32(this.offset, true);
            this.offset += 4;
            this.checkBounds(length * 8);
            
            const array = new Float64Array(length);
            for (let i = 0; i < length; i++) {
                array[i] = this.view.getFloat64(this.offset, true);
                this.offset += 8;
            }
            return Array.from(array);
        }
        
        // DateTime (0xD1) - ISO 8601 string
        if (tag === 0xD1) {
            this.checkBounds(4);
            const length = this.view.getUint32(this.offset, true);
            this.offset += 4;
            this.checkBounds(length);
            const bytes = new Uint8Array(this.view.buffer, this.view.byteOffset + this.offset, length);
            this.offset += length;
            const isoString = new TextDecoder().decode(bytes);
            return new Date(isoString);
        }
        
        // Date (0xD2) - ISO 8601 date string
        if (tag === 0xD2) {
            this.checkBounds(4);
            const length = this.view.getUint32(this.offset, true);
            this.offset += 4;
            this.checkBounds(length);
            const bytes = new Uint8Array(this.view.buffer, this.view.byteOffset + this.offset, length);
            this.offset += length;
            const isoString = new TextDecoder().decode(bytes);
            return new Date(isoString);
        }
        
        // Time (0xD3) - ISO 8601 time string
        if (tag === 0xD3) {
            this.checkBounds(4);
            const length = this.view.getUint32(this.offset, true);
            this.offset += 4;
            this.checkBounds(length);
            const bytes = new Uint8Array(this.view.buffer, this.view.byteOffset + this.offset, length);
            this.offset += length;
            return new TextDecoder().decode(bytes);
        }
        
        // UUID (0xD4) - hex string
        if (tag === 0xD4) {
            this.checkBounds(4);
            const length = this.view.getUint32(this.offset, true);
            this.offset += 4;
            this.checkBounds(length);
            const bytes = new Uint8Array(this.view.buffer, this.view.byteOffset + this.offset, length);
            this.offset += length;
            const hex = new TextDecoder().decode(bytes);
            // Format as UUID: xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx
            return `${hex.slice(0, 8)}-${hex.slice(8, 12)}-${hex.slice(12, 16)}-${hex.slice(16, 20)}-${hex.slice(20)}`;
        }
        
        // Decimal (0xD5) - decimal string
        if (tag === 0xD5) {
            this.checkBounds(4);
            const length = this.view.getUint32(this.offset, true);
            this.offset += 4;
            this.checkBounds(length);
            const bytes = new Uint8Array(this.view.buffer, this.view.byteOffset + this.offset, length);
            this.offset += length;
            const decimalString = new TextDecoder().decode(bytes);
            return parseFloat(decimalString);
        }
        
        throw new BFastError(`Unknown tag: 0x${tag.toString(16).padStart(2, '0')}`);
    }
}

function decompressBlockLz4(compressedData: Uint8Array): Uint8Array {
    if (compressedData.length < 4) {
        throw new BFastError('Compressed block too small');
    }
    const view = new DataView(compressedData.buffer, compressedData.byteOffset, compressedData.byteLength);
    const uncompressedSize = view.getUint32(0, true);
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

export class BFastDecoder {
    /**
     * Decode B-FAST binary data to JavaScript objects
     * @param buffer - ArrayBuffer or Uint8Array containing B-FAST data
     * @param isCompressed - Optional boolean indicating if the data is compressed
     * @returns Decoded JavaScript object
     */
    static decode(buffer: ArrayBuffer | Uint8Array, isCompressed?: boolean): any {
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
        return new BFastParser(view).parse();
    }
}

export class BFastError extends Error {
    constructor(message: string) {
        super(message);
        this.name = 'BFastError';
    }
}

export const STREAM_MAGIC = 0x4253; // 'BS'
export const STREAM_VERSION = 0x01;
export const FRAME_EOS = 0x00;
export const FRAME_DATA = 0x01;
export const FRAME_DICT_DELTA = 0x02;
export const DEFAULT_MAX_FRAME_SIZE = 64 * 1024 * 1024; // 64 MB

export interface StreamDecodeOptions {
    maxFrameSize?: number;
    expectHandshake?: boolean;
}

export class BFastStreamDecoder {
    private buffer: Uint8Array;
    private readPos: number = 0;
    private writePos: number = 0;
    private _handshakeReceived: boolean = false;
    private maxFrameSize: number;
    private expectHandshake?: boolean;
    private _isEos: boolean = false;

    constructor(maxFrameSize: number = DEFAULT_MAX_FRAME_SIZE, expectHandshake?: boolean) {
        this.buffer = new Uint8Array(8192);
        this.maxFrameSize = maxFrameSize;
        this.expectHandshake = expectHandshake;
        this._handshakeReceived = expectHandshake === false;
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

    feed(chunk: Uint8Array): any[] {
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

        const results: any[] = [];

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
                const decoded = BFastDecoder.decode(frameBytes, isCompressed);
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

export class BFastStreamEncoder {
    static getHandshake(): Uint8Array {
        return new Uint8Array([0x42, 0x53, STREAM_VERSION, 0x00]);
    }

    static getEosFrame(): Uint8Array {
        return new Uint8Array([0x00, 0x00, 0x00, 0x00, FRAME_EOS, 0x00]);
    }

    static encodeFrame(packetData: Uint8Array): Uint8Array {
        const isCompressed = packetData.length >= 2 && packetData[0] === 0x42 && packetData[1] === 0x46 ? 0 : 1;
        const frame = new Uint8Array(6 + packetData.length);
        const view = new DataView(frame.buffer, frame.byteOffset, frame.byteLength);
        view.setUint32(0, packetData.length, true);
        frame[4] = FRAME_DATA;
        frame[5] = isCompressed;
        frame.set(packetData, 6);
        return frame;
    }
}

export async function* decodeReadableStream(
    stream: ReadableStream<Uint8Array>,
    options?: StreamDecodeOptions
): AsyncGenerator<any, void, unknown> {
    const decoder = new BFastStreamDecoder(options?.maxFrameSize, options?.expectHandshake);
    const reader = stream.getReader();
    try {
        while (true) {
            const { done, value } = await reader.read();
            if (done) break;
            if (value && value.length > 0) {
                const items = decoder.feed(value);
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

export async function* decodeNodeStream(
    stream: any,
    options?: StreamDecodeOptions
): AsyncGenerator<any, void, unknown> {
    const decoder = new BFastStreamDecoder(options?.maxFrameSize, options?.expectHandshake);
    for await (const chunk of stream) {
        const bytes = chunk instanceof Uint8Array ? chunk : new Uint8Array(chunk);
        const items = decoder.feed(bytes);
        for (const item of items) {
            yield item;
        }
        if (decoder.isEos) break;
    }
}

export async function* decodeStream(
    stream: any,
    options?: StreamDecodeOptions
): AsyncGenerator<any, void, unknown> {
    if (stream && typeof stream.getReader === 'function') {
        yield* decodeReadableStream(stream, options);
    } else if (stream && Symbol.asyncIterator in Object(stream)) {
        yield* decodeNodeStream(stream, options);
    } else {
        throw new BFastError('Input must be a ReadableStream or an async iterable Node.js stream');
    }
}