import { describe, expect, it } from "vitest";
import { decodeBigEndianInt32sInto } from "./bigEndianDecode";
import { encodeBigEndianInt32s } from "../encoding/bigEndianEncode";

function decodeThenEncode(bytes: Uint8Array, offset: number, byteLength: number): Uint8Array {
    const out = new Int32Array(Math.ceil(byteLength / 4));
    const written = decodeBigEndianInt32sInto(bytes, offset, byteLength, out);
    return encodeBigEndianInt32s(out.subarray(0, written));
}

function expectDecodeEncodeRoundTrip(bytes: Uint8Array, offset: number, byteLength: number): Uint8Array {
    const encoded = decodeThenEncode(bytes, offset, byteLength);
    const expected = bytes.subarray(offset, offset + byteLength);

    expect(encoded.subarray(0, byteLength)).toEqual(expected);
    for (let i = byteLength; i < encoded.length; i++) expect(encoded[i]).toBe(0);
    expect(encoded.length).toBe(Math.ceil(byteLength / 4) * 4);

    return encoded;
}

describe("decodeBigEndianInt32s", () => {
    it("round-trips bytes (aligned offset)", () => {
        const bytes = new Uint8Array([0x12, 0x34, 0x56, 0x78, 0xff, 0xff, 0xff, 0xff]);
        expectDecodeEncodeRoundTrip(bytes, 0, bytes.length);
    });

    it("round-trips bytes (unaligned offset)", () => {
        const buffer = new Uint8Array(10);
        buffer[1] = 0x00;
        buffer[2] = 0x00;
        buffer[3] = 0x00;
        buffer[4] = 0x42;
        expectDecodeEncodeRoundTrip(buffer, 1, 4);
    });

    it("round-trips bytes with trailing bytes (length not multiple of 4)", () => {
        const bytes = new Uint8Array([0x00, 0x00, 0x01, 0x00, 0xab]);
        expectDecodeEncodeRoundTrip(bytes, 0, 5);
    });

    it("round-trips bytes with 1-3 trailing bytes", () => {
        expectDecodeEncodeRoundTrip(new Uint8Array([0xaa]), 0, 1);
        expectDecodeEncodeRoundTrip(new Uint8Array([0xaa, 0xbb]), 0, 2);
        expectDecodeEncodeRoundTrip(new Uint8Array([0xaa, 0xbb, 0xcc]), 0, 3);
    });

    it("throws on out of bounds", () => {
        const bytes = new Uint8Array(4);
        expect(() => decodeBigEndianInt32sInto(bytes, 0, 5, new Int32Array(2))).toThrow();
        expect(() => decodeBigEndianInt32sInto(bytes, -1, 4, new Int32Array(1))).toThrow();
    });

    it("round-trips with encodeBigEndianInt32s (aligned)", () => {
        const input = new Int32Array([0, 1, -1, 0x12345678]);
        const bytes = encodeBigEndianInt32s(input);

        const out = new Int32Array(input.length);
        const written = decodeBigEndianInt32sInto(bytes, 0, bytes.length, out);
        expect(written).toBe(input.length);
        expect(out).toEqual(input);
    });

    it("round-trips with encodeBigEndianInt32s (unaligned view)", () => {
        const input = new Int32Array([0x01020304, -123456789, 0, 42]);
        const bytes = encodeBigEndianInt32s(input);

        const buffer = new Uint8Array(bytes.length + 3);
        buffer.set([0xaa, 0xbb, 0xcc], 0);
        buffer.set(bytes, 3);

        const out = new Int32Array(input.length);
        const written = decodeBigEndianInt32sInto(buffer, 3, bytes.length, out);
        expect(written).toBe(input.length);
        expect(out).toEqual(input);
    });

    it("decodes into a provided buffer", () => {
        const input = new Int32Array([0x01020304, -123456789, 0, 42]);
        const bytes = encodeBigEndianInt32s(input);

        const out = new Int32Array(input.length + 8);
        const written = decodeBigEndianInt32sInto(bytes, 0, bytes.length, out);
        expect(written).toBe(input.length);

        const encoded = encodeBigEndianInt32s(out.subarray(0, written));
        expect(encoded).toEqual(bytes);
    });

    it("decodeBigEndianInt32sInto throws when output buffer is too small", () => {
        const bytes = new Uint8Array([0x12, 0x34, 0x56, 0x78]);
        const out = new Int32Array(0);
        expect(() => decodeBigEndianInt32sInto(bytes, 0, bytes.length, out)).toThrow();
    });
});
