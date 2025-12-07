import { describe, it, expect } from "vitest";
import {
    decodeVarintInt32,
    decodeVarintInt64,
    decodeVarintFloat64,
    decodeZigZagInt32Array,
    decodeZigZagInt64Array,
    decodeZigZagFloat64Array,
    decodeZigZagInt32Value,
    decodeZigZagInt64Value,
    decodeUnsignedRleInt32,
    decodeUnsignedRleInt64,
    decodeUnsignedRleFloat64,
    decodeZigZagDeltaInt64,
    decodeNullableZigZagDeltaInt32,
    decodeNullableZigZagDeltaInt64,
    padWithZerosInt64,
    padZigZagWithZerosInt64,
    decodeDeltaRleInt64,
    decodeUnsignedConstRleInt64,
    decodeZigZagConstRleInt64,
    decodeZigZagSequenceRleInt64,
    decodeZigZagRleInt32,
    decodeZigZagRleInt64,
    decodeZigZagRleFloat64,
    decodeZigZagRleDeltaInt32,
    fastInverseDelta,
} from "./integerDecodingUtils";
import IntWrapper from "./intWrapper";
import BitVector from "../vector/flat/bitVector";
import {
    encodeVarintInt32,
    encodeVarintInt64,
    encodeDeltaInt32,
    encodeDeltaRleInt64,
    encodeNullableZigZagDeltaInt32,
    encodeNullableZigZagDeltaInt64,
    encodeUnsignedRleFloat64,
    encodeUnsignedRleInt32,
    encodeUnsignedRleInt64,
    encodeZigZagDeltaInt64,
    encodeZigZagFloat64Array,
    encodeZigZagInt32Array,
    encodeZigZagInt32Value,
    encodeZigZagInt64Array,
    encodeZigZagInt64Value,
    encodeZigZagRleFloat64,
    encodeZigZagRleInt32,
    encodeZigZagRleInt64,
} from "../encoding/integerEncodingUtils";

