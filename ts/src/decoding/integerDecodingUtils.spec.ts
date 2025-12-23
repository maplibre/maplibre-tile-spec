import { describe, it, expect } from "vitest";
import {
    decodeUnsignedConstRleInt32,
    decodeUnsignedConstRleInt64,
    decodeZigZagConstRleInt32,
    decodeZigZagConstRleInt64,
    decodeZigZagSequenceRleInt32,
    decodeZigZagSequenceRleInt64,
    decodeZigZagRleInt32,
    decodeZigZagRleInt64,
    decodeZigZagDeltaOfDeltaInt32,
    fastInverseDelta,
    inverseDelta,
    decodeComponentwiseDeltaVec2,
    decodeComponentwiseDeltaVec2Scaled,
    decodeVarintInt32,
    decodeZigZagDeltaInt64,
    decodeZigZagDeltaFloat64,
    decodeZigZagDeltaInt32,
    decodeUnsignedRleInt32,
    decodeUnsignedRleInt64,
    decodeVarintInt64,
} from "./integerDecodingUtils";
import IntWrapper from "./intWrapper";
import { encodeZigZagInt64Value } from "../encoding/integerEncodingUtils";

/**
 * TypedArray comparison helper to avoid relying on deep-equality behavior
 * across different ArrayBuffer-backed types (e.g. Int32Array, BigInt64Array, Float64Array).
 */
function expectArrayLikeEqual<T extends ArrayLike<number> | ArrayLike<bigint>>(actual: T, expected: T) {
    expect(actual.length).toBe(expected.length);
    for (let i = 0; i < actual.length; i++) {
        expect(actual[i]).toBe(expected[i]);
    }
}

/**
 * Scope:
 * This file covers decoding-only invariants and non-canonical / edge-case encodings.
 * Canonical encode->decode roundtrip tests live in integerEncodingUtils.spec.ts.
 *
 * Rationale:
 * Keeping these concerns separated makes it clear which tests validate decoder robustness
 * versus which tests validate full roundtrip behavior.
 */
