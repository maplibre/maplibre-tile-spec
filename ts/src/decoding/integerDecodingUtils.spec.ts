
// This file focuses on decoding-only invariants and non-canonical encodings.
// Canonical encode->decode roundtrip tests live in integerEncodingUtils.spec.ts.

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
    decodeFastPfor,
    decodeVarintInt32,
    decodeZigZagDeltaInt64,
    decodeZigZagDeltaFloat64,
    decodeZigZagDeltaInt32,
    decodeUnsignedRleInt32,
    decodeUnsignedRleInt64,
} from "./integerDecodingUtils";
import IntWrapper from "./intWrapper";
import {
    encodeZigZagInt64Value,
    encodeFastPfor,
} from "../encoding/integerEncodingUtils";

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

    describe("Vector & Delta Optimizations", () => {
        it("decodeZigZagDeltaOfDeltaInt32 correctly reconstructs values", () => {
            // Uses a known encoded sequence for which the current implementation yields [0, 10, 40, 100].
            const data = new Int32Array([20, 40, 60]);
            const result = decodeZigZagDeltaOfDeltaInt32(data);
            expect(result.length).toBe(4);
            expect(Array.from(result)).toEqual([0, 10, 40, 100]);
        });

        it("decodeComponentwiseDeltaVec2 triggers optimization loop (>= 4 items)", () => {
            // x0=0, y0=0. x1=1, y1=1 => dx=1, dy=1. ZigZag(1)=2.
            const data = new Int32Array([0, 0, 2, 2, 2, 2, 2, 2]);
            decodeComponentwiseDeltaVec2(data);
            expect(Array.from(data)).toEqual([0, 0, 1, 1, 2, 2, 3, 3]);
        });

        it("decodeComponentwiseDeltaVec2Scaled triggers optimization loop (>= 4 items)", () => {
            // Deterministic test with enough items to trigger heavy JIT/loop unrolling branches
            const scale = 1;
            const min = -100;
            const max = 100;
            // 256 items (multiple of 16 to ensure logic coverage if steps are large)
            const data = new Int32Array(256).fill(20);
            // ZigZag(10)=20.
            // Decodes to (10, 10) deltas.
            // Final expected value logic:
            // prevX/Y starts at ZigZag(20)->10.
            // Then 127 pairs follow. Each adds (10, 10).
            // Values: (10,10), (20,20), (30,30)... (1280,1280).
            // But clamped to [-100, 100].
            // So eventually becomes 100, 100 ...

            decodeComponentwiseDeltaVec2Scaled(data, scale, min, max);
            expect(data[0]).toBe(10);
            expect(data[1]).toBe(10);
            expect(data[254]).toBe(100);
            expect(data[255]).toBe(100); // Clamped
        });

        it("should decode 5-byte varint32 (triggers remainder path)", () => {
            const buffer = new Uint8Array([0x80, 0x80, 0x80, 0x80, 0x01]);
            const off = new IntWrapper(0);
            const result = decodeVarintInt32(buffer, off, 1);
            expect(result[0]).toBe(268435456);
            expect(off.get()).toBe(5);
        });

        it("decodeZigZagDeltaInt64 triggers optimization loop (>= 4 items)", () => {
            // ZigZag(1) = 2.
            // Deltas: 1, 1, 1, 1...
            // Values: 1, 2, 3, 4...
            const count = 16;
            const data = new BigInt64Array(count).fill(2n);
            decodeZigZagDeltaInt64(data);
            // Verify.
            expect(data[0]).toBe(1n);
            expect(data[3]).toBe(4n);
            expect(data[15]).toBe(16n);
        });

        it("decodeZigZagDeltaFloat64 triggers optimization loop (>= 4 items)", () => {
            // ZigZag(1.0) = 2.0
            const count = 16;
            const data = new Float64Array(count).fill(2.0);
            decodeZigZagDeltaFloat64(data);
            expect(data[0]).toBe(1.0);
            expect(data[3]).toBe(4.0);
            expect(data[15]).toBe(16.0);
        });


        it("decodeZigZagDeltaInt32 triggers optimization loop (>= 4 items)", () => {
            // ZigZag(1) = 2.
            const count = 16;
            const data = new Int32Array(count).fill(2);
            decodeZigZagDeltaInt32(data);
            expect(data[0]).toBe(1);
            expect(data[3]).toBe(4);
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
            expect(Array.from(data)).toEqual([1, 2, 3, 4, 5, 6, 7, 8]);
        });

        it("inverseDelta computes prefix sum correctly", () => {
            const data = new Int32Array([10, -5, -2, 10]);
            inverseDelta(data);
            expect(Array.from(data)).toEqual([10, 5, 3, 13]);
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

            const decoded = decodeZigZagRleInt64(encoded, 1);
            expect(decoded.length).toBe(5);
            expect(decoded[0]).toBe(5n);
        });
    });

    describe("FastPFOR", () => {
        function generateBitWidthValues(count: number, bitWidth: number): Int32Array {
            const maxVal = bitWidth === 32 ? 0xffffffff >>> 0 : (1 << bitWidth) - 1;
            const data = new Int32Array(count);
            for (let i = 0; i < count; i++) {
                data[i] = ((i * 7 + 13) & maxVal) >>> 0;
            }
            if (count > 0) data[0] = maxVal;
            return data;
        }

        describe("roundtrip", () => {
            const cases: Array<{ name: string; data: Int32Array }> = [
                { name: "empty", data: new Int32Array([]) },
                { name: "small values", data: new Int32Array([1, 2, 3, 4, 5, 6, 7, 8]) },
                { name: "varied values", data: new Int32Array([100, 200, 300, 1000, 5000, 10000]) },
                { name: "all zeros (256)", data: new Int32Array(256) },
                { name: "constant 42 (256)", data: new Int32Array(256).fill(42) },
            ];

            for (const tc of cases) {
                it(`should roundtrip: ${tc.name}`, () => {
                    const encoded = encodeFastPfor(tc.data);
                    const decoded = decodeFastPfor(encoded, tc.data.length, encoded.length, new IntWrapper(0));
                    expect(Array.from(decoded)).toEqual(Array.from(tc.data));
                });
            }
        });

        describe("bit width dispatch", () => {
            const bitWidths = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 20, 24, 31, 32];
            for (const bw of bitWidths) {
                it(`should roundtrip bitwidth ${bw}`, () => {
                    const data = generateBitWidthValues(512, bw);
                    const encoded = encodeFastPfor(data);
                    const decoded = decodeFastPfor(encoded, data.length, encoded.length, new IntWrapper(0));
                    // Compare element by element to allow easier debugging if needed, or just Array.from
                    expect(Array.from(decoded)).toEqual(Array.from(data));
                });
            }
        });

        describe("block boundaries", () => {
            const sizes = [255, 256, 257, 511, 512, 513];
            for (const size of sizes) {
                it(`should roundtrip ${size} elements`, () => {
                    const data = Int32Array.from({ length: size }, (_, i) => i % 1000);
                    const encoded = encodeFastPfor(data);
                    const decoded = decodeFastPfor(encoded, data.length, encoded.length, new IntWrapper(0));
                    expect(Array.from(decoded)).toEqual(Array.from(data));
                });
            }
        });

        describe("exceptions (outliers)", () => {
            it("should handle sparse large outliers", () => {
                const data = new Int32Array(256).fill(7);
                data[10] = 100000;
                data[128] = 500000;
                data[250] = 999999;
                const encoded = encodeFastPfor(data);
                const decoded = decodeFastPfor(encoded, data.length, encoded.length, new IntWrapper(0));
                expect(Array.from(decoded)).toEqual(Array.from(data));
            });
        });

        describe("multi-page", () => {
            it("should roundtrip 66000 values", () => {
                const data = Int32Array.from({ length: 66000 }, (_, i) => i % 10000);
                const encoded = encodeFastPfor(data);
                const decoded = decodeFastPfor(encoded, data.length, encoded.length, new IntWrapper(0));
                expect(Array.from(decoded)).toEqual(Array.from(data));
            });
        });
    });
});
