import { describe, expect, it } from "vitest";
import { decodeBigEndianInt32s, decodeBigEndianInt32sInto } from "./bigEndianDecode";
import { encodeBigEndianInt32s } from "../encoding/bigEndianEncode";

function decodeEncode(bytes: Uint8Array, offset: number, byteLength: number): { decoded: Int32Array; encoded: Uint8Array } {
    const decoded = decodeBigEndianInt32s(bytes, offset, byteLength);
    const encoded = encodeBigEndianInt32s(decoded);
    return { decoded, encoded };
}

describe("decodeBigEndianInt32s", () => {
    it("converts aligned byte buffer back to Int32Array", () => {
        const bytes = new Uint8Array([
            0x12, 0x34, 0x56, 0x78,
            0xff, 0xff, 0xff, 0xff,
        ]);
        const { decoded, encoded } = decodeEncode(bytes, 0, bytes.length);
        expect(encoded).toEqual(bytes);

        expect(decoded.length).toBe(2);
        expect(decoded[0]).toBe(0x12345678);
        expect(decoded[1]).toBe(-1);
    });

    it("handles non-aligned offsets", () => {
        const buffer = new Uint8Array(10);
        buffer[1] = 0x00;
        buffer[2] = 0x00;
        buffer[3] = 0x00;
        buffer[4] = 0x42;
        const { decoded, encoded } = decodeEncode(buffer, 1, 4);
        expect(encoded.subarray(0, 4)).toEqual(buffer.subarray(1, 5));
        for (let i = 4; i < encoded.length; i++) expect(encoded[i]).toBe(0);

        expect(decoded[0]).toBe(0x42);
    });

    it("handles trailing bytes (length not multiple of 4)", () => {
        const bytes = new Uint8Array([
            0x00, 0x00, 0x01, 0x00,
            0xAB,
        ]);

        const byteLength = 5;
        const { decoded, encoded } = decodeEncode(bytes, 0, byteLength);
        expect(encoded.subarray(0, byteLength)).toEqual(bytes);
        for (let i = byteLength; i < encoded.length; i++) expect(encoded[i]).toBe(0);

        expect(decoded.length).toBe(2);
        expect(decoded[0]).toBe(256);
        expect(decoded[1]).toBe(0xAB000000 | 0);
    });

    it("handles 3 trailing bytes", () => {
        const bytes = new Uint8Array([0xAA, 0xBB, 0xCC]);
        const byteLength = 3;
        const { decoded, encoded } = decodeEncode(bytes, 0, byteLength);
        expect(encoded.subarray(0, byteLength)).toEqual(bytes);
        for (let i = byteLength; i < encoded.length; i++) expect(encoded[i]).toBe(0);

        expect(decoded.length).toBe(1);
        expect(decoded[0]).toBe((0xAA << 24) | (0xBB << 16) | (0xCC << 8));
    });

    it("throws on out of bounds", () => {
        const bytes = new Uint8Array(4);
        expect(() => decodeBigEndianInt32s(bytes, 0, 5)).toThrow();
        expect(() => decodeBigEndianInt32s(bytes, -1, 4)).toThrow();
    });

    it("round-trips with encodeBigEndianInt32s (aligned)", () => {
        const input = new Int32Array([0, 1, -1, 0x12345678]);
        const bytes = encodeBigEndianInt32s(input);
        const decoded = decodeBigEndianInt32s(bytes, 0, bytes.length);
        expect(decoded).toEqual(input);
    });

    it("round-trips with encodeBigEndianInt32s (unaligned view)", () => {
        const input = new Int32Array([0x01020304, -123456789, 0, 42]);
        const bytes = encodeBigEndianInt32s(input);

        const buffer = new Uint8Array(bytes.length + 3);
        buffer.set([0xaa, 0xbb, 0xcc], 0);
        buffer.set(bytes, 3);

        const decoded = decodeBigEndianInt32s(buffer, 3, bytes.length);
        expect(decoded).toEqual(input);
    });

    it("decodes into a provided buffer", () => {
        const input = new Int32Array([0x01020304, -123456789, 0, 42]);
        const bytes = encodeBigEndianInt32s(input);

        const out = new Int32Array(input.length + 8);
        const written = decodeBigEndianInt32sInto(bytes, 0, bytes.length, out);
        expect(written).toBe(input.length);

        const decoded = out.subarray(0, written);
        expect(decoded).toEqual(input);

        const encoded = encodeBigEndianInt32s(decoded);
        expect(encoded).toEqual(bytes);
    });

    it("decodeBigEndianInt32sInto throws when output buffer is too small", () => {
        const bytes = new Uint8Array([0x12, 0x34, 0x56, 0x78]);
        const out = new Int32Array(0);
        expect(() => decodeBigEndianInt32sInto(bytes, 0, bytes.length, out)).toThrow();
    });
});
