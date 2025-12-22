import { describe, expect, it } from "vitest";

import IntWrapper from "../decoding/intWrapper";
import { decodeVarintInt32, decodeVarintInt64 } from "../decoding/integerDecodingUtils";
import {
    encodeVarintInt32Value,
    encodeVarintInt32,
    encodeVarintInt64,
    encodeZigZagInt32Value,
    encodeZigZagInt64Value,
    encodeZigZagInt32,
    encodeZigZagInt64,
    encodeZigZagDeltaInt32,
    encodeZigZagDeltaInt64,
    encodeUnsignedRleInt32,
    encodeUnsignedRleInt64,
    encodeZigZagRleInt32,
    encodeZigZagRleInt64,
    encodeDeltaInt32,
    encodeRleDeltaInt32,
    encodeZigZagRleDeltaInt32,
    encodeDeltaRleInt32,
    encodeDeltaRleInt64,
} from "./integerEncodingUtils";

describe("integerEncodingUtils - Varint encoding", () => {
    it("encodes small varint int32 values correctly", () => {
        const buf = new Uint8Array(10);
        const offset = new IntWrapper(0);

        encodeVarintInt32Value(0, buf, offset);
        expect(offset.get()).toBe(1);
        expect(buf[0]).toBe(0);
    });

    it("encodes larger varint int32 values correctly", () => {
        const buf = new Uint8Array(10);
        const offset = new IntWrapper(0);

        // Varint protobuf of 300 = 0xAC 0x02
        encodeVarintInt32Value(300, buf, offset);
        expect(offset.get()).toBe(2);
        expect([...buf.slice(0, offset.get())]).toEqual([0xac, 0x02]);
    });

    it("encodes and decodes array of varint int32 values (roundtrip)", () => {
        const values = new Int32Array([0, 1, 127, 128, 16383, 16384]);
        const encoded = encodeVarintInt32(values);
        const offset = new IntWrapper(0);
        const decoded = decodeVarintInt32(encoded, offset, values.length);
        expect(decoded).toEqual(values);
        expect(offset.get()).toBe(encoded.length); // Verify all bytes consumed
    });

    it("encodes and decodes varint int64 values (roundtrip)", () => {
        const values = new BigInt64Array([BigInt(0)]);
        const encoded = encodeVarintInt64(values);
        const offset = new IntWrapper(0);
        const decoded = decodeVarintInt64(encoded, offset, values.length);
        expect(decoded).toEqual(values);
        expect(offset.get()).toBe(encoded.length); // Verify all bytes consumed
    });

    it("encodes and decodes array of varint int64 values (roundtrip)", () => {
        const values = new BigInt64Array([BigInt(0), BigInt(127), BigInt(128), BigInt(16384)]);
        const encoded = encodeVarintInt64(values);
        const offset = new IntWrapper(0);
        const decoded = decodeVarintInt64(encoded, offset, values.length);
        expect(decoded).toEqual(values);
        expect(offset.get()).toBe(encoded.length); // Verify all bytes consumed
    });
});

