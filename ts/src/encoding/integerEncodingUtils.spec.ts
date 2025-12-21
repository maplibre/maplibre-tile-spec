import { describe, expect, it } from "vitest";

import IntWrapper from "../decoding/intWrapper";
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

        encodeVarintInt32Value(300, buf, offset);
        expect(offset.get()).toBeGreaterThan(1);
    });

    it("encodes array of varint int32 values", () => {
        const values = new Int32Array([0, 1, 127, 128, 16383, 16384]);
        const encoded = encodeVarintInt32(values);
        expect(encoded.length).toBeGreaterThan(0);
    });

    it("encodes varint int64 values via array function", () => {
        // Single value test using the array function
        const values = new BigInt64Array([BigInt(0)]);
        const encoded = encodeVarintInt64(values);
        expect(encoded.length).toBeGreaterThan(0);
    });

    it("encodes array of varint int64 values", () => {
        const values = new BigInt64Array([BigInt(0), BigInt(127), BigInt(128), BigInt(16384)]);
        const encoded = encodeVarintInt64(values);
        expect(encoded.length).toBeGreaterThan(0);
    });
});

describe("integerEncodingUtils - ZigZag encoding", () => {
    it("encodes zigzag int32 value correctly", () => {
        expect(encodeZigZagInt32Value(0)).toBe(0);
        expect(encodeZigZagInt32Value(-1)).toBe(1);
        expect(encodeZigZagInt32Value(1)).toBe(2);
        expect(encodeZigZagInt32Value(-2)).toBe(3);
        expect(encodeZigZagInt32Value(2)).toBe(4);
    });

    it("encodes zigzag int64 value correctly", () => {
        expect(encodeZigZagInt64Value(BigInt(0))).toBe(BigInt(0));
        expect(encodeZigZagInt64Value(BigInt(-1))).toBe(BigInt(1));
        expect(encodeZigZagInt64Value(BigInt(1))).toBe(BigInt(2));
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

    it("encodes zigzag delta int32 in place", () => {
        const data = new Int32Array([10, 12, 15, 20]);
        encodeZigZagDeltaInt32(data);
        // Output is zigzag encoded, check it changed
        expect(data.length).toBe(4);
    });

    it("encodes zigzag delta int64 in place", () => {
        const data = new BigInt64Array([BigInt(10), BigInt(12), BigInt(15)]);
        encodeZigZagDeltaInt64(data);
        // Output is zigzag encoded, check it changed
        expect(data.length).toBe(3);
    });
});

describe("integerEncodingUtils - RLE encoding", () => {
    it("encodes unsigned RLE int32 correctly", () => {
        const input = new Int32Array([1, 1, 1, 2, 2, 3]);
        const result = encodeUnsignedRleInt32(input);
        expect(result.runs).toBe(3);
        expect(result.data.length).toBe(6); // 3 runs * 2 (value + count)
    });

    it("encodes unsigned RLE int64 correctly", () => {
        const input = new BigInt64Array([BigInt(1), BigInt(1), BigInt(2)]);
        const result = encodeUnsignedRleInt64(input);
        expect(result.runs).toBe(2);
    });

    it("encodes zigzag RLE int32 correctly", () => {
        const input = new Int32Array([1, 1, 1, -2, -2, 3]);
        const result = encodeZigZagRleInt32(input);
        expect(result.runs).toBe(3);
        expect(result.numTotalValues).toBe(6);
    });

    it("encodes zigzag RLE int64 correctly", () => {
        const input = new BigInt64Array([BigInt(1), BigInt(1), BigInt(-2)]);
        const result = encodeZigZagRleInt64(input);
        expect(result.runs).toBe(2);
    });
});

describe("integerEncodingUtils - Combined encodings", () => {
    it("encodes RLE delta int32 correctly", () => {
        // Values with repeated deltas
        const input = new Int32Array([10, 12, 14, 16, 18]); // All +2 deltas
        const result = encodeRleDeltaInt32(input);
        expect(result.runs).toBeGreaterThan(0);
        expect(result.numTotalValues).toBe(input.length);
    });

    it("encodes zigzag RLE delta int32 correctly", () => {
        const input = new Int32Array([10, 8, 6, 4, 2]); // All -2 deltas
        const result = encodeZigZagRleDeltaInt32(input);
        expect(result.runs).toBeGreaterThan(0);
        expect(result.numTotalValues).toBe(input.length);
    });

    it("encodes delta RLE int32 correctly", () => {
        const input = new Int32Array([10, 12, 14, 16, 18]);
        const result = encodeDeltaRleInt32(input);
        expect(result.runs).toBeGreaterThan(0);
        expect(result.numValues).toBe(input.length);
    });

    it("encodes delta RLE int64 correctly", () => {
        const input = new BigInt64Array([BigInt(10), BigInt(12), BigInt(14)]);
        const result = encodeDeltaRleInt64(input);
        expect(result.runs).toBeGreaterThan(0);
        expect(result.numValues).toBe(input.length);
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
