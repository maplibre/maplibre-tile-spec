import BitVector from "../vector/flat/bitVector";
import IntWrapper from "../decoding/intWrapper";
import { compressFastPforInt32, int32sToBigEndianBytes } from "../fastPforCodec";

export function encodeVarintInt32Value(value: number, dst: Uint8Array, offset: IntWrapper): void {
    let v = value;
    while (v > 0x7f) {
        dst[offset.get()] = (v & 0x7f) | 0x80;
        offset.increment();
        v >>>= 7;
    }
    dst[offset.get()] = v & 0x7f;
    offset.increment();
}

export function encodeVarintInt32(values: Int32Array): Uint8Array {
    const buffer = new Uint8Array(values.length * 5);
    const offset = new IntWrapper(0);

    for (const value of values) {
        encodeVarintInt32Value(value, buffer, offset);
    }
    return buffer.slice(0, offset.get());
}

export function encodeVarintInt64(values: BigInt64Array): Uint8Array {
    const buffer = new Uint8Array(values.length * 10);
    const offset = new IntWrapper(0);

    for (const value of values) {
        encodeVarintInt64Value(value, buffer, offset);
    }
    return buffer.slice(0, offset.get());
}

function encodeVarintInt64Value(value: bigint, dst: Uint8Array, offset: IntWrapper): void {
    let v = value;
    while (v > 0x7fn) {
        dst[offset.get()] = Number(v & 0x7fn) | 0x80;
        offset.increment();
        v >>= 7n;
    }
    dst[offset.get()] = Number(v & 0x7fn);
    offset.increment();
}

export function encodeVarintFloat64(values: Float64Array): Uint8Array {
    // 1. Calculate the exact size required for the buffer
    let size = 0;
    for (let i = 0; i < values.length; i++) {
        let val = values[i];
        // Ensure we handle the value as a positive integer
        val = val < 0 ? 0 : Math.floor(val);

        // 0 always takes 1 byte
        if (val === 0) {
            size++;
            continue;
        }

        // Calculate bytes needed: ceil(log128(val + 1))
        while (val > 0) {
            size++;
            val = Math.floor(val / 128);
        }
    }

    const dst = new Uint8Array(size);
    const offset = new IntWrapper(0);

    for (let i = 0; i < values.length; i++) {
        encodeVarintFloat64Value(values[i], dst, offset);
    }

    return dst;
}

/**
 * Encodes a single number into the buffer at the given offset using Varint encoding.
 * Handles numbers up to 2^53 (MAX_SAFE_INTEGER) correctly.
 */
function encodeVarintFloat64Value(val: number, buf: Uint8Array, offset: IntWrapper): void {
    // Ensure integer
    val = Math.floor(val);

    // Handle 0 explicitly or ensure loop runs once
    if (val === 0) {
        buf[offset.get()] = 0;
        offset.increment();
        return;
    }

    while (val >= 128) {
        // Write 7 bits of data | 0x80 (continuation bit)
        buf[offset.get()] = (val % 128) | 0x80;
        offset.increment();
        // Shift right by 7 bits
        val = Math.floor(val / 128);
    }

    // Write the last byte (no continuation bit)
    buf[offset.get()] = val;
    offset.increment();
}

