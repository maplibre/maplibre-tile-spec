import { describe, expect, it } from "vitest";

import { decodeFastPfor } from "../decoding/integerDecodingUtils";
import IntWrapper from "../decoding/intWrapper";
import { encodeFastPfor } from "./integerEncodingUtils";

/**
 * Round-trip test helper: encode then decode and compare.
 */
function roundTrip(values: Int32Array): Int32Array {
    const encoded = encodeFastPfor(values);
    const offset = new IntWrapper(0);
    return decodeFastPfor(encoded, values.length, encoded.length, offset);
}

/**
 * Generate random values within a specific bit range.
 */
function generateRandomValues(count: number, maxBits: number, seed: number = 12345): Int32Array {
    const values = new Int32Array(count);
    const maxValue = (1 << maxBits) - 1;
    let s = seed;
    for (let i = 0; i < count; i++) {
        s = (s * 1103515245 + 12345) >>> 0;
        values[i] = (s >>> (32 - maxBits)) & maxValue;
    }
    return values;
}

describe("fastpfor bitwidth coverage tests", () => {
    // Test each bitwidth from 1 to 16 (the specialized paths)
    const bitwidths = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 16];

    for (const bits of bitwidths) {
        it(`round-trips ${bits}-bit values correctly`, () => {
            // Use 512 values to ensure we have full FastPFOR blocks
            const values = generateRandomValues(512, bits);
            const decoded = roundTrip(values);
            expect(Array.from(decoded)).toEqual(Array.from(values));
        });
    }

    // Test generic fallback path (bitwidths 13-15)
    for (const bits of [13, 14, 15]) {
        it(`round-trips ${bits}-bit values via generic fallback`, () => {
            const values = generateRandomValues(512, bits);
            const decoded = roundTrip(values);
            expect(Array.from(decoded)).toEqual(Array.from(values));
        });
    }

    // Edge case: all zeros (bitwidth 0)
    it("round-trips all zeros correctly", () => {
        const values = new Int32Array(512);
        const decoded = roundTrip(values);
        expect(Array.from(decoded)).toEqual(Array.from(values));
    });

    // Edge case: all same non-zero value
    it("round-trips constant values correctly", () => {
        const values = new Int32Array(512).fill(42);
        const decoded = roundTrip(values);
        expect(Array.from(decoded)).toEqual(Array.from(values));
    });

    // Edge case: maximum 16-bit values
    it("round-trips maximum 16-bit values correctly", () => {
        const values = new Int32Array(512).fill(65535);
        const decoded = roundTrip(values);
        expect(Array.from(decoded)).toEqual(Array.from(values));
    });

    // Edge case: values with exceptions (some outliers)
    it("round-trips values with exceptions correctly", () => {
        const values = new Int32Array(512);
        // Most values are small (2-bit)
        for (let i = 0; i < 512; i++) {
            values[i] = i % 4;
        }
        // Add some large outliers (exceptions)
        values[50] = 1000;
        values[100] = 2000;
        values[200] = 65535;

        const decoded = roundTrip(values);
        expect(Array.from(decoded)).toEqual(Array.from(values));
    });

    // Large dataset: multi-page encoding
    it("round-trips large dataset (66000 values) correctly", () => {
        const values = generateRandomValues(66000, 12);
        const decoded = roundTrip(values);
        expect(decoded.length).toBe(values.length);
        expect(Array.from(decoded)).toEqual(Array.from(values));
    });

    // Bitwidth 17-20 (rare but should work via generic fallback)
    for (const bits of [17, 18, 19, 20]) {
        it(`round-trips ${bits}-bit values via generic fallback`, () => {
            const values = generateRandomValues(512, bits);
            const decoded = roundTrip(values);
            expect(Array.from(decoded)).toEqual(Array.from(values));
        });
    }
});
