declare module 'lz4js' {
  export function decompress(data: Uint8Array): Uint8Array;
  export function compress(data: Uint8Array): Uint8Array;
  export function compressBound(size: number): number;
  export function compressBlock(src: Uint8Array, dst: Uint8Array, sIndex: number, sLength: number, hashTable: Uint32Array): number;
  export function decompressBlock(src: Uint8Array, dst: Uint8Array, sIndex: number, sLength: number, dIndex: number): number;
}
