/**
 * Byte/word conversion helpers shared by the FastPFOR encoder/decoder.
 * Placed in `decoding/` to keep dependencies one-way (encoding -> decoding).
 */

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
export function int32sToBigEndianBytes(values: Int32Array): Uint8Array {
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
export function bigEndianBytesToInt32s(bytes: Uint8Array, offset: number, byteLength: number): Int32Array {
    if (offset < 0 || byteLength < 0 || offset + byteLength > bytes.length) {
        throw new RangeError(
            `bigEndianBytesToInt32s: out of bounds (offset=${offset}, byteLength=${byteLength}, bytes.length=${bytes.length})`,
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
