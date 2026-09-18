import { describe, expect, it } from "vitest";
import { encodeBooleanRle, encodeByteRle } from "./encodingUtils";
import { decodeBooleanRle, decodeByteRle } from "../decoding/decodingUtils";
import BitVector from "../vector/flat/bitVector";
import IntWrapper from "../decoding/intWrapper";

/**
 * A byte RLE header either opens a run of `header + 3` copies of the byte that follows it, or
 * introduces `256 - header` literal bytes, so one header carries at most 130 repeats or 128
 * literals; `docs/encodings.md` defers to the ORC specification for both. `encodeBooleanRle` packs
 * eight booleans into a byte first, so the same two limits fall at 1040 and 1024 booleans.
 */
describe("encodingUtils", () => {
    describe("encodeByteRle", () => {
        it.each([
            {
                name: "130 repeats as one run header",
                values: new Uint8Array(130).fill(7),
                expected: new Uint8Array([0x7f, 7]),
            },
            {
                name: "131 repeats as a full run header and a literal",
                values: new Uint8Array(131).fill(7),
                expected: new Uint8Array([0x7f, 7, 0xff, 7]),
            },
            {
                name: "128 literals as one literal header",
                values: Uint8Array.from({ length: 128 }, (_, index) => index),
                expected: new Uint8Array([0x80, ...Array.from({ length: 128 }, (_, index) => index)]),
            },
            {
                name: "129 literals as a full literal header and another",
                values: Uint8Array.from({ length: 129 }, (_, index) => index),
                expected: new Uint8Array([0x80, ...Array.from({ length: 128 }, (_, index) => index), 0xff, 128]),
            },
        ])("should encode $name", ({ values, expected }) => {
            expect(encodeByteRle(values)).toEqual(expected);
        });

        it.each([
            { name: "129 repeats, one short of a full run header", values: new Uint8Array(129).fill(7) },
            { name: "130 repeats, exactly a full run header", values: new Uint8Array(130).fill(7) },
            { name: "131 repeats, one past a full run header", values: new Uint8Array(131).fill(7) },
            {
                name: "127 literals, one short of a full literal header",
                values: Uint8Array.from({ length: 127 }, (_, index) => index),
            },
            {
                name: "128 literals, exactly a full literal header",
                values: Uint8Array.from({ length: 128 }, (_, index) => index),
            },
            {
                name: "129 literals, one past a full literal header",
                values: Uint8Array.from({ length: 129 }, (_, index) => index),
            },
        ])("should round trip $name", ({ values }) => {
            const encoded = encodeByteRle(values);
            const offset = new IntWrapper(0);

            expect(decodeByteRle(encoded, values.length, encoded.length, offset)).toEqual(values);
            expect(offset.get()).toBe(encoded.length);
        });
    });

    describe("encodeBooleanRle", () => {
        it.each([
            {
                name: "1040 set booleans as one run header over 130 packed bytes",
                values: Array.from({ length: 1040 }, () => true),
                expected: new Uint8Array([0x7f, 0xff]),
            },
            {
                name: "1048 set booleans as a full run header and a literal",
                values: Array.from({ length: 1048 }, () => true),
                expected: new Uint8Array([0x7f, 0xff, 0xff, 0xff]),
            },
        ])("should encode $name", ({ values, expected }) => {
            expect(encodeBooleanRle(values)).toEqual(expected);
        });

        it.each([
            {
                name: "1032 set booleans, one packed byte short of a full run header",
                values: Array.from({ length: 1032 }, () => true),
            },
            {
                name: "1040 set booleans, exactly a full run header",
                values: Array.from({ length: 1040 }, () => true),
            },
            {
                name: "1048 set booleans, one packed byte past a full run header",
                values: Array.from({ length: 1048 }, () => true),
            },
            {
                name: "1016 mixed booleans, one packed byte short of a full literal header",
                values: Array.from({ length: 1016 }, (_, index) => index % 3 === 0),
            },
            {
                name: "1024 mixed booleans, exactly a full literal header",
                values: Array.from({ length: 1024 }, (_, index) => index % 3 === 0),
            },
            {
                name: "1032 mixed booleans, one packed byte past a full literal header",
                values: Array.from({ length: 1032 }, (_, index) => index % 3 === 0),
            },
        ])("should round trip $name", ({ values }) => {
            const encoded = encodeBooleanRle(values);
            const offset = new IntWrapper(0);
            const decoded = new BitVector(
                decodeBooleanRle(encoded, values.length, encoded.length, offset),
                values.length,
            );

            expect(values.map((_, index) => decoded.get(index))).toEqual(values);
        });
    });
});
