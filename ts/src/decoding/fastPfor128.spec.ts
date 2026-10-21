import { describe, expect, it } from "vitest";
import { decodeFastPfor128 } from "./fastPfor128";

/** Byte vectors written by the `fastpfor` crate's `FastPFor128` codec, as mlt-core uses it. */
const TAIL_ONLY = [0, 0, 0, 0, 129, 44, 130, 112, 34, 132, 127, 127, 127, 127, 143, 135];
const DOC_EXAMPLE = [
    128, 0, 0, 0, 13, 0, 0, 0, 136, 70, 250, 136, 198, 250, 136, 198, 250, 136, 198, 250, 136, 198, 250, 136, 198, 250,
    136, 198, 250, 136, 198, 250, 136, 198, 250, 136, 70, 250, 136, 198, 250, 136, 198, 250, 136, 198, 250, 136, 198,
    250, 136, 198, 250, 136, 198, 250, 5, 0, 0, 0, 3, 2, 9, 5, 77, 0, 0, 0, 32, 0, 0, 0, 2, 0, 0, 0, 165, 0, 0, 0, 133,
    72, 129, 0,
];
const ZEROS_BLOCK = [128, 0, 0, 0, 1, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
const WIDTH32_BLOCK = [
    128,
    0,
    0,
    0,
    129,
    0,
    0,
    0,
    ...Array.from({ length: 128 }, (_, i) => {
        const v = (0xffffffff - i) >>> 0;
        return [v & 0xff, (v >>> 8) & 0xff, (v >>> 16) & 0xff, v >>> 24];
    }).flat(),
    2,
    0,
    0,
    0,
    32,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
];

function decode(bytes: number[], numValues: number): number[] {
    return Array.from(decodeFastPfor128(new Uint8Array(bytes), 0, bytes.length, numValues));
}

describe("decodeFastPfor128", () => {
    it("decodes an empty stream from an empty payload", () => {
        expect(decode([], 0)).toEqual([]);
    });

    it("decodes a stream of fewer than 128 values from the variable-byte tail alone", () => {
        expect(decode(TAIL_ONLY, 5)).toEqual([1, 300, 70000, 0xffffffff, 7]);
    });

    it("decodes the encodings doc example: one block with two exceptions, then a two-value tail", () => {
        const expected = Array.from({ length: 130 }, (_, i) => i % 8);
        expected[5] = 300;
        expected[77] = 20;
        expected[128] = 5;
        expected[129] = 200;
        expect(decode(DOC_EXAMPLE, 130)).toEqual(expected);
    });

    it("decodes a block of zeros and a block at the full width", () => {
        expect(decode(ZEROS_BLOCK, 128)).toEqual(new Array<number>(128).fill(0));
        expect(decode(WIDTH32_BLOCK, 128)).toEqual(Array.from({ length: 128 }, (_, i) => (0xffffffff - i) >>> 0));
    });

    it("rejects a payload that is not whole words, a payload on a stream of no values, and a short decode", () => {
        expect(() => decode([0, 0, 0], 1)).toThrow(/whole 32-bit words/);
        expect(() => decode([0, 0, 0, 0], 0)).toThrow(/no values/);
        expect(() => decode(DOC_EXAMPLE, 131)).toThrow();
    });

    it("rejects an exception position past the 128-value block", () => {
        const bytes = [...DOC_EXAMPLE];
        bytes[64] = 200; // the second exception's position byte
        expect(() => decode(bytes, 130)).toThrow(/past the 128-value block/);
    });
});
