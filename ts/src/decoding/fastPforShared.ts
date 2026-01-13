/**
 * Shared constants and helpers for the FastPFOR codec.
 */

export type Int32Buf = Int32Array<ArrayBufferLike>;
export type Uint8Buf = Uint8Array<ArrayBufferLike>;

/**
 * Bit masks for each bitwidth 0-32.
 * DO NOT MUTATE - this is a shared constant.
 */
const masks = new Uint32Array(33);
masks[0] = 0;
for (let bitWidth = 1; bitWidth <= 32; bitWidth++) {
    masks[bitWidth] = bitWidth === 32 ? 0xffffffff : 0xffffffff >>> (32 - bitWidth);
}
export const MASKS: Readonly<Uint32Array> = masks;

export const DEFAULT_PAGE_SIZE = 65536;
export const BLOCK_SIZE = 256;

export function greatestMultiple(value: number, factor: number): number {
    return value - (value % factor);
}

export function roundUpToMultipleOf32(value: number): number {
    return greatestMultiple(value + 31, 32);
}

export function normalizePageSize(pageSize: number): number {
    if (!Number.isFinite(pageSize) || pageSize <= 0) return DEFAULT_PAGE_SIZE;

    const aligned = greatestMultiple(Math.floor(pageSize), BLOCK_SIZE);
    return aligned === 0 ? BLOCK_SIZE : aligned;
}

/**
 * True if `TypedArray` views use little-endian byte order on this runtime.
 * (Most JS engines run on little-endian platforms, but big-endian platforms exist.)
 */
export const IS_LE = new Uint8Array(new Uint32Array([0x11223344]).buffer)[0] === 0x44;

export function bswap32(value: number): number {
    const x = value >>> 0;
    return (((x & 0xff) << 24) | ((x & 0xff00) << 8) | ((x >>> 8) & 0xff00) | ((x >>> 24) & 0xff)) >>> 0;
}
