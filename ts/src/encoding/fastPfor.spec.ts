/**
 * FastPFOR Unit Tests
 *
 * In-memory tests for FastPFOR codec functionality.
 * Tests bitwidth dispatch, edge cases, and roundtrip correctness.
 * Does NOT read from filesystem - see test/integration/ for fixture-based tests.
 */

import { describe, expect, it } from "vitest";

import { decodeFastPfor } from "../decoding/integerDecodingUtils";
import IntWrapper from "../decoding/intWrapper";
import { encodeFastPfor } from "./integerEncodingUtils";

const BLOCK_SIZE = 256;
const TWO_BLOCKS = BLOCK_SIZE * 2;

/**
 * Generate deterministic pseudo-random values constrained to a specific bitwidth using LCG.
 * Uses Math.imul for safe 32-bit multiplication.
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
        const u = shift === 0 ? s : s >>> shift;
        values[i] = u | 0;
    }

    // Force maxBits == bitwidth by injecting the max value at block boundaries
    const max = bitwidth === 32 ? -1 : (Math.pow(2, bitwidth) - 1) | 0;
    values[0] = max;
    if (count > BLOCK_SIZE) values[BLOCK_SIZE] = max;

    return values;
}

describe("FastPFOR: bitwidth dispatch", () => {
    const unrolledBitwidths = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 16];
    const genericBitwidths = [13, 14, 15, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32];

    for (const bw of unrolledBitwidths) {
        it(`unrolled bitwidth ${bw}: roundtrip`, () => {
            const values = generateDeterministicValues(TWO_BLOCKS, bw);
            const encoded = encodeFastPfor(values);

            const offset = new IntWrapper(0);
            const decoded = decodeFastPfor(encoded, values.length, encoded.length, offset);

            expect(decoded).toEqual(values);
            expect(offset.get()).toBe(encoded.length);
        });
    }

    for (const bw of genericBitwidths) {
        it(`generic bitwidth ${bw}: roundtrip`, () => {
            const values = generateDeterministicValues(TWO_BLOCKS, bw);
            const encoded = encodeFastPfor(values);

            const offset = new IntWrapper(0);
            const decoded = decodeFastPfor(encoded, values.length, encoded.length, offset);

            expect(decoded).toEqual(values);
            expect(offset.get()).toBe(encoded.length);
        });
    }
});

describe("FastPFOR: edge cases", () => {
    it("handles empty input", () => {
        const values = new Int32Array([]);
        const encoded = encodeFastPfor(values);

        const offset = new IntWrapper(0);
        const decoded = decodeFastPfor(encoded, values.length, encoded.length, offset);

        expect(decoded).toEqual(values);
        expect(offset.get()).toBe(encoded.length);
    });

    it("handles non-block sizes (255, 257)", () => {
        for (const size of [BLOCK_SIZE - 1, BLOCK_SIZE + 1]) {
            const values = new Int32Array(size);
            for (let i = 0; i < size; i++) {
                values[i] = i % 1000;
            }
            const encoded = encodeFastPfor(values);

            const offset = new IntWrapper(0);
            const decoded = decodeFastPfor(encoded, values.length, encoded.length, offset);

            expect(decoded).toEqual(values);
            expect(offset.get()).toBe(encoded.length);
        }
    });

    it("handles values with exceptions (outliers)", () => {
        const values = new Int32Array(TWO_BLOCKS);
        for (let i = 0; i < TWO_BLOCKS; i++) values[i] = i % 16;

        values[50] = 1000;
        values[100] = 50000;
        values[200] = 65535;
        values[400] = 100000;

        const encoded = encodeFastPfor(values);

        const offset = new IntWrapper(0);
        const decoded = decodeFastPfor(encoded, values.length, encoded.length, offset);

        expect(decoded).toEqual(values);
        expect(offset.get()).toBe(encoded.length);
    });

    it("handles all-zeros (bitwidth 0)", () => {
        const values = new Int32Array(TWO_BLOCKS);
        const encoded = encodeFastPfor(values);

        const offset = new IntWrapper(0);
        const decoded = decodeFastPfor(encoded, values.length, encoded.length, offset);

        expect(decoded).toEqual(values);
        expect(offset.get()).toBe(encoded.length);
    });

    it("handles constant non-zero values", () => {
        const values = new Int32Array(TWO_BLOCKS).fill(42);
        const encoded = encodeFastPfor(values);

        const offset = new IntWrapper(0);
        const decoded = decodeFastPfor(encoded, values.length, encoded.length, offset);

        expect(decoded).toEqual(values);
        expect(offset.get()).toBe(encoded.length);
    });

    it("handles large inputs (e.g. 66000 values)", () => {
        const values = new Int32Array(66000);
        for (let i = 0; i < values.length; i++) {
            values[i] = i % 1000;
        }
        const encoded = encodeFastPfor(values);

        const offset = new IntWrapper(0);
        const decoded = decodeFastPfor(encoded, values.length, encoded.length, offset);

        expect(decoded).toEqual(values);
        expect(offset.get()).toBe(encoded.length);
    });

    it("handles non-zero offsets in decodeFastPfor", () => {
        const values = new Int32Array(BLOCK_SIZE);
        for (let i = 0; i < values.length; i++) {
            values[i] = i;
        }
        const encoded = encodeFastPfor(values);

        // Add a prefix of 2 dummy bytes
        const prefix = new Uint8Array([0xaa, 0xbb]);
        const buffer = new Uint8Array(prefix.length + encoded.length);
        buffer.set(prefix, 0);
        buffer.set(encoded, prefix.length);

        const offset = new IntWrapper(prefix.length);
        const decoded = decodeFastPfor(buffer, values.length, encoded.length, offset);

        expect(decoded).toEqual(values);
        expect(offset.get()).toBe(buffer.length);
    });
});
