import { describe, it, expect } from "vitest";
import {
    decodeVarintInt32,
    decodeVarintInt64,
    decodeVarintFloat64,
    decodeZigZagInt32,
    decodeZigZagInt64,
    decodeZigZagFloat64,
    decodeZigZagInt32Value,
    decodeZigZagInt64Value,
    decodeUnsignedRleInt32,
    decodeUnsignedRleInt64,
    decodeUnsignedRleFloat64,
    decodeZigZagDeltaInt64,
    decodeDeltaRleInt32,
    decodeDeltaRleInt64,
    decodeUnsignedConstRleInt64,
    decodeZigZagConstRleInt64,
    decodeZigZagSequenceRleInt64,
    decodeZigZagRleInt32,
    decodeZigZagRleInt64,
    decodeZigZagRleFloat64,
    decodeZigZagRleDeltaInt32,
    fastInverseDelta,
    decodeZigZagSequenceRleInt32,
    decodeZigZagDeltaInt32,
    decodeZigZagDeltaFloat64,
    decodeRleDeltaInt32,
    decodeComponentwiseDeltaVec2,
    decodeComponentwiseDeltaVec2Scaled,
} from "./integerDecodingUtils";
import IntWrapper from "./intWrapper";
import {
    encodeVarintInt32,
    encodeVarintInt64,
    encodeDeltaInt32,
    encodeDeltaRleInt32,
    encodeDeltaRleInt64,
    encodeUnsignedRleFloat64,
    encodeUnsignedRleInt32,
    encodeUnsignedRleInt64,
    encodeZigZagDeltaInt64,
    encodeZigZagFloat64,
    encodeZigZagInt32,
    encodeZigZagInt32Value,
    encodeZigZagInt64,
    encodeZigZagInt64Value,
    encodeZigZagRleFloat64,
    encodeZigZagRleInt32,
    encodeZigZagRleInt64,
    encodeZigZagDeltaInt32,
    encodeZigZagDeltaFloat64,
    encodeVarintFloat64,
    encodeZigZagRleDeltaInt32,
    encodeRleDeltaInt32,
    encodeComponentwiseDeltaVec2,
    encodeComponentwiseDeltaVec2Scaled,
} from "../encoding/integerEncodingUtils";

