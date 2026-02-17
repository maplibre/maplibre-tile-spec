import { describe, expect, it } from "vitest";
import { readFileSync } from "node:fs";

import IntWrapper from "./intWrapper";
import { decodeBigEndianInt32sInto } from "./bigEndianDecode";
import {
    createFastPforWireDecodeWorkspace,
    decodeFastPfor,
    decodeFastPforWithWorkspace,
} from "./integerDecodingUtils";

describe("decodeFastPfor (wire format fixtures)", () => {
    const FIXTURE_NAMES = ["vector1", "vector2", "vector3", "vector4"] as const;

    function fixtureUrl(fileName: string): URL {
        return new URL(`../../../test/fixtures/fastpfor/${fileName}`, import.meta.url);
    }

    function readEncodedFixtureBytes(name: string): Uint8Array {
        const buf = readFileSync(fixtureUrl(`${name}_encoded.bin`));
        return new Uint8Array(buf.buffer, buf.byteOffset, buf.byteLength);
    }

    function readExpectedFixtureValues(name: string): Int32Array {
        const buf = readFileSync(fixtureUrl(`${name}_decoded.bin`));
        if ((buf.byteLength & 3) !== 0) {
            throw new Error(`Invalid decoded fixture byte length: ${buf.byteLength} (expected multiple of 4)`);
        }

        const bytes = new Uint8Array(buf.buffer, buf.byteOffset, buf.byteLength);
        const out = new Int32Array(bytes.byteLength >>> 2);
        decodeBigEndianInt32sInto(bytes, 0, bytes.byteLength, out);
        return out;
    }

    it.each(FIXTURE_NAMES)("%s decodes (no workspace)", (name) => {
        const encoded = readEncodedFixtureBytes(name);
        const expectedValues = readExpectedFixtureValues(name);

        const offset = new IntWrapper(0);
        const decoded = decodeFastPfor(encoded, expectedValues.length, encoded.length, offset);
        expect(decoded).toEqual(expectedValues);
        expect(offset.get()).toBe(encoded.length);
    });

    it.each(FIXTURE_NAMES)("%s decodes (with workspace reuse)", (name) => {
        const encoded = readEncodedFixtureBytes(name);
        const expectedValues = readExpectedFixtureValues(name);
        const workspace = createFastPforWireDecodeWorkspace();

        const offset1 = new IntWrapper(0);
        const decoded1 = decodeFastPforWithWorkspace(encoded, expectedValues.length, encoded.length, offset1, workspace);
        expect(decoded1).toEqual(expectedValues);
        expect(offset1.get()).toBe(encoded.length);

        const offset2 = new IntWrapper(0);
        const decoded2 = decodeFastPforWithWorkspace(encoded, expectedValues.length, encoded.length, offset2, workspace);
        expect(decoded2).toEqual(expectedValues);
        expect(offset2.get()).toBe(encoded.length);
    });

    it("does not depend on input ArrayBuffer alignment (prefix bytes)", () => {
        const name = FIXTURE_NAMES[0];
        const encoded = readEncodedFixtureBytes(name);
        const expectedValues = readExpectedFixtureValues(name);

        const prefix = new Uint8Array([0xaa, 0xbb, 0xcc]);
        const suffix = new Uint8Array([0xff, 0xff, 0xff, 0xff, 0xff, 0xff]);

        const buffer = new Uint8Array(prefix.length + encoded.length + suffix.length);
        buffer.set(prefix, 0);
        buffer.set(encoded, prefix.length);
        buffer.set(suffix, prefix.length + encoded.length);

        const offset = new IntWrapper(prefix.length);
        const decoded = decodeFastPfor(buffer, expectedValues.length, encoded.length, offset);
        expect(decoded).toEqual(expectedValues);
        expect(offset.get()).toBe(prefix.length + encoded.length);
        expect(buffer.subarray(prefix.length + encoded.length)).toEqual(suffix);
    });
});
