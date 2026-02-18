import type IntWrapper from "./intWrapper";
import type BitVector from "../vector/flat/bitVector";
import {
    createFastPforWireDecodeWorkspace,
    decodeFastPforInt32,
    ensureFastPforWireEncodedWordsCapacity,
    type FastPforWireDecodeWorkspace,
} from "./fastPforDecoder";
import { decodeBigEndianInt32sInto } from "./bigEndianDecode";
export type { FastPforWireDecodeWorkspace } from "./fastPforDecoder";
export { createFastPforWireDecodeWorkspace } from "./fastPforDecoder";

//based on https://github.com/mapbox/pbf/blob/main/index.js
export function decodeVarintInt32(buf: Uint8Array, bufferOffset: IntWrapper, numValues: number): Int32Array {
    const dst = new Int32Array(numValues);
    let dstOffset = 0;
    let offset = bufferOffset.get();
    for (let i = 0; i < dst.length; i++) {
        let b = buf[offset++];
        let val = b & 0x7f;
        if (b < 0x80) {
            dst[dstOffset++] = val;
            continue;
        }

        b = buf[offset++];
        val |= (b & 0x7f) << 7;
        if (b < 0x80) {
            dst[dstOffset++] = val;
            continue;
        }

        b = buf[offset++];
        val |= (b & 0x7f) << 14;
        if (b < 0x80) {
            dst[dstOffset++] = val;
            continue;
        }

        b = buf[offset++];
        val |= (b & 0x7f) << 21;
        if (b < 0x80) {
            dst[dstOffset++] = val;
            continue;
        }

        b = buf[offset++];
        val |= (b & 0x0f) << 28;
        dst[dstOffset++] = val;
    }

    bufferOffset.set(offset);
    return dst;
}

export function decodeVarintInt64(src: Uint8Array, offset: IntWrapper, numValues: number): BigInt64Array {
    const dst = new BigInt64Array(numValues);
    for (let i = 0; i < dst.length; i++) {
        dst[i] = decodeVarintInt64Value(src, offset);
    }
    return dst;
}

// Source: https://github.com/bazelbuild/bazel/blob/master/src/main/java/com/google/devtools/build/lib/util/VarInt.java
function decodeVarintInt64Value(bytes: Uint8Array, pos: IntWrapper): bigint {
    let value = 0n;
    let shift = 0;
    let index = pos.get();
    while (index < bytes.length) {
        const b = bytes[index++];
        value |= BigInt(b & 0x7f) << BigInt(shift);
        if ((b & 0x80) === 0) {
            break;
        }
        shift += 7;
        if (shift >= 64) {
            throw new Error("Varint too long");
        }
    }
    pos.set(index);
    return value;
}

/*
 * Since decoding Int64 values to BigInt is more than an order of magnitude slower in the tests then using a Float64,
 * this decoding method limits the max size of a Long value to 53 bits
 */
export function decodeVarintFloat64(src: Uint8Array, offset: IntWrapper, numValues: number): Float64Array {
    const dst = new Float64Array(numValues);
    for (let i = 0; i < numValues; i++) {
        dst[i] = decodeVarintFloat64Value(src, offset);
    }
    return dst;
}

//based on https://github.com/mapbox/pbf/blob/main/index.js
function decodeVarintFloat64Value(buf: Uint8Array, offset: IntWrapper): number {
    let val, b;
    b = buf[offset.get()];
    offset.increment();
    val = b & 0x7f;
    if (b < 0x80) return val;
    b = buf[offset.get()];
    offset.increment();
    val |= (b & 0x7f) << 7;
    if (b < 0x80) return val;
    b = buf[offset.get()];
    offset.increment();
    val |= (b & 0x7f) << 14;
    if (b < 0x80) return val;
    b = buf[offset.get()];
    offset.increment();
    val |= (b & 0x7f) << 21;
    if (b < 0x80) return val;
    b = buf[offset.get()];
    val |= (b & 0x0f) << 28;

    return decodeVarintRemainder(val, buf, offset);
}

