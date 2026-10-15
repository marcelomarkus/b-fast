import { BFastDecoder } from "./index";
import { getWasmTableBindings, initWasmLz4Sync, WasmTableBindings } from "./wasm";

export type AggregateOp = "sum" | "avg" | "min" | "max" | "count";
export type FilterOp = "==" | "!=" | "<" | "<=" | ">" | ">=";
export type ColumnTypeString = "float64" | "int64" | "boolean" | "string" | "unknown";

const UTF8_DECODER = new TextDecoder();

/**
 * BFastTable: Zero-GC Columnar Engine for Web & WebAssembly.
 * Decodes B-FAST binary records directly into contiguous TypedArrays in WebAssembly linear memory.
 */
export class BFastTable {
    private readonly _bindings: WasmTableBindings | null;
    private readonly _tablePtr: number;
    private readonly _rowCount: number;
    private readonly _colNames: string[];
    private readonly _colTypes: ColumnTypeString[];
    private readonly _colIndices: Map<string, number>;
    private readonly _fallbackData?: Record<string, any[]>;
    private _isDisposed = false;

    private constructor(
        bindings: WasmTableBindings | null,
        tablePtr: number,
        rowCount: number,
        colNames: string[],
        colTypes: ColumnTypeString[],
        fallbackData?: Record<string, any[]>
    ) {
        this._bindings = bindings;
        this._tablePtr = tablePtr;
        this._rowCount = rowCount;
        this._colNames = colNames;
        this._colTypes = colTypes;
        this._colIndices = new Map(colNames.map((name, i) => [name, i]));
        this._fallbackData = fallbackData;
    }

    /**
     * Create a BFastTable from a B-FAST binary buffer.
     * Uses zero-GC WebAssembly linear memory layout when available.
     */
    public static from(data: Uint8Array | ArrayBuffer): BFastTable {
        const bytes = data instanceof Uint8Array ? data : new Uint8Array(data);

        // Try initializing WASM if not already available
        let bindings = getWasmTableBindings();
        if (!bindings) {
            initWasmLz4Sync();
            bindings = getWasmTableBindings();
        }

        if (bindings) {
            const srcPtr = bindings.alloc(bytes.length);
            const memory = bindings.getMemory();
            new Uint8Array(memory.buffer, srcPtr, bytes.length).set(bytes);

            try {
                const tablePtr = bindings.tableCreate(srcPtr, bytes.length);
                if (tablePtr !== 0) {
                    const rowCount = bindings.tableRowCount(tablePtr);
                    const colCount = bindings.tableColumnCount(tablePtr);
                    const colNames: string[] = [];
                    const colTypes: ColumnTypeString[] = [];

                    for (let i = 0; i < colCount; i++) {
                        const namePtr = bindings.tableGetColumnNamePtr(tablePtr, i);
                        const nameLen = bindings.tableGetColumnNameLen(tablePtr, i);
                        const nameBytes = new Uint8Array(bindings.getMemory().buffer, namePtr, nameLen);
                        const name = UTF8_DECODER.decode(nameBytes);
                        colNames.push(name);

                        const typeId = bindings.tableGetColumnType(tablePtr, i);
                        switch (typeId) {
                            case 1:
                                colTypes.push("float64");
                                break;
                            case 2:
                                colTypes.push("int64");
                                break;
                            case 3:
                                colTypes.push("boolean");
                                break;
                            case 4:
                                colTypes.push("string");
                                break;
                            default:
                                colTypes.push("unknown");
                                break;
                        }
                    }

                    return new BFastTable(bindings, tablePtr, rowCount, colNames, colTypes);
                }
            } finally {
                bindings.dealloc(srcPtr, bytes.length);
            }
        }

        // Fallback: parse via BFastDecoder and build columnar representation in JS
        const decoded = BFastDecoder.decode(bytes);
        if (Array.isArray(decoded)) {
            const rowCount = decoded.length;
            if (rowCount === 0) {
                return new BFastTable(null, 0, 0, [], [], {});
            }
            const first = decoded[0] as Record<string, any>;
            const colNames = Object.keys(first);
            const colTypes: ColumnTypeString[] = [];
            const fallbackData: Record<string, any[]> = {};

            for (const col of colNames) {
                fallbackData[col] = new Array(rowCount);
                const sample = first[col];
                if (typeof sample === "number") {
                    colTypes.push(Number.isInteger(sample) ? "int64" : "float64");
                } else if (typeof sample === "boolean") {
                    colTypes.push("boolean");
                } else if (typeof sample === "string") {
                    colTypes.push("string");
                } else {
                    colTypes.push("unknown");
                }
            }

            for (let r = 0; r < rowCount; r++) {
                const row = decoded[r] as Record<string, any>;
                for (const col of colNames) {
                    fallbackData[col][r] = row[col];
                }
            }

            return new BFastTable(null, 0, rowCount, colNames, colTypes, fallbackData);
        } else if (typeof decoded === "object" && decoded !== null) {
            // Columnar dictionary: {"timestamps": [...], "prices": [...]}
            const dict = decoded as Record<string, any[]>;
            const colNames = Object.keys(dict);
            const rowCount = colNames.length > 0 && Array.isArray(dict[colNames[0]]) ? dict[colNames[0]].length : 0;
            const colTypes: ColumnTypeString[] = [];
            for (const col of colNames) {
                const sample = dict[col]?.[0];
                if (typeof sample === "number") {
                    colTypes.push(Number.isInteger(sample) ? "int64" : "float64");
                } else if (typeof sample === "boolean") {
                    colTypes.push("boolean");
                } else if (typeof sample === "string") {
                    colTypes.push("string");
                } else {
                    colTypes.push("unknown");
                }
            }
            return new BFastTable(null, 0, rowCount, colNames, colTypes, dict);
        }

        throw new Error("Cannot create BFastTable: payload is neither a list of objects nor a columnar dictionary");
    }

