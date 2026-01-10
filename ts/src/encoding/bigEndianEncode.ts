import { IS_LE, bswap32 } from "../decoding/fastPforShared";

/**
 * Serializes an `Int32Array` to a big-endian byte stream.
 *
 * @param values - Int32 words to serialize.
 * @returns Big-endian byte stream (`values.length * 4` bytes).
 */
export function encodeBigEndianInt32s(values: Int32Array): Uint8Array {
    const bytes = new Uint8Array(values.length * 4);
    const u32 = new Uint32Array(bytes.buffer, bytes.byteOffset, values.length);

    if (IS_LE) {
        for (let i = 0; i < values.length; i++) {
            u32[i] = bswap32(values[i]);
        }
    } else {
        for (let i = 0; i < values.length; i++) {
            u32[i] = values[i] >>> 0;
        }
    }
    return bytes;
}
