import varint from "varint";
import BitVector from "../vector/flat/bitVector";

export function encodeVarintInt32(value: number): Uint8Array {
    return varint.encode(value);
}

export function encodeVarintInt64(value: bigint): Uint8Array {
    const result: number[] = [];
    let num = value;
    while (num > 0n) {
        let byte = Number(num & 0x7fn);
        num >>= 7n;
        if (num > 0n) byte |= 0x80;
        result.push(byte);
    }
    return new Uint8Array(result.length > 0 ? result : [0]);
}

export function encodeZigZagInt32Value(value: number): number {
    return (value << 1) ^ (value >> 31);
}

export function encodeZigZagInt64Value(value: bigint): bigint {
    return (value << 1n) ^ (value >> 63n);
}

export function encodeZigZagFloat64Value(n: number): number {
    return n >= 0 ? n * 2 : n * -2 - 1;
}
export function encodeZigZagInt32Array(data: Int32Array): void {
    for (let i = 0; i < data.length; i++) {
        data[i] = encodeZigZagInt32Value(data[i]);
    }
}

export function encodeZigZagInt64Array(data: BigInt64Array): void {
    for (let i = 0; i < data.length; i++) {
        data[i] = encodeZigZagInt64Value(data[i]);
    }
}

export function encodeZigZagFloat64Array(data: Float64Array): void {
    for (let i = 0; i < data.length; i++) {
        data[i] = encodeZigZagFloat64Value(data[i]);
    }
}

export function encodeUnsignedRleInt32(input: Int32Array): Int32Array {
    if (input.length === 0) {
        return new Int32Array(0);
    }

    const runLengths: number[] = [];
    const runValues: number[] = [];

    let currentRunLength = 0;
    let currentValue = input[0];

    for (let i = 0; i < input.length; i++) {
        const nextValue = input[i];

        if (nextValue === currentValue) {
            currentRunLength++;
        } else {
            // End of the current run, record it
            runLengths.push(currentRunLength);
            runValues.push(currentValue);

            // Start a new run
            currentValue = nextValue;
            currentRunLength = 1;
        }
    }

    // Record the final run after the loop finishes
    runLengths.push(currentRunLength);
    runValues.push(currentValue);

    // Combine lengths and values into the final structured output array
    const numRuns = runLengths.length;
    const encodedData = new Int32Array(numRuns * 2);

    // Populate the first half with lengths
    encodedData.set(runLengths, 0);

    // Populate the second half with values, offset by the total number of runs
    encodedData.set(runValues, numRuns);

    return encodedData;
}

export function encodeUnsignedRleInt64(input: BigInt64Array): BigInt64Array {
    if (input.length === 0) {
        return new BigInt64Array(0);
    }

    const runLengths: number[] = [];
    const runValues: bigint[] = [];

    let currentRunLength = 0;
    let currentValue: bigint = input[0];

    for (let i = 0; i < input.length; i++) {
        const nextValue = input[i];

        if (nextValue === currentValue) {
            currentRunLength++;
        } else {
            // End of the current run, record it
            runLengths.push(currentRunLength);
            runValues.push(currentValue);

            // Start a new run
            currentValue = nextValue;
            currentRunLength = 1;
        }
    }

    // Record the final run after the loop finishes
    runLengths.push(currentRunLength);
    runValues.push(currentValue);

    // Combine lengths and values into the final structured output array (BigInt64Array)
    const numRuns = runLengths.length;
    const encodedData = new BigInt64Array(numRuns * 2);

    // Populate the first half with lengths. We must convert the numbers back to BigInts here.
    for (let i = 0; i < numRuns; i++) {
        encodedData[i] = BigInt(runLengths[i]);
    }

    // Populate the second half with values, offset by the total number of runs
    encodedData.set(runValues, numRuns);

    return encodedData;
}

