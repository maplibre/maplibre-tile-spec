import { describe, it, expect } from "vitest";
import { decodeFloatsLE, decodeDoublesLE, decodeBooleanRle, decodeString, decodeByteRle } from "./decodingUtils";
import IntWrapper from "./intWrapper";
import BitVector from "../vector/flat/bitVector";
import {
    encodeFloatsLE,
    encodeDoubleLE,
    encodeBooleanRle,
    encodeByteRle,
    encodeStrings,
} from "../encoding/encodingUtils";

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
            const data = new Float32Array([3.14159, 2.71828]);
            const encoded = encodeDoubleLE(data);
            const offset = new IntWrapper(0);
            const result = decodeDoublesLE(encoded, offset, 2);

            expect(result[0]).toBeCloseTo(3.14159);
            expect(result[1]).toBeCloseTo(2.71828);
            expect(offset.get()).toBe(16);
        });
    });

    describe("decodeFloatsLE with nullability", () => {
        it("should decode nullable float values with nullability buffer", () => {
            // Only encode non-null values
            const data = new Float32Array([1.5, 2.5]);
            const encoded = encodeFloatsLE(data);
            const offset = new IntWrapper(0);
            const bitVectorData = new Uint8Array([0b00000101]); // bits 0 and 2 are set
            const nullabilityBuffer = new BitVector(bitVectorData, 3);

            const result = decodeFloatsLE(encoded, offset, 2, nullabilityBuffer);

            expect(result.length).toBe(3);
            expect(result[0]).toBeCloseTo(1.5); // bit 0 is set
            expect(result[1]).toBe(0); // bit 1 is not set (null)
            expect(result[2]).toBeCloseTo(2.5); // bit 2 is set
        });
    });

    describe("decodeDoublesLE with nullability", () => {
        it("should decode nullable double values with nullability buffer", () => {
            // Only encode non-null values
            const data = new Float32Array([3.14159, 2.71828]);
            const encoded = encodeDoubleLE(data);
            const offset = new IntWrapper(0);
            const bitVectorData = new Uint8Array([0b00000011]); // bits 0 and 1 are set
            const nullabilityBuffer = new BitVector(bitVectorData, 2);

            const result = decodeDoublesLE(encoded, offset, 2, nullabilityBuffer);

            expect(result.length).toBe(2);
            expect(result[0]).toBeCloseTo(3.14159); // bit 0 is set
            expect(result[1]).toBeCloseTo(2.71828); // bit 1 is set
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

            // decodeString takes (buffer, start, end) where end is the position after the last byte
            const result = decodeString(encoded, prefixLength, encoded.length);

            expect(result).toBe(expectedText);
        });
    });
});