    public get rowCount(): number {
        return this._rowCount;
    }

    public get columnNames(): string[] {
        return [...this._colNames];
    }

    public getColumnType(name: string): ColumnTypeString {
        const idx = this._colIndices.get(name);
        if (idx === undefined) throw new Error(`Column '${name}' not found in table`);
        return this._colTypes[idx];
    }

    /**
     * Get a zero-copy Float64Array view directly into WebAssembly memory.
     * Allocates 0 bytes on the JavaScript heap.
     */
    public getFloat64Array(name: string): Float64Array {
        this.assertNotDisposed();
        const idx = this._colIndices.get(name);
        if (idx === undefined) throw new Error(`Column '${name}' not found in table`);

        if (this._bindings && this._tablePtr !== 0) {
            const dataPtr = this._bindings.tableGetColumnDataPtr(this._tablePtr, idx);
            const memory = this._bindings.getMemory();
            const colType = this._colTypes[idx];
            if (colType === "int64") {
                const bigIntView = new BigInt64Array(memory.buffer, dataPtr, this._rowCount);
                const float64 = new Float64Array(this._rowCount);
                for (let i = 0; i < this._rowCount; i++) {
                    float64[i] = Number(bigIntView[i]);
                }
                return float64;
            }
            return new Float64Array(memory.buffer, dataPtr, this._rowCount);
        }

        // Fallback
        const colData = this._fallbackData![name];
        return new Float64Array(colData);
    }

    /**
     * Get a zero-copy BigInt64Array view directly into WebAssembly memory.
     */
    public getBigInt64Array(name: string): BigInt64Array {
        this.assertNotDisposed();
        const idx = this._colIndices.get(name);
        if (idx === undefined) throw new Error(`Column '${name}' not found in table`);

        if (this._bindings && this._tablePtr !== 0) {
            const dataPtr = this._bindings.tableGetColumnDataPtr(this._tablePtr, idx);
            const memory = this._bindings.getMemory();
            const colType = this._colTypes[idx];
            if (colType === "float64") {
                const floatView = new Float64Array(memory.buffer, dataPtr, this._rowCount);
                const bigInt = new BigInt64Array(this._rowCount);
                for (let i = 0; i < this._rowCount; i++) {
                    bigInt[i] = BigInt(Math.trunc(floatView[i]));
                }
                return bigInt;
            }
            return new BigInt64Array(memory.buffer, dataPtr, this._rowCount);
        }

        const colData = this._fallbackData![name];
        const res = new BigInt64Array(this._rowCount);
        for (let i = 0; i < this._rowCount; i++) {
            res[i] = BigInt(colData[i]);
        }
        return res;
    }

