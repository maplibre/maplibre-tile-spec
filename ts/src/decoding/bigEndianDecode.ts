/**
 * Decodes big-endian bytes to int32 words.
 */

function bswap32(value: number): number {
    const x = value >>> 0;
    return (((x & 0xff) << 24) | ((x & 0xff00) << 8) | ((x >>> 8) & 0xff00) | ((x >>> 24) & 0xff)) >>> 0;
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
    if (offset < 0 || byteLength < 0 || offset + byteLength > bytes.length) {
        throw new RangeError(
            `decodeBigEndianInt32s: out of bounds (offset=${offset}, byteLength=${byteLength}, bytes.length=${bytes.length})`,
        );
    }

    const numCompleteInts = Math.floor(byteLength / 4);
    const hasTrailingBytes = byteLength % 4 !== 0;
    const numInts = hasTrailingBytes ? numCompleteInts + 1 : numCompleteInts;

    const ints = new Int32Array(numInts);
    if (numCompleteInts > 0) {
        const absoluteOffset = bytes.byteOffset + offset;
        if ((absoluteOffset & 3) === 0) {
            const u32 = new Uint32Array(bytes.buffer, absoluteOffset, numCompleteInts);
            for (let i = 0; i < numCompleteInts; i++) {
                ints[i] = bswap32(u32[i]) | 0;
            }
        } else {
            for (let i = 0; i < numCompleteInts; i++) {
                const base = offset + i * 4;
                ints[i] = (bytes[base] << 24) | (bytes[base + 1] << 16) | (bytes[base + 2] << 8) | bytes[base + 3] | 0;
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
        ints[numCompleteInts] = v | 0;
    }
    return ints;
}
