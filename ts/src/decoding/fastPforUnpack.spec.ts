import { describe, expect, it } from "vitest";
import { fastPack32 } from "../encoding/fastPforEncoder";
import { MASKS } from "./fastPforShared";
import {
    fastUnpack32_1,
    fastUnpack32_2,
    fastUnpack32_3,
    fastUnpack32_4,
    fastUnpack32_5,
    fastUnpack32_6,
    fastUnpack32_7,
    fastUnpack32_8,
    fastUnpack32_9,
    fastUnpack32_10,
    fastUnpack32_11,
    fastUnpack32_12,
    fastUnpack32_16,
    fastUnpack256_1,
    fastUnpack256_2,
    fastUnpack256_3,
    fastUnpack256_4,
    fastUnpack256_5,
    fastUnpack256_6,
    fastUnpack256_7,
    fastUnpack256_8,
    fastUnpack256_16,
    fastUnpack256_Generic,
} from "./fastPforUnpack";

describe("FastPFOR Unpack Library", () => {
    type Unpacker = (inValues: Int32Array, inPos: number, out: Int32Array, outPos: number) => void;

    const UNPACK32_TEST_CASES: Array<{ bitWidth: number; unpacker: Unpacker }> = [
        { bitWidth: 1, unpacker: fastUnpack32_1 },
        { bitWidth: 2, unpacker: fastUnpack32_2 },
        { bitWidth: 3, unpacker: fastUnpack32_3 },
        { bitWidth: 4, unpacker: fastUnpack32_4 },
        { bitWidth: 5, unpacker: fastUnpack32_5 },
        { bitWidth: 6, unpacker: fastUnpack32_6 },
        { bitWidth: 7, unpacker: fastUnpack32_7 },
        { bitWidth: 8, unpacker: fastUnpack32_8 },
        { bitWidth: 9, unpacker: fastUnpack32_9 },
        { bitWidth: 10, unpacker: fastUnpack32_10 },
        { bitWidth: 11, unpacker: fastUnpack32_11 },
        { bitWidth: 12, unpacker: fastUnpack32_12 },
        { bitWidth: 16, unpacker: fastUnpack32_16 },
    ];

    const UNPACK256_SPECIALIZED_TEST_CASES: Array<{ bitWidth: number; unpacker: Unpacker }> = [
        { bitWidth: 1, unpacker: fastUnpack256_1 },
        { bitWidth: 2, unpacker: fastUnpack256_2 },
        { bitWidth: 3, unpacker: fastUnpack256_3 },
        { bitWidth: 4, unpacker: fastUnpack256_4 },
        { bitWidth: 5, unpacker: fastUnpack256_5 },
        { bitWidth: 6, unpacker: fastUnpack256_6 },
        { bitWidth: 7, unpacker: fastUnpack256_7 },
        { bitWidth: 8, unpacker: fastUnpack256_8 },
        { bitWidth: 16, unpacker: fastUnpack256_16 },
    ];

    const UNPACK256_GENERIC_BIT_WIDTHS: number[] = [9, 10, 11, 12, 13, 14, 15];

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

    function makeRamp(length: number, valueMask: number): Int32Array {
        const values = new Int32Array(length);
        for (let i = 0; i < length; i++) values[i] = i & valueMask;
        return values;
    }

    function makeMaxPattern(length: number, valueMask: number): Int32Array {
        return new Int32Array(length).fill(valueMask);
    }

    function assertUnpack32RoundTrip(bitWidth: number, unpacker: Unpacker, expected: Int32Array): void {
        const out = new Int32Array(32);
        unpacker(pack32(expected, bitWidth), 0, out, 0);
        expect(out).toEqual(expected);
    }

    function assertUnpack256RoundTrip(bitWidth: number, unpacker: Unpacker, expected: Int32Array): void {
        const out = new Int32Array(256);
        unpacker(pack256(expected, bitWidth), 0, out, 0);
        expect(out).toEqual(expected);
    }

    function assertFastUnpack256MatchesGeneric(
        bitWidth: number,
        specializedUnpacker: Unpacker,
        expected: Int32Array,
    ): void {
        const input = pack256(expected, bitWidth);
        const outSpecific = new Int32Array(256);
        const outGeneric = new Int32Array(256);

        specializedUnpacker(input, 0, outSpecific, 0);
        fastUnpack256_Generic(input, 0, outGeneric, 0, bitWidth);

        expect(outSpecific).toEqual(outGeneric);
    }

    for (const { bitWidth, unpacker } of UNPACK32_TEST_CASES) {
        describe(`fastUnpack32_${bitWidth}`, () => {
            it("round-trips ramp", () => {
                const valueMask = MASKS[bitWidth] | 0;
                assertUnpack32RoundTrip(bitWidth, unpacker, makeRamp(32, valueMask));
            });

            it("round-trips max", () => {
                const valueMask = MASKS[bitWidth] | 0;
                assertUnpack32RoundTrip(bitWidth, unpacker, makeMaxPattern(32, valueMask));
            });
        });
    }

    for (const { bitWidth, unpacker } of UNPACK256_SPECIALIZED_TEST_CASES) {
        describe(`fastUnpack256_${bitWidth}`, () => {
            it("round-trips ramp", () => {
                const valueMask = MASKS[bitWidth] | 0;
                assertUnpack256RoundTrip(bitWidth, unpacker, makeRamp(256, valueMask));
            });

            it("round-trips max", () => {
                const valueMask = MASKS[bitWidth] | 0;
                assertUnpack256RoundTrip(bitWidth, unpacker, makeMaxPattern(256, valueMask));
            });

            it("matches fastUnpack256_Generic", () => {
                const valueMask = MASKS[bitWidth] | 0;
                assertFastUnpack256MatchesGeneric(bitWidth, unpacker, makeRamp(256, valueMask));
            });
        });
    }

    for (const bitWidth of UNPACK256_GENERIC_BIT_WIDTHS) {
        const unpacker: Unpacker = (inValues, inPos, out, outPos) =>
            fastUnpack256_Generic(inValues, inPos, out, outPos, bitWidth);

        describe(`fastUnpack256_Generic bitWidth=${bitWidth}`, () => {
            it("round-trips ramp", () => {
                const valueMask = MASKS[bitWidth] | 0;
                assertUnpack256RoundTrip(bitWidth, unpacker, makeRamp(256, valueMask));
            });

            it("round-trips max", () => {
                const valueMask = MASKS[bitWidth] | 0;
                assertUnpack256RoundTrip(bitWidth, unpacker, makeMaxPattern(256, valueMask));
            });
        });
    }
});