export function encodeUnsignedRleFloat64(input: Float64Array): Float64Array {
    if (input.length === 0) {
        return new Float64Array(0);
    }

    const runLengths: number[] = [];
    const runValues: number[] = [];

    let currentRunLength = 0;
    let currentValue = input[0];

    for (let i = 0; i < input.length; i++) {
        const nextValue = input[i];

        if (nextValue === currentValue) {
            currentRunLength++;
        } else {
            // End of the current run, record it
            runLengths.push(currentRunLength);
            runValues.push(currentValue);

            // Start a new run
            currentValue = nextValue;
            currentRunLength = 1;
        }
    }

    // Record the final run after the loop finishes
    runLengths.push(currentRunLength);
    runValues.push(currentValue);

    // Combine lengths and values into the final structured output array (Float64Array)
    const numRuns = runLengths.length;
    // The final array is twice the size of the number of runs
    const encodedData = new Float64Array(numRuns * 2);

    // Populate the first half with lengths
    encodedData.set(runLengths, 0);

    // Populate the second half with values, offset by the total number of runs
    encodedData.set(runValues, numRuns);

    return encodedData;
}

export function encodeZigZagDeltaInt64(data: BigInt64Array): void {
    if (data.length === 0) {
        return;
    }

    let previousValue = data[0];
    data[0] = encodeZigZagInt64Value(previousValue);

    for (let i = 1; i < data.length; i++) {
        const currentValue = data[i];
        const delta = currentValue - previousValue;
        const encodedDelta = encodeZigZagInt64Value(delta);

        // Store the encoded delta back into the array
        data[i] = encodedDelta;

        // Update the previous value tracker for the next iteration's delta calculation
        previousValue = currentValue;
    }
}

/**
 * This is not really a encode, but more of a decode method...
 */
export function encodeDeltaInt32(data: Int32Array): void {
    if (data.length === 0) {
        return;
    }
    for (let i = data.length - 1; i >= 1; i--) {
        data[i] = data[i] - data[i - 1];
    }
}

export function encodeNullableZigZagDeltaInt32(inputData: Int32Array): {
    encodedData: Int32Array;
    bitVector: BitVector;
    totalSize: number;
} {
    if (inputData.length === 0) {
        return { encodedData: new Int32Array(0), bitVector: new BitVector(new Uint8Array(0), 0), totalSize: 0 };
    }

    const totalSize = inputData.length;
    const bitVector = new BitVector(new Uint8Array(totalSize), totalSize);
    // We will collect the non-null/non-zero deltas here first
    const encodedDeltas: number[] = [];

    let previousValue = 0; // The base for the first delta is implicitly zero

    for (let i = 0; i < totalSize; i++) {
        const currentValue = inputData[i];

        // Calculate the delta (difference from the previous value)
        const delta = currentValue - previousValue;

        // If the delta is non-zero, we store it.
        if (delta !== 0) {
            // Mark presence in the bit vector
            bitVector.set(i, true);

            // Zigzag encode the non-zero delta
            // Formula: (delta << 1) ^ (delta >> 31)
            const zigzagEncoded = ((delta << 1) ^ (delta >> 31)) >>> 0;

            encodedDeltas.push(zigzagEncoded);
        }
        // If delta is 0, we don't store anything in encodedDeltas, and the bit is left false.

        // Update the previous value tracker for the next iteration's delta calculation
        previousValue = currentValue;
    }

    // Convert the temporary list of deltas into a final Int32Array for storage
    const finalEncodedData = new Int32Array(encodedDeltas);

    return {
        encodedData: finalEncodedData,
        bitVector: bitVector,
        totalSize: totalSize,
    };
}

export function encodeNullableZigZagDeltaInt64(inputData: BigInt64Array): {
    encodedData: BigInt64Array;
    bitVector: BitVector;
    totalSize: number;
} {
    if (inputData.length === 0) {
        return { encodedData: new BigInt64Array(0), bitVector: new BitVector(new Uint8Array(0), 0), totalSize: 0 };
    }

    const totalSize = inputData.length;
    const bitVector = new BitVector(new Uint8Array(totalSize), totalSize);
    // We will collect the non-null/non-zero deltas here first
    const encodedDeltas: bigint[] = [];

    // The base for the first delta is implicitly zero
    let previousValue: bigint = 0n;

    for (let i = 0; i < totalSize; i++) {
        const currentValue = inputData[i];

        // Calculate the delta (difference from the previous value)
        const delta = currentValue - previousValue;

        // If the delta is non-zero, we store it.
        if (delta !== 0n) {
            // Mark presence in the bit vector
            bitVector.set(i, true);

            // Zigzag encode the non-zero delta using 64-bit logic
            // Formula: (delta << 1n) ^ (delta >> 63n)
            const zigzagEncoded = (delta << 1n) ^ (delta >> 63n);

            encodedDeltas.push(zigzagEncoded);
        }
        // If delta is 0n, we don't store anything in encodedDeltas, and the bit is left false.

        // Update the previous value tracker for the next iteration's delta calculation
        previousValue = currentValue;
    }

    // Convert the temporary list of deltas into a final BigInt64Array for storage
    const finalEncodedData = new BigInt64Array(encodedDeltas);

    return {
        encodedData: finalEncodedData,
        bitVector: bitVector,
        totalSize: totalSize,
    };
}

