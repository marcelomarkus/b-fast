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
        
        // Small integers (bit-packed)
        if ((tag & 0xF0) === 0x30) return tag & 0x0F;
        
        // Int64
        if (tag === 0x38) {
            this.checkBounds(8);
            const value = this.view.getBigInt64(this.offset, true);
            this.offset += 8;
            return Number(value);
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
        
        // UUID
        if (tag === 0x80) {
            this.checkBounds(16);
            const bytes = new Uint8Array(this.view.buffer, this.view.byteOffset + this.offset, 16);
            this.offset += 16;
            
            // Convert to standard UUID format
            const hex = Array.from(bytes, b => b.toString(16).padStart(2, '0')).join('');
            return `${hex.slice(0, 8)}-${hex.slice(8, 12)}-${hex.slice(12, 16)}-${hex.slice(16, 20)}-${hex.slice(20, 32)}`;
        }
        
        // DateTime (Unix timestamp)
        if (tag === 0x81) {
            this.checkBounds(8);
            const timestamp = this.view.getBigInt64(this.offset, true);
            this.offset += 8;
            return new Date(Number(timestamp) * 1000);
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