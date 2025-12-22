import { describe, expect, it } from "vitest";

import { decodeFastPfor } from "../decoding/integerDecodingUtils";
import IntWrapper from "../decoding/intWrapper";
import { encodeFastPfor } from "./integerEncodingUtils";
import { bigEndianBytesToInt32s } from "../fastPforCodec";

function readInt32BigEndian(buf: Uint8Array, offset: number): number {
    return ((buf[offset] << 24) | (buf[offset + 1] << 16) | (buf[offset + 2] << 8) | buf[offset + 3]) | 0;
}

describe("fastpfor wire format and hardening", () => {
    // ===========================================
    // Endian / byte conversion tests
    // ===========================================

    it("bigEndianBytesToInt32s pads trailing bytes to last int32", () => {
        const bytes = new Uint8Array([0x01, 0x02, 0x03, 0x04, 0xaa, 0xbb]);
        const ints = bigEndianBytesToInt32s(bytes, 0, bytes.length);
        expect(ints.length).toBe(2);
        expect(ints[0]).toBe(0x01020304);
        expect(ints[1]).toBe(0xaabb0000 | 0); // trailing bytes padded with zeros
    });

    // ===========================================
    // Wire format header tests (alignedLength field)
    // ===========================================

    it("writes alignedLength=0 for <256 values (VariableByte-only)", () => {
        const values = new Int32Array(Array.from({ length: 100 }, (_, i) => i));
        const encoded = encodeFastPfor(values);
        // First 4 bytes = alignedLength in big-endian; should be 0 (no FastPFOR blocks)
        expect(readInt32BigEndian(encoded, 0)).toBe(0);
    });

    it("writes alignedLength=256 for 256 values (one full block)", () => {
        const values = new Int32Array(Array.from({ length: 256 }, (_, i) => i));
        const encoded = encodeFastPfor(values);
        expect(readInt32BigEndian(encoded, 0)).toBe(256);
    });

    it("writes valid alignedLength for multi-page (66000 values)", () => {
        const values = new Int32Array(Array.from({ length: 66000 }, (_, i) => i % 1000));
        const encoded = encodeFastPfor(values);
        const alignedLength = readInt32BigEndian(encoded, 0);
        // Must be multiple of 256, positive, and <= input length
        expect(alignedLength % 256).toBe(0);
        expect(alignedLength).toBeGreaterThan(0);
        expect(alignedLength).toBeLessThanOrEqual(66000);
    });

    // ===========================================
    // Corruption hardening tests
    // ===========================================

    it("throws on corrupted alignedLength (negative: 0xFFFFFFFF)", () => {
        const values = new Int32Array(Array.from({ length: 512 }, (_, i) => i));
        const encoded = encodeFastPfor(values);
        // Corrupt to 0xFFFFFFFF = -1 in signed int32
        encoded[0] = 0xff;
        encoded[1] = 0xff;
        encoded[2] = 0xff;
        encoded[3] = 0xff;

        const offset = new IntWrapper(0);
        expect(() => decodeFastPfor(encoded, values.length, encoded.length, offset)).toThrow(/FastPFOR/i);
    });

    it("throws on corrupted alignedLength (not multiple of 256: 255)", () => {
        const values = new Int32Array(Array.from({ length: 512 }, (_, i) => i));
        const encoded = encodeFastPfor(values);
        // Set alignedLength = 255 (0x000000FF) in big-endian
        encoded[0] = 0x00;
        encoded[1] = 0x00;
        encoded[2] = 0x00;
        encoded[3] = 0xff;

        const offset = new IntWrapper(0);
        expect(() => decodeFastPfor(encoded, values.length, encoded.length, offset)).toThrow(/FastPFOR/i);
    });
});
