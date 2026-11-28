import { describe, it, expect } from "vitest";
import { unpackNullable, unpackNullableBoolean } from "./unpackNullableUtils.js";
import BitVector from "../vector/flat/bitVector.js";
import { packNullable, packNullableBoolean } from "../encoding/packNullableUtils.js";

describe("unpackNullableUtils", () => {
    describe("unpackNullable", () => {
        it("should return the original array when presentBits is null", () => {
            const dataStream = new Int32Array([1, 2, 3]);
            const packed = packNullable(dataStream, null);
            const result = unpackNullable(packed, null, 0);

            expect(result).toBe(dataStream);
            expect(result).toEqual(new Int32Array([1, 2, 3]));
        });

        it("should return the original array when presentBits is undefined", () => {
            const dataStream = new Float32Array([1.5, 2.5, 3.5]);
            const result = unpackNullable(dataStream, undefined, 0);

            expect(result).toBe(dataStream);
            expect(result).toEqual(new Float32Array([1.5, 2.5, 3.5]));
        });

        it("should return the original BigInt64Array when presentBits is null", () => {
            const dataStream = new BigInt64Array([10n, 20n, 30n]);
            const result = unpackNullable(dataStream, null, 0n);

            expect(result).toBe(dataStream);
            expect(result).toEqual(new BigInt64Array([10n, 20n, 30n]));
        });

        it("should return the original array when presentBits is fully set", () => {
            const dataStream = new BigInt64Array([10n, 20n, 30n]);
            const presentBits = new BitVector(new Uint8Array([0b111]), 3);
            const packed = packNullable(dataStream, presentBits);
            const result = unpackNullable(packed, presentBits, 0n);

            expect(result).toEqual(new BigInt64Array([10n, 20n, 30n]));
        });

        it("should return partial array when presentBits is partially set", () => {
            const dataStream = new Int32Array([0, 20, 30]); // first number is "null"
            const presentBits = new BitVector(new Uint8Array([0b110]), 3);
            const packed = packNullable(dataStream, presentBits);
            const result = unpackNullable(packed, presentBits, 0);

            expect(result).toEqual(new Int32Array([0, 20, 30]));
        });

        it("should place the values across several presence bytes", () => {
            const dataStream = new Uint32Array([1, 2, 3, 4, 5, 6]);
            const presentBits = new BitVector(new Uint8Array([0b10100001, 0x00, 0b10000110]), 24);
            const result = unpackNullable(dataStream, presentBits, 0);

            expect(result).toEqual(
                new Uint32Array([1, 0, 0, 0, 0, 2, 0, 3, 0, 0, 0, 0, 0, 0, 0, 0, 0, 4, 5, 0, 0, 0, 0, 6]),
            );
        });

        it("should ignore the presence bits past the size", () => {
            const dataStream = new BigInt64Array([10n, 20n]);
            const presentBits = new BitVector(new Uint8Array([0b11111101]), 3);
            const result = unpackNullable(dataStream, presentBits, 0n);

            expect(result).toEqual(new BigInt64Array([10n, 0n, 20n]));
        });

        it("should insert a default value other than zero at null positions", () => {
            const dataStream = new Float64Array([1.5, 2.5]);
            const presentBits = new BitVector(new Uint8Array([0b1001]), 4);
            const result = unpackNullable(dataStream, presentBits, -1);

            expect(result).toEqual(new Float64Array([1.5, -1, -1, 2.5]));
        });
    });

    describe("unpackNullableBoolean", () => {
        it("should return the original array when presentBits is null", () => {
            const dataStream = new Uint8Array([0b11010101]);
            const packed = packNullableBoolean(dataStream, 8, null);
            const result = unpackNullableBoolean(packed, 8, null);

            expect(result).toBe(dataStream);
            expect(result).toEqual(new Uint8Array([0b11010101]));
        });

        it("should return the original array when presentBits is undefined", () => {
            const dataStream = new Uint8Array([0b00001111]);
            const result = unpackNullableBoolean(dataStream, 8, undefined);

            expect(result).toBe(dataStream);
            expect(result).toEqual(new Uint8Array([0b00001111]));
        });

        it("should return the original array when presentBits is fully set", () => {
            const dataStream = new Uint8Array([0b11010101]);
            const presentBits = new BitVector(new Uint8Array([0b11111111]), 8);
            const packed = packNullableBoolean(dataStream, 8, presentBits);
            const result = unpackNullableBoolean(packed, 8, presentBits);

            expect(result).toEqual(new Uint8Array([0b11010101]));
        });

        it("should return partial array when presentBits is partially set", () => {
            const dataStream = new Uint8Array([0b11111111]);
            const presentBits = new BitVector(new Uint8Array([0b11110000]), 8);
            const packed = packNullableBoolean(dataStream, 8, presentBits);
            const result = unpackNullableBoolean(packed, 8, presentBits);

            expect(result).toEqual(new Uint8Array([0b11110000]));
        });
    });
});
