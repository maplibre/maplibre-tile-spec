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
import { decodeFastPforInt32 } from "../../src/decoding/fastPforDecoder";
import { int32sToBigEndianBytes } from "../../src/decoding/byteIO";

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const FIXTURES_DIR = path.resolve(__dirname, "../../../test/fixtures/fastpfor");

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
    if (!fs.existsSync(FIXTURES_DIR)) return [];
    return fs
        .readdirSync(FIXTURES_DIR)
        .map((f) => f.match(/^vector(\d+)_compressed\.bin$/)?.[1])
        .filter((v): v is string => v !== undefined)
        .map((v) => parseInt(v, 10))
        .sort((a, b) => a - b);
}

describe("FastPFOR Integration: C++ encoded → TS decoded", () => {
    const vectorIndices = discoverFixtureVectors();

    it("has at least one fixture vector", () => {
        expect(vectorIndices.length).toBeGreaterThan(0);
    });

    for (const idx of vectorIndices) {
        it(`decodes C++ vector${idx}_compressed → vector${idx}_uncompressed`, () => {
            const encoded = loadBinaryFixture(`vector${idx}_compressed.bin`);
            const expected = loadBinaryFixture(`vector${idx}_uncompressed.bin`);

            // Test using int32 array directly
            const decoded = decodeFastPforInt32(encoded, expected.length);
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