function decodeVarintRemainder(l, buf, offset) {
    let h, b;
    b = buf[offset.get()];
    offset.increment();
    h = (b & 0x70) >> 4;
    if (b < 0x80) return h * 0x100000000 + (l >>> 0);
    b = buf[offset.get()];
    offset.increment();
    h |= (b & 0x7f) << 3;
    if (b < 0x80) return h * 0x100000000 + (l >>> 0);
    b = buf[offset.get()];
    offset.increment();
    h |= (b & 0x7f) << 10;
    if (b < 0x80) return h * 0x100000000 + (l >>> 0);
    b = buf[offset.get()];
    offset.increment();
    h |= (b & 0x7f) << 17;
    if (b < 0x80) return h * 0x100000000 + (l >>> 0);
    b = buf[offset.get()];
    offset.increment();
    h |= (b & 0x7f) << 24;
    if (b < 0x80) return h * 0x100000000 + (l >>> 0);
    b = buf[offset.get()];
    offset.increment();
    h |= (b & 0x01) << 31;
    if (b < 0x80) return h * 0x100000000 + (l >>> 0);

    throw new Error("Expected varint not more than 10 bytes");
}

export function decodeFastPfor(
    encodedBytes: Uint8Array,
    expectedValueCount: number,
    encodedByteLength: number,
    offset: IntWrapper,
): Int32Array {
    const workspace = createFastPforWireDecodeWorkspace(encodedByteLength >>> 2);
    return decodeFastPforWithWorkspace(encodedBytes, expectedValueCount, encodedByteLength, offset, workspace);
}

export function decodeFastPforWithWorkspace(
    encodedBytes: Uint8Array,
    expectedValueCount: number,
    encodedByteLength: number,
    offset: IntWrapper,
    workspace: FastPforWireDecodeWorkspace,
): Int32Array {
    const inputByteOffset = offset.get();
    if ((encodedByteLength & 3) !== 0) {
        throw new Error(
            `FastPFOR: invalid encodedByteLength=${encodedByteLength} at offset=${inputByteOffset} (encodedBytes.length=${encodedBytes.length}; expected a multiple of 4 bytes for an int32 big-endian word stream)`,
        );
    }

    const encodedWordCount = encodedByteLength >>> 2;
    const encodedWordBuffer = ensureFastPforWireEncodedWordsCapacity(workspace, encodedWordCount);
    decodeBigEndianInt32sInto(encodedBytes, inputByteOffset, encodedByteLength, encodedWordBuffer);

    const decodedValues = decodeFastPforInt32(
        encodedWordBuffer.subarray(0, encodedWordCount),
        expectedValueCount,
        workspace.decoderWorkspace,
    );
    offset.add(encodedByteLength);
    return decodedValues;
}

export function decodeZigZagInt32Value(encoded: number): number {
    return (encoded >>> 1) ^ -(encoded & 1);
}

export function decodeZigZagInt64Value(encoded: bigint): bigint {
    return (encoded >> 1n) ^ -(encoded & 1n);
}

export function decodeZigZagFloat64Value(encoded: number): number {
    return encoded % 2 === 1 ? (encoded + 1) / -2 : encoded / 2;
}

export function decodeZigZagInt32(encodedData: Int32Array): void {
    for (let i = 0; i < encodedData.length; i++) {
        encodedData[i] = decodeZigZagInt32Value(encodedData[i]);
    }
}

export function decodeZigZagInt64(encodedData: BigInt64Array): void {
    for (let i = 0; i < encodedData.length; i++) {
        encodedData[i] = decodeZigZagInt64Value(encodedData[i]);
    }
}

export function decodeZigZagFloat64(encodedData: Float64Array): void {
    for (let i = 0; i < encodedData.length; i++) {
        encodedData[i] = decodeZigZagFloat64Value(encodedData[i]);
    }
}

