const IS_LE = new Uint8Array(new Uint32Array([0x11223344]).buffer)[0] === 0x44;

function bswap32(value: number): number {
    const x = value >>> 0;
    return (((x & 0xff) << 24) | ((x & 0xff00) << 8) | ((x >>> 8) & 0xff00) | ((x >>> 24) & 0xff)) >>> 0;
}

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