describe("IntegerDecodingUtils", () => {
    describe("Varint decoding", () => {
        it("should decode Int32", () => {
            const value = 2 ** 10;
            const encoded = encodeVarintInt32(new Int32Array([value]));
            const decoded = decodeVarintInt32(encoded, new IntWrapper(0), 1);
            expect(decoded[0]).toEqual(value);
        });

        it("should decode Int64", () => {
            const value = 2n ** 50n;
            const encoded = encodeVarintInt64(new BigInt64Array([value]));
            const decoded = decodeVarintInt64(encoded, new IntWrapper(0), 1);
            expect(decoded[0]).toEqual(value);
        });

        it("should return valid decoded values for varint long to float64", () => {
            const value = 2 ** 40;
            const varintEncoded = encodeVarintFloat64(new Float64Array([value]));
            const actualValues = decodeVarintFloat64(varintEncoded, new IntWrapper(0), 1);
            expect(actualValues[0]).toEqual(value);
        });
    });

    describe("ZigZag encoding", () => {
        it("should decode zigzag Int32Array", () => {
            const data = new Int32Array([0, 1, 2, 3]);
            encodeZigZagInt32(data);
            decodeZigZagInt32(data);
            expect(Array.from(data)).toEqual([0, 1, 2, 3]);
        });

        it("should decode zigzag BigInt64Array", () => {
            const data = new BigInt64Array([0n, 1n, 2n, 3n]);
            encodeZigZagInt64(data);
            decodeZigZagInt64(data);
            expect(Array.from(data)).toEqual([0n, 1n, 2n, 3n]);
        });

        it("should decode zigzag Float64Array", () => {
            const value = 2 ** 35;
            const data = new Float64Array([value]);
            encodeZigZagFloat64(data);
            decodeZigZagFloat64(data);
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
    });

    describe("RLE decoding", () => {
        describe("Unsigned RLE", () => {
            it("should decode empty unsigned RLE", () => {
                const data = new Int32Array([]);
                const encodedRle = encodeUnsignedRleInt32(data);
                const decoded = decodeUnsignedRleInt32(encodedRle.data, encodedRle.runs, data.length);
                expect(Array.from(decoded)).toEqual([]);
            });

            it("should decode unsigned RLE", () => {
                const data = new Int32Array([10, 10, 20, 20, 20]);
                const encodedRle = encodeUnsignedRleInt32(data);
                const decoded = decodeUnsignedRleInt32(encodedRle.data, encodedRle.runs, data.length);
                expect(Array.from(decoded)).toEqual([10, 10, 20, 20, 20]);
            });

            it("should decode empty unsigned RLE Int64", () => {
                const data = new BigInt64Array([]);
                const encodedRle = encodeUnsignedRleInt64(data);
                const decoded = decodeUnsignedRleInt64(encodedRle.data, encodedRle.runs, data.length);
                expect(Array.from(decoded)).toEqual([]);
            });

            it("should decode unsigned RLE Int64", () => {
                const data = new BigInt64Array([10n, 10n, 20n, 20n, 20n]);
                const encodedRle = encodeUnsignedRleInt64(data);
                const decoded = decodeUnsignedRleInt64(encodedRle.data, encodedRle.runs, data.length);
                expect(Array.from(decoded)).toEqual([10n, 10n, 20n, 20n, 20n]);
            });

            it("should decode empty unsigned RLE Float64", () => {
                const data = new Float64Array([]);
                const encodedRle = encodeUnsignedRleFloat64(data);
                const decoded = decodeUnsignedRleFloat64(encodedRle.data, encodedRle.runs, data.length);
                expect(Array.from(decoded)).toEqual([]);
            });

            it("should decode unsigned RLE Float64", () => {
                const data = new Float64Array([10.5, 10.5, 20.5, 20.5, 20.5]);
                const encodedRle = encodeUnsignedRleFloat64(data);
                const decoded = decodeUnsignedRleFloat64(encodedRle.data, encodedRle.runs, data.length);
                expect(Array.from(decoded)).toEqual([10.5, 10.5, 20.5, 20.5, 20.5]);
            });
        });

        describe("ZigZag RLE", () => {
            it("should decode empty ZigZag RLE Int32", () => {
                const data = new Int32Array([]);
                const encoded = encodeZigZagRleInt32(data);
                const decoded = decodeZigZagRleInt32(encoded.data, encoded.runs, encoded.numTotalValues);
                expect(Array.from(decoded)).toEqual([]);
            });

            it("should decode ZigZag RLE Int32", () => {
                const encoded = new Int32Array([2, 2, 3, 3, 3]);
                const encodedData = encodeZigZagRleInt32(encoded);
                const decoded = decodeZigZagRleInt32(encodedData.data, encodedData.runs, encodedData.numTotalValues);
                expect(Array.from(decoded)).toEqual([2, 2, 3, 3, 3]);
            });

            it("should decode empty ZigZag RLE Int64", () => {
                const data = new BigInt64Array([]);
                const encoded = encodeZigZagRleInt64(data);
                const decoded = decodeZigZagRleInt64(encoded.data, encoded.runs, encoded.numTotalValues);
                expect(Array.from(decoded)).toEqual([]);
            });

            it("should decode ZigZag RLE Int64", () => {
                const encoded = new BigInt64Array([2n, 2n, 3n, 3n, 3n]);
                const encodedData = encodeZigZagRleInt64(encoded);
                const decoded = decodeZigZagRleInt64(encodedData.data, encodedData.runs, encodedData.numTotalValues);
                expect(Array.from(decoded)).toEqual([2n, 2n, 3n, 3n, 3n]);
            });

            it("should decode empty ZigZag RLE Float64", () => {
                const data = new Float64Array([]);
                const encoded = encodeZigZagRleFloat64(data);
                const decoded = decodeZigZagRleFloat64(encoded.data, encoded.runs, encoded.numTotalValues);
                expect(Array.from(decoded)).toEqual([]);
            });

            it("should decode ZigZag RLE Float64", () => {
                const encoded = new Float64Array([2, 2, 3, 3, 3]);
                const encodedData = encodeZigZagRleFloat64(encoded);
                const decoded = decodeZigZagRleFloat64(encodedData.data, encodedData.runs, encodedData.numTotalValues);
                expect(Array.from(decoded)).toEqual([2, 2, 3, 3, 3]);
            });
        });
    });

    describe("Delta encoding", () => {
        describe("ZigZag Delta", () => {
            it("should decode zigzag delta Int32", () => {
                const data = new Int32Array([1, 2, 3, 5, 6, 7]);
                encodeZigZagDeltaInt32(data);
                decodeZigZagDeltaInt32(data);
                expect(Array.from(data)).toEqual([1, 2, 3, 5, 6, 7]);
            });

            it("should decode zigzag delta Int64", () => {
                const data = new BigInt64Array([1n, 2n, 3n, 5n, 6n, 7n]);
                encodeZigZagDeltaInt64(data);
                decodeZigZagDeltaInt64(data);
                expect(Array.from(data)).toEqual([1n, 2n, 3n, 5n, 6n, 7n]);
            });

            it("should decode zigzag delta Float64", () => {
                const data = new Float64Array([1.0, 2.0, 3.0, 5.0, 6.0, 7.0]);
                encodeZigZagDeltaFloat64(data);
                decodeZigZagDeltaFloat64(data);
                expect(Array.from(data)).toEqual([1.0, 2.0, 3.0, 5.0, 6.0, 7.0]);
            });
        });

        describe("Fast inverse delta", () => {
            it("should apply fast inverse delta", () => {
                const data = new Int32Array([10, 15, 18, 20]);
                fastInverseDelta(data);
                encodeDeltaInt32(data);
                expect(Array.from(data)).toEqual([10, 15, 18, 20]);
            });
        });

        describe("Componentwise Delta Vec2", () => {
            it("should decode empty array", () => {
                const data = new Int32Array([]);
                const expected = new Int32Array(data);
                encodeComponentwiseDeltaVec2(data);
                decodeComponentwiseDeltaVec2(data);
                expect(Array.from(data)).toEqual(Array.from(expected));
            });

            it("should decode single vertex", () => {
                const data = new Int32Array([10, 20]);
                const expected = new Int32Array(data);
                encodeComponentwiseDeltaVec2(data);
                decodeComponentwiseDeltaVec2(data);
                expect(Array.from(data)).toEqual(Array.from(expected));
            });

            it("should decode many vertices (unrolled loop test)", () => {
                const data = new Int32Array([0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9]);
                const expected = new Int32Array(data);
                encodeComponentwiseDeltaVec2(data);
                decodeComponentwiseDeltaVec2(data);
                expect(Array.from(data)).toEqual(Array.from(expected));
            });
        });

        describe("Componentwise Delta Vec2 Scaled", () => {
            const scale = 2.0;
            const min = 0;
            const max = 4096;

            it("should decode empty array", () => {
                const data = new Int32Array([]);
                const expected = new Int32Array(data);
                encodeComponentwiseDeltaVec2Scaled(data, scale);
                decodeComponentwiseDeltaVec2Scaled(data, scale, min, max);
                expect(Array.from(data)).toEqual(Array.from(expected));
            });

            it("should decode single vertex", () => {
                const data = new Int32Array([100, 200]);
                const expected = new Int32Array(data);
                encodeComponentwiseDeltaVec2Scaled(data, scale);
                decodeComponentwiseDeltaVec2Scaled(data, scale, min, max);
                expect(Array.from(data)).toEqual(Array.from(expected));
            });

            it("should decode with different scale", () => {
                const testScale = 10.0;
                const data = new Int32Array([1000, 2000, 1100, 2200]);
                const expected = new Int32Array(data);
                encodeComponentwiseDeltaVec2Scaled(data, testScale);
                decodeComponentwiseDeltaVec2Scaled(data, testScale, min, max);
                expect(Array.from(data)).toEqual(Array.from(expected));
            });

            it("should decode many vertices (unrolled loop test)", () => {
                const data = new Int32Array([
                    0, 0, 10, 10, 20, 20, 30, 30, 40, 40, 50, 50, 60, 60, 70, 70, 80, 80, 90, 90,
                ]);
                const expected = new Int32Array(data);
                encodeComponentwiseDeltaVec2Scaled(data, scale);
                decodeComponentwiseDeltaVec2Scaled(data, scale, min, max);
                expect(Array.from(data)).toEqual(Array.from(expected));
            });
        });

        describe("Delta RLE", () => {
            it("should decode empty delta RLE Int32", () => {
                const data = new Int32Array([]);
                const encoded = encodeDeltaRleInt32(data);
                const decoded = decodeDeltaRleInt32(encoded.data, encoded.runs, encoded.numValues);
                expect(Array.from(decoded)).toEqual([]);
            });

            it("should decode delta RLE Int32", () => {
                const data = new Int32Array([1, 2, 3, 5, 6, 7]);
                const encoded = encodeDeltaRleInt32(data);
                const decoded = decodeDeltaRleInt32(encoded.data, encoded.runs, encoded.numValues);
                expect(Array.from(decoded)).toEqual([1, 2, 3, 5, 6, 7]);
            });

            it("should decode empty delta RLE Int64", () => {
                const data = new BigInt64Array([]);
                const encoded = encodeDeltaRleInt64(data);
                const decoded = decodeDeltaRleInt64(encoded.data, encoded.runs, encoded.numValues);
                expect(Array.from(decoded)).toEqual([]);
            });

            it("should decode delta RLE Int64", () => {
                const data = new BigInt64Array([1n, 2n, 3n, 5n, 6n, 7n]);
                const encoded = encodeDeltaRleInt64(data);
                const decoded = decodeDeltaRleInt64(encoded.data, encoded.runs, encoded.numValues);
                expect(Array.from(decoded)).toEqual([1n, 2n, 3n, 5n, 6n, 7n]);
            });
        });

        describe("ZigZag RLE Delta", () => {
            it("should decode zigzag RLE delta", () => {
                const data = new Int32Array([1, 2, 3, 4]);
                const encoded = encodeZigZagRleDeltaInt32(data);
                const decoded = decodeZigZagRleDeltaInt32(encoded.data, encoded.runs, encoded.numTotalValues);
                // The decoder is adding a 0 at the start
                expect(Array.from(decoded)).toEqual([0, 1, 2, 3, 4]);
            });

            it("should decode RLE delta", () => {
                const data = new Int32Array([1, 2, 3, 4]);
                const encoded = encodeRleDeltaInt32(data);
                const decoded = decodeRleDeltaInt32(encoded.data, encoded.runs, encoded.numTotalValues);
                // The decoder is adding a 0 at the start
                expect(Array.from(decoded)).toEqual([0, 1, 2, 3, 4]);
            });
        });
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

        it("should decode zigzag sequence RLE Int32", () => {
            const data = new Int32Array([5, 2]);
            const [base, delta] = decodeZigZagSequenceRleInt32(data);
            expect(base).toBe(1);
            expect(delta).toBe(1);
        });

        it("should decode zigzag sequence RLE Int32 with delta", () => {
            const data = new Int32Array([5, 2, 5, 2]);
            const [base, delta] = decodeZigZagSequenceRleInt32(data);
            expect(base).toBe(-3);
            expect(delta).toBe(1);
        });

        it("should decode zigzag sequence RLE Int64", () => {
            const data = new BigInt64Array([5n, 2n]);
            const [base, delta] = decodeZigZagSequenceRleInt64(data);
            expect(base).toBe(1n);
            expect(delta).toBe(1n);
        });

        it("should decode zigzag sequence RLE Int64 with delta", () => {
            const data = new BigInt64Array([5n, 2n, 5n, 2n]);
            const [base, delta] = decodeZigZagSequenceRleInt64(data);
            expect(base).toBe(-3n);
            expect(delta).toBe(1n);
        });
    });
});

describe("decodeFastPfor (wire format)", () => {
    it.todo("Add encodeFastPfor -> decodeFastPfor round-trip tests in PR8");
});
