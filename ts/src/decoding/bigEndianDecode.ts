import { IS_LE, bswap32 } from "./fastPforShared";

/**
 * Decodes big-endian bytes into `out` without allocating the output buffer.
 *
 * This function does not copy `bytes`; it writes decoded words into the provided `out` array.
 * For aligned inputs it may create a temporary typed-array view (`Uint32Array`) over `bytes.buffer`
 * to speed up decoding.
 *
 * If `byteLength` is not a multiple of 4, the final word is padded with zeros.
 *
 * @returns Number of int32 words written.
 * @throws RangeError If `(offset, byteLength)` is out of bounds, or if `out` is too small.
 */
export function decodeBigEndianInt32sInto(
    bytes: Uint8Array,
    offset: number,
    byteLength: number,
    out: Int32Array,
): number {
    if (offset < 0 || byteLength < 0 || offset + byteLength > bytes.length) {
        throw new RangeError(
            `decodeBigEndianInt32sInto: out of bounds (offset=${offset}, byteLength=${byteLength}, bytes.length=${bytes.length})`,
        );
    }

    const numCompleteInts = Math.floor(byteLength / 4);
    const hasTrailingBytes = byteLength % 4 !== 0;
    const numInts = hasTrailingBytes ? numCompleteInts + 1 : numCompleteInts;

    if (out.length < numInts) {
        throw new RangeError(`decodeBigEndianInt32sInto: out.length=${out.length} < ${numInts}`);
    }

    if (numCompleteInts > 0) {
        const absoluteOffset = bytes.byteOffset + offset;
        if ((absoluteOffset & 3) === 0) {
            const u32 = new Uint32Array(bytes.buffer, absoluteOffset, numCompleteInts);
            if (IS_LE) {
                for (let i = 0; i < numCompleteInts; i++) {
                    out[i] = bswap32(u32[i]) | 0;
                }
            } else {
                for (let i = 0; i < numCompleteInts; i++) {
                    out[i] = u32[i] | 0;
                }
            }
        } else {
            for (let i = 0; i < numCompleteInts; i++) {
                const base = offset + i * 4;
                out[i] =
                    (bytes[base] << 24) |
                    (bytes[base + 1] << 16) |
                    (bytes[base + 2] << 8) |
                    bytes[base + 3] |
                    0;
            }
        }
    }

    if (hasTrailingBytes) {
        const base = offset + numCompleteInts * 4;
        const remaining = byteLength - numCompleteInts * 4;
        let v = 0;
        for (let i = 0; i < remaining; i++) {
            v |= bytes[base + i] << (24 - i * 8);
        }
        out[numCompleteInts] = v | 0;
    }

    return numInts;
}
