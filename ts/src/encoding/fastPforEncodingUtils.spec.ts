import { describe, expect, it } from "vitest";

import { decodeFastPfor } from "../decoding/integerDecodingUtils";
import IntWrapper from "../decoding/intWrapper";
import { encodeFastPfor } from "./integerEncodingUtils";
import { bigEndianBytesToInt32s } from "../decoding/byteIO";

const BLOCK = 256;

/**
 * Reads a signed int32 value from a big-endian byte buffer.
 *
 * @param buf - Source byte buffer.
 * @param offset - Byte offset.
 * @returns The decoded int32 value.
 */
function readInt32BigEndian(buf: Uint8Array, offset: number): number {
    return (
        (buf[offset] << 24) |
        (buf[offset + 1] << 16) |
        (buf[offset + 2] << 8) |
        buf[offset + 3]
    ) | 0;
}

/**
 * Writes a signed int32 value to a big-endian byte buffer.
 *
 * @param buf - Destination byte buffer.
 * @param offset - Byte offset.
 * @param value - Value to write.
 */
function writeInt32BigEndian(buf: Uint8Array, offset: number, value: number): void {
    buf[offset] = (value >>> 24) & 0xff;
    buf[offset + 1] = (value >>> 16) & 0xff;
    buf[offset + 2] = (value >>> 8) & 0xff;
    buf[offset + 3] = value & 0xff;
}

function alignedCount(n: number): number {
    return n - (n % BLOCK);
}

