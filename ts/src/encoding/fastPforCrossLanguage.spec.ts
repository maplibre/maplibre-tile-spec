/**
 * Cross-Language Validation Tests for FastPFOR
 *
 * These tests validate that the TypeScript encoder/decoder produces
 * results compatible with the C++ reference implementation.
 *
 * Strategy:
 * 1. Load pre-generated binary fixtures (C++ encoded data)
 * 2. Validate TS decoding of C++ encoded data
 * 3. Exercise unrolled (1-12, 16) and generic (13-15) bitwidth paths
 */

import { describe, expect, it } from "vitest";

import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

import { decodeFastPfor } from "../decoding/integerDecodingUtils";
import IntWrapper from "../decoding/intWrapper";
import { int32sToBigEndianBytes, uncompressFastPforInt32 } from "../fastPforCodec";
import { encodeFastPfor } from "./integerEncodingUtils";

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const FIXTURES_DIR = path.resolve(__dirname, "../../../test/fixtures/fastpfor");

// FastPFOR processes blocks of 256 values
const BLOCK_SIZE = 256;
const TWO_BLOCKS = BLOCK_SIZE * 2;

/**
 * Load a binary fixture as Int32Array (big-endian format).
 */
function loadBinaryFixture(name: string): Int32Array {
    const filepath = path.join(FIXTURES_DIR, name);
    const buffer = fs.readFileSync(filepath);
    const values = new Int32Array(buffer.length / 4);
    for (let i = 0; i < values.length; i++) {
        values[i] = buffer.readInt32BE(i * 4);
    }
    return values;
}

/**
 * Generate random values constrained to a specific bitwidth using LCG.
 * Uses Math.imul for safe 32-bit multiplication.
 */
function maxUnsigned(bits: number): number {
    if (bits <= 0) return 0;
    if (bits >= 32) return 0xffffffff;
    return (Math.pow(2, bits) - 1) >>> 0;
}

function generateBitwidthValues(count: number, bitwidth: number, seed: number = 42): Int32Array {
    if (bitwidth < 0 || bitwidth > 32) {
        throw new RangeError(`bitwidth must be 0-32, got ${bitwidth}`);
    }

    const values = new Int32Array(count);
    if (bitwidth === 0) return values;

    let s = seed >>> 0;
    const shift = 32 - bitwidth;

    for (let i = 0; i < count; i++) {
        s = (Math.imul(s, 1103515245) + 12345) >>> 0;
        values[i] = (shift === 0 ? (s | 0) : (s >>> shift)) | 0;
    }

    // Force maxBits == bitwidth by injecting max value at block boundaries
    const max = maxUnsigned(bitwidth) | 0;
    if (count > 0) values[0] = max;
    if (count > BLOCK_SIZE) values[BLOCK_SIZE] = max;

    return values;
}

function roundTrip(values: Int32Array): { encoded: Uint8Array; decoded: Int32Array } {
    const encoded = encodeFastPfor(values);
    const offset = new IntWrapper(0);
    const decoded = decodeFastPfor(encoded, values.length, encoded.length, offset);
    // Verify decoder consumed all bytes
    expect(offset.get()).toBe(encoded.length);
    return { encoded, decoded };
}

describe("cross-language: C++ encoded → TS decoded", () => {
    // Test all 4 reference vectors from binary fixtures
    for (const idx of [1, 2, 3, 4]) {
        it(`decodes C++ vector${idx}_compressed → vector${idx}_uncompressed`, () => {
            const encoded = loadBinaryFixture(`vector${idx}_compressed.bin`);
            const expected = loadBinaryFixture(`vector${idx}_uncompressed.bin`);

            // Test using int32 array directly
            const decoded = uncompressFastPforInt32(encoded, expected.length);
            expect(decoded).toEqual(expected);

            // Test using byte array (how it comes from MLT tiles)
            const bytes = int32sToBigEndianBytes(encoded);
            const offset = new IntWrapper(0);
            const decodedFromBytes = decodeFastPfor(bytes, expected.length, bytes.length, offset);
            expect(decodedFromBytes).toEqual(expected);
            expect(offset.get()).toBe(bytes.length);
        });
    }
});

describe("cross-language: TS encode/decode with bitwidth validation", () => {
    // Unrolled bitwidths (specialized functions)
    const unrolledBitwidths = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 16];

    // Generic fallback bitwidths (including high bitwidths to test sign bit handling)
    const genericBitwidths = [13, 14, 15, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32];

    for (const bw of unrolledBitwidths) {
        it(`unrolled bitwidth ${bw}: TS roundtrip produces correct results`, () => {
            const values = generateBitwidthValues(TWO_BLOCKS, bw);
            const { decoded } = roundTrip(values);
            expect(decoded).toEqual(values);
        });
    }

    for (const bw of genericBitwidths) {
        it(`generic fallback bitwidth ${bw}: TS roundtrip produces correct results`, () => {
            const values = generateBitwidthValues(TWO_BLOCKS, bw);
            const { decoded } = roundTrip(values);
            expect(decoded).toEqual(values);
        });
    }
});

describe("cross-language: TS encoder determinism", () => {
    it("produces identical encoding on consecutive calls", () => {
        const values = new Int32Array(Array.from({ length: TWO_BLOCKS }, (_, i) => i % 1000));

        const encoded1 = encodeFastPfor(values);
        const encoded2 = encodeFastPfor(values);

        expect(encoded1).toEqual(encoded2);
    });

    it("double roundtrip produces identical encodings", () => {
        const original = new Int32Array(Array.from({ length: TWO_BLOCKS }, (_, i) => i % 1000));

        // First roundtrip
        const encoded1 = encodeFastPfor(original);
        const offset1 = new IntWrapper(0);
        const decoded1 = decodeFastPfor(encoded1, original.length, encoded1.length, offset1);
        expect(offset1.get()).toBe(encoded1.length); // anti-garbage check

        // Second roundtrip
        const encoded2 = encodeFastPfor(decoded1);
        const offset2 = new IntWrapper(0);
        const decoded2 = decodeFastPfor(encoded2, decoded1.length, encoded2.length, offset2);
        expect(offset2.get()).toBe(encoded2.length); // anti-garbage check

        // Both decoded values should be identical to original
        expect(decoded1).toEqual(original);
        expect(decoded2).toEqual(original);

        // Encodings should be identical (deterministic)
        expect(encoded1).toEqual(encoded2);
    });
});

describe("cross-language: edge cases", () => {
    // Note: empty input, VB-only, 256+remainder, multi-page are tested in fastPforEncodingUtils.spec.ts

    it("handles values with exceptions (outliers)", () => {
        const values = new Int32Array(TWO_BLOCKS);
        // Base values are 4-bit (0-15)
        for (let i = 0; i < TWO_BLOCKS; i++) {
            values[i] = i % 16;
        }
        // Add large outliers that require exception handling
        values[50] = 1000;
        values[100] = 50000;
        values[200] = 65535;
        values[400] = 100000;

        const { decoded } = roundTrip(values);
        expect(decoded).toEqual(values);
    });

    it("handles all-zeros (bitwidth 0)", () => {
        const values = new Int32Array(TWO_BLOCKS);
        const { decoded } = roundTrip(values);
        expect(decoded).toEqual(values);
    });

    it("handles constant non-zero values", () => {
        const values = new Int32Array(TWO_BLOCKS).fill(42);
        const { decoded } = roundTrip(values);
        expect(decoded).toEqual(values);
    });
});