    /**
     * Get an Int32Array view / copy for integer columns.
     */
    public getInt32Array(name: string): Int32Array {
        this.assertNotDisposed();
        const idx = this._colIndices.get(name);
        if (idx === undefined) throw new Error(`Column '${name}' not found in table`);

        if (this._bindings && this._tablePtr !== 0) {
            const dataPtr = this._bindings.tableGetColumnDataPtr(this._tablePtr, idx);
            const memory = this._bindings.getMemory();
            const colType = this._colTypes[idx];
            if (colType === "float64") {
                const floatView = new Float64Array(memory.buffer, dataPtr, this._rowCount);
                const int32 = new Int32Array(this._rowCount);
                for (let i = 0; i < this._rowCount; i++) {
                    int32[i] = floatView[i] | 0;
                }
                return int32;
            }
            const bigIntView = new BigInt64Array(memory.buffer, dataPtr, this._rowCount);
            const int32 = new Int32Array(this._rowCount);
            for (let i = 0; i < this._rowCount; i++) {
                int32[i] = Number(bigIntView[i]);
            }
            return int32;
        }

        const colData = this._fallbackData![name];
        return new Int32Array(colData);
    }

    /**
     * Get a zero-copy Uint8Array view for boolean columns (1 = true, 0 = false).
     */
    public getBooleanArray(name: string): Uint8Array {
        this.assertNotDisposed();
        const idx = this._colIndices.get(name);
        if (idx === undefined) throw new Error(`Column '${name}' not found in table`);

        if (this._bindings && this._tablePtr !== 0) {
            const dataPtr = this._bindings.tableGetColumnDataPtr(this._tablePtr, idx);
            const memory = this._bindings.getMemory();
            return new Uint8Array(memory.buffer, dataPtr, this._rowCount);
        }

        const colData = this._fallbackData![name];
        const res = new Uint8Array(this._rowCount);
        for (let i = 0; i < this._rowCount; i++) {
            res[i] = colData[i] ? 1 : 0;
        }
        return res;
    }

    /**
     * Read string value for a row on demand.
     */
    public getString(name: string, rowIndex: number): string {
        this.assertNotDisposed();
        if (rowIndex < 0 || rowIndex >= this._rowCount) {
            throw new RangeError(`Row index ${rowIndex} out of bounds (rowCount: ${this._rowCount})`);
        }
        const idx = this._colIndices.get(name);
        if (idx === undefined) throw new Error(`Column '${name}' not found in table`);

        if (this._bindings && this._tablePtr !== 0) {
            const dataPtr = this._bindings.tableGetColumnDataPtr(this._tablePtr, idx);
            const offsetsPtr = this._bindings.tableGetColumnOffsetsPtr(this._tablePtr, idx);
            const memory = this._bindings.getMemory();

            const offsets = new Uint32Array(memory.buffer, offsetsPtr, this._rowCount + 1);
            const start = offsets[rowIndex];
            const end = offsets[rowIndex + 1];
            if (end <= start) return "";
            const strBytes = new Uint8Array(memory.buffer, dataPtr + start, end - start);
            return UTF8_DECODER.decode(strBytes);
        }

        return String(this._fallbackData![name][rowIndex] ?? "");
    }

    /**
     * Materialize entire string array for a column.
     */
    public getStringArray(name: string): string[] {
        const res = new Array<string>(this._rowCount);
        for (let i = 0; i < this._rowCount; i++) {
            res[i] = this.getString(name, i);
        }
        return res;
    }