describe("integerEncodingUtils - ZigZag encoding", () => {
    it("encodes zigzag int32 value correctly", () => {
        expect(encodeZigZagInt32Value(0)).toBe(0);
        expect(encodeZigZagInt32Value(-1)).toBe(1);
        expect(encodeZigZagInt32Value(1)).toBe(2);
        expect(encodeZigZagInt32Value(-2)).toBe(3);
        expect(encodeZigZagInt32Value(2)).toBe(4);
        // INT32 limits (compare as unsigned bit pattern)
        expect(encodeZigZagInt32Value(2147483647) >>> 0).toBe(0xfffffffe);
        expect(encodeZigZagInt32Value(-2147483648) >>> 0).toBe(0xffffffff);
    });

    it("encodes zigzag int64 value correctly", () => {
        expect(encodeZigZagInt64Value(BigInt(0))).toBe(BigInt(0));
        expect(encodeZigZagInt64Value(BigInt(-1))).toBe(BigInt(1));
        expect(encodeZigZagInt64Value(BigInt(1))).toBe(BigInt(2));
    });

    it("encodes zigzag int64 limits", () => {
        const maxInt64 = (1n << 63n) - 1n;
        const minInt64 = -(1n << 63n);
        // ZigZag maps int64 range [-2^63, 2^63-1] to [0, 2^64-1]
        // ZigZag(max) = 2^64 - 2
        expect(encodeZigZagInt64Value(maxInt64)).toBe((1n << 64n) - 2n);

        // ZigZag(min) = 2^64 - 1
        expect(encodeZigZagInt64Value(minInt64)).toBe((1n << 64n) - 1n);
    });

    it("encodes array of zigzag int32 in place", () => {
        const data = new Int32Array([0, -1, 1, -2, 2]);
        encodeZigZagInt32(data);
        expect(Array.from(data)).toEqual([0, 1, 2, 3, 4]);
    });

    it("encodes array of zigzag int64 in place", () => {
        const data = new BigInt64Array([BigInt(0), BigInt(-1), BigInt(1)]);
        encodeZigZagInt64(data);
        expect(Array.from(data)).toEqual([BigInt(0), BigInt(1), BigInt(2)]);
    });
});

describe("integerEncodingUtils - Delta encoding", () => {
    it("encodes delta int32 in place", () => {
        const data = new Int32Array([10, 12, 15, 20]);
        encodeDeltaInt32(data);
        expect(Array.from(data)).toEqual([10, 2, 3, 5]);
    });

    it("handles empty input for delta int32", () => {
        const empty = new Int32Array([]);
        encodeDeltaInt32(empty);
        expect(empty).toEqual(new Int32Array([]));
    });

    it("handles single value for delta int32", () => {
        const single = new Int32Array([5]);
        encodeDeltaInt32(single);
        expect(single).toEqual(new Int32Array([5]));
    });

    it("encodes zigzag delta int32 in place", () => {
        const data = new Int32Array([10, 12, 15, 20]);
        encodeZigZagDeltaInt32(data);
        // First value: zigzag(10) = 20, then deltas: zigzag(2)=4, zigzag(3)=6, zigzag(5)=10
        expect(Array.from(data)).toEqual([20, 4, 6, 10]);
    });

    it("encodes zigzag delta int64 in place", () => {
        const data = new BigInt64Array([BigInt(10), BigInt(12), BigInt(15)]);
        encodeZigZagDeltaInt64(data);
        // First value: zigzag(10) = 20, then deltas: zigzag(2)=4, zigzag(3)=6
        expect(Array.from(data)).toEqual([BigInt(20), BigInt(4), BigInt(6)]);
    });
});

describe("integerEncodingUtils - RLE encoding", () => {
    it("encodes unsigned RLE int32 correctly", () => {
        const input = new Int32Array([1, 1, 1, 2, 2, 3]);
        const result = encodeUnsignedRleInt32(input);
        expect(result.runs).toBe(3);
        expect(result.data.length).toBe(6); // 2 * runs (lengths + values)
        // Verify exact data: format is [lengths... | values...] (first half lengths, second half values)
        // For runs: (3x1), (2x2), (1x3) => lengths=[3,2,1], values=[1,2,3]
        expect(Array.from(result.data)).toEqual([3, 2, 1, 1, 2, 3]);
    });

    it("encodes unsigned RLE int64 correctly", () => {
        const input = new BigInt64Array([BigInt(1), BigInt(1), BigInt(2)]);
        const result = encodeUnsignedRleInt64(input);
        expect(result.runs).toBe(2);
        expect(result.data.length).toBe(4); // 2 runs * 2
        // Format: [lengths... | values...] => [2, 1, 1, 2]
        expect(Array.from(result.data)).toEqual([2n, 1n, 1n, 2n]);
    });

    it("encodes zigzag RLE int32 correctly", () => {
        const input = new Int32Array([1, 1, 1, -2, -2, 3]);
        const result = encodeZigZagRleInt32(input);
        expect(result.runs).toBe(3);
        expect(result.numTotalValues).toBe(6);
        // zigzag(1)=2, zigzag(-2)=3, zigzag(3)=6
        // Format: [lengths... | zigzag_values...] => [3, 2, 1, 2, 3, 6]
        expect(Array.from(result.data)).toEqual([3, 2, 1, 2, 3, 6]);
    });

    it("encodes zigzag RLE int64 correctly", () => {
        const input = new BigInt64Array([BigInt(1), BigInt(1), BigInt(-2)]);
        const result = encodeZigZagRleInt64(input);
        expect(result.runs).toBe(2);
        // zigzag(1)=2, zigzag(-2)=3
        // Format: [lengths... | zigzag_values...] => [2, 1, 2, 3]
        expect(Array.from(result.data)).toEqual([2n, 1n, 2n, 3n]);
    });
});

