import { describe, expect, it } from "vitest";

import IntWrapper from "../decoding/intWrapper";
import {
    decodeVarintInt32,
    decodeVarintInt64,
    decodeUnsignedRleInt32,
    decodeUnsignedRleInt64,
    decodeZigZagRleInt32,
    decodeZigZagRleInt64,
    decodeZigZagDeltaInt32,
    decodeZigZagDeltaInt64,
    fastInverseDelta,
    decodeRleDeltaInt32,
    decodeZigZagRleDeltaInt32,
    decodeDeltaRleInt32,
    decodeDeltaRleInt64,
    decodeVarintFloat64,
    decodeZigZagFloat64,
    decodeUnsignedRleFloat64,
    decodeZigZagRleFloat64,
    decodeZigZagDeltaFloat64
} from "../decoding/integerDecodingUtils";
import BitVector from "../vector/flat/bitVector";
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
    encodeVarintFloat64,
    encodeZigZagFloat64,
    encodeUnsignedRleFloat64,
    encodeZigZagRleFloat64,
    encodeZigZagDeltaFloat64,
    encodeNullableUnsignedRleInt32,
    encodeNullableZigZagRleInt32,
    encodeNullableUnsignedRleInt64,
    encodeNullableZigZagRleInt64,
    encodeNullableZigZagDeltaInt32,
    encodeNullableZigZagDeltaInt64,
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
        expect(Array.from(buf.slice(0, offset.get()))).toEqual([0xac, 0x02]);
    });

    it("encodes and decodes array of varint int32 values (roundtrip)", () => {
        const values = new Int32Array([0, 1, 127, 128, 16383, 16384]);
        const encoded = encodeVarintInt32(values);
        const offset = new IntWrapper(0);
        const decoded = decodeVarintInt32(encoded, offset, values.length);
        expect(Array.from(decoded)).toEqual(Array.from(values));
        expect(offset.get()).toBe(encoded.length);
    });

    it("encodes and decodes varint int64 values (roundtrip)", () => {
        const values = new BigInt64Array([BigInt(0)]);
        const encoded = encodeVarintInt64(values);
        const offset = new IntWrapper(0);
        const decoded = decodeVarintInt64(encoded, offset, values.length);
        expect(Array.from(decoded)).toEqual(Array.from(values));
        expect(offset.get()).toBe(encoded.length);
    });

    it("encodes and decodes array of varint int64 values (roundtrip)", () => {
        const values = new BigInt64Array([BigInt(0), BigInt(127), BigInt(128), BigInt(16384)]);
        const encoded = encodeVarintInt64(values);
        const offset = new IntWrapper(0);
        const decoded = decodeVarintInt64(encoded, offset, values.length);
        expect(Array.from(decoded)).toEqual(Array.from(values));
        expect(offset.get()).toBe(encoded.length);
    });

    it("roundtrips int32 max varint length", () => {
        const values = new Int32Array([2147483647]);
        const encoded = encodeVarintInt32(values);
        const o = new IntWrapper(0);
        const decoded = decodeVarintInt32(encoded, o, 1);
        expect(Array.from(decoded)).toEqual(Array.from([2147483647]));
        expect(o.get()).toBe(encoded.length);
    });

    it("roundtrips int64 max varint length", () => {
        const maxVal = (1n << 63n) - 1n;
        const values = new BigInt64Array([maxVal]);
        const encoded = encodeVarintInt64(values);
        const o = new IntWrapper(0);
        const decoded = decodeVarintInt64(encoded, o, 1);
        expect(Array.from(decoded)).toEqual([maxVal]);
        expect(o.get()).toBe(encoded.length);
    });

    it("treats negative int64 varint inputs as invalid (returns 0n)", () => {
        const minVal = -(1n << 63n); // -2^63
        const values = new BigInt64Array([minVal]);
        const encoded = encodeVarintInt64(values);

        // Contract: encodeVarintInt64 is strictly unsigned.
        // Negative values are not supported and currently result in 0n (loop condition fails immediately).
        // Users must use ZigZag encoding for signed integers.
        const o = new IntWrapper(0);
        const decoded = decodeVarintInt64(encoded, o, 1);

        expect(decoded[0]).toBe(0n); // Current behavior validation
        expect(o.get()).toBe(encoded.length); // Ensure decoder consumed the (invalid) data
        expect(decoded[0]).not.toBe(values[0]); // Sanity check: does not roundtrip
    });

    it("roundtrips varint int32 with non-zero offset", () => {
        const prefix = new Uint8Array([0xff, 0xee, 0xdd]);
        const values = new Int32Array([42, 300, 16384]);

        const encoded = encodeVarintInt32(values);
        const buf = new Uint8Array(prefix.length + encoded.length);
        buf.set(prefix, 0);
        buf.set(encoded, prefix.length);

        const off = new IntWrapper(prefix.length);
        const decoded = decodeVarintInt32(buf, off, values.length);

        expect(Array.from(decoded)).toEqual(Array.from(values));
        expect(off.get()).toBe(buf.length);
    });
});