describe("fastpfor wire format and hardening", () => {
    describe("bigEndianBytesToInt32s", () => {
        it("pads trailing bytes to last int32", () => {
            const bytes = new Uint8Array([0x01, 0x02, 0x03, 0x04, 0xaa, 0xbb]);
            const ints = bigEndianBytesToInt32s(bytes, 0, bytes.length);

            expect(ints.length).toBe(2);
            expect(ints[0]).toBe(0x01020304);
            expect(ints[1]).toBe(0xaabb0000 | 0);
        });

        it("returns empty for empty range", () => {
            const bytes = new Uint8Array([0xde, 0xad, 0xbe, 0xef]);
            const ints = bigEndianBytesToInt32s(bytes, 0, 0);
            expect(ints.length).toBe(0);
        });

        it("handles unaligned buffer (byteOffset not multiple of 4)", () => {
            const baseBuffer = new ArrayBuffer(16);
            const fullView = new Uint8Array(baseBuffer);

            fullView[1] = 0x01;
            fullView[2] = 0x02;
            fullView[3] = 0x03;
            fullView[4] = 0x04;
            fullView[5] = 0x05;
            fullView[6] = 0x06;
            fullView[7] = 0x07;
            fullView[8] = 0x08;

            const unalignedBytes = new Uint8Array(baseBuffer, 1, 8);
            const ints = bigEndianBytesToInt32s(unalignedBytes, 0, 8);

            expect(ints.length).toBe(2);
            expect(ints[0]).toBe(0x01020304);
            expect(ints[1]).toBe(0x05060708);
        });

        it("handles unaligned buffer with trailing bytes", () => {
            const baseBuffer = new ArrayBuffer(16);
            const fullView = new Uint8Array(baseBuffer);

            fullView[1] = 0xaa;
            fullView[2] = 0xbb;
            fullView[3] = 0xcc;
            fullView[4] = 0xdd;
            fullView[5] = 0xee;
            fullView[6] = 0xff;

            const unalignedBytes = new Uint8Array(baseBuffer, 1, 6);
            const ints = bigEndianBytesToInt32s(unalignedBytes, 0, 6);

            expect(ints.length).toBe(2);
            expect(ints[0]).toBe(0xaabbccdd | 0);
            expect(ints[1]).toBe(0xeeff0000 | 0);
        });
    });

    describe("alignedLength header", () => {
        it("writes alignedLength = floor(n/256)*256 for a variety of sizes", () => {
            const sizes = [0, 1, 17, 100, 255, 256, 257, 511, 512, 513, 66000];

            for (const n of sizes) {
                const values = new Int32Array(n);
                for (let i = 0; i < n; i++) values[i] = i % 1000;

                const encoded = encodeFastPfor(values);
                const a = readInt32BigEndian(encoded, 0);

                expect(a).toBe(alignedCount(n));
            }
        });

        it("does not depend on input ArrayBuffer alignment (prefix bytes)", () => {
            const values = new Int32Array(512);
            for (let i = 0; i < values.length; i++) values[i] = i % 1000;

            const encoded = encodeFastPfor(values);
            const prefix = new Uint8Array([0xaa, 0xbb, 0xcc]);
            const suffix = new Uint8Array([0xff, 0xff, 0xff, 0xff, 0xff, 0xff]);

            const buffer = new Uint8Array(prefix.length + encoded.length + suffix.length);
            buffer.set(prefix, 0);
            buffer.set(encoded, prefix.length);
            buffer.set(suffix, prefix.length + encoded.length);

            const offset = new IntWrapper(prefix.length);
            const decoded = decodeFastPfor(buffer, values.length, encoded.length, offset);

            expect(decoded).toEqual(values);
            expect(offset.get()).toBe(prefix.length + encoded.length);
            expect(buffer.subarray(prefix.length + encoded.length)).toEqual(suffix);
        });
    });

    describe("corruption hardening", () => {
        it("throws on corrupted alignedLength (negative: 0xFFFFFFFF)", () => {
            const values = new Int32Array(512);
            for (let i = 0; i < values.length; i++) values[i] = i;

            const encoded = encodeFastPfor(values);
            writeInt32BigEndian(encoded, 0, -1);

            const offset = new IntWrapper(0);
            expect(() => decodeFastPfor(encoded, values.length, encoded.length, offset)).toThrow();
            expect(offset.get()).toBe(0);
        });

        it("throws on corrupted alignedLength (not multiple of 256: 255)", () => {
            const values = new Int32Array(512);
            for (let i = 0; i < values.length; i++) values[i] = i;

            const encoded = encodeFastPfor(values);
            writeInt32BigEndian(encoded, 0, 255);

            const offset = new IntWrapper(0);
            expect(() => decodeFastPfor(encoded, values.length, encoded.length, offset)).toThrow();
            expect(offset.get()).toBe(0);
        });

        it("throws when alignedLength > outputLength", () => {
            const values = new Int32Array(512);
            for (let i = 0; i < values.length; i++) values[i] = i;

            const encoded = encodeFastPfor(values);
            writeInt32BigEndian(encoded, 0, 768);

            const offset = new IntWrapper(0);
            expect(() => decodeFastPfor(encoded, values.length, encoded.length, offset)).toThrow();
            expect(offset.get()).toBe(0);
        });

        it("throws on truncated header (less than 4 bytes)", () => {
            const values = new Int32Array(512);
            for (let i = 0; i < values.length; i++) values[i] = i;

            const encoded = encodeFastPfor(values);
            const truncated = encoded.slice(0, 3);

            const offset = new IntWrapper(0);
            expect(() => decodeFastPfor(truncated, values.length, truncated.length, offset)).toThrow();
            expect(offset.get()).toBe(0);
        });

        it("throws when alignedLength > 0 but buffer ends right after header", () => {
            const values = new Int32Array(512);
            for (let i = 0; i < values.length; i++) values[i] = i;

            const encoded = encodeFastPfor(values);
            const truncated = encoded.slice(0, 4);

            const offset = new IntWrapper(0);
            expect(() => decodeFastPfor(truncated, values.length, truncated.length, offset)).toThrow();
            expect(offset.get()).toBe(0);
        });

        it("throws when declared encodedLength is too short for the stream", () => {
            const values = new Int32Array(512);
            for (let i = 0; i < values.length; i++) values[i] = i % 1000;

            const encoded = encodeFastPfor(values);

            const offset = new IntWrapper(0);
            expect(() => decodeFastPfor(encoded, values.length, encoded.length - 4, offset)).toThrow();
            expect(offset.get()).toBe(0);
        });
    });
});