describe("IntegerDecodingUtils", () => {
    describe("Const RLE", () => {
        it("should decode unsigned const RLE Int32", () => {
            const data = new Int32Array([1, 42]);
            const result = decodeUnsignedConstRleInt32(data);
            expect(result).toBe(42);
        });

        it("should decode unsigned const RLE Int64", () => {
            const data = new BigInt64Array([5n, 42n]);
            expect(decodeUnsignedConstRleInt64(data)).toBe(42n);
        });

        it("should decode zigzag const RLE Int32", () => {
            const data = new Int32Array([1, 84]); // ZigZag(42) = 84
            const result = decodeZigZagConstRleInt32(data);
            expect(result).toBe(42);
        });

        it("should decode zigzag const RLE Int64", () => {
            const data = new BigInt64Array([5n, encodeZigZagInt64Value(2n)]);
            expect(decodeZigZagConstRleInt64(data)).toBe(2n);
        });
    });

    describe("Sequence RLE", () => {
        it("should decode zigzag sequence RLE Int32 (base == delta)", () => {
            const data = new Int32Array([1, 4]); // ZigZag(2) = 4
            const [base, delta] = decodeZigZagSequenceRleInt32(data);
            expect(base).toBe(2);
            expect(delta).toBe(2);
        });

        it("should decode zigzag sequence RLE Int32 (base != delta)", () => {
            const data = new Int32Array([0, 0, 4, 2]); // ZigZag(2)=4, ZigZag(1)=2
            const [base, delta] = decodeZigZagSequenceRleInt32(data);
            expect(base).toBe(2);
            expect(delta).toBe(1);
        });

        it("should decode zigzag sequence RLE Int64 (base == delta)", () => {
            const data = new BigInt64Array([5n, 2n]); // ZigZag(1n) = 2n
            const [base, delta] = decodeZigZagSequenceRleInt64(data);
            expect(base).toBe(1n);
            expect(delta).toBe(1n);
        });

        it("should decode zigzag sequence RLE Int64 (base != delta)", () => {
            const data = new BigInt64Array([5n, 2n, 5n, 2n]);
            const [base, delta] = decodeZigZagSequenceRleInt64(data);
            expect(base).toBe(-3n);
            expect(delta).toBe(1n);
        });
    });

    describe("Optimized decoding paths & heuristics", () => {
        it("decodeZigZagDeltaOfDeltaInt32 correctly reconstructs values", () => {
            // Uses a known encoded sequence for which the current implementation yields [0, 10, 40, 100].
            // This fixture protects against accidental regression of the specific delta-of-delta reconstruction algorithm.
            const data = new Int32Array([20, 40, 60]);
            const result = decodeZigZagDeltaOfDeltaInt32(data);
            expect(result.length).toBe(4);
            expectArrayLikeEqual(result, new Int32Array([0, 10, 40, 100]));
        });

        it("decodeComponentwiseDeltaVec2 triggers optimization loop (>= 4 items)", () => {
            // x0=0, y0=0. x1=1, y1=1 => dx=1, dy=1. ZigZag(1)=2.
            const data = new Int32Array([0, 0, 2, 2, 2, 2, 2, 2]);
            decodeComponentwiseDeltaVec2(data);
            expectArrayLikeEqual(data, new Int32Array([0, 0, 1, 1, 2, 2, 3, 3]));
        });

        it("decodeComponentwiseDeltaVec2Scaled triggers optimization loop (>= 4 items)", () => {
            const scale = 1;
            const min = -100;
            const max = 100;
            const data = new Int32Array(256).fill(20); // ZigZag(10) = 20

            decodeComponentwiseDeltaVec2Scaled(data, scale, min, max);

            // First pair (base)
            expect(data[0]).toBe(10);
            expect(data[1]).toBe(10);
            // Second pair (base + first delta)
            expect(data[2]).toBe(20);
            expect(data[3]).toBe(20);
            // End values should be clamped
            expect(data[254]).toBe(100);
            expect(data[255]).toBe(100);
        });

        it("should decode 5-byte varint32 (triggers remainder path)", () => {
            const buffer = new Uint8Array([0x80, 0x80, 0x80, 0x80, 0x01]);
            const off = new IntWrapper(0);
            const result = decodeVarintInt32(buffer, off, 1);
            expect(result[0]).toBe(268435456);
            expect(off.get()).toBe(5);
        });

        it("decodeVarintInt64 decodes multi-byte values and advances offset", () => {
            // 16385 = 0x4001 -> encoded as 0x81, 0x80, 0x01
            const buffer = new Uint8Array([0x81, 0x80, 0x01]);
            const off = new IntWrapper(0);
            const result = decodeVarintInt64(buffer, off, 1);
            expect(result[0]).toBe(16385n);
            expect(off.get()).toBe(3);
        });

        it("decodeVarintInt64 handles truncated input safely", () => {
            const buffer = new Uint8Array([0x80, 0x80]); // No terminating byte (msb=0)
            const off = new IntWrapper(0);
            const result = decodeVarintInt64(buffer, off, 1);
            // Contract: consumes available bytes up to end of buffer and advances offset.
            // Returning 0n or a partial value is implementation-defined, but offset must be safe.
            expect(off.get()).toBe(2);
            expect(result.length).toBe(1);
            expect(typeof result[0]).toBe("bigint");
        });

        it("decodeZigZagDeltaInt64 triggers optimization loop (>= 4 items)", () => {
            const count = 16;
            const data = new BigInt64Array(count).fill(2n); // ZigZag(1) = 2
            decodeZigZagDeltaInt64(data);
            expect(data[0]).toBe(1n);
            expect(data[1]).toBe(2n);
            expect(data[2]).toBe(3n);
            expect(data[15]).toBe(16n);
        });

        it("decodeZigZagDeltaFloat64 triggers optimization loop (>= 4 items)", () => {
            const count = 16;
            const data = new Float64Array(count).fill(2.0); // ZigZag(1.0) = 2.0
            decodeZigZagDeltaFloat64(data);
            expect(data[0]).toBe(1.0);
            expect(data[1]).toBe(2.0);
            expect(data[2]).toBe(3.0);
            expect(data[15]).toBe(16.0);
        });

        it("decodeZigZagDeltaInt32 triggers optimization loop (>= 4 items)", () => {
            const count = 16;
            const data = new Int32Array(count).fill(2); // ZigZag(1) = 2
            decodeZigZagDeltaInt32(data);
            expect(data[0]).toBe(1);
            expect(data[1]).toBe(2);
            expect(data[2]).toBe(3);
            expect(data[15]).toBe(16);
        });

        it("decodeUnsignedRleInt32 calculates total values if undefined", () => {
            // 1 run of length 5, value 10
            const encoded = new Int32Array([5, 10]);
            const decoded = decodeUnsignedRleInt32(encoded, 1, undefined);
            expect(decoded.length).toBe(5);
            expect(decoded[0]).toBe(10);
        });

        it("decodeUnsignedRleInt64 calculates total values if undefined", () => {
            // 1 run of length 5, value 10n
            const encoded = new BigInt64Array([5n, 10n]);
            const decoded = decodeUnsignedRleInt64(encoded, 1, undefined);
            expect(decoded.length).toBe(5);
            expect(decoded[0]).toBe(10n);
        });

        it("fastInverseDelta triggers unrolled loop (>= 4 items)", () => {
            // Values: 1, 1, 1, 1, 1, 1, 1, 1 -> Prefix sum: 1, 2, 3, 4, 5, 6, 7, 8
            const data = new Int32Array([1, 1, 1, 1, 1, 1, 1, 1]);
            fastInverseDelta(data);
            expectArrayLikeEqual(data, new Int32Array([1, 2, 3, 4, 5, 6, 7, 8]));
        });

        it("inverseDelta computes prefix sum correctly", () => {
            const data = new Int32Array([10, -5, -2, 10]);
            inverseDelta(data);
            expectArrayLikeEqual(data, new Int32Array([10, 5, 3, 13]));
        });
    });

    describe("Internal Logic & Heuristics", () => {
        it("decodeZigZagRleInt32 calculates total values if undefined", () => {
            const numRuns = 1;
            const runLength = 5;
            const val = 10; // ZigZag(5)
            const encoded = new Int32Array([runLength, val]);

            // Pass undefined explicitly
            const decoded = decodeZigZagRleInt32(encoded, numRuns, undefined);
            expect(decoded.length).toBe(runLength);
            expect(decoded[0]).toBe(5);
        });

        it("decodeZigZagRleInt64 calculates total values if undefined", () => {
            const numRuns = 1;
            const runLength = 5n;
            const val = 10n; // ZigZag(5) = 10
            const encoded = new BigInt64Array([runLength, val]);

            const decoded = decodeZigZagRleInt64(encoded, numRuns, undefined);
            expect(decoded.length).toBe(5);
            expect(decoded[0]).toBe(5n);
            expect(decoded[4]).toBe(5n);
        });
    });
});
