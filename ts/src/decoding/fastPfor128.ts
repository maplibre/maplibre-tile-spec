import { decodeFastPforInt32 } from "./fastPforDecoder";

/**
 * The v2 FastPFOR variant (`docs/encodings.md#fastpfor`): 128-value blocks over little-endian
 * words. The `fastpfor` crate mlt-core decodes with packs every block as four sequential 32-value
 * groups, exactly as the v1 kernel does for eight, so the v1 decoder runs with the block size
 * changed. 64-bit words have no v2 FastPFOR form in the reference decoder.
 */
export function decodeFastPfor128(
    bytes: Uint8Array,
    start: number,
    byteLength: number,
    numValues: number,
): Uint32Array {
    if ((byteLength & 3) !== 0) {
        throw new Error(`FastPFOR payload of ${byteLength} bytes is not whole 32-bit words`);
    }
    if (numValues === 0) {
        if (byteLength !== 0) {
            throw new Error(`FastPFOR stream of no values carries ${byteLength} bytes`);
        }
        return new Uint32Array(0);
    }
    const view = new DataView(bytes.buffer, bytes.byteOffset + start, byteLength);
    const words = new Uint32Array(byteLength >>> 2);
    for (let i = 0; i < words.length; i++) {
        words[i] = view.getUint32(i * 4, true);
    }
    return decodeFastPforInt32(words, numValues, undefined, 128);
}
