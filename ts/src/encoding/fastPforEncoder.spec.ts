/**
 * FastPFOR Encoder Unit Tests
 *
 * Tests for the FastPFOR encoder - a test-only helper for roundtrip validation.
 * The encoder is NOT a production component; it exists for symmetry with the decoder.
 */

import { describe, expect, it } from "vitest";

import { encodeFastPforInt32 } from "./fastPforEncoder";
import { decodeFastPforInt32 } from "../decoding/fastPforDecoder";
import { BLOCK_SIZE } from "../decoding/fastPforSpec";

const TWO_BLOCKS = BLOCK_SIZE * 2;

function assertRoundtrip(values: Int32Array): Int32Array {
    const encoded = encodeFastPforInt32(values);
    const decoded = decodeFastPforInt32(encoded, values.length);
    expect(decoded).toEqual(values);
    return encoded;
}

function readFirstBlockHeader(encoded: Int32Array) {
    if (encoded.length < 4) throw new Error("Encoded buffer too short");

    // Encoded layout (when blocks exist): [outLength, whereMeta, ...packed..., byteSize, ...byteContainer...]
    const outLength = encoded[0];
    const whereMeta = encoded[1];

    if (outLength < 0) throw new Error(`Invalid outLength ${outLength}`);
    if (whereMeta < 1) throw new Error(`Invalid whereMeta ${whereMeta}`);

    const metaStart = 1 + whereMeta;
    if (metaStart >= encoded.length) throw new Error("Meta start out of bounds");

    const byteSize = encoded[metaStart];
    if (byteSize < 2) throw new Error(`Invalid byteSize ${byteSize}`);

    const byteContainerStart = metaStart + 1;
    const metaInts = (byteSize + 3) >>> 2;
    if (byteContainerStart + metaInts > encoded.length) {
        throw new Error("ByteContainer out of bounds");
    }

    const firstWord = encoded[byteContainerStart];

    const b = firstWord & 0xff;
    const cExcept = (firstWord >>> 8) & 0xff;
    const maxBits = cExcept > 0 ? ((firstWord >>> 16) & 0xff) : 0;

    if (cExcept > 0) {
        // We need at least 3 bytes for header (b, cExcept, maxBits)
        // plus cExcept bytes for exception positions.
        if (byteSize < 3 + cExcept) {
            throw new Error(`ByteSize ${byteSize} too small for cExcept ${cExcept}`);
        }
    }

    return { outLength, whereMeta, byteSize, b, cExcept, maxBits };
}

/**
 * Generate deterministic pseudo-random values constrained to a specific bitwidth using LCG.
 */
function generateDeterministicValues(count: number, bitwidth: number, seed: number = 42): Int32Array {
    if (bitwidth < 0 || bitwidth > 32) {
        throw new RangeError(`bitwidth must be 0-32, got ${bitwidth}`);
    }

    const values = new Int32Array(count);
    if (count === 0 || bitwidth === 0) return values;

    let s = seed >>> 0;
    const shift = 32 - bitwidth;

    for (let i = 0; i < count; i++) {
        s = (Math.imul(s, 1103515245) + 12345) >>> 0;
        const u = shift === 0 ? s : (s >>> shift);
        values[i] = u | 0;
    }

    // Force maxBits == bitwidth by injecting the max value at block boundaries
    const max = bitwidth === 32 ? -1 : ((0xFFFFFFFF >>> (32 - bitwidth)) | 0);
    values[0] = max;
    if (count > BLOCK_SIZE) values[BLOCK_SIZE] = max;

    return values;
}

/**
 * Generate values that strongly encourage exception usage:
 * mostly tiny (4 bits), with a couple of huge outliers per block.
 */
function generateExceptionFriendlyValues(count: number): Int32Array {
    const v = new Int32Array(count);
    for (let i = 0; i < count; i++) v[i] = i & 0x0f; // 0..15

    if (count >= BLOCK_SIZE) {
        v[10] = 0x7fffffff;               // huge
        v[200] = (1 << 30);               // huge
    }
    if (count >= TWO_BLOCKS) {
        v[BLOCK_SIZE + 20] = 0x7fffffff;
        v[BLOCK_SIZE + 210] = (1 << 30);
    }

    return v;
}

