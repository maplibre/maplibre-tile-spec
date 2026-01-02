
import { describe, expect, it } from "vitest";
import { int32sToBigEndianBytes, bigEndianBytesToInt32s } from "./byteIO";

describe("ByteIO (Endianness Utils)", () => {
    describe("int32sToBigEndianBytes", () => {
        it("converts Int32Array to Big-Endian Uint8Array", () => {
            const input = new Int32Array([0x12345678, -1, 0, 1]);
            const output = int32sToBigEndianBytes(input);

            expect(output).toBeInstanceOf(Uint8Array);
            expect(output.length).toBe(16);

            expect(output[0]).toBe(0x12);
            expect(output[1]).toBe(0x34);
            expect(output[2]).toBe(0x56);
            expect(output[3]).toBe(0x78);

            expect(output[4]).toBe(0xff);
            expect(output[7]).toBe(0xff);
        });

        it("round-trips with bigEndianBytesToInt32s (aligned)", () => {
            const input = new Int32Array([0, 1, -1, 0x12345678]);
            const bytes = int32sToBigEndianBytes(input);
            const decoded = bigEndianBytesToInt32s(bytes, 0, bytes.length);
            expect(decoded).toEqual(input);
        });
    });

    describe("bigEndianBytesToInt32s", () => {
        it("converts aligned byte buffer back to Int32Array", () => {
            const bytes = new Uint8Array([
                0x12, 0x34, 0x56, 0x78,
                0xff, 0xff, 0xff, 0xff,
            ]);
            const ints = bigEndianBytesToInt32s(bytes, 0, bytes.length);

            expect(ints.length).toBe(2);
            expect(ints[0]).toBe(0x12345678);
            expect(ints[1]).toBe(-1);
        });

        it("handles non-aligned offsets", () => {
            const buffer = new Uint8Array(10);
            buffer[1] = 0x00;
            buffer[2] = 0x00;
            buffer[3] = 0x00;
            buffer[4] = 0x42;
            const ints = bigEndianBytesToInt32s(buffer, 1, 4);
            expect(ints[0]).toBe(0x42);
        });

        it("handles trailing bytes (length not multiple of 4)", () => {
            const bytes = new Uint8Array([
                0x00, 0x00, 0x01, 0x00,
                0xAB,
            ]);

            const ints = bigEndianBytesToInt32s(bytes, 0, 5);
            expect(ints.length).toBe(2);
            expect(ints[0]).toBe(256);
            expect(ints[1]).toBe(0xAB000000 | 0);
        });

        it("handles 3 trailing bytes", () => {
            const bytes = new Uint8Array([0xAA, 0xBB, 0xCC]);
            const ints = bigEndianBytesToInt32s(bytes, 0, 3);
            expect(ints.length).toBe(1);
            expect(ints[0]).toBe((0xAA << 24) | (0xBB << 16) | (0xCC << 8));
        });

        it("throws on out of bounds", () => {
            const bytes = new Uint8Array(4);
            expect(() => bigEndianBytesToInt32s(bytes, 0, 5)).toThrow();
            expect(() => bigEndianBytesToInt32s(bytes, -1, 4)).toThrow();
        });

        it("round-trips with int32sToBigEndianBytes (unaligned view)", () => {
            const input = new Int32Array([0x01020304, -123456789, 0, 42]);
            const bytes = int32sToBigEndianBytes(input);

            const buffer = new Uint8Array(bytes.length + 3);
            buffer.set([0xaa, 0xbb, 0xcc], 0);
            buffer.set(bytes, 3);

            const decoded = bigEndianBytesToInt32s(buffer, 3, bytes.length);
            expect(decoded).toEqual(input);
        });
    });
});