export function decodeUnsignedRleInt32(encodedData: Int32Array, numRuns: number, numTotalValues?: number): Int32Array {
    // If numTotalValues not provided, calculate from runs (nullable case)
    if (numTotalValues === undefined) {
        numTotalValues = 0;
        for (let i = 0; i < numRuns; i++) {
            numTotalValues += encodedData[i];
        }
    }

    const decodedValues = new Int32Array(numTotalValues);
    let offset = 0;
    for (let i = 0; i < numRuns; i++) {
        const runLength = encodedData[i];
        const value = encodedData[i + numRuns];
        decodedValues.fill(value, offset, offset + runLength);
        offset += runLength;
    }
    return decodedValues;
}

export function decodeUnsignedRleInt64(
    encodedData: BigInt64Array,
    numRuns: number,
    numTotalValues?: number,
): BigInt64Array {
    // If numTotalValues not provided, calculate from runs (nullable case)
    if (numTotalValues === undefined) {
        numTotalValues = 0;
        for (let i = 0; i < numRuns; i++) {
            numTotalValues += Number(encodedData[i]);
        }
    }

    const decodedValues = new BigInt64Array(numTotalValues);
    let offset = 0;
    for (let i = 0; i < numRuns; i++) {
        const runLength = Number(encodedData[i]);
        const value = encodedData[i + numRuns];
        decodedValues.fill(value, offset, offset + runLength);
        offset += runLength;
    }
    return decodedValues;
}

export function decodeUnsignedRleFloat64(
    encodedData: Float64Array,
    numRuns: number,
    numTotalValues: number,
): Float64Array {
    const decodedValues = new Float64Array(numTotalValues);
    let offset = 0;
    for (let i = 0; i < numRuns; i++) {
        const runLength = encodedData[i];
        const value = encodedData[i + numRuns];
        decodedValues.fill(value, offset, offset + runLength);
        offset += runLength;
    }
    return decodedValues;
}

/*
 * In place decoding of the zigzag encoded delta values.
 * Inspired by https://github.com/lemire/JavaFastPFOR/blob/master/src/main/java/me/lemire/integercompression/differential/Delta.java
 */
export function decodeZigZagDeltaInt32(data: Int32Array) {
    data[0] = decodeZigZagInt32Value(data[0]);
    const sz0 = (data.length / 4) * 4;
    let i = 1;
    if (sz0 >= 4) {
        for (; i < sz0 - 4; i += 4) {
            const data1 = data[i];
            const data2 = data[i + 1];
            const data3 = data[i + 2];
            const data4 = data[i + 3];

            data[i] = decodeZigZagInt32Value(data1) + data[i - 1];
            data[i + 1] = decodeZigZagInt32Value(data2) + data[i];
            data[i + 2] = decodeZigZagInt32Value(data3) + data[i + 1];
            data[i + 3] = decodeZigZagInt32Value(data4) + data[i + 2];
        }
    }

    for (; i != data.length; ++i) {
        data[i] = decodeZigZagInt32Value(data[i]) + data[i - 1];
    }
}

export function decodeZigZagDeltaInt64(data: BigInt64Array) {
    data[0] = decodeZigZagInt64Value(data[0]);
    const sz0 = (data.length / 4) * 4;
    let i = 1;
    if (sz0 >= 4) {
        for (; i < sz0 - 4; i += 4) {
            const data1 = data[i];
            const data2 = data[i + 1];
            const data3 = data[i + 2];
            const data4 = data[i + 3];

            data[i] = decodeZigZagInt64Value(data1) + data[i - 1];
            data[i + 1] = decodeZigZagInt64Value(data2) + data[i];
            data[i + 2] = decodeZigZagInt64Value(data3) + data[i + 1];
            data[i + 3] = decodeZigZagInt64Value(data4) + data[i + 2];
        }
    }

    for (; i != data.length; ++i) {
        data[i] = decodeZigZagInt64Value(data[i]) + data[i - 1];
    }
}

