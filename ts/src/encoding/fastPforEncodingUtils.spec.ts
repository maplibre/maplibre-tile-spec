import { describe, expect, it } from "vitest";

import { decodeFastPfor } from "../decoding/integerDecodingUtils";
import IntWrapper from "../decoding/intWrapper";
import { encodeFastPfor } from "./integerEncodingUtils";
import { bigEndianBytesToInt32s } from "../fastPforCodec";

function readInt32BigEndian(buf: Uint8Array, offset: number): number {
    return ((buf[offset] << 24) | (buf[offset + 1] << 16) | (buf[offset + 2] << 8) | buf[offset + 3]) | 0;
}

function roundTrip(values: Int32Array): { encoded: Uint8Array; decoded: Int32Array; offset: number } {
    const encoded = encodeFastPfor(values);
    const offset = new IntWrapper(0);
    const decoded = decodeFastPfor(encoded, values.length, encoded.length, offset);
    return { encoded, decoded, offset: offset.get() };
}

describe("fastpfor encode/decode round-trip", () => {
    it("converts trailing bytes to last int32 (big-endian)", () => {
        const bytes = new Uint8Array([0x01, 0x02, 0x03, 0x04, 0xaa, 0xbb]);
        const ints = bigEndianBytesToInt32s(bytes, 0, bytes.length);
        expect(ints.length).toBe(2);
        expect(ints[0]).toBe(0x01020304);
        expect(ints[1]).toBe(0xaabb0000 | 0);
    });

    it("handles empty input", () => {
        const values = new Int32Array(0);

        const { encoded, decoded, offset } = roundTrip(values);

        expect(encoded.length).toBe(0);
        expect(Array.from(decoded)).toEqual([]);
        expect(offset).toBe(0);
    });

    it("uses VariableByte-only path for <256 values", () => {
        const values = new Int32Array(Array.from({ length: 100 }, (_, i) => (i % 17) * 12345));

        const { encoded, decoded, offset } = roundTrip(values);

        expect(encoded.length % 4).toBe(0);
        expect(readInt32BigEndian(encoded, 0)).toBe(0);
        expect(Array.from(decoded)).toEqual(Array.from(values));
        expect(offset).toBe(encoded.length);
    });

    it("encodes 256 FastPFOR values + VariableByte remainder", () => {
        const values = new Int32Array(Array.from({ length: 358 }, (_, i) => i));

        const { encoded, decoded, offset } = roundTrip(values);

        expect(encoded.length % 4).toBe(0);
        expect(readInt32BigEndian(encoded, 0)).toBe(256);
        expect(Array.from(decoded)).toEqual(Array.from(values));
        expect(offset).toBe(encoded.length);
    });

    it("encodes full FastPFOR blocks when divisible by 256", () => {
        const values = new Int32Array(Array.from({ length: 512 }, (_, i) => i));

        const { encoded, decoded, offset } = roundTrip(values);

        expect(encoded.length % 4).toBe(0);
        expect(readInt32BigEndian(encoded, 0)).toBe(512);
        expect(Array.from(decoded)).toEqual(Array.from(values));
        expect(offset).toBe(encoded.length);
    });

    it("encodes multi-page FastPFOR streams (>65536 values)", () => {
        const values = new Int32Array(Array.from({ length: 66000 }, (_, i) => i % 10000));

        const { encoded, decoded, offset } = roundTrip(values);

        expect(encoded.length % 4).toBe(0);
        expect(readInt32BigEndian(encoded, 0)).toBe(65792);
        expect(decoded.length).toBe(values.length);
        expect(decoded[0]).toBe(0);
        expect(decoded[12345]).toBe(2345);
        expect(decoded[65999]).toBe(5999);
        expect(offset).toBe(encoded.length);
    });
});