export function encodeDeltaRleInt32(input: Int32Array): {
    encodedData: Int32Array;
    numRuns: number;
    numValues: number;
} {
    if (input.length === 0) {
        return { encodedData: new Int32Array(0), numRuns: 0, numValues: 0 };
    }

    const deltasAndEncoded: number[] = [];
    let previousValue: number = 0;

    // Step 1 & 2: Calculate Deltas and Zigzag Encode them
    for (let i = 0; i < input.length; i++) {
        const currentValue = input[i];
        const delta = currentValue - previousValue;
        const encodedDelta = encodeZigZagInt32Value(delta);
        deltasAndEncoded.push(encodedDelta);
        previousValue = currentValue;
    }
    // deltasAndEncoded now holds the intermediate stream of zigzagged deltas

    // Step 3: Apply RLE to the stream of zigzag-encoded deltas
    const runLengths: number[] = [];
    const runZigZagDeltas: number[] = [];

    let currentRunLength = 0;
    let currentRunValue = deltasAndEncoded[0];

    for (let i = 0; i < deltasAndEncoded.length; i++) {
        const nextValue = deltasAndEncoded[i];

        if (nextValue === currentRunValue) {
            currentRunLength++;
        } else {
            runLengths.push(currentRunLength);
            runZigZagDeltas.push(currentRunValue);
            currentRunValue = nextValue;
            currentRunLength = 1;
        }
    }
    // Record the final run
    runLengths.push(currentRunLength);
    runZigZagDeltas.push(currentRunValue);

    // Step 4: Combine lengths and values into the final structured output array
    const numRuns = runLengths.length;
    const encodedData = new Int32Array(numRuns * 2);

    // Populate the first half with lengths
    for (let i = 0; i < numRuns; i++) {
        encodedData[i] = runLengths[i];
    }

    // Populate the second half with zigzagged deltas
    // Int32Array.set() works with standard number arrays
    encodedData.set(runZigZagDeltas, numRuns);

    return {
        encodedData: encodedData,
        numRuns: numRuns,
        numValues: input.length, // Total original values count
    };
}

export function encodeDeltaRleInt64(input: BigInt64Array): {
    encodedData: BigInt64Array;
    numRuns: number;
    numValues: number;
} {
    if (input.length === 0) {
        return { encodedData: new BigInt64Array(0), numRuns: 0, numValues: 0 };
    }

    const deltasAndEncoded: bigint[] = [];
    let previousValue: bigint = 0n;

    // Step 1 & 2: Calculate Deltas and Zigzag Encode them
    for (let i = 0; i < input.length; i++) {
        const currentValue = input[i];
        const delta = currentValue - previousValue;
        const encodedDelta = encodeZigZagInt64Value(delta);
        deltasAndEncoded.push(encodedDelta);
        previousValue = currentValue;
    }
    // deltasAndEncoded now holds the intermediate stream of zigzagged deltas

    // Step 3: Apply RLE to the stream of zigzag-encoded deltas
    const runLengths: number[] = [];
    const runZigZagDeltas: bigint[] = [];

    let currentRunLength = 0;
    let currentValue = deltasAndEncoded[0];

    for (let i = 0; i < deltasAndEncoded.length; i++) {
        const nextValue = deltasAndEncoded[i];

        if (nextValue === currentValue) {
            currentRunLength++;
        } else {
            runLengths.push(currentRunLength);
            runZigZagDeltas.push(currentValue);
            currentValue = nextValue;
            currentRunLength = 1;
        }
    }
    // Record the final run
    runLengths.push(currentRunLength);
    runZigZagDeltas.push(currentValue);

    // Step 4: Combine lengths and values into the final structured output array
    const numRuns = runLengths.length;
    const encodedData = new BigInt64Array(numRuns * 2);

    // Populate the first half with lengths (converting numbers back to BigInts for storage)
    for (let i = 0; i < numRuns; i++) {
        encodedData[i] = BigInt(runLengths[i]);
    }

    // Populate the second half with zigzagged deltas
    encodedData.set(runZigZagDeltas, numRuns);

    return {
        encodedData: encodedData,
        numRuns: numRuns,
        numValues: input.length, // Total original values count
    };
}

