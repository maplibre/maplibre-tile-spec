/**
 * Encodes int32 words to big-endian bytes.
 */

/**
 * Serializes an `Int32Array` to a big-endian byte stream.
 *
 * @param values - Int32 words to serialize.
 * @returns Big-endian byte stream (`values.length * 4` bytes).
 */
export function encodeBigEndianInt32s(values: Int32Array): Uint8Array {
    const bytes = new Uint8Array(values.length * 4);
    for (let i = 0; i < values.length; i++) {
        const v = values[i];
        const base = i * 4;
        bytes[base] = (v >>> 24) & 0xff;
        bytes[base + 1] = (v >>> 16) & 0xff;
        bytes[base + 2] = (v >>> 8) & 0xff;
        bytes[base + 3] = v & 0xff;
    }
    return bytes;
}
