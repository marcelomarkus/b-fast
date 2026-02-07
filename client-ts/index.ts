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

export class BFastDecoder {
    /**
     * Decode B-FAST binary data to JavaScript objects
     * @param buffer - ArrayBuffer or Uint8Array containing B-FAST data
     * @returns Decoded JavaScript object
     */
    static decode(buffer: ArrayBuffer | Uint8Array): any {
        let data = buffer instanceof Uint8Array ? buffer : new Uint8Array(buffer);

        // Auto-detect LZ4 compression (if doesn't start with 'BF' magic)
        if (data.length >= 2 && (data[0] !== 0x42 || data[1] !== 0x46)) {
            try {
                data = lz4.decompress(data);
            } catch (error) {
                throw new BFastError(`LZ4 decompression failed: ${error}`);
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