export function encodeZigZagRleInt32(input: Int32Array): {
    encodedData: Int32Array;
    numRuns: number;
    numTotalValues: number;
} {
    if (input.length === 0) {
        return { encodedData: new Int32Array(0), numRuns: 0, numTotalValues: 0 };
    }

    const zigzagEncodedStream: number[] = [];

    // Step 1: Apply Zigzag Encoding to all values
    for (let i = 0; i < input.length; i++) {
        zigzagEncodedStream.push(encodeZigZagInt32Value(input[i]));
    }
    // zigzagEncodedStream now holds the intermediate stream of zigzag values

    // Step 2: Apply RLE to the stream of zigzag-encoded values
    const runLengths: number[] = [];
    const runZigZagValues: number[] = [];

    let currentRunLength = 0;
    let currentValue = zigzagEncodedStream[0];

    for (let i = 0; i < zigzagEncodedStream.length; i++) {
        const nextValue = zigzagEncodedStream[i];

        if (nextValue === currentValue) {
            currentRunLength++;
        } else {
            runLengths.push(currentRunLength);
            runZigZagValues.push(currentValue);
            currentValue = nextValue;
            currentRunLength = 1;
        }
    }
    // Record the final run
    runLengths.push(currentRunLength);
    runZigZagValues.push(currentValue);

    // Step 3: Combine lengths and values into the final structured output array
    const numRuns = runLengths.length;
    // The final array uses Int32Array for lengths AND values
    const encodedData = new Int32Array(numRuns * 2);

    // Populate the first half with lengths
    encodedData.set(runLengths, 0);

    // Populate the second half with zigzagged values
    encodedData.set(runZigZagValues, numRuns);

    return {
        encodedData: encodedData,
        numRuns: numRuns,
        numTotalValues: input.length, // Total original values count
    };
}

export function encodeZigZagRleInt64(input: BigInt64Array): {
    encodedData: BigInt64Array;
    numRuns: number;
    numTotalValues: number;
} {
    if (input.length === 0) {
        return { encodedData: new BigInt64Array(0), numRuns: 0, numTotalValues: 0 };
    }

    const zigzagEncodedStream: bigint[] = [];

    // Step 1: Apply Zigzag Encoding to all values
    for (let i = 0; i < input.length; i++) {
        zigzagEncodedStream.push(encodeZigZagInt64Value(input[i]));
    }
    // zigzagEncodedStream now holds the intermediate stream of zigzag values

    // Step 2: Apply RLE to the stream of zigzag-encoded values
    const runLengths: number[] = [];
    const runZigZagValues: bigint[] = [];

    let currentRunLength = 0;
    let currentValue: bigint = zigzagEncodedStream[0];

    for (let i = 0; i < zigzagEncodedStream.length; i++) {
        const nextValue = zigzagEncodedStream[i];

        if (nextValue === currentValue) {
            currentRunLength++;
        } else {
            runLengths.push(currentRunLength);
            runZigZagValues.push(currentValue);
            currentValue = nextValue;
            currentRunLength = 1;
        }
    }
    // Record the final run
    runLengths.push(currentRunLength);
    runZigZagValues.push(currentValue);

    // Step 3: Combine lengths and values into the final structured output array
    const numRuns = runLengths.length;
    // The final array uses BigInt64Array for lengths AND values
    const encodedData = new BigInt64Array(numRuns * 2);

    // Populate the first half with lengths (converting numbers back to BigInts)
    for (let i = 0; i < numRuns; i++) {
        encodedData[i] = BigInt(runLengths[i]);
    }

    // Populate the second half with zigzagged values
    encodedData.set(runZigZagValues, numRuns);

    return {
        encodedData: encodedData,
        numRuns: numRuns,
        numTotalValues: input.length, // Total original values count
    };
}

