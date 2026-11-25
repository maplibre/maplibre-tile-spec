import BitVector from "../vector/flat/bitVector.js";

/**
 * Type constraint for TypedArray types that can be unpacked
 */
export type TypedArrayConstructor =
    | Int32ArrayConstructor
    | Uint32ArrayConstructor
    | BigInt64ArrayConstructor
    | BigUint64ArrayConstructor
    | Float32ArrayConstructor
    | Float64ArrayConstructor;

export type TypedArrayInstance =
    | Int32Array
    | Uint32Array
    | BigInt64Array
    | BigUint64Array
    | Float32Array
    | Float64Array;

/**
 * Reconstructs the full array of a nullable column, placing each value of the compact data stream at the position of
 * its presence bit and the default value at the positions that have none.
 *
 * The presence bits are read a byte at a time, so that eight absent positions cost one comparison and a byte that
 * holds values only visits its set bits.
 *
 * @param dataStream The compact data stream containing only non-null values
 * @param presentBits BitVector indicating which positions have values (null if non-nullable)
 * @param defaultValue The default value to insert at null positions (0, 0n, etc.)
 * @returns Full array with default values at null positions
 */
export function unpackNullable<T extends TypedArrayInstance>(
    dataStream: T,
    presentBits: BitVector | null,
    defaultValue: number | bigint,
): T {
    if (!presentBits) {
        return dataStream;
    }

    const size = presentBits.size();
    const constructor = dataStream.constructor as TypedArrayConstructor;
    const result = new constructor(size) as T;
    if (defaultValue !== 0 && defaultValue !== 0n) {
        result.fill(defaultValue as never);
    }

    const presence = presentBits.getBuffer();
    const numBytes = (size + 7) >> 3;
    const lastByteMask = 0xff >> ((numBytes << 3) - size);
    let counter = 0;
    for (let byteIndex = 0; byteIndex < numBytes; byteIndex++) {
        const position = byteIndex << 3;
        let bits = presence[byteIndex] & (byteIndex === numBytes - 1 ? lastByteMask : 0xff);
        while (bits !== 0) {
            const bit = 31 - Math.clz32(bits & -bits);
            result[position + bit] = dataStream[counter++];
            bits &= bits - 1;
        }
    }

    return result;
}

/**
 * Special case for boolean columns because BitVector is not directly compatible with TypedArray.
 *
 * @param dataStream The compact BitVector data containing only non-null boolean values
 * @param dataStreamSize The number of actual values in dataStream
 * @param presentBits BitVector indicating which positions have values (null if non-nullable)
 * @returns Uint8Array buffer for BitVector with false at null positions
 */
export function unpackNullableBoolean(
    dataStream: Uint8Array,
    dataStreamSize: number,
    presentBits: BitVector | null,
): Uint8Array {
    // Non-nullable case
    if (!presentBits) {
        return dataStream;
    }

    const numFeatures = presentBits.size();
    const bitVector = new BitVector(dataStream, dataStreamSize);
    const result = new BitVector(new Uint8Array(Math.ceil(numFeatures / 8)), numFeatures);

    let counter = 0;
    for (let i = 0; i < numFeatures; i++) {
        // If position has a value, take from data stream; otherwise use false
        const value = presentBits.get(i) ? bitVector.get(counter++) : false;
        result.set(i, value);
    }

    return result.getBuffer();
}
