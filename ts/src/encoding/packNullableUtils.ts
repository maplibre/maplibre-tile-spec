import { type TypedArrayConstructor, type TypedArrayInstance } from "../decoding/unpackNullableUtils";
import BitVector from "../vector/flat/bitVector";

export function packNullable<T extends TypedArrayInstance>(data: T, presentBits: BitVector | null): T {
    // Non-nullable case: if no mask is provided, the data is already "packed"
    if (!presentBits) {
        return data;
    }

    const size = data.length;

    // 1. First pass: Count how many elements are actually present
    // This is required to allocate the correct size for the TypedArray
    let packedCount = 0;
    for (let i = 0; i < size; i++) {
        if (presentBits.get(i)) {
            packedCount++;
        }
    }

    // 2. Create a new array of the same type with the reduced size
    const constructor = data.constructor as TypedArrayConstructor;
    const result = new constructor(packedCount) as T;

    // 3. Second pass: Fill the result array with valid values
    let counter = 0;
    for (let i = 0; i < size; i++) {
        if (presentBits.get(i)) {
            result[counter++] = data[i];
        }
    }

    return result;
}

export function packNullableBoolean(data: Uint8Array, dataSize: number, presentBits: BitVector | null): Uint8Array {
    // Non-nullable case: if no mask is provided, the data is already "packed"
    if (!presentBits) {
        return data;
    }

    const inputBitVector = new BitVector(data, dataSize);

    // 1. Calculate how many bits are actually marked as 'present'
    // This determines the size of the final packed buffer.
    let packedCount = 0;
    for (let i = 0; i < dataSize; i++) {
        if (presentBits.get(i)) {
            packedCount++;
        }
    }

    // 2. Initialize the result BitVector with the correct compressed size
    const resultBuffer = new Uint8Array(Math.ceil(packedCount / 8));
    const resultBitVector = new BitVector(resultBuffer, packedCount);

    // 3. Fill the result: only copy bits where the mask is true
    let targetIndex = 0;
    for (let i = 0; i < dataSize; i++) {
        if (presentBits.get(i)) {
            const value = inputBitVector.get(i);
            resultBitVector.set(targetIndex++, value);
        }
    }

    return resultBitVector.getBuffer();
}
