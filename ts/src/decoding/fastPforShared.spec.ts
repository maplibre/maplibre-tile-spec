
import { describe, expect, it } from "vitest";
import { BLOCK_SIZE, DEFAULT_PAGE_SIZE, MASKS, greatestMultiple, normalizePageSize, roundUpToMultipleOf32 } from "./fastPforShared";

describe("FastPforShared", () => {
    describe("normalizePageSize", () => {
        it("returns DEFAULT_PAGE_SIZE for invalid inputs", () => {
            expect(normalizePageSize(0)).toBe(DEFAULT_PAGE_SIZE);
            expect(normalizePageSize(-1)).toBe(DEFAULT_PAGE_SIZE);
            expect(normalizePageSize(NaN)).toBe(DEFAULT_PAGE_SIZE);
            expect(normalizePageSize(Infinity)).toBe(DEFAULT_PAGE_SIZE);
            expect(normalizePageSize(-Infinity)).toBe(DEFAULT_PAGE_SIZE);
        });

        it("rounds down to nearest multiple of BLOCK_SIZE", () => {
            expect(normalizePageSize(BLOCK_SIZE * 2 + 10)).toBe(BLOCK_SIZE * 2);
            expect(normalizePageSize(BLOCK_SIZE * 10 + BLOCK_SIZE - 1)).toBe(BLOCK_SIZE * 10);
        });

        it("clamps small values to BLOCK_SIZE (min size)", () => {
            expect(normalizePageSize(1)).toBe(BLOCK_SIZE);
            expect(normalizePageSize(BLOCK_SIZE - 1)).toBe(BLOCK_SIZE);
            expect(normalizePageSize(BLOCK_SIZE)).toBe(BLOCK_SIZE);
        });

        it("handles float inputs by flooring", () => {
            expect(normalizePageSize(BLOCK_SIZE * 2.5)).toBe(BLOCK_SIZE * 2);
        });

        it("returns input when already a valid multiple of BLOCK_SIZE", () => {
            expect(normalizePageSize(BLOCK_SIZE * 4)).toBe(BLOCK_SIZE * 4);
        });

        it("handles large values", () => {
            expect(normalizePageSize(BLOCK_SIZE * 1_000)).toBe(BLOCK_SIZE * 1_000);
            expect(normalizePageSize(BLOCK_SIZE * 1_000 + 123)).toBe(BLOCK_SIZE * 1_000);
        });

        it("returns DEFAULT_PAGE_SIZE for non-number inputs", () => {
            expect(normalizePageSize(undefined as any)).toBe(DEFAULT_PAGE_SIZE);
            expect(normalizePageSize(null as any)).toBe(DEFAULT_PAGE_SIZE);
        });
    });

    describe("greatestMultiple", () => {
        it("rounds down to the nearest multiple", () => {
            expect(greatestMultiple(10, 3)).toBe(9);
            expect(greatestMultiple(12, 3)).toBe(12);
        });
    });

    describe("roundUpToMultipleOf32", () => {
        it("rounds up to a multiple of 32", () => {
            expect(roundUpToMultipleOf32(0)).toBe(0);
            expect(roundUpToMultipleOf32(1)).toBe(32);
            expect(roundUpToMultipleOf32(32)).toBe(32);
            expect(roundUpToMultipleOf32(33)).toBe(64);
        });
    });
});
