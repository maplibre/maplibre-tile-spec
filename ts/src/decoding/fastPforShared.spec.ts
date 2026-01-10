
import { describe, expect, it } from "vitest";
import {
    BLOCK_SIZE,
    DEFAULT_PAGE_SIZE,
    IS_LE,
    MASKS,
    bswap32,
    greatestMultiple,
    normalizePageSize,
    roundUpToMultipleOf32,
} from "./fastPforShared";

describe("FastPforShared", () => {
    describe("MASKS", () => {
        it("contains 33 masks (0..32) and matches expected bit patterns", () => {
            expect(MASKS.length).toBe(33);
            expect(MASKS[0]).toBe(0);
            expect(MASKS[32]).toBe(0xffffffff);

            for (let bw = 1; bw < 32; bw++) {
                const expected = (1n << BigInt(bw)) - 1n;
                expect(BigInt(MASKS[bw] >>> 0)).toBe(expected);
            }
        });
    });

    describe("endian helpers", () => {
        it("IS_LE matches runtime endianness", () => {
            const buf = new ArrayBuffer(4);
            new Uint32Array(buf)[0] = 0x11223344;
            const firstByte = new Uint8Array(buf)[0];
            expect(IS_LE).toBe(firstByte === 0x44);
        });

        it("bswap32 swaps bytes", () => {
            expect(bswap32(0x11223344)).toBe(0x44332211);
            expect(bswap32(0x00000000)).toBe(0x00000000);
            expect(bswap32(0xffffffff)).toBe(0xffffffff);
            expect(bswap32(0x89abcdef)).toBe(0xefcdab89);
        });
    });

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