export function encodeZigZagRleFloat64(input: Float64Array): {
    encodedData: Float64Array;
    numRuns: number;
    numTotalValues: number;
} {
    if (input.length === 0) {
        return { encodedData: new Float64Array(0), numRuns: 0, numTotalValues: 0 };
    }

    const zigzagEncodedStream: number[] = [];

    // Step 1: Apply Float-based Zigzag Encoding to all values
    for (let i = 0; i < input.length; i++) {
        zigzagEncodedStream.push(encodeZigZagFloat64Value(input[i]));
    }
    // zigzagEncodedStream now holds the intermediate stream of zigzag values (as floats acting as integers)

    // Step 2: Apply RLE to the stream of zigzag-encoded values
    const runLengths: number[] = [];
    const runZigZagValues: number[] = [];

    let currentRunLength = 0;
    let currentValue = zigzagEncodedStream[0];

    for (let i = 0; i < zigzagEncodedStream.length; i++) {
        const nextValue = zigzagEncodedStream[i];

        if (nextValue === currentValue) {
            currentRunLength++;
        } else {
            runLengths.push(currentRunLength);
            runZigZagValues.push(currentValue);
            currentValue = nextValue;
            currentRunLength = 1;
        }
    }
    // Record the final run
    runLengths.push(currentRunLength);
    runZigZagValues.push(currentValue);

    // Step 3: Combine lengths and values into the final structured output array
    const numRuns = runLengths.length;
    // The final array uses Float64Array for lengths AND values
    const encodedData = new Float64Array(numRuns * 2);

    // Populate the first half with lengths
    encodedData.set(runLengths, 0);

    // Populate the second half with zigzagged values
    encodedData.set(runZigZagValues, numRuns);

    return {
        encodedData: encodedData,
        numRuns: numRuns,
        numTotalValues: input.length, // Total original values count
    };
}

export function encodeNullableUnsignedRleInt32(
    values: Int32Array,
    bitVector: BitVector,
): { data: Int32Array; numRuns: number } {
    const lengths: number[] = [];
    const runValues: number[] = [];
    let i = 0;
    const size = values.length;
    while (i < size) {
        let searchIndex = i;
        let validValue: number | null = null;
        while (searchIndex < size) {
            if (bitVector.get(searchIndex)) {
                validValue = values[searchIndex];
                break;
            }
            searchIndex++;
        }
        if (validValue === null) break;
        let currentRunValidCount = 0;
        let scanIndex = i;
        while (scanIndex < size) {
            const isSet = bitVector.get(scanIndex);
            if (!isSet) {
                scanIndex++;
            } else {
                const val = values[scanIndex];
                if (val === validValue) {
                    currentRunValidCount++;
                    scanIndex++;
                } else {
                    break;
                }
            }
        }
        lengths.push(currentRunValidCount);
        runValues.push(validValue);
        i = scanIndex;
    }
    const numRuns = lengths.length;
    const resultData = new Int32Array(numRuns * 2);
    for (let k = 0; k < numRuns; k++) {
        resultData[k] = lengths[k];
        resultData[k + numRuns] = runValues[k];
    }
    return { data: resultData, numRuns };
}

export function encodeNullableUnsignedRleInt64(
    values: BigInt64Array,
    bitVector: BitVector,
): { data: BigInt64Array; numRuns: number } {
    const lengths: number[] = [];
    const runValues: bigint[] = [];
    let i = 0;
    const size = values.length;
    while (i < size) {
        let searchIndex = i;
        let validValue: bigint | null = null;
        while (searchIndex < size) {
            if (bitVector.get(searchIndex)) {
                validValue = values[searchIndex];
                break;
            }
            searchIndex++;
        }
        if (validValue === null) break;
        let currentRunValidCount = 0;
        let scanIndex = i;
        while (scanIndex < size) {
            const isSet = bitVector.get(scanIndex);
            if (!isSet) {
                scanIndex++;
            } else {
                const val = values[scanIndex];
                if (val === validValue) {
                    currentRunValidCount++;
                    scanIndex++;
                } else {
                    break;
                }
            }
        }
        lengths.push(currentRunValidCount);
        runValues.push(validValue);
        i = scanIndex;
    }
    const numRuns = lengths.length;
    const resultData = new BigInt64Array(numRuns * 2);
    for (let k = 0; k < numRuns; k++) {
        resultData[k] = BigInt(lengths[k]);
        resultData[k + numRuns] = runValues[k];
    }
    return { data: resultData, numRuns };
}