    /**
     * Instant native aggregation executed in WebAssembly (sum, avg, min, max, count).
     */
    public aggregate(name: string, op: AggregateOp): number | bigint {
        this.assertNotDisposed();
        const idx = this._colIndices.get(name);
        if (idx === undefined) throw new Error(`Column '${name}' not found in table`);

        const colType = this._colTypes[idx];
        const opCode = op === "sum" ? 1 : op === "avg" ? 2 : op === "min" ? 3 : op === "max" ? 4 : 5;

        if (this._bindings && this._tablePtr !== 0) {
            if (colType === "float64") {
                return this._bindings.tableAggregateF64(this._tablePtr, idx, opCode);
            } else if (colType === "int64") {
                return this._bindings.tableAggregateI64(this._tablePtr, idx, opCode);
            }
        }

        // Fallback aggregation in JS
        const arr = colType === "float64" ? this.getFloat64Array(name) : this.getInt32Array(name);
        if (arr.length === 0) return 0;

        if (op === "count") return arr.length;
        let sum = 0;
        let min = Infinity;
        let max = -Infinity;
        for (let i = 0; i < arr.length; i++) {
            const v = arr[i];
            sum += v;
            if (v < min) min = v;
            if (v > max) max = v;
        }
        if (op === "sum") return sum;
        if (op === "avg") return sum / arr.length;
        if (op === "min") return min;
        if (op === "max") return max;
        return 0;
    }

    /**
     * Vectorized filtering directly in WebAssembly.
     * Returns matching row indices in a Uint32Array without allocating row objects.
     */
    public filter(name: string, op: FilterOp, target: number): Uint32Array {
        this.assertNotDisposed();
        const idx = this._colIndices.get(name);
        if (idx === undefined) throw new Error(`Column '${name}' not found in table`);

        const opCode = op === "==" ? 1 : op === "!=" ? 2 : op === "<" ? 3 : op === "<=" ? 4 : op === ">" ? 5 : 6;

        if (this._bindings && this._tablePtr !== 0) {
            const count = this._bindings.tableFilterF64(this._tablePtr, idx, opCode, target);
            if (count === 0) return new Uint32Array(0);
            const indicesPtr = this._bindings.tableGetFilterIndicesPtr();
            const memory = this._bindings.getMemory();
            return new Uint32Array(memory.buffer, indicesPtr, count).slice();
        }

        // Fallback
        const arr = this.getFloat64Array(name);
        const matches: number[] = [];
        for (let i = 0; i < arr.length; i++) {
            const v = arr[i];
            const ok =
                op === "==" ? v === target :
                op === "!=" ? v !== target :
                op === "<" ? v < target :
                op === "<=" ? v <= target :
                op === ">" ? v > target :
                v >= target;
            if (ok) matches.push(i);
        }
        return new Uint32Array(matches);
    }

    /**
     * Materialize table back into standard JavaScript array of objects.
     */
    public toObjects(): Record<string, any>[] {
        const rows: Record<string, any>[] = new Array(this._rowCount);
        const colViews = this._colNames.map((name, c) => {
            const type = this._colTypes[c];
            if (type === "float64") return this.getFloat64Array(name);
            if (type === "int64") return this.getBigInt64Array(name);
            if (type === "boolean") return this.getBooleanArray(name);
            return null;
        });

        for (let r = 0; r < this._rowCount; r++) {
            const obj: Record<string, any> = {};
            for (let c = 0; c < this._colNames.length; c++) {
                const name = this._colNames[c];
                const type = this._colTypes[c];
                if (type === "float64") {
                    obj[name] = (colViews[c] as Float64Array)[r];
                } else if (type === "int64") {
                    obj[name] = Number((colViews[c] as BigInt64Array)[r]);
                } else if (type === "boolean") {
                    obj[name] = (colViews[c] as Uint8Array)[r] === 1;
                } else if (type === "string") {
                    obj[name] = this.getString(name, r);
                } else {
                    obj[name] = null;
                }
            }
            rows[r] = obj;
        }
        return rows;
    }

    /**
     * Free WebAssembly allocated linear memory for this table.
     */
    public free(): void {
        if (!this._isDisposed) {
            this._isDisposed = true;
            if (this._bindings && this._tablePtr !== 0) {
                this._bindings.tableDestroy(this._tablePtr);
            }
        }
    }

    public [Symbol.dispose](): void {
        this.free();
    }

    private assertNotDisposed(): void {
        if (this._isDisposed) {
            throw new Error("BFastTable has been freed / disposed");
        }
    }
}
