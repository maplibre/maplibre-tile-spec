import { describe, it, expect } from "vitest";
import {
    decodeFloatsLE,
    decodeDoublesLE,
    decodeBooleanRle,
    decodeString,
    decodeByteRle,
} from "./decodingUtils";
import IntWrapper from "./intWrapper";

describe("decodingUtils", () => {
    describe("decodeFloatsLE", () => {
        it("should decode float values from little-endian bytes", () => {
            const buffer = new ArrayBuffer(8);
            const view = new Float32Array(buffer);
            view[0] = 1.5;
            view[1] = 2.5;

            const data = new Uint8Array(buffer);
            const offset = new IntWrapper(0);
            const result = decodeFloatsLE(data, offset, 2);

            expect(result[0]).toBeCloseTo(1.5);
            expect(result[1]).toBeCloseTo(2.5);
            expect(offset.get()).toBe(8);
        });
    });

    describe("decodeDoublesLE", () => {
        it("should decode double values from little-endian bytes", () => {
            const buffer = new ArrayBuffer(16);
            const view = new Float64Array(buffer);
            view[0] = 3.14159;
            view[1] = 2.71828;

            const data = new Uint8Array(buffer);
            const offset = new IntWrapper(0);
            const result = decodeDoublesLE(data, offset, 2);

            expect(result[0]).toBeCloseTo(3.14159);
            expect(result[1]).toBeCloseTo(2.71828);
            expect(offset.get()).toBe(16);
        });
    });

    describe("decodeBooleanRle", () => {
        it("should decode boolean RLE", () => {
            const buffer = new Uint8Array([254, 0xFF]);
            const offset = new IntWrapper(0);
            const result = decodeBooleanRle(buffer, 8, offset);

            expect(result[0]).toBe(0xFF);
        });
    });

    describe("decodeByteRle", () => {
        it("should decode byte RLE with runs", () => {
            // header=2 means numRuns=2+3=5, followed by value byte
            const data = new Uint8Array([2, 42]);
            const offset = new IntWrapper(0);
            const result = decodeByteRle(data, 5, offset);

            expect(result.length).toBe(5);
            expect(result[0]).toBe(42);
            expect(result[1]).toBe(42);
            expect(result[2]).toBe(42);
            expect(result[3]).toBe(42);
            expect(result[4]).toBe(42);
        });

        it("should decode byte RLE with literals", () => {
            // header=253 means numLiterals=256-253=3, followed by 3 literal bytes
            const data = new Uint8Array([253, 1, 2, 3]);
            const offset = new IntWrapper(0);
            const result = decodeByteRle(data, 3, offset);

            expect(result.length).toBe(3);
            expect(result[0]).toBe(1);
            expect(result[1]).toBe(2);
            expect(result[2]).toBe(3);
        });
    });

    describe("decodeString", () => {
        it("should decode short string", () => {
            const text = "Hello";
            const buffer = new TextEncoder().encode(text);
            const result = decodeString(buffer, 0, buffer.length);

            expect(result).toBe(text);
        });

        it("should decode long string", () => {
            const text = "This is a longer string for testing TextDecoder path";
            const buffer = new TextEncoder().encode(text);
            const result = decodeString(buffer, 0, buffer.length);

            expect(result).toBe(text);
        });

        it("should handle string with offset", () => {
            const text = "World";
            const prefix = new TextEncoder().encode("Hello");
            const textBytes = new TextEncoder().encode(text);
            const combined = new Uint8Array(prefix.length + textBytes.length);
            combined.set(prefix, 0);
            combined.set(textBytes, prefix.length);

            // decodeString takes (buffer, start, end) where end is the position after the last byte
            const result = decodeString(combined, prefix.length, prefix.length + textBytes.length);

            expect(result).toBe(text);
        });
    });
});