export function decodeZigZagDeltaFloat64(data: Float64Array) {
    data[0] = decodeZigZagFloat64Value(data[0]);
    const sz0 = (data.length / 4) * 4;
    let i = 1;
    if (sz0 >= 4) {
        for (; i < sz0 - 4; i += 4) {
            const data1 = data[i];
            const data2 = data[i + 1];
            const data3 = data[i + 2];
            const data4 = data[i + 3];

            data[i] = decodeZigZagFloat64Value(data1) + data[i - 1];
            data[i + 1] = decodeZigZagFloat64Value(data2) + data[i];
            data[i + 2] = decodeZigZagFloat64Value(data3) + data[i + 1];
            data[i + 3] = decodeZigZagFloat64Value(data4) + data[i + 2];
        }
    }

    for (; i != data.length; ++i) {
        data[i] = decodeZigZagFloat64Value(data[i]) + data[i - 1];
    }
}

export function decodeZigZagRleInt32(data: Int32Array, numRuns: number, numTotalValues?: number): Int32Array {
    // If numTotalValues not provided, calculate from runs (nullable case)
    if (numTotalValues === undefined) {
        numTotalValues = 0;
        for (let i = 0; i < numRuns; i++) {
            numTotalValues += data[i];
        }
    }

    const decodedValues = new Int32Array(numTotalValues);
    let offset = 0;
    for (let i = 0; i < numRuns; i++) {
        const runLength = data[i];
        let value = data[i + numRuns];
        value = decodeZigZagInt32Value(value);
        decodedValues.fill(value, offset, offset + runLength);
        offset += runLength;
    }
    return decodedValues;
}

export function decodeZigZagRleInt64(data: BigInt64Array, numRuns: number, numTotalValues?: number): BigInt64Array {
    // If numTotalValues not provided, calculate from runs (nullable case)
    if (numTotalValues === undefined) {
        numTotalValues = 0;
        for (let i = 0; i < numRuns; i++) {
            numTotalValues += Number(data[i]);
        }
    }

    const decodedValues = new BigInt64Array(numTotalValues);
    let offset = 0;
    for (let i = 0; i < numRuns; i++) {
        const runLength = Number(data[i]);
        let value = data[i + numRuns];
        value = decodeZigZagInt64Value(value);
        decodedValues.fill(value, offset, offset + runLength);
        offset += runLength;
    }
    return decodedValues;
}

export function decodeZigZagRleFloat64(data: Float64Array, numRuns: number, numTotalValues: number): Float64Array {
    const decodedValues = new Float64Array(numTotalValues);
    let offset = 0;
    for (let i = 0; i < numRuns; i++) {
        const runLength = data[i];
        let value = data[i + numRuns];
        value = decodeZigZagFloat64Value(value);
        decodedValues.fill(value, offset, offset + runLength);
        offset += runLength;
    }
    return decodedValues;
}

/*
 * Inspired by https://github.com/lemire/JavaFastPFOR/blob/master/src/main/java/me/lemire/integercompression/differential/Delta.java
 */
export function fastInverseDelta(data: Uint32Array | Int32Array) {
    const sz0 = (data.length / 4) * 4;
    let i = 1;
    if (sz0 >= 4) {
        for (let a = data[0]; i < sz0 - 4; i += 4) {
            a = data[i] += a;
            a = data[i + 1] += a;
            a = data[i + 2] += a;
            a = data[i + 3] += a;
        }
    }

    while (i != data.length) {
        data[i] += data[i - 1];
        ++i;
    }
}

export function inverseDelta(data: Int32Array) {
    let prevValue = 0;
    for (let i = 0; i < data.length; i++) {
        data[i] += prevValue;
        prevValue = data[i];
    }
}

/*
 * In place decoding of the zigzag delta encoded Vec2.
 * Inspired by https://github.com/lemire/JavaFastPFOR/blob/master/src/main/java/me/lemire/integercompression/differential/Delta.java
 */
