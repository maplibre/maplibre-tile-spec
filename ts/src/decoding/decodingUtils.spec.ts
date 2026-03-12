import { describe, expect, it } from "vitest";
import {
    encodeBooleanRle,
    encodeByteRle,
    encodeDoubleLE,
    encodeFloatsLE,
    encodeUint32sLE,
    encodeUint64sLE,
    encodeStrings,
} from "../encoding/encodingUtils";
import BitVector from "../vector/flat/bitVector";
import {
    decodeBooleanRle,
    decodeByteRle,
    decodeDoublesLE,
    decodeFloatsLE,
    decodeString,
    decodeUint32sLE,
    decodeUint64sLE,
} from "./decodingUtils";
import IntWrapper from "./intWrapper";

describe("decodingUtils", () => {
    describe("decodeFloatsLE", () => {
        it("should decode float values from little-endian bytes", () => {
            const data = new Float32Array([1.5, 2.5]);
            const encoded = encodeFloatsLE(data);
            const offset = new IntWrapper(0);
            const result = decodeFloatsLE(encoded, offset, 2);

            expect(result).toEqual(data);
            expect(offset.get()).toBe(8);
        });
    });

    describe("decodeDoublesLE", () => {
        it("should decode double values from little-endian bytes", () => {
            const data = new Float64Array([Math.PI, Math.E]);
            const encoded = encodeDoubleLE(data);
            const offset = new IntWrapper(0);
            const result = decodeDoublesLE(encoded, offset, 2);

            expect(result[0]).toBeCloseTo(Math.PI);
            expect(result[1]).toBeCloseTo(Math.E);
            expect(offset.get()).toBe(Float64Array.BYTES_PER_ELEMENT * 2);
        });
    });

    describe("decodeUint32sLE", () => {
        it("round-trips uint32 values through little-endian bytes", () => {
            const data = new Uint32Array([0x12345678, 1]);
            const encoded = encodeUint32sLE(data);
            const offset = new IntWrapper(0);

            const result = decodeUint32sLE(encoded, offset, 2);

            expect(result).toEqual(data);
            expect(offset.get()).toBe(8);
        });

        it("should decode uint32 values from little-endian bytes", () => {
            const encoded = new Uint8Array([0x78, 0x56, 0x34, 0x12, 0x01, 0x00, 0x00, 0x00]);
            const offset = new IntWrapper(0);

            const result = decodeUint32sLE(encoded, offset, 2);

            expect(result).toEqual(new Uint32Array([0x12345678, 1]));
            expect(offset.get()).toBe(8);
        });
    });

    describe("decodeUint64sLE", () => {
        it("round-trips uint64 values through little-endian bytes", () => {
            const data = new BigUint64Array([1n, 0x1122334455667788n]);
            const encoded = encodeUint64sLE(data);
            const offset = new IntWrapper(0);

            const result = decodeUint64sLE(encoded, offset, 2);

            expect(result).toEqual(data);
            expect(offset.get()).toBe(16);
        });

        it("should decode uint64 values from little-endian bytes", () => {
            const encoded = new Uint8Array([
                0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x88, 0x77, 0x66, 0x55, 0x44, 0x33, 0x22, 0x11,
            ]);
            const offset = new IntWrapper(0);

            const result = decodeUint64sLE(encoded, offset, 2);

            expect(result).toEqual(new BigUint64Array([1n, 0x1122334455667788n]));
            expect(offset.get()).toBe(16);
        });
    });

    describe("decodeFloatsLE with nullability", () => {
        it("should decode nullable float values with nullability buffer", () => {
            const data = new Float32Array([1.5, 2.5]);
            const encoded = encodeFloatsLE(data);
            const offset = new IntWrapper(0);
            const bitVectorData = new Uint8Array([0b00000101]);
            const nullabilityBuffer = new BitVector(bitVectorData, 3);

            const result = decodeFloatsLE(encoded, offset, 2, nullabilityBuffer);

            expect(result.length).toBe(3);
            expect(result[0]).toBeCloseTo(1.5);
            expect(result[1]).toBe(0);
            expect(result[2]).toBeCloseTo(2.5);
        });
    });

    describe("decodeDoublesLE with nullability", () => {
        it("should decode nullable double values with nullability buffer", () => {
            const data = new Float32Array([Math.PI, Math.E]);
            const encoded = encodeDoubleLE(data);
            const offset = new IntWrapper(0);
            const bitVectorData = new Uint8Array([0b00000011]);
            const nullabilityBuffer = new BitVector(bitVectorData, 2);

            const result = decodeDoublesLE(encoded, offset, 2, nullabilityBuffer);

            expect(result.length).toBe(2);
            expect(result[0]).toBeCloseTo(Math.PI);
            expect(result[1]).toBeCloseTo(Math.E);
        });
    });

    describe("decodeBooleanRle", () => {
        it("should decode boolean RLE", () => {
            // Create 8 true boolean values
            const data = [true, true, true, true, true, true, true, true];
            const encoded = encodeBooleanRle(data);
            const offset = new IntWrapper(0);
            const result = decodeBooleanRle(encoded, 8, encoded.length, offset);

            // All 8 bits should be set in the first byte
            expect(result[0]).toBe(0xff);
        });
    });

    describe("decodeByteRle", () => {
        it("should decode byte RLE with runs", () => {
            // Encode 5 identical bytes
            const data = new Uint8Array([42, 42, 42, 42, 42]);
            const encoded = encodeByteRle(data);
            const offset = new IntWrapper(0);
            const result = decodeByteRle(encoded, 5, encoded.length, offset);

            expect(result).toEqual(data);
        });

        it("should decode byte RLE with literals", () => {
            // Encode 3 different bytes (will be encoded as literals)
            const data = new Uint8Array([1, 2, 3]);
            const encoded = encodeByteRle(data);
            const offset = new IntWrapper(0);
            const result = decodeByteRle(encoded, 3, encoded.length, offset);

            expect(result).toEqual(data);
            expect(offset.get()).toBe(encoded.length);
        });

        it("should handle truncated stream when byteLength runs out before numBytes", () => {
            // Request 10 bytes but byteLength only allows 2 bytes (header + value)
            // header=0 means numRuns=3, but stream ends after value byte
            const data = new Uint8Array([0, 42]);
            const offset = new IntWrapper(0);
            const result = decodeByteRle(data, 10, 2, offset);

            // Should only fill 3 bytes (what the run specified) then stop at stream boundary
            expect(result.length).toBe(10);
            expect(result[0]).toBe(42);
            expect(result[1]).toBe(42);
            expect(result[2]).toBe(42);
            // Remaining bytes should be 0
            expect(result[3]).toBe(0);
            expect(result[9]).toBe(0);
            expect(offset.get()).toBe(2); // Should stop at byteLength boundary
        });

        it("should decode mixed literals and runs", () => {
            const data = new Uint8Array([1, 2, 5, 5, 5, 5, 5, 7, 8]);
            const encoded = encodeByteRle(data);
            const offset = new IntWrapper(0);
            const result = decodeByteRle(encoded, 9, encoded.length, offset);

            expect(result).toEqual(data);
        });

        it("should handle 128 literal max", () => {
            const data = new Uint8Array(130);
            for (let i = 0; i < 130; i++) {
                data[i] = i % 256;
            }
            const encoded = encodeByteRle(data);
            const offset = new IntWrapper(0);
            const result = decodeByteRle(encoded, 130, encoded.length, offset);

            expect(result).toEqual(data);
        });
    });

    describe("decodeString", () => {
        it("should decode short string", () => {
            const data = "Hello";
            const encoded = encodeStrings([data]);
            const result = decodeString(encoded, 0, encoded.length);

            expect(result).toBe(data);
        });

        it("should decode long string", () => {
            const data = "This is a longer string for testing TextDecoder path";
            const encoded = encodeStrings([data]);
            const result = decodeString(encoded, 0, encoded.length);

            expect(result).toBe(data);
        });

        it("should handle string with offset", () => {
            const prefix = "Hello";
            const expectedText = "World";
            const encoded = encodeStrings([prefix, expectedText]);
            const prefixLength = new TextEncoder().encode(prefix).length;

            const result = decodeString(encoded, prefixLength, encoded.length);

            expect(result).toBe(expectedText);
        });
    });
});
