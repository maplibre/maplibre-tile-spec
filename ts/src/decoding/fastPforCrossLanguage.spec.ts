import { describe, expect, it } from "vitest";
import { readdirSync, readFileSync } from "node:fs";

import IntWrapper from "./intWrapper";
import { decodeBigEndianInt32sInto } from "./bigEndianDecode";
import {
    createFastPforWireDecodeWorkspace,
    decodeFastPfor,
    decodeFastPforWithWorkspace,
} from "./integerDecodingUtils";

describe("decodeFastPfor (wire format fixtures)", () => {
    const FIXTURES_DIR_URL = new URL("../../../test/fixtures/fastpfor/", import.meta.url);

    function fixtureUrl(fileName: string): URL {
        return new URL(fileName, FIXTURES_DIR_URL);
    }

    function loadFixtureNames(): string[] {
        const encoded = new Set<string>();
        const decoded = new Set<string>();

        for (const entry of readdirSync(FIXTURES_DIR_URL, { withFileTypes: true })) {
            if (!entry.isFile()) continue;
            if (entry.name.endsWith("_encoded.bin")) {
                encoded.add(entry.name.slice(0, -"_encoded.bin".length));
            } else if (entry.name.endsWith("_decoded.bin")) {
                decoded.add(entry.name.slice(0, -"_decoded.bin".length));
            }
        }

        const missingDecoded = Array.from(encoded).filter((name) => !decoded.has(name));
        const missingEncoded = Array.from(decoded).filter((name) => !encoded.has(name));
        if (missingDecoded.length > 0 || missingEncoded.length > 0) {
            throw new Error(
                `Invalid fixture set: missing decoded=[${missingDecoded.join(", ")}], missing encoded=[${missingEncoded.join(", ")}]`,
            );
        }

        const names = Array.from(encoded).sort();
        if (names.length === 0) {
            throw new Error(`No FastPFOR fixtures found in ${FIXTURES_DIR_URL}`);
        }
        return names;
    }

    function readEncodedFixtureBytes(name: string): Uint8Array {
        const buf = readFileSync(fixtureUrl(`${name}_encoded.bin`));
        return new Uint8Array(buf.buffer, buf.byteOffset, buf.byteLength);
    }

    function readExpectedFixtureValues(name: string): Int32Array {
        const buf = readFileSync(fixtureUrl(`${name}_decoded.bin`));
        const bytes = new Uint8Array(buf.buffer, buf.byteOffset, buf.byteLength);
        const out = new Int32Array(bytes.byteLength >>> 2);
        decodeBigEndianInt32sInto(bytes, 0, bytes.byteLength, out);
        return out;
    }

    const fixtureNames = loadFixtureNames();
    for (const name of fixtureNames) {
        describe(name, () => {
            it("decodes (no workspace)", () => {
                const encoded = readEncodedFixtureBytes(name);
                const expectedValues = readExpectedFixtureValues(name);

                const offset = new IntWrapper(0);
                const decoded = decodeFastPfor(encoded, expectedValues.length, encoded.length, offset);
                expect(decoded).toEqual(expectedValues);
                expect(offset.get()).toBe(encoded.length);
            });

            it("decodes (with workspace reuse)", () => {
                const encoded = readEncodedFixtureBytes(name);
                const expectedValues = readExpectedFixtureValues(name);
                const workspace = createFastPforWireDecodeWorkspace();

                const offset1 = new IntWrapper(0);
                const decoded1 = decodeFastPforWithWorkspace(
                    encoded,
                    expectedValues.length,
                    encoded.length,
                    offset1,
                    workspace,
                );
                expect(decoded1).toEqual(expectedValues);
                expect(offset1.get()).toBe(encoded.length);

                const offset2 = new IntWrapper(0);
                const decoded2 = decodeFastPforWithWorkspace(
                    encoded,
                    expectedValues.length,
                    encoded.length,
                    offset2,
                    workspace,
                );
                expect(decoded2).toEqual(expectedValues);
                expect(offset2.get()).toBe(encoded.length);
            });

            it("does not depend on input ArrayBuffer alignment (prefix bytes)", () => {
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
    }
});