export function encodeFastPfor(data: Int32Array): Uint8Array {
    const compressed = compressFastPforInt32(data);
    return int32sToBigEndianBytes(compressed);
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
export function encodeZigZagInt32(data: Int32Array): void {
    for (let i = 0; i < data.length; i++) {
        data[i] = encodeZigZagInt32Value(data[i]);
    }
}

export function encodeZigZagInt64(data: BigInt64Array): void {
    for (let i = 0; i < data.length; i++) {
        data[i] = encodeZigZagInt64Value(data[i]);
    }
}

export function encodeZigZagFloat64(data: Float64Array): void {
    for (let i = 0; i < data.length; i++) {
        data[i] = encodeZigZagFloat64Value(data[i]);
    }
}

export function encodeUnsignedRleInt32(input: Int32Array): { data: Int32Array; runs: number } {
    if (input.length === 0) {
        return { data: new Int32Array(0), runs: 0 };
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

    return { data: encodedData, runs: numRuns };
}

export function encodeUnsignedRleInt64(input: BigInt64Array): { data: BigInt64Array; runs: number } {
    if (input.length === 0) {
        return { data: new BigInt64Array(0), runs: 0 };
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

    return { data: encodedData, runs: numRuns };
}

export function encodeUnsignedRleFloat64(input: Float64Array): { data: Float64Array; runs: number } {
    if (input.length === 0) {
        return { data: new Float64Array(0), runs: 0 };
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

    return { data: encodedData, runs: numRuns };
}

export function encodeZigZagDeltaInt32(data: Int32Array): void {
    if (data.length === 0) {
        return;
    }

    let previousValue = data[0];
    data[0] = encodeZigZagInt32Value(previousValue);

    for (let i = 1; i < data.length; i++) {
        const currentValue = data[i];
        const delta = currentValue - previousValue;
        const encodedDelta = encodeZigZagInt32Value(delta);

        // Store the encoded delta back into the array
        data[i] = encodedDelta;

        // Update the previous value tracker for the next iteration's delta calculation
        previousValue = currentValue;
    }
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

export function encodeZigZagDeltaFloat64(data: Float64Array): void {
    if (data.length === 0) {
        return;
    }

    let previousValue = data[0];
    data[0] = encodeZigZagFloat64Value(previousValue);

    for (let i = 1; i < data.length; i++) {
        const currentValue = data[i];
        const delta = currentValue - previousValue;
        const encodedDelta = encodeZigZagFloat64Value(delta);

        // Store the encoded delta back into the array
        data[i] = encodedDelta;

        // Update the previous value tracker for the next iteration's delta calculation
        previousValue = currentValue;
    }
}

export function encodeZigZagRleInt32(input: Int32Array): {
    data: Int32Array;
    runs: number;
    numTotalValues: number;
} {
    if (input.length === 0) {
        return { data: new Int32Array(0), runs: 0, numTotalValues: 0 };
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
        data: encodedData,
        runs: numRuns,
        numTotalValues: input.length, // Total original values count
    };
}

export function encodeZigZagRleInt64(input: BigInt64Array): {
    data: BigInt64Array;
    runs: number;
    numTotalValues: number;
} {
    if (input.length === 0) {
        return { data: new BigInt64Array(0), runs: 0, numTotalValues: 0 };
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
        data: encodedData,
        runs: numRuns,
        numTotalValues: input.length, // Total original values count
    };
}

export function encodeZigZagRleFloat64(input: Float64Array): {
    data: Float64Array;
    runs: number;
    numTotalValues: number;
} {
    if (input.length === 0) {
        return { data: new Float64Array(0), runs: 0, numTotalValues: 0 };
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
        data: encodedData,
        runs: numRuns,
        numTotalValues: input.length, // Total original values count
    };
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

// HM TODO:
// encodeComponentwiseDeltaVec2
// decodeComponentwiseDeltaVec2Scaled

export function encodeNullableZigZagDeltaInt32(inputData: Int32Array): {
    data: Int32Array;
    bitVector: BitVector;
    totalSize: number;
} {
    if (inputData.length === 0) {
        return { data: new Int32Array(0), bitVector: new BitVector(new Uint8Array(0), 0), totalSize: 0 };
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
        data: finalEncodedData,
        bitVector: bitVector,
        totalSize: totalSize,
    };
}

export function encodeNullableZigZagDeltaInt64(inputData: BigInt64Array): {
    data: BigInt64Array;
    bitVector: BitVector;
    totalSize: number;
} {
    if (inputData.length === 0) {
        return { data: new BigInt64Array(0), bitVector: new BitVector(new Uint8Array(0), 0), totalSize: 0 };
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
        data: finalEncodedData,
        bitVector: bitVector,
        totalSize: totalSize,
    };
}

// HM TODO:
// zigZagDeltaOfDeltaDecoding

export function encodeZigZagRleDeltaInt32(values: Int32Array | number[]): {
    data: Int32Array;
    runs: number;
    numTotalValues: number;
} {
    if (values.length === 0) {
        return { data: new Int32Array(0), runs: 0, numTotalValues: 0 };
    }

    const runLengths: number[] = [];
    const encodedDeltas: number[] = [];

    // The decoder explicitly sets decodedValues[0] = 0 and uses previousValue = 0.
    // Therefore, we initialize our 'previous' tracker to 0 to calculate the first delta correctly.
    let previousValue = 0;

    // Variables to track the current run
    let currentDelta: number | null = null;
    let currentRunLength = 0;

    for (let i = 0; i < values.length; i++) {
        const value = values[i];
        const delta = value - previousValue;
        previousValue = value;

        if (currentDelta === null) {
            // First element initialization
            currentDelta = delta;
            currentRunLength = 1;
        } else if (delta === currentDelta) {
            // Continuation of the current run
            currentRunLength++;
        } else {
            // The run has broken (delta changed)
            // 1. Push the length of the previous run
            runLengths.push(currentRunLength);
            // 2. ZigZag encode the previous delta and push it
            encodedDeltas.push(encodeZigZagInt32Value(currentDelta));

            // Start the new run
            currentDelta = delta;
            currentRunLength = 1;
        }
    }

    // Flush the final run remaining after the loop finishes
    if (currentDelta !== null) {
        runLengths.push(currentRunLength);
        encodedDeltas.push(encodeZigZagInt32Value(currentDelta));
    }

    const numRuns = runLengths.length;

    // The decoder expects 'data' to be: [RunLength 1, RunLength 2... | Value 1, Value 2...]
    // Size is numRuns * 2 (First half lengths, second half values)
    const data = new Int32Array(numRuns * 2);

    for (let i = 0; i < numRuns; i++) {
        data[i] = runLengths[i]; // First half: Run Lengths
        data[i + numRuns] = encodedDeltas[i]; // Second half: ZigZag Encoded Deltas
    }

    return {
        data: data,
        runs: numRuns,
        numTotalValues: values.length,
    };
}

export function encodeRleDeltaInt32(values: Int32Array | number[]): {
    data: Int32Array;
    runs: number;
    numTotalValues: number;
} {
    if (values.length === 0) {
        return { data: new Int32Array(0), runs: 0, numTotalValues: 0 };
    }

    const runLengths: number[] = [];
    const deltas: number[] = [];

    // The decoder logic relies on: decodedValues[0] = 0; previousValue = 0;
    // So the encoder must assume the sequence starts relative to 0.
    let previousValue = 0;

    // Track the current run of deltas
    let currentDelta: number | null = null;
    let currentRunLength = 0;

    for (let i = 0; i < values.length; i++) {
        const value = values[i];
        const delta = value - previousValue;
        previousValue = value;

        if (currentDelta === null) {
            // Initialize first run
            currentDelta = delta;
            currentRunLength = 1;
        } else if (delta === currentDelta) {
            // Continue current run
            currentRunLength++;
        } else {
            // Delta changed: flush the previous run
            runLengths.push(currentRunLength);
            deltas.push(currentDelta);

            // Start new run
            currentDelta = delta;
            currentRunLength = 1;
        }
    }

    // Flush the final run
    if (currentDelta !== null) {
        runLengths.push(currentRunLength);
        deltas.push(currentDelta);
    }

    const numRuns = runLengths.length;

    // Pack into Int32Array: [ RunLength 1...N | Delta 1...N ]
    const data = new Int32Array(numRuns * 2);
    for (let i = 0; i < numRuns; i++) {
        data[i] = runLengths[i];
        data[i + numRuns] = deltas[i];
    }

    return {
        data: data,
        runs: numRuns,
        numTotalValues: values.length,
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

export function encodeNullableZigZagRleInt32(
    values: Int32Array | number[],
    bitVector: BitVector,
): { data: Int32Array; runs: number } {
    const size = bitVector.size();
    if (size === 0) {
        return { data: new Int32Array(0), runs: 0 };
    }

    const runLengths: number[] = [];
    const encodedValues: number[] = [];

    let currentRunValue: number | null = null;
    let currentRunLength = 0;

    for (let i = 0; i < size; i++) {
        const isValid = bitVector.get(i);
        const val = values[i];

        if (!isValid) {
            // Null Encountered: Flush the current run if it exists, and reset run tracking.
            if (currentRunValue !== null) {
                runLengths.push(currentRunLength);
                encodedValues.push(encodeZigZagInt32Value(currentRunValue));
            }
            // Reset for the next run (since nulls cannot start a run)
            currentRunValue = null;
            currentRunLength = 0;
            continue;
        }

        // Valid Value Encountered
        if (currentRunValue === null) {
            // Start of a new run
            currentRunValue = val;
            currentRunLength = 1;
        } else if (val === currentRunValue) {
            // Continuation of the current run
            currentRunLength++;
        } else {
            // Value changed: Flush the previous run
            runLengths.push(currentRunLength);
            encodedValues.push(encodeZigZagInt32Value(currentRunValue));

            // Start new run
            currentRunValue = val;
            currentRunLength = 1;
        }
    }

    // Flush the final run
    if (currentRunValue !== null) {
        runLengths.push(currentRunLength);
        encodedValues.push(encodeZigZagInt32Value(currentRunValue));
    }

    const numRuns = runLengths.length;

    // Pack: [ RunLength 1...N | Value 1...N ]
    const data = new Int32Array(numRuns * 2);
    for (let i = 0; i < numRuns; i++) {
        data[i] = runLengths[i];
        data[i + numRuns] = encodedValues[i];
    }

    return {
        data,
        runs: numRuns,
    };
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

export function encodeNullableZigZagRleInt64(
    values: BigInt64Array,
    bitVector: BitVector,
): { data: BigInt64Array; numRuns: number } {
    const size = bitVector.size();
    // Temporary storage for runs
    const runs: { length: number; value: bigint }[] = [];

    let currentRunValue: bigint | null = null;
    let currentRunLength = 0;

    for (let i = 0; i < size; i++) {
        // We only care about non-null values for run grouping.
        // The decoder's logic automatically handles skipping nulls (offset++)
        // while applying the run value.
        if (bitVector.get(i)) {
            const val = values[i];

            if (currentRunLength === 0) {
                // Start of a new run
                currentRunValue = val;
                currentRunLength = 1;
            } else if (val === currentRunValue) {
                // Continue current run
                currentRunLength++;
            } else {
                // Value changed: Commit the current run and start a new one
                runs.push({ length: currentRunLength, value: currentRunValue });
                currentRunValue = val;
                currentRunLength = 1;
            }
        }
    }

    // Commit the final run if it exists
    if (currentRunLength > 0 && currentRunValue !== null) {
        runs.push({ length: currentRunLength, value: currentRunValue });
    }

    const numRuns = runs.length;
    // Allocate the data array: first half for lengths, second half for values
    const data = new BigInt64Array(numRuns * 2);

    for (let i = 0; i < numRuns; i++) {
        const run = runs[i];
        // 1. Store Run Length
        data[i] = BigInt(run.length);

        // 2. Store ZigZag Encoded Value
        data[i + numRuns] = encodeZigZagInt64Value(run.value);
    }

    return { data, numRuns };
}


export function encodeDeltaRleInt32(input: Int32Array): {
    data: Int32Array;
    runs: number;
    numValues: number;
} {
    if (input.length === 0) {
        return { data: new Int32Array(0), runs: 0, numValues: 0 };
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
        data: encodedData,
        runs: numRuns,
        numValues: input.length, // Total original values count
    };
}

export function encodeDeltaRleInt64(input: BigInt64Array): {
    data: BigInt64Array;
    runs: number;
    numValues: number;
} {
    if (input.length === 0) {
        return { data: new BigInt64Array(0), runs: 0, numValues: 0 };
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
        data: encodedData,
        runs: numRuns,
        numValues: input.length, // Total original values count
    };
}
