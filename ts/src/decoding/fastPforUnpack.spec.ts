import { describe, expect, it } from "vitest";
import {
    fastUnpack32_1, fastUnpack32_2, fastUnpack32_3, fastUnpack32_4,
    fastUnpack32_5, fastUnpack32_6, fastUnpack32_7, fastUnpack32_8,
    fastUnpack32_9, fastUnpack32_10, fastUnpack32_11, fastUnpack32_12,
    fastUnpack32_16,
} from "./fastPforUnpack";

type Unpacker = (inValues: Int32Array, inPos: number, out: Int32Array, outPos: number) => void;

describe("FastPFOR Unpack Library", () => {
    const unpackers: Record<number, Unpacker> = {
        1: fastUnpack32_1, 2: fastUnpack32_2, 3: fastUnpack32_3, 4: fastUnpack32_4,
        5: fastUnpack32_5, 6: fastUnpack32_6, 7: fastUnpack32_7, 8: fastUnpack32_8,
        9: fastUnpack32_9, 10: fastUnpack32_10, 11: fastUnpack32_11, 12: fastUnpack32_12,
        16: fastUnpack32_16,
    };

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

    for (const bw of sortedBws) {
        const unpacker = unpackers[bw];
        const mask = bw === 32 ? -1 : (0xFFFFFFFF >>> (32 - bw));

        it(`fastUnpack32_${bw} unpacks ramp`, () => {
            const expected = makeRamp(mask);
            const input = pack32(expected, bw);
            expect(input.length).toBe(bw);

            const out = new Int32Array(32);
            unpacker(input, 0, out, 0);

            expect(out).toEqual(expected);
        });

        it(`fastUnpack32_${bw} unpacks max pattern`, () => {
            const expected = makeMax(mask);
            const input = pack32(expected, bw);
            expect(input.length).toBe(bw);

            const out = new Int32Array(32);
            unpacker(input, 0, out, 0);

            expect(out).toEqual(expected);
        });

        it(`fastUnpack32_${bw} unpacks deterministic random pattern`, () => {
            const expected = makeDeterministicRandom(mask);
            const input = pack32(expected, bw);
            expect(input.length).toBe(bw);

            const out = new Int32Array(32);
            unpacker(input, 0, out, 0);

            expect(out).toEqual(expected);
        });

        it(`fastUnpack32_${bw} supports non-zero inPos/outPos without clobbering`, () => {
            const expected = makeDeterministicRandom(mask);
            const packed = pack32(expected, bw);
            expect(packed.length).toBe(bw);

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
});