describe("integerEncodingUtils - Combined encodings", () => {
    it("encodes RLE delta int32 correctly", () => {
        // Values with repeated deltas: [10,12,14,16,18] => deltas from 0: [10,2,2,2,2]
        const input = new Int32Array([10, 12, 14, 16, 18]);
        const result = encodeRleDeltaInt32(input);
        expect(result.runs).toBe(2); // (1x10), (4x2)
        expect(result.numTotalValues).toBe(input.length);
        // Format: [lengths... | deltas...] => [1, 4, 10, 2]
        expect(Array.from(result.data)).toEqual([1, 4, 10, 2]);
    });

    it("encodes zigzag RLE delta int32 correctly", () => {
        // Values: [10,8,6,4,2] => deltas from 0: [10,-2,-2,-2,-2]
        // zigzag: zigzag(10)=20, zigzag(-2)=3
        const input = new Int32Array([10, 8, 6, 4, 2]);
        const result = encodeZigZagRleDeltaInt32(input);
        expect(result.runs).toBe(2); // (1x20), (4x3)
        expect(result.numTotalValues).toBe(input.length);
        // Format: [lengths... | zigzag_deltas...] => [1, 4, 20, 3]
        expect(Array.from(result.data)).toEqual([1, 4, 20, 3]);
    });

    it("encodes delta RLE int32 correctly", () => {
        // Input: [10,12,14,16,18] => deltas from 0: [10,2,2,2,2]
        // zigzag: [20,4,4,4,4] => RLE: (1x20), (4x4)
        const input = new Int32Array([10, 12, 14, 16, 18]);
        const result = encodeDeltaRleInt32(input);
        expect(result.runs).toBe(2);
        expect(result.numValues).toBe(input.length);
        expect(result.data.length).toBe(4); // 2 * runs
        // Format: [lengths... | zigzag_deltas...] => [1, 4, 20, 4]
        expect(Array.from(result.data)).toEqual([1, 4, 20, 4]);
    });

    it("encodes delta RLE int64 correctly", () => {
        // Input: [10n,12n,14n] => deltas from 0: [10n,2n,2n]
        // zigzag: [20n,4n,4n] => RLE: (1x20), (2x4)
        const input = new BigInt64Array([BigInt(10), BigInt(12), BigInt(14)]);
        const result = encodeDeltaRleInt64(input);
        expect(result.runs).toBe(2);
        expect(result.numValues).toBe(input.length);
        expect(result.data.length).toBe(4); // 2 * runs
        // Format: [lengths... | zigzag_deltas...] => [1n, 2n, 20n, 4n]
        expect(Array.from(result.data)).toEqual([1n, 2n, 20n, 4n]);
    });

    it("handles empty input for RLE", () => {
        const input = new Int32Array(0);
        const result = encodeUnsignedRleInt32(input);
        expect(result.runs).toBe(0);
        expect(result.data.length).toBe(0);
    });

    it("handles single value for RLE", () => {
        const input = new Int32Array([42]);
        const result = encodeUnsignedRleInt32(input);
        expect(result.runs).toBe(1);
    });
});