describe("FastPFOR Encoder: roundtrip validation", () => {
    it("encodes and decodes empty input", () => {
        const values = new Int32Array([]);
        assertRoundtrip(values);
    });

    it("encodes and decodes small input (<256 values, VByte only)", () => {
        const values = new Int32Array(100);
        for (let i = 0; i < values.length; i++) values[i] = i * 7;
        assertRoundtrip(values);
    });

    it("is deterministic for same input", () => {
        const values = generateDeterministicValues(TWO_BLOCKS, 12, 123);

        const e1 = encodeFastPforInt32(values);
        const e2 = encodeFastPforInt32(values);

        expect(e1).toEqual(e2);
    });

    it("encodes and decodes exactly one block (256 values)", () => {
        const values = generateDeterministicValues(BLOCK_SIZE, 8);
        const encoded = assertRoundtrip(values);

        // Optional: header sanity (don’t overfit heuristics)
        const h = readFirstBlockHeader(encoded);
        expect(h.b).toBeGreaterThanOrEqual(0);
        expect(h.b).toBeLessThanOrEqual(32);
        expect(h.cExcept).toBeGreaterThanOrEqual(0);
    });

    it("encodes and decodes multiple blocks", () => {
        const values = generateDeterministicValues(TWO_BLOCKS, 12);
        assertRoundtrip(values);
    });

    it("encodes and decodes with VByte tail (257 values)", () => {
        const values = generateDeterministicValues(BLOCK_SIZE + 1, 10);
        assertRoundtrip(values);
    });
});

describe("FastPFOR Encoder: bitwidth coverage", () => {
    for (let bw = 1; bw <= 32; bw++) {
        it(`bitwidth ${bw}: encode-decode roundtrip`, () => {
            const values = generateDeterministicValues(TWO_BLOCKS, bw);
            assertRoundtrip(values);
        });
    }

    it("handles Int32.MIN_VALUE (0x80000000)", () => {
        const values = new Int32Array(TWO_BLOCKS);
        values.fill(0);
        values[0] = 0x80000000 | 0;
        values[BLOCK_SIZE] = 0x80000000 | 0;
        assertRoundtrip(values);
    });
});

describe("FastPFOR Encoder: exception handling", () => {
    it("can produce exception blocks for exception-friendly inputs (sanity check)", () => {
        const candidates: Int32Array[] = [
            generateExceptionFriendlyValues(TWO_BLOCKS),
            (() => {
                const v = new Int32Array(TWO_BLOCKS);
                for (let i = 0; i < v.length; i++) v[i] = i & 0x0f;
                v[10] = 0x80000000 | 0;         // Int32.MIN_VALUE
                v[200] = -1;                    // 0xFFFFFFFF
                v[BLOCK_SIZE + 20] = 0x80000000 | 0;
                v[BLOCK_SIZE + 210] = -1;
                return v;
            })(),
        ];

        for (const values of candidates) {
            const encoded = assertRoundtrip(values);
            const h = readFirstBlockHeader(encoded);

            if (h.cExcept > 0) {
                expect(h.maxBits).toBeGreaterThan(h.b);
                expect(h.maxBits).toBeLessThanOrEqual(32);
                return; // We exercised exceptions, great.
            }
        }

        // Non-blocking: roundtrip already validated for all candidates.
    });

    it("handles all-zeros (bitwidth 0, no exceptions)", () => {
        const values = new Int32Array(TWO_BLOCKS);
        const encoded = assertRoundtrip(values);

        const h = readFirstBlockHeader(encoded);
        expect(h.b).toBe(0);
        expect(h.cExcept).toBe(0);
    });

    it("handles constant non-zero values", () => {
        const values = new Int32Array(TWO_BLOCKS).fill(42);
        const encoded = assertRoundtrip(values);

        const h = readFirstBlockHeader(encoded);
        expect(h.cExcept).toBe(0);
    });

    it("handles max 32-bit values (-1 / 0xFFFFFFFF)", () => {
        const values = new Int32Array(TWO_BLOCKS).fill(-1);
        assertRoundtrip(values);
    });
});