export function decodeComponentwiseDeltaVec2(data: Int32Array): void {
    if (data.length < 2) return;
    data[0] = decodeZigZagInt32Value(data[0]);
    data[1] = decodeZigZagInt32Value(data[1]);
    const sz0 = (data.length / 4) * 4;
    let i = 2;
    if (sz0 >= 4) {
        for (; i < sz0 - 4; i += 4) {
            const x1 = data[i];
            const y1 = data[i + 1];
            const x2 = data[i + 2];
            const y2 = data[i + 3];

            data[i] = decodeZigZagInt32Value(x1) + data[i - 2];
            data[i + 1] = decodeZigZagInt32Value(y1) + data[i - 1];
            data[i + 2] = decodeZigZagInt32Value(x2) + data[i];
            data[i + 3] = decodeZigZagInt32Value(y2) + data[i + 1];
        }
    }

    for (; i != data.length; i += 2) {
        data[i] = decodeZigZagInt32Value(data[i]) + data[i - 2];
        data[i + 1] = decodeZigZagInt32Value(data[i + 1]) + data[i - 1];
    }
}

export function decodeComponentwiseDeltaVec2Scaled(data: Int32Array, scale: number, min: number, max: number): void {
    if (data.length < 2) return;
    let previousVertexX = decodeZigZagInt32Value(data[0]);
    let previousVertexY = decodeZigZagInt32Value(data[1]);
    data[0] = clamp(Math.round(previousVertexX * scale), min, max);
    data[1] = clamp(Math.round(previousVertexY * scale), min, max);
    const sz0 = data.length / 16;
    let i = 2;
    if (sz0 >= 4) {
        for (; i < sz0 - 4; i += 4) {
            const x1 = data[i];
            const y1 = data[i + 1];
            const currentVertexX = decodeZigZagInt32Value(x1) + previousVertexX;
            const currentVertexY = decodeZigZagInt32Value(y1) + previousVertexY;
            data[i] = clamp(Math.round(currentVertexX * scale), min, max);
            data[i + 1] = clamp(Math.round(currentVertexY * scale), min, max);

            const x2 = data[i + 2];
            const y2 = data[i + 3];
            previousVertexX = decodeZigZagInt32Value(x2) + currentVertexX;
            previousVertexY = decodeZigZagInt32Value(y2) + currentVertexY;
            data[i + 2] = clamp(Math.round(previousVertexX * scale), min, max);
            data[i + 3] = clamp(Math.round(previousVertexY * scale), min, max);
        }
    }

    for (; i != data.length; i += 2) {
        previousVertexX += decodeZigZagInt32Value(data[i]);
        previousVertexY += decodeZigZagInt32Value(data[i + 1]);
        data[i] = clamp(Math.round(previousVertexX * scale), min, max);
        data[i + 1] = clamp(Math.round(previousVertexY * scale), min, max);
    }
}

function clamp(n: number, min: number, max: number): number {
    return Math.min(max, Math.max(min, n));
}

/* Transform data to allow util access ------------------------------------------------------------------------ */

export function decodeZigZagDeltaOfDeltaInt32(data: Int32Array): Uint32Array {
    const decodedData = new Int32Array(data.length + 1);
    decodedData[0] = 0;
    decodedData[1] = decodeZigZagInt32Value(data[0]);
    let deltaSum = decodedData[1];
    for (let i = 2; i != decodedData.length; ++i) {
        const zigZagValue = data[i - 1];
        const delta = decodeZigZagInt32Value(zigZagValue);
        deltaSum += delta;
        decodedData[i] = decodedData[i - 1] + deltaSum;
    }

    return new Uint32Array(decodedData);
}

