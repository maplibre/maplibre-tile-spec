/**
 * Cross-Language Integration Tests for FastPFOR
 *
 * These tests validate that the TypeScript encoder/decoder produces
 * results compatible with the C++ reference implementation by loading
 * pre-generated binary fixtures.
 */

import { describe, expect, it } from "vitest";

import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

import { decodeFastPfor } from "../../src/decoding/integerDecodingUtils";
import IntWrapper from "../../src/decoding/intWrapper";
import { int32sToBigEndianBytes, uncompressFastPforInt32 } from "../../src/fastPforCodec";

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const FIXTURES_DIR = path.resolve(__dirname, "../../../test/fixtures/fastpfor");

/**
 * TypedArray comparison helper to avoid relying on deep-equality behavior
 * across different ArrayBuffer-backed types (e.g. Int32Array, BigInt64Array, Float64Array).
 */
function expectArrayLikeEqual<T extends ArrayLike<number> | ArrayLike<bigint>>(actual: T, expected: T) {
    expect(actual.length).toBe(expected.length);
    for (let i = 0; i < actual.length; i++) {
        expect(actual[i]).toBe(expected[i]);
    }
}

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
 * Discover fixture vector pairs by scanning the fixtures directory.
 * Returns array of vector indices (e.g., [1, 2, 3, 4]).
 */
function discoverFixtureVectors(): number[] {
    const files = fs.readdirSync(FIXTURES_DIR);
    const indices = new Set<number>();

    for (const file of files) {
        const match = file.match(/^vector(\d+)_compressed\.bin$/);
        if (match) {
            const idx = parseInt(match[1], 10);
            // Verify both compressed and uncompressed files exist
            const uncompressed = `vector${idx}_uncompressed.bin`;
            if (files.includes(uncompressed)) {
                indices.add(idx);
            }
        }
    }

    return Array.from(indices).sort((a, b) => a - b);
}

describe("FastPFOR Integration: C++ encoded → TS decoded", () => {
    const vectorIndices = discoverFixtureVectors();

    for (const idx of vectorIndices) {
        it(`decodes C++ vector${idx}_compressed → vector${idx}_uncompressed`, () => {
            const encoded = loadBinaryFixture(`vector${idx}_compressed.bin`);
            const expected = loadBinaryFixture(`vector${idx}_uncompressed.bin`);

            // Test using int32 array directly
            const decoded = uncompressFastPforInt32(encoded, expected.length);
            expectArrayLikeEqual(decoded, expected);

            // Test using byte array (how it comes from MLT tiles)
            const bytes = int32sToBigEndianBytes(encoded);
            const offset = new IntWrapper(0);
            const decodedFromBytes = decodeFastPfor(bytes, expected.length, bytes.length, offset);
            expectArrayLikeEqual(decodedFromBytes, expected);
            expect(offset.get()).toBe(bytes.length);
        });
    }
});
