/**
 * Cross-Language Validation Tests for FastPFOR
 *
 * These tests validate that the TypeScript encoder/decoder produces
 * results compatible with the C++ reference implementation.
 *
 * Strategy:
 * 1. C++ encoded → TS decoded (reference compatibility)
 * 2. TS encoded → TS decoded → TS encoded (deterministic roundtrip)
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
const CPP_FASTPFOR_TEST = path.resolve(__dirname, "../../../cpp/test/test_fastpfor.cpp");

// FastPFOR processes blocks of 256 values
const BLOCK_SIZE = 256;
const TWO_BLOCKS = BLOCK_SIZE * 2;

/**
 * Parse a C++ uint32_t array from source code.
 * Returns Uint32Array to preserve bit-exact representation of unsigned values.
 */
function parseCppUint32Array(source: string, name: string): Uint32Array {
    const re = new RegExp(
        String.raw`\b(?:static\s+)?(?:constexpr\s+)?const\s+(?:std::)?uint32_t\s+${name}\s*\[\]\s*=\s*\{([\s\S]*?)\};`,
        "m",
    );
    const match = re.exec(source);
    if (!match) {
        throw new Error(`Failed to locate C++ array ${name}`);
    }

    // Strip C++ comments for robust parsing
    const body = match[1]
        .replace(/\/\/.*$/gm, "")
        .replace(/\/\*[\s\S]*?\*\//g, "");

    const tokens = body
        .split(",")
        .map((t) => t.trim())
        .filter((t) => t.length > 0);

    const values = new Uint32Array(tokens.length);
    for (let i = 0; i < tokens.length; i++) {
        let token = tokens[i];
        token = token.replace(/u$/i, "");
        token = token.replace(/^(?:UINT32_C|INT32_C)\((.*)\)$/, "$1");

        // Support hex literals (0x...)
        const parsed = token.startsWith("0x") || token.startsWith("0X")
            ? Number.parseInt(token, 16)
            : Number(token);

        if (!Number.isFinite(parsed)) {
            throw new Error(`Failed to parse token '${tokens[i]}' in ${name}`);
        }
        // Use >>> 0 to wrap to uint32 (handles negative values correctly)
        values[i] = parsed >>> 0;
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
    const cpp = fs.readFileSync(CPP_FASTPFOR_TEST, "utf8");

    // Test all 4 reference vectors from C++
    for (const idx of [1, 2, 3, 4]) {
        it(`decodes C++ compressed${idx} → uncompressed${idx}`, () => {
            const encoded = parseCppUint32Array(cpp, `compressed${idx}`);
            const expected = parseCppUint32Array(cpp, `uncompressed${idx}`);
            // Re-interpret as Int32Array for API compatibility (same bits, respecting byteOffset)
            const expectedI32 = new Int32Array(expected.buffer, expected.byteOffset, expected.length);
            expect(expectedI32.length).toBe(expected.length); // Sanity check

            // Test using int32 array directly
            // Re-interpret Uint32Array as Int32Array (same bits, signed view)
            const encodedI32 = new Int32Array(encoded.buffer, encoded.byteOffset, encoded.length);
            expect(encodedI32.length).toBe(encoded.length); // Sanity check
            const decoded = uncompressFastPforInt32(encodedI32, expectedI32.length);
            expect(decoded).toEqual(expectedI32);

            // Test using byte array (how it comes from MLT tiles)
            const bytes = int32sToBigEndianBytes(encodedI32);
            const offset = new IntWrapper(0);
            const decodedFromBytes = decodeFastPfor(bytes, expectedI32.length, bytes.length, offset);
            expect(decodedFromBytes).toEqual(expectedI32);
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

