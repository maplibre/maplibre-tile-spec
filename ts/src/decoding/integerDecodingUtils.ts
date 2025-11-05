import type IntWrapper from "./intWrapper";
import { type RleEncodedStreamMetadata } from "../metadata/tile/rleEncodedStreamMetadata";
import type BitVector from "../vector/flat/bitVector";
import { type StreamMetadata } from "../mltMetadata";

/* Null suppression (physical level) techniques ------------------------------------------------------------------*/

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
        dst[i] = decodeSingleVarintInt64(src, offset);
    }
    return dst;
}

/* Since decoding Int64 values to BigInt is more than an order of magnitude slower in the tests
 *  then using a Float64, this decoding method limits the max size of a Long value to 53 bits   */
export function decodeVarintFloat64(src: Uint8Array, numValues: number, offset: IntWrapper): Float64Array {
    const dst = new Float64Array(numValues);
    for (let i = 0; i < numValues; i++) {
        dst[i] = decodeSingleVarintFloat64(src, offset);
    }
    return dst;
}

//based on https://github.com/mapbox/pbf/blob/main/index.js
function decodeSingleVarintFloat64(buf: Uint8Array, offset: IntWrapper): number {
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
    data: Uint8Array,
    numValues: number,
    byteLength: number,
    offset: IntWrapper,
): Int32Array {
    throw new Error("FastPFor is not implemented yet.");
}

export function decodeZigZag(encodedData: Int32Array): void {
    for (let i = 0; i < encodedData.length; i++) {
        const encoded = encodedData[i];
        encodedData[i] = (encoded >>> 1) ^ -(encoded & 1);
    }
}

export function decodeZigZagInt64(encodedData: BigInt64Array): void {
    for (let i = 0; i < encodedData.length; i++) {
        const encoded = encodedData[i];
        encodedData[i] = (encoded >> 1n) ^ -(encoded & 1n);
    }
}

export function decodeZigZagFloat64(encodedData: Float64Array): void {
    for (let i = 0; i < encodedData.length; i++) {
        const encoded = encodedData[i];
        //Get rid of branch? -> var v = encoded % 2 && 1; encodedData[i] = (encoded + v) / (v * 2 - 1) * 2;
        encodedData[i] = encoded % 2 === 1 ? (encoded + 1) / -2 : encoded / 2;
    }
}

export function decodeZigZagValue(encoded: number): number {
    return (encoded >>> 1) ^ -(encoded & 1);
}

export function decodeZigZagValueInt64(encoded: bigint): bigint {
    return (encoded >> 1n) ^ -(encoded & 1n);
}

