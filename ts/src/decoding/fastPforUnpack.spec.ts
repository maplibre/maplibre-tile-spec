import { describe, expect, it } from "vitest";
import * as fastPforUnpack from "./fastPforUnpack";
import { fastPack32 } from "../encoding/fastPforEncoder";
import { MASKS } from "./fastPforShared";

type Unpacker = (inValues: Int32Array, inPos: number, out: Int32Array, outPos: number) => void;

describe("FastPFOR Unpack Library", () => {
    const unpack32ByBitWidth: Array<[bitWidth: number, unpacker: Unpacker]> = [
        [1, fastPforUnpack.fastUnpack32_1],
        [2, fastPforUnpack.fastUnpack32_2],
        [3, fastPforUnpack.fastUnpack32_3],
        [4, fastPforUnpack.fastUnpack32_4],
        [5, fastPforUnpack.fastUnpack32_5],
        [6, fastPforUnpack.fastUnpack32_6],
        [7, fastPforUnpack.fastUnpack32_7],
        [8, fastPforUnpack.fastUnpack32_8],
        [9, fastPforUnpack.fastUnpack32_9],
        [10, fastPforUnpack.fastUnpack32_10],
        [11, fastPforUnpack.fastUnpack32_11],
        [12, fastPforUnpack.fastUnpack32_12],
        [16, fastPforUnpack.fastUnpack32_16],
    ];

    const unpack256ByBitWidth: Array<[bitWidth: number, unpacker: Unpacker]> = [
        [1, fastPforUnpack.fastUnpack256_1],
        [2, fastPforUnpack.fastUnpack256_2],
        [3, fastPforUnpack.fastUnpack256_3],
        [4, fastPforUnpack.fastUnpack256_4],
        [5, fastPforUnpack.fastUnpack256_5],
        [6, fastPforUnpack.fastUnpack256_6],
        [7, fastPforUnpack.fastUnpack256_7],
        [8, fastPforUnpack.fastUnpack256_8],
        [16, fastPforUnpack.fastUnpack256_16],
    ];

    function pack32(values: Int32Array, bitWidth: number): Int32Array {
        const out = new Int32Array(bitWidth);
        fastPack32(values, 0, out, 0, bitWidth);
        return out;
    }

    function pack256(values: Int32Array, bitWidth: number): Int32Array {
        const out = new Int32Array(bitWidth * 8);
        for (let chunk = 0; chunk < 8; chunk++) {
            fastPack32(values, chunk * 32, out, chunk * bitWidth, bitWidth);
        }
        return out;
    }

    function makeRamp(length: number, mask: number): Int32Array {
        const values = new Int32Array(length);
        for (let i = 0; i < length; i++) values[i] = i & mask;
        return values;
    }

    function makeMax(length: number, mask: number): Int32Array {
        return new Int32Array(length).fill(mask);
    }

    for (const [bitWidth, unpacker] of unpack32ByBitWidth) {
        it(`fastUnpack32_${bitWidth} unpacks ramp and max patterns`, () => {
            const mask = MASKS[bitWidth];

            const expectedRamp = makeRamp(32, mask);
            const outRamp = new Int32Array(32);
            unpacker(pack32(expectedRamp, bitWidth), 0, outRamp, 0);
            expect(outRamp).toEqual(expectedRamp);

            const expectedMax = makeMax(32, mask);
            const outMax = new Int32Array(32);
            unpacker(pack32(expectedMax, bitWidth), 0, outMax, 0);
            expect(outMax).toEqual(expectedMax);
        });
    }

    for (const [bitWidth, specific] of unpack256ByBitWidth) {
        it(`fastUnpack256 bitWidth=${bitWidth} unpacks ramp and max patterns (specific + generic)`, () => {
            const mask = MASKS[bitWidth];
            const generic = fastPforUnpack.fastUnpack256_Generic;

            const expectedRamp = makeRamp(256, mask);
            const outRampSpecific = new Int32Array(256);
            const outRampGeneric = new Int32Array(256);
            const inputRamp = pack256(expectedRamp, bitWidth);
            specific(inputRamp, 0, outRampSpecific, 0);
            generic(inputRamp, 0, outRampGeneric, 0, bitWidth);
            expect(outRampSpecific).toEqual(expectedRamp);
            expect(outRampGeneric).toEqual(expectedRamp);

            const expectedMax = makeMax(256, mask);
            const outMaxSpecific = new Int32Array(256);
            const outMaxGeneric = new Int32Array(256);
            const inputMax = pack256(expectedMax, bitWidth);
            specific(inputMax, 0, outMaxSpecific, 0);
            generic(inputMax, 0, outMaxGeneric, 0, bitWidth);
            expect(outMaxSpecific).toEqual(expectedMax);
            expect(outMaxGeneric).toEqual(expectedMax);
        });
    }

    it("fastUnpack256_Generic supports bit widths without a specialized unpacker (e.g. 12)", () => {
        const bitWidth = 12;
        const mask = MASKS[bitWidth];
        const generic = fastPforUnpack.fastUnpack256_Generic;

        const expectedRamp = makeRamp(256, mask);
        const outRamp = new Int32Array(256);
        generic(pack256(expectedRamp, bitWidth), 0, outRamp, 0, bitWidth);
        expect(outRamp).toEqual(expectedRamp);

        const expectedMax = makeMax(256, mask);
        const outMax = new Int32Array(256);
        generic(pack256(expectedMax, bitWidth), 0, outMax, 0, bitWidth);
        expect(outMax).toEqual(expectedMax);
    });
});