describe("integerEncodingUtils - ZigZag encoding", () => {
    it("encodes zigzag int32 value correctly", () => {
        expect(encodeZigZagInt32Value(0)).toBe(0);
        expect(encodeZigZagInt32Value(-1)).toBe(1);
        expect(encodeZigZagInt32Value(1)).toBe(2);
        expect(encodeZigZagInt32Value(-2)).toBe(3);
        expect(encodeZigZagInt32Value(2)).toBe(4);
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
        expect(encodeZigZagInt64Value(maxInt64)).toBe((1n << 64n) - 2n);
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
    it("encodes delta int32 in place and roundtrips", () => {
        const original = [10, 12, 15, 20];
        const data = new Int32Array(original);
        encodeDeltaInt32(data);
        expect(Array.from(data)).toEqual([10, 2, 3, 5]);

        fastInverseDelta(data);
        expect(Array.from(data)).toEqual(original);
    });

    it("handles empty input for delta int32", () => {
        const empty = new Int32Array([]);
        encodeDeltaInt32(empty);
        expect(Array.from(empty)).toEqual([]);
    });

    it("handles single value for delta int32", () => {
        const single = new Int32Array([5]);
        encodeDeltaInt32(single);
        expect(Array.from(single)).toEqual([5]);
    });

    it("encodes zigzag delta int32 in place and roundtrips", () => {
        const original = [10, 12, 15, 20];
        const data = new Int32Array(original);
        encodeZigZagDeltaInt32(data);
        expect(Array.from(data)).toEqual([20, 4, 6, 10]);

        decodeZigZagDeltaInt32(data);
        expect(Array.from(data)).toEqual(original);
    });

    it("encodes zigzag delta int64 in place and roundtrips", () => {
        const original = [10n, 12n, 15n];
        const data = new BigInt64Array(original);
        encodeZigZagDeltaInt64(data);
        expect(Array.from(data)).toEqual([20n, 4n, 6n]);

        decodeZigZagDeltaInt64(data);
        expect(Array.from(data)).toEqual(Array.from(original));
    });
});

describe("integerEncodingUtils - RLE encoding", () => {
    it("encodes and decodes unsigned RLE int32 (roundtrip)", () => {
        const input = new Int32Array([1, 1, 1, 2, 2, 3]);
        const result = encodeUnsignedRleInt32(input);
        expect(result.runs).toBe(3);
        // KEEP one exact wire format test
        expect(Array.from(result.data)).toEqual([3, 2, 1, 1, 2, 3]);

        const decoded = decodeUnsignedRleInt32(result.data, result.runs, input.length);
        expect(Array.from(decoded)).toEqual(Array.from(input));
    });

    it("encodes and decodes unsigned RLE int64 (roundtrip)", () => {
        const input = new BigInt64Array([1n, 1n, 2n]);
        const result = encodeUnsignedRleInt64(input);
        expect(result.runs).toBe(2);

        const decoded = decodeUnsignedRleInt64(result.data, result.runs, input.length);
        expect(Array.from(decoded)).toEqual(Array.from(input));
    });

    it("encodes and decodes zigzag RLE int32 (roundtrip)", () => {
        const input = new Int32Array([1, 1, 1, -2, -2, 3]);
        const result = encodeZigZagRleInt32(input);
        expect(result.runs).toBe(3);

        const decoded = decodeZigZagRleInt32(result.data, result.runs, result.numTotalValues);
        expect(Array.from(decoded)).toEqual(Array.from(input));
    });

    it("encodes and decodes zigzag RLE int64 (roundtrip)", () => {
        const input = new BigInt64Array([1n, 1n, -2n]);
        const result = encodeZigZagRleInt64(input);
        expect(result.runs).toBe(2);

        const decoded = decodeZigZagRleInt64(result.data, result.runs, result.numTotalValues);
        expect(Array.from(decoded)).toEqual(Array.from(input));
    });

    it("encodes unsigned RLE int32 with single run (all same - roundtrip)", () => {
        const input = new Int32Array(100).fill(7);
        const result = encodeUnsignedRleInt32(input);
        expect(result.runs).toBe(1);

        const decoded = decodeUnsignedRleInt32(result.data, result.runs, input.length);
        expect(Array.from(decoded)).toEqual(Array.from(input));
    });

    it("encodes unsigned RLE int32 with many short runs (alternating - roundtrip)", () => {
        const input = new Int32Array([1, 2, 1, 2, 1, 2, 1, 2]);
        const result = encodeUnsignedRleInt32(input);
        expect(result.runs).toBe(8);
        const decoded = decodeUnsignedRleInt32(result.data, result.runs, input.length);
        expect(Array.from(decoded)).toEqual(Array.from(input));
    });
});

describe("integerEncodingUtils - Combined encodings", () => {
    it("encodes and decodes RLE delta int32 (roundtrip)", () => {
        const input = new Int32Array([10, 12, 14, 16, 18]);
        const result = encodeRleDeltaInt32(input);

        const decoded = decodeRleDeltaInt32(result.data, result.runs, result.numTotalValues);
        // decodeRleDeltaInt32 returns a prefix base value (0) by design.
        expect(decoded.length).toBe(input.length + 1);
        expect(decoded[0]).toBe(0);
        expect(Array.from(decoded.subarray(1))).toEqual(Array.from(input));
    });

    it("handles empty input for RLE delta int32", () => {
        const input = new Int32Array([]);
        const result = encodeRleDeltaInt32(input);
        expect(result.runs).toBe(0);
        expect(result.numTotalValues).toBe(0);
        expect(result.data.length).toBe(0);
    });

    it("encodes and decodes zigzag RLE delta int32 (roundtrip)", () => {
        const input = new Int32Array([10, 8, 6, 4, 2]);
        const result = encodeZigZagRleDeltaInt32(input);

        const decoded = decodeZigZagRleDeltaInt32(result.data, result.runs, result.numTotalValues);
        // decodeZigZagRleDeltaInt32 returns a prefix base value (0) by design.
        expect(decoded.length).toBe(input.length + 1);
        expect(decoded[0]).toBe(0);
        expect(Array.from(decoded.subarray(1))).toEqual(Array.from(input));
    });

    it("handles empty input for zigzag RLE delta int32", () => {
        const input = new Int32Array([]);
        const result = encodeZigZagRleDeltaInt32(input);
        expect(result.runs).toBe(0);
        expect(result.numTotalValues).toBe(0);
        expect(result.data.length).toBe(0);
    });

    it("encodes and decodes delta RLE int32 (roundtrip)", () => {
        const input = new Int32Array([10, 12, 14, 16, 18]);
        const result = encodeDeltaRleInt32(input);

        const decoded = decodeDeltaRleInt32(result.data, result.runs, result.numValues);
        expect(Array.from(decoded)).toEqual(Array.from(input));
    });

    it("encodes and decodes delta RLE int64 (roundtrip)", () => {
        const input = new BigInt64Array([10n, 12n, 14n]);
        const result = encodeDeltaRleInt64(input);

        const decoded = decodeDeltaRleInt64(result.data, result.runs, result.numValues);
        expect(Array.from(decoded)).toEqual(Array.from(input));
    });

    it("handles empty input for delta RLE int32", () => {
        const input = new Int32Array([]);
        const result = encodeDeltaRleInt32(input);
        expect(result.runs).toBe(0);
        expect(result.numValues).toBe(0);
    });

    it("handles empty input for delta RLE int64", () => {
        const input = new BigInt64Array([]);
        const result = encodeDeltaRleInt64(input);
        expect(result.runs).toBe(0);
        expect(result.numValues).toBe(0);
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

describe("integerEncodingUtils - Nullable encodings", () => {
    // Helper to create BitVector from boolean array
    function createBitVector(bits: boolean[]): BitVector {
        const byteLength = Math.ceil(bits.length / 8);
        const buffer = new Uint8Array(byteLength);
        const bv = new BitVector(buffer, bits.length);
        for (let i = 0; i < bits.length; i++) {
            if (bits[i]) bv.set(i, true);
        }
        return bv;
    }

    describe("encodeNullableUnsignedRleInt32", () => {
        it("encodes with all values present", () => {
            const values = new Int32Array([1, 1, 2, 2, 2, 3]);
            const bitVector = createBitVector([true, true, true, true, true, true]);
            const result = encodeNullableUnsignedRleInt32(values, bitVector);
            expect(result.numRuns).toBe(3);
            // Format: [lengths... | values...]
            expect(Array.from(result.data)).toEqual([2, 3, 1, 1, 2, 3]);
        });

        it("encodes with some null values", () => {
            // Values: [1, null, 1, 2, null, 2]
            const values = new Int32Array([1, 0, 1, 2, 0, 2]);
            const bitVector = createBitVector([true, false, true, true, false, true]);
            const result = encodeNullableUnsignedRleInt32(values, bitVector);
            expect(result.numRuns).toBe(2); // Run of 1s (count=2), run of 2s (count=2)
        });

        it("handles empty input", () => {
            const values = new Int32Array([]);
            const bitVector = createBitVector([]);
            const result = encodeNullableUnsignedRleInt32(values, bitVector);
            expect(result.numRuns).toBe(0);
        });

        it("encodes with all values null", () => {
            const values = new Int32Array([0, 0, 0]);
            const bitVector = createBitVector([false, false, false]);
            const result = encodeNullableUnsignedRleInt32(values, bitVector);
            expect(result.numRuns).toBe(0);
            expect(result.data.length).toBe(0);
        });
    });

    describe("encodeNullableZigZagRleInt32", () => {
        it("encodes with all values present", () => {
            const values = new Int32Array([1, 1, -2, -2, 3]);
            const bitVector = createBitVector([true, true, true, true, true]);
            const result = encodeNullableZigZagRleInt32(values, bitVector);
            expect(result.runs).toBe(3);
        });

        it("encodes with some null values", () => {
            const values = new Int32Array([1, 0, 1, -2, 0]);
            const bitVector = createBitVector([true, false, true, true, false]);
            const result = encodeNullableZigZagRleInt32(values, bitVector);
            expect(result.runs).toBeGreaterThanOrEqual(1);
        });

        it("handles empty input", () => {
            const values = new Int32Array([]);
            const bitVector = createBitVector([]);
            const result = encodeNullableZigZagRleInt32(values, bitVector);
            expect(result.runs).toBe(0);
        });
    });

    describe("encodeNullableUnsignedRleInt64", () => {
        it("encodes with all values present", () => {
            const values = new BigInt64Array([1n, 1n, 2n, 2n]);
            const bitVector = createBitVector([true, true, true, true]);
            const result = encodeNullableUnsignedRleInt64(values, bitVector);
            expect(result.numRuns).toBe(2);
        });

        it("encodes with some null values", () => {
            const values = new BigInt64Array([1n, 0n, 1n, 2n]);
            const bitVector = createBitVector([true, false, true, true]);
            const result = encodeNullableUnsignedRleInt64(values, bitVector);
            expect(result.numRuns).toBeGreaterThanOrEqual(1);
        });

        it("encodes with null values at start", () => {
            const values = new BigInt64Array([0n, 1n, 2n]);
            const bitVector = createBitVector([false, true, true]);
            const result = encodeNullableUnsignedRleInt64(values, bitVector);
            expect(result.numRuns).toBe(2);
        });

        it("encodes with all values null", () => {
            const values = new BigInt64Array([0n, 0n, 0n]);
            const bitVector = createBitVector([false, false, false]);
            const result = encodeNullableUnsignedRleInt64(values, bitVector);
            expect(result.numRuns).toBe(0);
            expect(result.data.length).toBe(0);
        });
    });

    describe("encodeNullableZigZagRleInt64", () => {
        it("encodes with all values present", () => {
            const values = new BigInt64Array([1n, 1n, -2n, -2n]);
            const bitVector = createBitVector([true, true, true, true]);
            const result = encodeNullableZigZagRleInt64(values, bitVector);
            expect(result.numRuns).toBe(2);
        });

        it("handles empty input", () => {
            const values = new BigInt64Array([]);
            const bitVector = createBitVector([]);
            const result = encodeNullableZigZagRleInt64(values, bitVector);
            expect(result.numRuns).toBe(0);
        });
    });

    describe("encodeNullableZigZagDeltaInt32", () => {
        it("encodes with all values present", () => {
            const values = new Int32Array([10, 12, 15, 20]);
            const result = encodeNullableZigZagDeltaInt32(values);
            expect(result.totalSize).toBe(4);
            expect(result.data.length).toBe(4);
        });

        it("handles empty input", () => {
            const values = new Int32Array([]);
            const result = encodeNullableZigZagDeltaInt32(values);
            expect(result.totalSize).toBe(0);
        });

        it("handles single value", () => {
            const values = new Int32Array([42]);
            const result = encodeNullableZigZagDeltaInt32(values);
            expect(result.totalSize).toBe(1);
        });
    });

    describe("encodeNullableZigZagDeltaInt64", () => {
        it("encodes with values", () => {
            const values = new BigInt64Array([10n, 12n, 15n]);
            const result = encodeNullableZigZagDeltaInt64(values);
            expect(result.totalSize).toBe(3);
        });

        it("handles empty input", () => {
            const values = new BigInt64Array([]);
            const result = encodeNullableZigZagDeltaInt64(values);
            expect(result.totalSize).toBe(0);
        });
    });
});

describe("integerEncodingUtils - Float64 encodings", () => {
    it("encodes and decodes varint float64 (roundtrip)", () => {
        // Varint float64 supports up to 53-bit integers safe range
        // 9007199254740991 is Number.MAX_SAFE_INTEGER
        const values = new Float64Array([0, 1.0, 123456.0, 9007199254740991]);
        const encoded = encodeVarintFloat64(values);

        const offset = new IntWrapper(0);
        const decoded = decodeVarintFloat64(encoded, offset, values.length);

        expect(Array.from(decoded)).toEqual(Array.from(values));
        expect(offset.get()).toBe(encoded.length);
    });

    it("encodes zigzag float64 in place (roundtrip)", () => {
        // Checking standard behavior with integers stored as floats
        const data = new Float64Array([0, -1, 1, -2, 2]);
        encodeZigZagFloat64(data);

        // It's in-place modification
        decodeZigZagFloat64(data);
        expect(Array.from(data)).toEqual([0, -1, 1, -2, 2]);
    });

    it("handles empty input for zigzag delta float64", () => {
        const data = new Float64Array([]);
        encodeZigZagDeltaFloat64(data);
        expect(data.length).toBe(0);
    });

    it("handles empty input for zigzag RLE float64", () => {
        const input = new Float64Array([]);
        const result = encodeZigZagRleFloat64(input);
        expect(result.runs).toBe(0);
        expect(result.numTotalValues).toBe(0);
    });

    it("encodes unsigned RLE float64 (roundtrip)", () => {
        const input = new Float64Array([1.0, 1.0, 2.0, 2.0, 3.0]);
        const result = encodeUnsignedRleFloat64(input);
        expect(result.runs).toBe(3);

        const decoded = decodeUnsignedRleFloat64(result.data, result.runs, input.length);
        expect(Array.from(decoded)).toEqual(Array.from(input));
    });

    it("encodes zigzag RLE float64 (roundtrip)", () => {
        const input = new Float64Array([1.0, 1.0, -2.0, -2.0, 3.0]);
        const result = encodeZigZagRleFloat64(input);
        expect(result.runs).toBe(3);

        const decoded = decodeZigZagRleFloat64(result.data, result.runs, result.numTotalValues);
        expect(Array.from(decoded)).toEqual(Array.from(input));
    });

    it("encodes zigzag delta float64 in place (roundtrip)", () => {
        const original = [10.0, 12.0, 15.0, 20.0];
        const data = new Float64Array(original);
        encodeZigZagDeltaFloat64(data);
        // [20, 4, 6, 10] (zigzag encoded deltas)

        decodeZigZagDeltaFloat64(data);
        expect(Array.from(data)).toEqual(original);
    });
});