export function decodeZigZagRleDeltaInt32(data: Int32Array, numRuns: number, numTotalValues: number): Uint32Array {
    const decodedValues = new Int32Array(numTotalValues + 1);
    decodedValues[0] = 0;
    let offset = 1;
    let previousValue = decodedValues[0];
    for (let i = 0; i < numRuns; i++) {
        const runLength = data[i];
        let value = data[i + numRuns];
        value = decodeZigZagInt32Value(value);
        for (let j = offset; j < offset + runLength; j++) {
            decodedValues[j] = value + previousValue;
            previousValue = decodedValues[j];
        }

        offset += runLength;
    }
    return new Uint32Array(decodedValues);
}

export function decodeRleDeltaInt32(data: Int32Array, numRuns: number, numTotalValues: number): Uint32Array {
    const decodedValues = new Int32Array(numTotalValues + 1);
    decodedValues[0] = 0;
    let offset = 1;
    let previousValue = decodedValues[0];
    for (let i = 0; i < numRuns; i++) {
        const runLength = data[i];
        const value = data[i + numRuns];
        for (let j = offset; j < offset + runLength; j++) {
            decodedValues[j] = value + previousValue;
            previousValue = decodedValues[j];
        }

        offset += runLength;
    }

    return new Uint32Array(decodedValues);
}

/**
 * Decode Delta-RLE with multiple runs by fully reconstructing values.
 *
 * @param data RLE encoded data: [run1, run2, ..., value1, value2, ...]
 * @param numRuns Number of runs in the RLE encoding
 * @param numValues Total number of values to reconstruct
 * @returns Reconstructed values with deltas applied
 */
export function decodeDeltaRleInt32(data: Int32Array, numRuns: number, numValues: number): Int32Array {
    const result = new Int32Array(numValues);
    let outPos = 0;
    let previousValue = 0;

    for (let i = 0; i < numRuns; i++) {
        const runLength = data[i];
        const zigZagDelta = data[i + numRuns];
        const delta = decodeZigZagInt32Value(zigZagDelta);

        for (let j = 0; j < runLength; j++) {
            previousValue += delta;
            result[outPos++] = previousValue;
        }
    }

    return result;
}

/**
 * Decode Delta-RLE with multiple runs for 64-bit integers.
 */
export function decodeDeltaRleInt64(data: BigInt64Array, numRuns: number, numValues: number): BigInt64Array {
    const result = new BigInt64Array(numValues);
    let outPos = 0;
    let previousValue = 0n;

    for (let i = 0; i < numRuns; i++) {
        const runLength = Number(data[i]);
        const zigZagDelta = data[i + numRuns];
        const delta = decodeZigZagInt64Value(zigZagDelta);

        for (let j = 0; j < runLength; j++) {
            previousValue += delta;
            result[outPos++] = previousValue;
        }
    }

    return result;
}

export function decodeUnsignedConstRleInt32(data: Int32Array): number {
    return data[1];
}

export function decodeZigZagConstRleInt32(data: Int32Array): number {
    return decodeZigZagInt32Value(data[1]);
}

export function decodeZigZagSequenceRleInt32(data: Int32Array): [baseValue: number, delta: number] {
    /* base value and delta value are equal */
    if (data.length == 2) {
        const value = decodeZigZagInt32Value(data[1]);
        return [value, value];
    }

    /* base value and delta value are not equal -> 2 runs and 2 values*/
    const base = decodeZigZagInt32Value(data[2]);
    const delta = decodeZigZagInt32Value(data[3]);
    return [base, delta];
}

export function decodeUnsignedConstRleInt64(data: BigInt64Array): bigint {
    return data[1];
}

export function decodeZigZagConstRleInt64(data: BigInt64Array): bigint {
    return decodeZigZagInt64Value(data[1]);
}

export function decodeZigZagSequenceRleInt64(data: BigInt64Array): [baseValue: bigint, delta: bigint] {
    /* base value and delta value are equal */
    if (data.length == 2) {
        const value = decodeZigZagInt64Value(data[1]);
        return [value, value];
    }

    /* base value and delta value are not equal -> 2 runs and 2 values*/
    const base = decodeZigZagInt64Value(data[2]);
    const delta = decodeZigZagInt64Value(data[3]);
    return [base, delta];
}
