import { describe, expect, it } from "vitest";
import * as fastPforUnpack from "./fastPforUnpack.g";

type Unpacker = (inValues: Int32Array, inPos: number, out: Int32Array, outPos: number) => void;

describe("FastPFOR Unpack Library", () => {
    function discoverUnpackers32(): Record<number, Unpacker> {
        const out: Record<number, Unpacker> = {};
        for (const [key, value] of Object.entries(fastPforUnpack as Record<string, unknown>)) {
            const match = /^fastUnpack32_(\d+)$/.exec(key);
            if (!match) continue;
            if (typeof value !== "function") continue;
            out[Number(match[1])] = value as Unpacker;
        }
        return out;
    }

    const unpackers = discoverUnpackers32();

    function discoverUnpackers256(): Record<number, Unpacker> {
        const out: Record<number, Unpacker> = {};
        for (const [key, value] of Object.entries(fastPforUnpack as Record<string, unknown>)) {
            const match = /^fastUnpack256_(\d+)$/.exec(key);
            if (!match) continue;
            if (typeof value !== "function") continue;
            out[Number(match[1])] = value as Unpacker;
        }
        return out;
    }

    const unpackers256 = discoverUnpackers256();

    function pack32(values: Int32Array, bw: number): Int32Array {
        const inWords = new Int32Array(bw);
        if (bw === 0) return inWords;

        const mask = bw === 32 ? -1 : (0xFFFFFFFF >>> (32 - bw));

        let bit = 0;
        let word = 0;

        for (let i = 0; i < 32; i++) {
            const v = (values[i] & mask) >>> 0;

            if (bit + bw <= 32) {
                inWords[word] |= (v << bit);
                bit += bw;
                if (bit === 32) {
                    bit = 0;
                    word++;
                }
            } else {
                const low = 32 - bit;
                inWords[word] |= (v << bit);
                word++;
                if (word >= inWords.length) throw new Error("packer overflow");
                inWords[word] |= (v >>> low);
                bit = bw - low;
            }
        }

        return inWords;
    }

    function pack256(values: Int32Array, bw: number): Int32Array {
        if (values.length !== 256) throw new Error("pack256 expects 256 values");

        const out = new Int32Array(bw * 8);
        for (let c = 0; c < 8; c++) {
            const chunk = values.subarray(c * 32, (c + 1) * 32);
            out.set(pack32(chunk, bw), c * bw);
        }
        return out;
    }

    function makeRamp(mask: number): Int32Array {
        const v = new Int32Array(32);
        for (let i = 0; i < 32; i++) v[i] = i & mask;
        return v;
    }

    function makeMax(mask: number): Int32Array {
        return new Int32Array(32).fill(mask);
    }

    function makeDeterministicRandom(mask: number): Int32Array {
        const v = new Int32Array(32);
        let x = 0xC0FFEE01 | 0;
        for (let i = 0; i < 32; i++) {
            x ^= (x << 13);
            x ^= (x >>> 17);
            x ^= (x << 5);
            v[i] = (x & mask) | 0;
        }
        return v;
    }

    const sortedBws = Object.keys(unpackers).map(Number).sort((a, b) => a - b);

    it("exports the expected fastUnpack32_* bit widths", () => {
        expect(sortedBws).toEqual([1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 16]);
    });

    it("exports the expected fastUnpack256_* bit widths", () => {
        const sortedBws256 = Object.keys(unpackers256).map(Number).sort((a, b) => a - b);
        expect(sortedBws256).toEqual([1, 2, 3, 4, 5, 6, 7, 8, 16]);
        expect(typeof (fastPforUnpack as Record<string, unknown>).fastUnpack256_Generic).toBe("function");
    });

    for (const bw of sortedBws) {
        const unpacker = unpackers[bw];
        const mask = bw === 32 ? -1 : (0xFFFFFFFF >>> (32 - bw));

        it(`fastUnpack32_${bw} unpacks ramp`, () => {
            const expected = makeRamp(mask);
            const input = pack32(expected, bw);

            const out = new Int32Array(32);
            unpacker(input, 0, out, 0);

            expect(out).toEqual(expected);
        });

        it(`fastUnpack32_${bw} unpacks max pattern`, () => {
            const expected = makeMax(mask);
            const input = pack32(expected, bw);

            const out = new Int32Array(32);
            unpacker(input, 0, out, 0);

            expect(out).toEqual(expected);
        });

        it(`fastUnpack32_${bw} unpacks deterministic random pattern`, () => {
            const expected = makeDeterministicRandom(mask);
            const input = pack32(expected, bw);

            const out = new Int32Array(32);
            unpacker(input, 0, out, 0);

            expect(out).toEqual(expected);
        });

        it(`fastUnpack32_${bw} supports non-zero inPos/outPos without clobbering`, () => {
            const expected = makeDeterministicRandom(mask);
            const packed = pack32(expected, bw);

            const inPos = 2;
            const outPos = 3;

            const sentinel = 0x13579bdf | 0;
            const inArr = new Int32Array(inPos + packed.length + 2);
            inArr.fill(0x7f7f7f7f | 0);
            inArr.set(packed, inPos);

            const out = new Int32Array(outPos + 32 + 4);
            out.fill(sentinel);

            unpacker(inArr, inPos, out, outPos);

            expect(out.subarray(outPos, outPos + 32)).toEqual(expected);

            const isCleanPrefix = out.subarray(0, outPos).every(x => x === sentinel);
            expect(isCleanPrefix, "Prefix sentinels corrupted").toBe(true);

            const isCleanSuffix = out.subarray(outPos + 32).every(x => x === sentinel);
            expect(isCleanSuffix, "Suffix sentinels corrupted").toBe(true);
        });
    }

    function makeRamp256(mask: number): Int32Array {
        const v = new Int32Array(256);
        for (let i = 0; i < 256; i++) v[i] = i & mask;
        return v;
    }

    function makeMax256(mask: number): Int32Array {
        return new Int32Array(256).fill(mask);
    }

    function makeDeterministicRandom256(mask: number): Int32Array {
        const v = new Int32Array(256);
        let x = 0xC0FFEE01 | 0;
        for (let i = 0; i < 256; i++) {
            x ^= (x << 13);
            x ^= (x >>> 17);
            x ^= (x << 5);
            v[i] = (x & mask) | 0;
        }
        return v;
    }

    for (let bw = 1; bw <= 16; bw++) {
        const mask = bw === 32 ? -1 : (0xFFFFFFFF >>> (32 - bw));
        const generic = fastPforUnpack.fastUnpack256_Generic;
        const specific = unpackers256[bw];
        const unpacker: Unpacker =
            specific ??
            ((inValues, inPos, out, outPos) => {
                generic(inValues, inPos, out, outPos, bw);
            });

        it(`fastUnpack256 bw=${bw} unpacks ramp`, () => {
            const expected = makeRamp256(mask);
            const input = pack256(expected, bw);
            expect(input.length).toBe(bw * 8);

            const out = new Int32Array(256);
            unpacker(input, 0, out, 0);
            expect(out).toEqual(expected);
        });

        it(`fastUnpack256 bw=${bw} unpacks max pattern`, () => {
            const expected = makeMax256(mask);
            const input = pack256(expected, bw);
            expect(input.length).toBe(bw * 8);

            const out = new Int32Array(256);
            unpacker(input, 0, out, 0);
            expect(out).toEqual(expected);
        });

        it(`fastUnpack256 bw=${bw} unpacks deterministic random pattern`, () => {
            const expected = makeDeterministicRandom256(mask);
            const input = pack256(expected, bw);
            expect(input.length).toBe(bw * 8);

            const out = new Int32Array(256);
            unpacker(input, 0, out, 0);
            expect(out).toEqual(expected);
        });

        it(`fastUnpack256 bw=${bw} supports non-zero inPos/outPos without clobbering`, () => {
            const expected = makeDeterministicRandom256(mask);
            const packed = pack256(expected, bw);
            expect(packed.length).toBe(bw * 8);

            const inPos = 3;
            const outPos = 5;

            const sentinel = 0x13579bdf | 0;
            const inArr = new Int32Array(inPos + packed.length + 2);
            inArr.fill(0x7f7f7f7f | 0);
            inArr.set(packed, inPos);

            const out = new Int32Array(outPos + 256 + 4);
            out.fill(sentinel);

            unpacker(inArr, inPos, out, outPos);

            expect(out.subarray(outPos, outPos + 256)).toEqual(expected);

            const isCleanPrefix = out.subarray(0, outPos).every(x => x === sentinel);
            expect(isCleanPrefix, "Prefix sentinels corrupted").toBe(true);

            const isCleanSuffix = out.subarray(outPos + 256).every(x => x === sentinel);
            expect(isCleanSuffix, "Suffix sentinels corrupted").toBe(true);
        });

        if (specific) {
            it(`fastUnpack256_${bw} matches fastUnpack256_Generic`, () => {
                const expected = makeDeterministicRandom256(mask);
                const input = pack256(expected, bw);

                const outSpecific = new Int32Array(256);
                const outGeneric = new Int32Array(256);

                specific(input, 0, outSpecific, 0);
                generic(input, 0, outGeneric, 0, bw);

                expect(outSpecific).toEqual(outGeneric);
            });
        }
    }
});
