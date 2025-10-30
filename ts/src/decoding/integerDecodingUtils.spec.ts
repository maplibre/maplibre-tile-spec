import { describe, it, expect } from "vitest";
import varint from "varint";
import {
    decodeVarintInt64,
    decodeVarintFloat64,
    decodeZigZag,
    decodeZigZagInt64,
    decodeZigZagFloat64,
    decodeZigZagValue,
    decodeZigZagValueInt64,
    decodeUnsignedRle,
    decodeUnsignedRleInt64,
    decodeUnsignedRleFloat64,
    decodeZigZagDeltaInt64,
    fastInverseDelta,
    decodeNullableZigZagDeltaInt64,
    padWithZerosInt64,
    padZigZagWithZerosInt64,
    decodeDeltaRleInt64,
    decodeUnsignedConstRleInt64,
    decodeZigZagConstRleInt64,
    decodeZigZagSequenceRleInt64,
    decodeZigZagRle,
    decodeZigZagRleInt64,
    decodeZigZagRleFloat64,
    zigZagRleDeltaDecoding,
    decodeNullableRleInt64,
} from "./integerDecodingUtils";
import IntWrapper from "./intWrapper";
import BitVector from "../vector/flat/bitVector";

describe("IntegerDecodingUtils", () => {

    describe("decodeVarintInt64", () => {
        it("should decode BigInt values", () => {
            const value = 2n ** 50n;
            const encoded = varintEncodeBigInt(value);
            const decoded = decodeVarintInt64(encoded, new IntWrapper(0), 1);
            expect(decoded[0]).toEqual(value);
        });
    });

    describe("decodeVarintLongToFloat64", () => {
        it("should return valid decoded values", () => {
            const value = 2 ** 40;
            const varintEncoded = varintEncodeNum(value);
            const actualValues = decodeVarintFloat64(varintEncoded, 1, new IntWrapper(0));
            expect(actualValues[0]).toEqual(value);
        });
    });

    describe("decodeZigZag", () => {
        it("should decode zigzag Int32Array", () => {
            const encoded = new Int32Array([0, 1, 2, 3]);
            decodeZigZag(encoded);
            expect(Array.from(encoded)).toEqual([0, -1, 1, -2]);
        });
    });

    describe("decodeZigZagInt64", () => {
        it("should decode zigzag BigInt64Array", () => {
            const encoded = new BigInt64Array([0n, 1n, 2n, 3n]);
            decodeZigZagInt64(encoded);
            expect(Array.from(encoded)).toEqual([0n, -1n, 1n, -2n]);
        });
    });

    describe("decodeZigZagFloat64", () => {
        it("should return valid decoded values for zigZag Varint decoding", () => {
            const value = 2 ** 35;
            const zigZagValue = value * 2;
            const varintEncoded = varintEncodeNum(zigZagValue);
            const actualValues = decodeVarintFloat64(varintEncoded, 1, new IntWrapper(0));
            decodeZigZagFloat64(actualValues);
            expect(actualValues[0]).toEqual(value);
        });

        it("should return valid decoded values for zigZag Varint decoding", () => {
            const value = 3298190;
            const zigZagValue = value << 1;
            const varintEncoded = varintEncodeNum(zigZagValue);
            const actualValues = decodeVarintFloat64(varintEncoded, 1, new IntWrapper(0));
            decodeZigZagFloat64(actualValues);
            expect(actualValues[0]).toEqual(value);
        });
    });

    describe("decodeZigZagValue", () => {
        it("should decode single zigzag values", () => {
            expect(decodeZigZagValue(0)).toBe(0);
            expect(decodeZigZagValue(1)).toBe(-1);
            expect(decodeZigZagValue(2)).toBe(1);
        });
    });

    describe("decodeZigZagValueInt64", () => {
        it("should decode single BigInt zigzag values", () => {
            expect(decodeZigZagValueInt64(0n)).toBe(0n);
            expect(decodeZigZagValueInt64(1n)).toBe(-1n);
        });
    });

    describe("RLE Decoding", () => {
        it("should decode unsigned RLE", () => {
            const encoded = new Int32Array([2, 3, 10, 20]);
            const decoded = decodeUnsignedRle(encoded, 2, 5);
            expect(Array.from(decoded)).toEqual([10, 10, 20, 20, 20]);
        });

        it("should decode unsigned RLE Int64", () => {
            const encoded = new BigInt64Array([2n, 3n, 10n, 20n]);
            const decoded = decodeUnsignedRleInt64(encoded, 2, 5);
            expect(Array.from(decoded)).toEqual([10n, 10n, 20n, 20n, 20n]);
        });

        it("should decode unsigned RLE Float64", () => {
            const encoded = new Float64Array([2, 3, 10.5, 20.5]);
            const decoded = decodeUnsignedRleFloat64(encoded, 2, 5);
            expect(Array.from(decoded)).toEqual([10.5, 10.5, 20.5, 20.5, 20.5]);
        });
    });

    describe("Delta Decoding", () => {
        it("should decode zigzag delta Int64", () => {
            const data = new BigInt64Array([2n, 2n, 2n]);
            decodeZigZagDeltaInt64(data);
            expect(Array.from(data)).toEqual([1n, 2n, 3n]);
        });

        it("should apply fast inverse delta", () => {
            const data = new Int32Array([10, 5, 3, 2]);
            fastInverseDelta(data);
            expect(Array.from(data)).toEqual([10, 15, 18, 20]);
        });
    });

    describe("Nullable Decoding", () => {
        it("should decode nullable zigzag delta Int64", () => {
            const bitVectorData = new Uint8Array([0b00000011]);
            const bitVector = new BitVector(bitVectorData, 2);
            const data = new BigInt64Array([2n, 2n]);
            const decoded = decodeNullableZigZagDeltaInt64(bitVector, data);
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
    });

    describe("Delta RLE", () => {
        it("should decode delta RLE Int64", () => {
            const data = new BigInt64Array([3n, 2n]);
            const decoded = decodeDeltaRleInt64(data, 1, 3);
            expect(Array.from(decoded)).toEqual([1n, 2n, 3n]);
        });
    });

    describe("Const and Sequence RLE", () => {
        it("should decode unsigned const RLE Int64", () => {
            const data = new BigInt64Array([5n, 42n]);
            expect(decodeUnsignedConstRleInt64(data)).toBe(42n);
        });

        it("should decode zigzag const RLE Int64", () => {
            const data = new BigInt64Array([5n, 4n]);
            expect(decodeZigZagConstRleInt64(data)).toBe(2n);
        });

        it("should decode zigzag sequence RLE Int64", () => {
            const data = new BigInt64Array([5n, 2n]);
            const [base, delta] = decodeZigZagSequenceRleInt64(data);
            expect(base).toBe(1n);
            expect(delta).toBe(1n);
        });
    });

    describe("decode RLE", () => {
        it("should decode RLE Int64", () => {
            const encoded = new BigInt64Array([2n, 3n, 10n, 20n]);
            const decoded = decodeUnsignedRleInt64(encoded, 2, 5);
            expect(Array.from(decoded)).toEqual([10n, 10n, 20n, 20n, 20n]);
        });

        it("should decode RLE Float64", () => {
            const encoded = new Float64Array([2, 3, 10.5, 20.5]);
            const decoded = decodeUnsignedRleFloat64(encoded, 2, 5);
            expect(Array.from(decoded)).toEqual([10.5, 10.5, 20.5, 20.5, 20.5]);
        });

        it("should decode ZigZag RLE Int32", () => {
            const encoded = new Int32Array([2, 3, 4, 6]);
            const decoded = decodeZigZagRle(encoded, 2, 5);
            expect(Array.from(decoded)).toEqual([2, 2, 3, 3, 3]);
        });

        it("should decode ZigZag RLE Int64", () => {
            const encoded = new BigInt64Array([2n, 3n, 4n, 6n]);
            const decoded = decodeZigZagRleInt64(encoded, 2, 5);
            expect(Array.from(decoded)).toEqual([2n, 2n, 3n, 3n, 3n]);
        });

        it("should decode ZigZag RLE Float64", () => {
            const encoded = new Float64Array([2, 3, 4, 6]);
            const decoded = decodeZigZagRleFloat64(encoded, 2, 5);
            expect(Array.from(decoded)).toEqual([2, 2, 3, 3, 3]);
        });
    });

    describe("ZigZag RLE Delta", () => {
        it("should decode zigzag RLE delta", () => {
            const data = new Int32Array([2, 2, 2, 2]);
            const decoded = zigZagRleDeltaDecoding(data, 2, 4);
            expect(decoded.length).toBe(5);
        });
    });

    describe("Nullable RLE Int64", () => {
        it("should decode nullable RLE Int64", () => {
            const bitVectorData = new Uint8Array([0b00000011]);
            const bitVector = new BitVector(bitVectorData, 2);
            const data = new BigInt64Array([2n, 3n, 10n, 20n]);
            const decoded = decodeNullableRleInt64(data, { runs: 2 } as any, true, bitVector);
            expect(decoded.length).toBe(2);
        });
    });
});

function varintEncodeNum(value: number) {
    const v = varint.encode(value);
    return new Uint8Array(v);
}

function varintEncodeBigInt(value: bigint): Uint8Array {
    const result: number[] = [];
    let num = value;
    while (num > 0n) {
        let byte = Number(num & 0x7fn);
        num >>= 7n;
        if (num > 0n) byte |= 0x80;
        result.push(byte);
    }
    return new Uint8Array(result.length > 0 ? result : [0]);
}