// Source: https://github.com/bazelbuild/bazel/blob/master/src/main/java/com/google/devtools/build/lib/util/VarInt.java
function decodeSingleVarintInt64(bytes: Uint8Array, pos: IntWrapper): bigint {
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

/* Logical Level Techniques Flat Vectors ------------------------------------------------------------------ */

export function decodeRle(data: Int32Array, streamMetadata: RleEncodedStreamMetadata, isSigned: boolean): Int32Array {
    return isSigned
        ? decodeZigZagRle(data, streamMetadata.runs, streamMetadata.numRleValues)
        : decodeUnsignedRle(data, streamMetadata.runs, streamMetadata.numRleValues);
}

export function decodeRleInt64(
    data: BigInt64Array,
    streamMetadata: RleEncodedStreamMetadata,
    isSigned: boolean,
): BigInt64Array {
    return isSigned
        ? decodeZigZagRleInt64(data, streamMetadata.runs, streamMetadata.numRleValues)
        : decodeUnsignedRleInt64(data, streamMetadata.runs, streamMetadata.numRleValues);
}

export function decodeRleFloat64(
    data: Float64Array,
    streamMetadata: RleEncodedStreamMetadata,
    isSigned: boolean,
): Float64Array {
    return isSigned
        ? decodeZigZagRleFloat64(data, streamMetadata.runs, streamMetadata.numRleValues)
        : decodeUnsignedRleFloat64(data, streamMetadata.runs, streamMetadata.numRleValues);
}

export function decodeUnsignedRle(encodedData: Int32Array, numRuns: number, numTotalValues: number): Int32Array {
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
    numTotalValues: number,
): BigInt64Array {
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
export function decodeZigZagDelta(data: Int32Array) {
    data[0] = (data[0] >>> 1) ^ -(data[0] & 1);
    const sz0 = (data.length / 4) * 4;
    let i = 1;
    if (sz0 >= 4) {
        for (; i < sz0 - 4; i += 4) {
            const data1 = data[i];
            const data2 = data[i + 1];
            const data3 = data[i + 2];
            const data4 = data[i + 3];

            data[i] = ((data1 >>> 1) ^ -(data1 & 1)) + data[i - 1];
            data[i + 1] = ((data2 >>> 1) ^ -(data2 & 1)) + data[i];
            data[i + 2] = ((data3 >>> 1) ^ -(data3 & 1)) + data[i + 1];
            data[i + 3] = ((data4 >>> 1) ^ -(data4 & 1)) + data[i + 2];
        }
    }

    for (; i != data.length; ++i) {
        data[i] = ((data[i] >>> 1) ^ -(data[i] & 1)) + data[i - 1];
    }
}

export function decodeZigZagDeltaInt64(data: BigInt64Array) {
    data[0] = (data[0] >> 1n) ^ -(data[0] & 1n);
    const sz0 = (data.length / 4) * 4;
    let i = 1;
    if (sz0 >= 4) {
        for (; i < sz0 - 4; i += 4) {
            const data1 = data[i];
            const data2 = data[i + 1];
            const data3 = data[i + 2];
            const data4 = data[i + 3];

            data[i] = ((data1 >> 1n) ^ -(data1 & 1n)) + data[i - 1];
            data[i + 1] = ((data2 >> 1n) ^ -(data2 & 1n)) + data[i];
            data[i + 2] = ((data3 >> 1n) ^ -(data3 & 1n)) + data[i + 1];
            data[i + 3] = ((data4 >> 1n) ^ -(data4 & 1n)) + data[i + 2];
        }
    }

    for (; i != data.length; ++i) {
        data[i] = ((data[i] >> 1n) ^ -(data[i] & 1n)) + data[i - 1];
    }
}

export function decodeZigZagDeltaFloat64(data: Float64Array) {
    data[0] = data[0] % 2 === 1 ? (data[0] + 1) / -2 : data[0] / 2;
    const sz0 = (data.length / 4) * 4;
    let i = 1;
    if (sz0 >= 4) {
        for (; i < sz0 - 4; i += 4) {
            const data1 = data[i];
            const data2 = data[i + 1];
            const data3 = data[i + 2];
            const data4 = data[i + 3];

            data[i] = (data1 % 2 === 1 ? (data1 + 1) / -2 : data1 / 2) + data[i - 1];
            data[i + 1] = (data2 % 2 === 1 ? (data2 + 1) / -2 : data2 / 2) + data[i];
            data[i + 2] = (data3 % 2 === 1 ? (data3 + 1) / -2 : data3 / 2) + data[i + 1];
            data[i + 3] = (data4 % 2 === 1 ? (data4 + 1) / -2 : data4 / 2) + data[i + 2];
        }
    }

    for (; i != data.length; ++i) {
        data[i] = (data[i] % 2 === 1 ? (data[i] + 1) / -2 : data[i] / 2) + data[i - 1];
    }
}

export function decodeZigZagRle(data: Int32Array, numRuns: number, numTotalValues: number): Int32Array {
    const decodedValues = new Int32Array(numTotalValues);
    let offset = 0;
    for (let i = 0; i < numRuns; i++) {
        const runLength = data[i];
        let value = data[i + numRuns];
        value = (value >>> 1) ^ -(value & 1);
        decodedValues.fill(value, offset, offset + runLength);
        offset += runLength;
    }
    return decodedValues;
}

export function decodeZigZagRleInt64(data: BigInt64Array, numRuns: number, numTotalValues: number): BigInt64Array {
    const decodedValues = new BigInt64Array(numTotalValues);
    let offset = 0;
    for (let i = 0; i < numRuns; i++) {
        const runLength = Number(data[i]);
        let value = data[i + numRuns];
        value = (value >> 1n) ^ -(value & 1n);
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
        //TODO: get rid of branch? -> var v = value % 2 && 1; a = (value + v) / (v * 2 - 1) * 2;
        value = value % 2 === 1 ? (value + 1) / -2 : value / 2;
        decodedValues.fill(value, offset, offset + runLength);
        offset += runLength;
    }
    return decodedValues;
}

/*
 * Inspired by https://github.com/lemire/JavaFastPFOR/blob/master/src/main/java/me/lemire/integercompression/differential/Delta.java
 */
export function fastInverseDelta(data: Int32Array) {
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
    data[0] = (data[0] >>> 1) ^ -(data[0] & 1);
    data[1] = (data[1] >>> 1) ^ -(data[1] & 1);
    const sz0 = (data.length / 4) * 4;
    let i = 2;
    if (sz0 >= 4) {
        for (; i < sz0 - 4; i += 4) {
            const x1 = data[i];
            const y1 = data[i + 1];
            const x2 = data[i + 2];
            const y2 = data[i + 3];

            data[i] = ((x1 >>> 1) ^ -(x1 & 1)) + data[i - 2];
            data[i + 1] = ((y1 >>> 1) ^ -(y1 & 1)) + data[i - 1];
            data[i + 2] = ((x2 >>> 1) ^ -(x2 & 1)) + data[i];
            data[i + 3] = ((y2 >>> 1) ^ -(y2 & 1)) + data[i + 1];
        }
    }

    for (; i != data.length; i += 2) {
        data[i] = ((data[i] >>> 1) ^ -(data[i] & 1)) + data[i - 2];
        data[i + 1] = ((data[i + 1] >>> 1) ^ -(data[i + 1] & 1)) + data[i - 1];
    }
}

export function decodeComponentwiseDeltaVec2Scaled(data: Int32Array, scale: number, min: number, max: number): void {
    let previousVertexX = (data[0] >>> 1) ^ -(data[0] & 1);
    let previousVertexY = (data[1] >>> 1) ^ -(data[1] & 1);
    data[0] = clamp(Math.round(previousVertexX * scale), min, max);
    data[1] = clamp(Math.round(previousVertexY * scale), min, max);
    const sz0 = data.length / 16;
    let i = 2;
    if (sz0 >= 4) {
        for (; i < sz0 - 4; i += 4) {
            const x1 = data[i];
            const y1 = data[i + 1];
            const currentVertexX = ((x1 >>> 1) ^ -(x1 & 1)) + previousVertexX;
            const currentVertexY = ((y1 >>> 1) ^ -(y1 & 1)) + previousVertexY;
            data[i] = clamp(Math.round(currentVertexX * scale), min, max);
            data[i + 1] = clamp(Math.round(currentVertexY * scale), min, max);

            const x2 = data[i + 2];
            const y2 = data[i + 3];
            previousVertexX = ((x2 >>> 1) ^ -(x2 & 1)) + currentVertexX;
            previousVertexY = ((y2 >>> 1) ^ -(y2 & 1)) + currentVertexY;
            data[i + 2] = clamp(Math.round(previousVertexX * scale), min, max);
            data[i + 3] = clamp(Math.round(previousVertexY * scale), min, max);
        }
    }

    for (; i != data.length; i += 2) {
        previousVertexX += (data[i] >>> 1) ^ -(data[i] & 1);
        previousVertexY += (data[i + 1] >>> 1) ^ -(data[i + 1] & 1);
        data[i] = clamp(Math.round(previousVertexX * scale), min, max);
        data[i + 1] = clamp(Math.round(previousVertexY * scale), min, max);
    }
}

function clamp(n: number, min: number, max: number): number {
    return Math.min(max, Math.max(min, n));
}

export function decodeNullableZigZagDelta(bitVector: BitVector, data: Int32Array): Int32Array {
    const decodedData = new Int32Array(bitVector.size());
    let dataCounter = 0;
    if (bitVector.get(0)) {
        decodedData[0] = bitVector.get(0) ? (data[0] >>> 1) ^ -(data[0] & 1) : 0;
        dataCounter = 1;
    } else {
        decodedData[0] = 0;
    }

    let i = 1;
    for (; i != decodedData.length; ++i) {
        decodedData[i] = bitVector.get(i)
            ? decodedData[i - 1] + ((data[dataCounter] >>> 1) ^ -(data[dataCounter++] & 1))
            : decodedData[i - 1];
    }

    return decodedData;
}

export function decodeNullableZigZagDeltaInt64(bitVector: BitVector, data: BigInt64Array): BigInt64Array {
    const decodedData = new BigInt64Array(bitVector.size());
    let dataCounter = 0;
    if (bitVector.get(0)) {
        decodedData[0] = bitVector.get(0) ? (data[0] >> 1n) ^ -(data[0] & 1n) : 0n;
        dataCounter = 1;
    } else {
        decodedData[0] = 0n;
    }

    let i = 1;
    for (; i != decodedData.length; ++i) {
        decodedData[i] = bitVector.get(i)
            ? decodedData[i - 1] + ((data[dataCounter] >> 1n) ^ -(data[dataCounter++] & 1n))
            : decodedData[i - 1];
    }

    return decodedData;
}

/* Transform data to allow util access ------------------------------------------------------------------------ */

export function zigZagDeltaOfDeltaDecoding(data: Int32Array): Int32Array {
    const decodedData = new Int32Array(data.length + 1);
    decodedData[0] = 0;
    decodedData[1] = decodeZigZagValue(data[0]);
    let deltaSum = decodedData[1];
    let i = 2;
    for (; i != decodedData.length; ++i) {
        const zigZagValue = data[i - 1];
        const delta = (zigZagValue >>> 1) ^ -(zigZagValue & 1);
        deltaSum += delta;
        decodedData[i] = decodedData[i - 1] + deltaSum;
    }

    return decodedData;
}

export function zigZagRleDeltaDecoding(data: Int32Array, numRuns: number, numTotalValues: number): Int32Array {
    const decodedValues = new Int32Array(numTotalValues + 1);
    decodedValues[0] = 0;
    let offset = 1;
    let previousValue = decodedValues[0];
    for (let i = 0; i < numRuns; i++) {
        const runLength = data[i];
        let value = data[i + numRuns];
        value = (value >>> 1) ^ -(value & 1);
        for (let j = offset; j < offset + runLength; j++) {
            decodedValues[j] = value + previousValue;
            previousValue = decodedValues[j];
        }

        offset += runLength;
    }

    return decodedValues;
}

export function rleDeltaDecoding(data: Int32Array, numRuns: number, numTotalValues: number): Int32Array {
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

    return decodedValues;
}

export function padWithZeros(bitVector: BitVector, data: Int32Array): Int32Array {
    const decodedData = new Int32Array(bitVector.size());
    let dataCounter = 0;
    let i = 0;
    for (; i != decodedData.length; ++i) {
        decodedData[i] = bitVector.get(i) ? data[dataCounter++] : 0;
    }

    return decodedData;
}

export function padZigZagWithZeros(bitVector: BitVector, data: Int32Array): Int32Array {
    const decodedData = new Int32Array(bitVector.size());
    let dataCounter = 0;
    let i = 0;
    for (; i != decodedData.length; ++i) {
        if (bitVector.get(i)) {
            const value = data[dataCounter++];
            decodedData[i] = (value >>> 1) ^ -(value & 1);
        } else {
            decodedData[i] = 0;
        }
    }

    return decodedData;
}

export function padWithZerosInt64(bitVector: BitVector, data: BigInt64Array): BigInt64Array {
    const decodedData = new BigInt64Array(bitVector.size());
    let dataCounter = 0;
    let i = 0;
    for (; i != decodedData.length; ++i) {
        decodedData[i] = bitVector.get(i) ? data[dataCounter++] : 0n;
    }

    return decodedData;
}

export function padZigZagWithZerosInt64(bitVector: BitVector, data: BigInt64Array): BigInt64Array {
    const decodedData = new BigInt64Array(bitVector.size());
    let dataCounter = 0;
    let i = 0;
    for (; i != decodedData.length; ++i) {
        if (bitVector.get(i)) {
            const value = data[dataCounter++];
            decodedData[i] = (value >> 1n) ^ -(value & 1n);
        } else {
            decodedData[i] = 0n;
        }
    }

    return decodedData;
}

export function decodeNullableRle(
    data: Int32Array,
    streamMetadata: StreamMetadata,
    isSigned: boolean,
    bitVector: BitVector,
): Int32Array {
    const rleMetadata = streamMetadata as RleEncodedStreamMetadata;
    return isSigned
        ? decodeNullableZigZagRle(bitVector, data, rleMetadata.runs)
        : decodeNullableUnsignedRle(bitVector, data, rleMetadata.runs);
}

function decodeNullableUnsignedRle(bitVector: BitVector, data: Int32Array, numRuns: number): Int32Array {
    const values = new Int32Array(bitVector.size());
    let offset = 0;
    for (let i = 0; i < numRuns; i++) {
        const runLength = data[i];
        const value = data[i + numRuns];
        for (let j = offset; j < offset + runLength; j++) {
            /* There can be null values in a run */
            if (bitVector.get(j)) {
                values[j] = value;
            } else {
                values[j] = 0;
                offset++;
            }
        }
        offset += runLength;
    }

    return values;
}

function decodeNullableZigZagRle(bitVector, data: Int32Array, numRuns: number): Int32Array {
    const values = new Int32Array(bitVector.size());
    let offset = 0;
    for (let i = 0; i < numRuns; i++) {
        const runLength = data[i];
        let value = data[i + numRuns];
        value = (value >>> 1) ^ -(value & 1);
        for (let j = offset; j < offset + runLength; j++) {
            /* There can be null values in a run */
            if (bitVector.get(j)) {
                values[j] = value;
            } else {
                values[j] = 0;
                offset++;
            }
        }
        offset += runLength;
    }

    return values;
}

export function decodeNullableRleInt64(
    data: BigInt64Array,
    streamMetadata: StreamMetadata,
    isSigned: boolean,
    bitVector: BitVector,
): BigInt64Array {
    const rleMetadata = streamMetadata as RleEncodedStreamMetadata;
    return isSigned
        ? decodeNullableZigZagRleInt64(bitVector, data, rleMetadata.runs)
        : decodeNullableUnsignedRleInt64(bitVector, data, rleMetadata.runs);
}

function decodeNullableUnsignedRleInt64(bitVector: BitVector, data: BigInt64Array, numRuns: number): BigInt64Array {
    const values = new BigInt64Array(bitVector.size());
    let offset = 0;
    for (let i = 0; i < numRuns; i++) {
        const runLength = Number(data[i]);
        const value = data[i + numRuns];
        for (let j = offset; j < offset + runLength; j++) {
            /* There can be null values in a run */
            if (bitVector.get(j)) {
                values[j] = value;
            } else {
                values[j] = 0n;
                offset++;
            }
        }
        offset += runLength;
    }

    return values;
}

function decodeNullableZigZagRleInt64(bitVector, data: BigInt64Array, numRuns: number): BigInt64Array {
    const values = new BigInt64Array(bitVector.size());
    let offset = 0;
    for (let i = 0; i < numRuns; i++) {
        const runLength = Number(data[i]);
        let value = data[i + numRuns];
        value = (value >> 1n) ^ -(value & 1n);
        for (let j = offset; j < offset + runLength; j++) {
            /* There can be null values in a run */
            if (bitVector.get(j)) {
                values[j] = value;
            } else {
                values[j] = 0n;
                offset++;
            }
        }
        offset += runLength;
    }

    return values;
}

/* Logical Level Techniques Const and Sequence Vectors ------------------------------------------------------------- */

/**
 * Decode Delta-RLE with multiple runs by fully reconstructing values.
 *
 * @param data RLE encoded data: [run1, run2, ..., value1, value2, ...]
 * @param numRuns Number of runs in the RLE encoding
 * @param numValues Total number of values to reconstruct
 * @returns Reconstructed values with deltas applied
 */
export function decodeDeltaRle(data: Int32Array, numRuns: number, numValues: number): Int32Array {
    const result = new Int32Array(numValues);
    let outPos = 0;
    let previousValue = 0;

    for (let i = 0; i < numRuns; i++) {
        const runLength = data[i];
        const zigZagDelta = data[i + numRuns];
        const delta = decodeZigZagValue(zigZagDelta);

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
        const delta = decodeZigZagValueInt64(zigZagDelta);

        for (let j = 0; j < runLength; j++) {
            previousValue += delta;
            result[outPos++] = previousValue;
        }
    }

    return result;
}

export function decodeUnsignedConstRle(data: Int32Array): number {
    return data[1];
}

export function decodeZigZagConstRle(data: Int32Array): number {
    return decodeZigZagValue(data[1]);
}

export function decodeZigZagSequenceRle(data: Int32Array): [baseValue: number, delta: number] {
    /* base value and delta value are equal */
    if (data.length == 2) {
        const value = decodeZigZagValue(data[1]);
        return [value, value];
    }

    /* base value and delta value are not equal -> 2 runs and 2 values*/
    const base = decodeZigZagValue(data[2]);
    const delta = decodeZigZagValue(data[3]);
    return [base, delta];
}

export function decodeUnsignedConstRleInt64(data: BigInt64Array): bigint {
    return data[1];
}

export function decodeZigZagConstRleInt64(data: BigInt64Array): bigint {
    return decodeZigZagValueInt64(data[1]);
}

export function decodeZigZagSequenceRleInt64(data: BigInt64Array): [baseValue: bigint, delta: bigint] {
    /* base value and delta value are equal */
    if (data.length == 2) {
        const value = decodeZigZagValueInt64(data[1]);
        return [value, value];
    }

    /* base value and delta value are not equal -> 2 runs and 2 values*/
    const base = decodeZigZagValueInt64(data[2]);
    const delta = decodeZigZagValueInt64(data[3]);
    return [base, delta];
}
