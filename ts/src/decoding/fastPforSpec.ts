/**
 * FastPFOR Codec Specification Constants
 *
 * Single source of truth for constants and types shared by encoder and decoder.
 * Kept in decoding/ to keep the dependency graph directional (encoding → decoding).
 */

export type Int32Buf = Int32Array<ArrayBufferLike>;
export type Uint8Buf = Uint8Array<ArrayBufferLike>;

/**
 * Bit masks for each bitwidth 0-32.
 * DO NOT MUTATE - this is a shared constant.
 */
export const MASKS: Readonly<Uint32Array> = (() => {
    const masks = new Uint32Array(33);
    masks[0] = 0;
    for (let bitWidth = 1; bitWidth <= 32; bitWidth++) {
        masks[bitWidth] = bitWidth === 32 ? 0xffffffff : 0xffffffff >>> (32 - bitWidth);
    }
    return masks;
})();

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
