import { type ByteReader, allocU8, readBytes, readVarintU64 } from "./binary";

export const PRES_BITMAP = 0;
export const PRES_RUNS = 1;
export const PRES_SPARSE = 2;

const POP = new Uint8Array(256);
for (let i = 1; i < 256; i++) POP[i] = POP[i >> 1] + (i & 1);

export function maskBitmapTail(bytes: Uint8Array, count: number): Uint8Array {
    const byteLen = (count + 7) >>> 3;
    const rem = count & 7;
    if (rem === 0) return bytes.length === byteLen ? bytes : bytes.subarray(0, byteLen);
    const mask = (1 << rem) - 1;
    if ((bytes[byteLen - 1] & ~mask) === 0 && bytes.length === byteLen) return bytes;
    const copy = bytes.slice(0, byteLen);
    copy[byteLen - 1] &= mask;
    return copy;
}

export function bitmapPopcount(bitmap: Uint8Array, count: number): number {
    const full = count >>> 3;
    let ones = 0;
    for (let i = 0; i < full; i++) ones += POP[bitmap[i]];
    const rem = count & 7;
    if (rem > 0) ones += POP[bitmap[full] & ((1 << rem) - 1)];
    return ones;
}

export function isBitSet(bitmap: Uint8Array | undefined, i: number): boolean {
    return bitmap === undefined || ((bitmap[i >>> 3] >>> (i & 7)) & 1) === 1;
}

export function normalizePresentBitmap(bitmap: Uint8Array | undefined, count: number): Uint8Array | undefined {
    if (bitmap === undefined || count === 0) return undefined;
    return bitmapPopcount(bitmap, count) === count ? undefined : maskBitmapTail(bitmap, count);
}

export interface MutableIndexable<T> {
    [index: number]: T;
}

export function scatterPresent<T>(
    bitmap: Uint8Array,
    count: number,
    dense: ArrayLike<T>,
    target: MutableIndexable<T>,
): void {
    let d = 0;
    for (let i = 0; i < count; i++) {
        if (((bitmap[i >>> 3] >>> (i & 7)) & 1) !== 0) target[i] = dense[d++];
    }
}

export function readPresenceBits(r: ByteReader, count: number, coding: number): Uint8Array {
    const byteLen = (count + 7) >>> 3;
    if (coding === PRES_BITMAP) return maskBitmapTail(readBytes(r, byteLen), count);
    if (coding === PRES_SPARSE) {
        const sumLen = (byteLen + 7) >>> 3;
        const summary = readBytes(r, sumLen);
        for (let b = byteLen; b < sumLen * 8; b++) {
            if (((summary[b >>> 3] >>> (b & 7)) & 1) !== 0) throw new Error("sparse summary");
        }
        const stored = readBytes(r, bitmapPopcount(summary, byteLen));
        const plain = allocU8(byteLen);
        let s = 0;
        for (let i = 0; i < byteLen; i++) {
            if (((summary[i >>> 3] >>> (i & 7)) & 1) !== 0) {
                const v = stored[s++];
                if (v === 0) throw new Error("sparse zero");
                plain[i] = v;
            }
        }
        return maskBitmapTail(plain, count);
    }
    if (coding === PRES_RUNS) {
        const plain = allocU8(byteLen);
        let at = 0;
        let present = false;
        const maxBig = BigInt(count);
        while (at < count) {
            const lenBig = readVarintU64(r);
            if (lenBig > maxBig - BigInt(at)) throw new Error("runs overflow");
            const end = at + Number(lenBig);
            if (present) {
                for (let i = at; i < end; i++) plain[i >>> 3] |= 1 << (i & 7);
            }
            at = end;
            present = !present;
        }
        return plain;
    }
    throw new Error("presence coding");
}
