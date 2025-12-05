import { encodeVarintInt32Array, encodeVarintInt64Array, encodeZigZag32, encodeZigZag64 } from "./encodingUtils";

/**
 * Encodes Int32 values with zigzag encoding and varint compression
 */
export function encodeInt32SignedNone(values: Int32Array): Uint8Array {
    const zigzagEncoded = new Int32Array(values.length);
    for (let i = 0; i < values.length; i++) {
        zigzagEncoded[i] = encodeZigZag32(values[i]);
    }
    return encodeVarintInt32Array(zigzagEncoded);
}

/**
 * Encodes Int32 values with delta encoding, zigzag, and varint
 */
export function encodeInt32SignedDelta(values: Int32Array): Uint8Array {
    const deltaEncoded = new Int32Array(values.length);
    deltaEncoded[0] = values[0];
    for (let i = 1; i < values.length; i++) {
        deltaEncoded[i] = values[i] - values[i - 1];
    }
    const zigzagEncoded = new Int32Array(deltaEncoded.length);
    for (let i = 0; i < deltaEncoded.length; i++) {
        zigzagEncoded[i] = encodeZigZag32(deltaEncoded[i]);
    }
    return encodeVarintInt32Array(zigzagEncoded);
}

/**
 * Encodes Int32 values with RLE, zigzag, and varint
 * @param runs - Array of [runLength, value] pairs
 */
export function encodeInt32SignedRle(runs: Array<[number, number]>): Uint8Array {
    const runLengths: number[] = [];
    const values: number[] = [];

    for (const [runLength, value] of runs) {
        runLengths.push(runLength);
        values.push(encodeZigZag32(value));
    }

    const rleValues = [...runLengths, ...values];
    return encodeVarintInt32Array(new Int32Array(rleValues));
}

export function encodeInt32ArrayToRle(values: Int32Array): { data: Uint8Array, runs: number } {
    const rleRuns: Array<[number, number]> = [];
    let currentValue = values[0];
    let currentCount = 1;

    for (let i = 1; i < values.length; i++) {
        if (values[i] === currentValue) {
            currentCount++;
        } else {
            rleRuns.push([currentCount, currentValue]);
            currentValue = values[i];
            currentCount = 1;
        }
    }
    rleRuns.push([currentCount, currentValue]);

    return {
        data: encodeInt32SignedRle(rleRuns),
        runs: rleRuns.length
    };
}

export function encodeFloat64ArrayToRle(values: Float64Array): { data: Float64Array, runs: number } {
    const rleRuns: Array<[number, number]> = [];
    let currentValue = values[0];
    let currentCount = 1;

    for (let i = 1; i < values.length; i++) {
        if (values[i] === currentValue) {
            currentCount++;
        } else {
            rleRuns.push([currentCount, currentValue]);
            currentValue = values[i];
            currentCount = 1;
        }
    }
    rleRuns.push([currentCount, currentValue]);

    // Flatten to [count1, count2, value1, value2, ...]
    const data = new Float64Array(rleRuns.length * 2);
    for (let i = 0; i < rleRuns.length; i++) {
        data[i] = rleRuns[i][0]; // count
        data[rleRuns.length + i] = rleRuns[i][1]; // value
    }

    return {
        data: data,
        runs: rleRuns.length
    };
}

/**
 * Encodes Int32 values with MORTON encoding (delta without zigzag)
 */
export function encodeInt32Morton(values: Int32Array): Uint8Array {
    const deltaEncoded = new Int32Array(values.length);
    deltaEncoded[0] = values[0];
    for (let i = 1; i < values.length; i++) {
        deltaEncoded[i] = values[i] - values[i - 1];
    }
    return encodeVarintInt32Array(deltaEncoded);
}

/**
 * Encodes BigInt64 values with zigzag encoding and varint compression
 */
export function encodeInt64SignedNone(values: BigInt64Array): Uint8Array {
    const zigzagEncoded = new BigInt64Array(Array.from(values, (val) => encodeZigZag64(val)));
    return encodeVarintInt64Array(zigzagEncoded);
}

/**
 * Encodes BigInt64 values with delta encoding, zigzag, and varint
 */
export function encodeInt64SignedDelta(values: BigInt64Array): Uint8Array {
    const deltaEncoded = new BigInt64Array(values.length);
    deltaEncoded[0] = values[0];
    for (let i = 1; i < values.length; i++) {
        deltaEncoded[i] = values[i] - values[i - 1];
    }
    const zigzagEncoded = new BigInt64Array(deltaEncoded.length);
    for (let i = 0; i < deltaEncoded.length; i++) {
        zigzagEncoded[i] = encodeZigZag64(deltaEncoded[i]);
    }
    return encodeVarintInt64Array(zigzagEncoded);
}

/**
 * Encodes BigInt64 values with RLE, zigzag, and varint
 * @param runs - Array of [runLength, value] pairs
 */
export function encodeInt64SignedRle(runs: Array<[number, bigint]>): Uint8Array {
    const runLengths: bigint[] = [];
    const values: bigint[] = [];

    for (const [runLength, value] of runs) {
        runLengths.push(BigInt(runLength));
        values.push(encodeZigZag64(value));
    }

    const rleValues = [...runLengths, ...values];
    return encodeVarintInt64Array(new BigInt64Array(rleValues));
}

/**
 * Encodes BigInt64 values with delta+RLE, zigzag, and varint
 * @param runs - Array of [runLength, deltaValue] pairs representing RLE-encoded delta values
 */
export function encodeInt64SignedDeltaRle(runs: Array<[number, bigint]>): Uint8Array {
    const runLengths: bigint[] = [];
    const values: bigint[] = [];

    for (const [runLength, value] of runs) {
        runLengths.push(BigInt(runLength));
        values.push(encodeZigZag64(value));
    }

    const rleValues = [...runLengths, ...values];
    return encodeVarintInt64Array(new BigInt64Array(rleValues));
}

/**
 * Encodes unsigned BigInt64 values with varint compression (no zigzag)
 */
export function encodeInt64UnsignedNone(values: BigInt64Array): Uint8Array {
    return encodeVarintInt64Array(values);
}
