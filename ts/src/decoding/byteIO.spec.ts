
import { describe, expect, it } from "vitest";
import { int32sToBigEndianBytes, bigEndianBytesToInt32s } from "./byteIO";

describe("ByteIO (Endianness Utils)", () => {
    describe("int32sToBigEndianBytes", () => {
        it("converts Int32Array to Big-Endian Uint8Array", () => {
            const input = new Int32Array([0x12345678, -1, 0, 1]);
            const output = int32sToBigEndianBytes(input);

            expect(output).toBeInstanceOf(Uint8Array);
            expect(output.length).toBe(16);

            // 0x12345678 -> 12 34 56 78
            expect(output[0]).toBe(0x12);
            expect(output[1]).toBe(0x34);
            expect(output[2]).toBe(0x56);
            expect(output[3]).toBe(0x78);

            // -1 -> FF FF FF FF
            expect(output[4]).toBe(0xff);
            expect(output[7]).toBe(0xff);
        });
    });

    describe("bigEndianBytesToInt32s", () => {
        it("converts aligned byte buffer back to Int32Array", () => {
            const bytes = new Uint8Array([
                0x12, 0x34, 0x56, 0x78, // 0x12345678
                0xff, 0xff, 0xff, 0xff  // -1
            ]);
            const ints = bigEndianBytesToInt32s(bytes, 0, bytes.length);

            expect(ints.length).toBe(2);
            expect(ints[0]).toBe(0x12345678);
            expect(ints[1]).toBe(-1);
        });

        it("handles non-aligned offsets", () => {
            // Create a buffer where the data starts at offset 1
            const buffer = new Uint8Array(10);
            buffer[1] = 0x00; buffer[2] = 0x00; buffer[3] = 0x00; buffer[4] = 0x42; // 0x42
            const ints = bigEndianBytesToInt32s(buffer, 1, 4);
            expect(ints[0]).toBe(0x42);
        });

        it("handles trailing bytes (length not multiple of 4)", () => {
            // 4 bytes + 1 trailing byte
            const bytes = new Uint8Array([
                0x00, 0x00, 0x01, 0x00, // 256
                0xAB                    // Extra byte
            ]);

            // Should read 2 integers. First is 256. Second is constructed from 0xAB...
            // the logic is: v |= bytes[base + i] << (24 - i * 8);
            // i=0: 0xAB << 24 = 0xAB000000
            const ints = bigEndianBytesToInt32s(bytes, 0, 5);
            expect(ints.length).toBe(2);
            expect(ints[0]).toBe(256);
            expect(ints[1]).toBe(0xAB000000 | 0); // Signed Int32
        });

        it("handles 3 trailing bytes", () => {
            const bytes = new Uint8Array([0xAA, 0xBB, 0xCC]);
            const ints = bigEndianBytesToInt32s(bytes, 0, 3);
            expect(ints.length).toBe(1);
            // 0xAA << 24 | 0xBB << 16 | 0xCC << 8
            expect(ints[0]).toBe((0xAA << 24) | (0xBB << 16) | (0xCC << 8));
        });

        it("throws on out of bounds", () => {
            const bytes = new Uint8Array(4);
            expect(() => bigEndianBytesToInt32s(bytes, 0, 5)).toThrow();
            expect(() => bigEndianBytesToInt32s(bytes, -1, 4)).toThrow();
        });
    });
});
