export const IS_LE = new Uint8Array(new Uint32Array([0x11223344]).buffer)[0] === 0x44;

function bswap32(value: number): number {
    const x = value >>> 0;
    return (((x & 0xff) << 24) | ((x & 0xff00) << 8) | ((x >>> 8) & 0xff00) | ((x >>> 24) & 0xff)) >>> 0;
}

/**
 * Decodes big-endian bytes into `out` without allocating.
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

/**
 * Reads a big-endian byte range as int32 words.
 *
 * If `byteLength` is not a multiple of 4, the final word is padded with zeros.
 *
 * @param bytes - Source byte buffer.
 * @param offset - Start offset within `bytes`.
 * @param byteLength - Number of bytes to read.
 * @returns Decoded int32 words.
 * @throws RangeError If `(offset, byteLength)` is out of bounds for `bytes`.
 */
export function decodeBigEndianInt32s(bytes: Uint8Array, offset: number, byteLength: number): Int32Array {
    const numCompleteInts = Math.floor(byteLength / 4);
    const hasTrailingBytes = byteLength % 4 !== 0;
    const numInts = hasTrailingBytes ? numCompleteInts + 1 : numCompleteInts;

    const ints = new Int32Array(numInts);
    decodeBigEndianInt32sInto(bytes, offset, byteLength, ints);
    return ints;
}