describe("IntegerDecodingUtils", () => {
    it("should decode Int32", () => {
        const value = 2 ** 10;
        const encoded = encodeVarintInt32(value);
        const decoded = decodeVarintInt32(encoded, new IntWrapper(0), 1);
        expect(decoded[0]).toEqual(value);
    });

    it("should decode Int64", () => {
        const value = 2n ** 50n;
        const encoded = encodeVarintInt64(value);
        const decoded = decodeVarintInt64(encoded, new IntWrapper(0), 1);
        expect(decoded[0]).toEqual(value);
    });

    it("should return valid decoded values for varint long to float64", () => {
        const value = 2 ** 40;
        const varintEncoded = encodeVarintInt32(value);
        const actualValues = decodeVarintFloat64(varintEncoded, new IntWrapper(0), 1);
        expect(actualValues[0]).toEqual(value);
    });

    it("should decode zigzag Int32Array", () => {
        const data = new Int32Array([0, 1, 2, 3]);
        encodeZigZagInt32Array(data);
        decodeZigZagInt32Array(data);
        expect(Array.from(data)).toEqual([0, 1, 2, 3]);
    });

    it("should decode zigzag BigInt64Array", () => {
        const data = new BigInt64Array([0n, 1n, 2n, 3n]);
        encodeZigZagInt64Array(data);
        decodeZigZagInt64Array(data);
        expect(Array.from(data)).toEqual([0n, 1n, 2n, 3n]);
    });

    it("should decode zigzag Float64Array", () => {
        const value = 2 ** 35;
        const data = new Float64Array([value]);
        encodeZigZagFloat64Array(data);
        decodeZigZagFloat64Array(data);
        expect(Array.from(data)).toEqual([value]);
    });

    it("should decode single Int32 zigzag values", () => {
        expect(encodeZigZagInt32Value(decodeZigZagInt32Value(0))).toBe(0);
        expect(encodeZigZagInt32Value(decodeZigZagInt32Value(1))).toBe(1);
        expect(encodeZigZagInt32Value(decodeZigZagInt32Value(2))).toBe(2);
    });

    it("should decode single BigInt zigzag values", () => {
        expect(encodeZigZagInt64Value(decodeZigZagInt64Value(0n))).toBe(0n);
        expect(encodeZigZagInt64Value(decodeZigZagInt64Value(1n))).toBe(1n);
    });

    it("should decode empty unsigned RLE", () => {
        const data = new Int32Array([]);
        const encodedRle = encodeUnsignedRleInt32(data);
        const decoded = decodeUnsignedRleInt32(encodedRle, encodedRle.length / 2, data.length);
        expect(Array.from(decoded)).toEqual([]);
    });

    it("should decode unsigned RLE", () => {
        const data = new Int32Array([10, 10, 20, 20, 20]);
        const encodedRle = encodeUnsignedRleInt32(data);
        const decoded = decodeUnsignedRleInt32(encodedRle, encodedRle.length / 2, data.length);
        expect(Array.from(decoded)).toEqual([10, 10, 20, 20, 20]);
    });

    it("should decode empty unsigned RLE Int64", () => {
        const data = new BigInt64Array([]);
        const encodedRle = encodeUnsignedRleInt64(data);
        const decoded = decodeUnsignedRleInt64(encodedRle, encodedRle.length / 2, data.length);
        expect(Array.from(decoded)).toEqual([]);
    });

    it("should decode unsigned RLE Int64", () => {
        const data = new BigInt64Array([10n, 10n, 20n, 20n, 20n]);
        const encodedRle = encodeUnsignedRleInt64(data);
        const decoded = decodeUnsignedRleInt64(encodedRle, encodedRle.length / 2, data.length);
        expect(Array.from(decoded)).toEqual([10n, 10n, 20n, 20n, 20n]);
    });

    it("should decode empty unsigned RLE Float64", () => {
        const data = new Float64Array([]);
        const encodedRle = encodeUnsignedRleFloat64(data);
        const decoded = decodeUnsignedRleFloat64(encodedRle, encodedRle.length / 2, data.length);
        expect(Array.from(decoded)).toEqual([]);
    });

    it("should decode unsigned RLE Float64", () => {
        const data = new Float64Array([10.5, 10.5, 20.5, 20.5, 20.5]);
        const encodedRle = encodeUnsignedRleFloat64(data);
        const decoded = decodeUnsignedRleFloat64(encodedRle, encodedRle.length / 2, data.length);
        expect(Array.from(decoded)).toEqual([10.5, 10.5, 20.5, 20.5, 20.5]);
    });

    it("should decode zigzag delta Int64", () => {
        const data = new BigInt64Array([1n, 2n, 3n, 5n, 6n, 7n]);
        encodeZigZagDeltaInt64(data);
        decodeZigZagDeltaInt64(data);
        expect(Array.from(data)).toEqual([1n, 2n, 3n, 5n, 6n, 7n]);
    });

    it("should apply fast inverse delta", () => {
        const data = new Int32Array([10, 15, 18, 20]);
        fastInverseDelta(data);
        encodeDeltaInt32(data);
        expect(Array.from(data)).toEqual([10, 15, 18, 20]);
    });

    it("should decode empty nullable zigzag delta Int32", () => {
        const encodedData = encodeNullableZigZagDeltaInt32(new Int32Array([]));
        const decoded = decodeNullableZigZagDeltaInt32(encodedData.bitVector, encodedData.encodedData);
        expect(Array.from(decoded)).toEqual([]);
    });

    it("should decode nullable zigzag delta Int32", () => {
        const encodedData = encodeNullableZigZagDeltaInt32(new Int32Array([1, 2]));
        const decoded = decodeNullableZigZagDeltaInt32(encodedData.bitVector, encodedData.encodedData);
        expect(Array.from(decoded)).toEqual([1, 2]);
    });

    it("should decode empty nullable zigzag delta Int64", () => {
        const encodedData = encodeNullableZigZagDeltaInt64(new BigInt64Array([]));
        const decoded = decodeNullableZigZagDeltaInt64(encodedData.bitVector, encodedData.encodedData);
        expect(Array.from(decoded)).toEqual([]);
    });

    it("should decode nullable zigzag delta Int64", () => {
        const encodedData = encodeNullableZigZagDeltaInt64(new BigInt64Array([1n, 2n]));
        const decoded = decodeNullableZigZagDeltaInt64(encodedData.bitVector, encodedData.encodedData);
        expect(Array.from(decoded)).toEqual([1n, 2n]);
    });

    it("should pad Int64 with zeros", () => {
        const bitVectorData = new Uint8Array([0b00000011]);
        const bitVector = new BitVector(bitVectorData, 3);
        const data = new BigInt64Array([10n, 20n]);
        const decoded = padWithZerosInt64(bitVector, data);
        expect(Array.from(decoded)).toEqual([10n, 20n, 0n]);
    });

    it("should pad zigzag Int64 with zeros", () => {
        const bitVectorData = new Uint8Array([0b00000101]);
        const bitVector = new BitVector(bitVectorData, 3);
        const data = new BigInt64Array([2n, 4n]);
        const decoded = padZigZagWithZerosInt64(bitVector, data);
        expect(Array.from(decoded)).toEqual([1n, 0n, 2n]);
    });

    it("should decode empty delta RLE Int64", () => {
        const data = new BigInt64Array([]);
        const encoded = encodeDeltaRleInt64(data);
        const decoded = decodeDeltaRleInt64(encoded.encodedData, encoded.numRuns, encoded.numValues);
        expect(Array.from(decoded)).toEqual([]);
    });

    it("should decode delta RLE Int64", () => {
        const data = new BigInt64Array([1n, 2n, 3n, 5n, 6n, 7n]);
        const encoded = encodeDeltaRleInt64(data);
        const decoded = decodeDeltaRleInt64(encoded.encodedData, encoded.numRuns, encoded.numValues);
        expect(Array.from(decoded)).toEqual([1n, 2n, 3n, 5n, 6n, 7n]);
    });

    describe("Const and Sequence RLE", () => {
        it("should decode unsigned const RLE Int64", () => {
            const data = new BigInt64Array([5n, 42n]);
            expect(decodeUnsignedConstRleInt64(data)).toBe(42n);
        });

        it("should decode zigzag const RLE Int64", () => {
            const data = new BigInt64Array([5n, encodeZigZagInt64Value(2n)]);
            expect(decodeZigZagConstRleInt64(data)).toBe(2n);
        });

        it("should decode zigzag sequence RLE Int64", () => {
            const data = new BigInt64Array([5n, 2n]);
            const [base, delta] = decodeZigZagSequenceRleInt64(data);
            expect(base).toBe(1n);
            expect(delta).toBe(1n);
        });
    });

    it("should decode empty ZigZag RLE Int32", () => {
        const data = new Int32Array([]);
        const encoded = encodeZigZagRleInt32(data);
        const decoded = decodeZigZagRleInt32(encoded.encodedData, encoded.numRuns, encoded.numTotalValues);
        expect(Array.from(decoded)).toEqual([]);
    });

    it("should decode ZigZag RLE Int32", () => {
        const encoded = new Int32Array([2, 2, 3, 3, 3]);
        const encodedData = encodeZigZagRleInt32(encoded);
        const decoded = decodeZigZagRleInt32(encodedData.encodedData, encodedData.numRuns, encodedData.numTotalValues);
        expect(Array.from(decoded)).toEqual([2, 2, 3, 3, 3]);
    });

    it("should decode empty ZigZag RLE Int64", () => {
        const data = new BigInt64Array([]);
        const encoded = encodeZigZagRleInt64(data);
        const decoded = decodeZigZagRleInt64(encoded.encodedData, encoded.numRuns, encoded.numTotalValues);
        expect(Array.from(decoded)).toEqual([]);
    });

    it("should decode ZigZag RLE Int64", () => {
        const encoded = new BigInt64Array([2n, 2n, 3n, 3n, 3n]);
        const encodedData = encodeZigZagRleInt64(encoded);
        const decoded = decodeZigZagRleInt64(encodedData.encodedData, encodedData.numRuns, encodedData.numTotalValues);
        expect(Array.from(decoded)).toEqual([2n, 2n, 3n, 3n, 3n]);
    });

    it("should decode empty ZigZag RLE Float64", () => {
        const data = new Float64Array([]);
        const encoded = encodeZigZagRleFloat64(data);
        const decoded = decodeZigZagRleFloat64(encoded.encodedData, encoded.numRuns, encoded.numTotalValues);
        expect(Array.from(decoded)).toEqual([]);
    });

    it("should decode ZigZag RLE Float64", () => {
        const encoded = new Float64Array([2, 2, 3, 3, 3]);
        const encodedData = encodeZigZagRleFloat64(encoded);
        const decoded = decodeZigZagRleFloat64(
            encodedData.encodedData,
            encodedData.numRuns,
            encodedData.numTotalValues,
        );
        expect(Array.from(decoded)).toEqual([2, 2, 3, 3, 3]);
    });

    describe("ZigZag RLE Delta", () => {
        it("should decode zigzag RLE delta", () => {
            const data = new Int32Array([2, 2, 2, 2]);
            const decoded = decodeZigZagRleDeltaInt32(data, 2, 4);
            expect(decoded.length).toBe(5);
        });
    });
});
