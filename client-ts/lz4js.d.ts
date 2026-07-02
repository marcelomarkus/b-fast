declare module 'lz4js' {
  export function decompress(data: Uint8Array): Uint8Array;
  export function compress(data: Uint8Array): Uint8Array;
  export function decompressBlock(src: Uint8Array, dst: Uint8Array, sIndex: number, sLength: number, dIndex: number): number;
}
