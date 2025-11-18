import { describe, it, expect } from "vitest";
import { decodeFastPfor } from "./fastPforDecoder";
import IntWrapper from "./intWrapper";
import * as fs from "fs";
import * as path from "path";

describe("FastPFOR Decoder - Java Generated Test Vectors", () => {
    const tvDir = path.join(__dirname, "../../test/data/fastpfor");

    interface TestVector {
        name: string;
        numValues: number;
        byteLength: number;
        inputValues: (number | string)[];
        encodedBytes: number[];
        encodedHex: string;
    }

    function loadTestVector(name: string): TestVector {
        const jsonPath = path.join(tvDir, `${name}.json`);
        const content = fs.readFileSync(jsonPath, "utf-8");
        return JSON.parse(content);
    }

    describe("Core Functionality", () => {
        it("should decode non_aligned_358 (256 FastPFOR + 102 VariableByte)", () => {
            const tv = loadTestVector("non_aligned_358");
            const encoded = new Uint8Array(tv.encodedBytes);

            const decoded = decodeFastPfor(encoded, tv.numValues, tv.byteLength, new IntWrapper(0));

            expect(decoded.length).toBe(358);

            // Verify sequential values
            expect(decoded[0]).toBe(0);
            expect(decoded[255]).toBe(255);
            expect(decoded[256]).toBe(256);
            expect(decoded[357]).toBe(357);
        });
    });

    describe("Exception Handling", () => {
        it("should decode large_exceptions", () => {
            const tv = loadTestVector("large_exceptions");
            const encoded = new Uint8Array(tv.encodedBytes);

            const decoded = decodeFastPfor(encoded, tv.numValues, tv.byteLength, new IntWrapper(0));

            expect(decoded.length).toBe(500);

            // Most values are 7
            expect(decoded[0]).toBe(7);
            expect(decoded[1]).toBe(7);

            // Exceptions
            expect(decoded[10]).toBe(100034530);
            expect(decoded[50]).toBe(20000);
            expect(decoded[100]).toBe(30000);
            expect(decoded[200]).toBe(50000000);
        });

        it("should decode small_values (3-bit wide)", () => {
            const tv = loadTestVector("small_values");
            const encoded = new Uint8Array(tv.encodedBytes);

            const decoded = decodeFastPfor(encoded, tv.numValues, tv.byteLength, new IntWrapper(0));

            expect(decoded.length).toBe(256);

            // Values cycle 0-7
            for (let i = 0; i < 256; i++) {
                expect(decoded[i]).toBe(i % 8);
            }
        });
    });

    describe("Special Cases", () => {
        it("should decode zeros (all zeros)", () => {
            const tv = loadTestVector("zeros");
            const encoded = new Uint8Array(tv.encodedBytes);

            const decoded = decodeFastPfor(encoded, tv.numValues, tv.byteLength, new IntWrapper(0));

            expect(decoded.length).toBe(256);
            expect(Array.from(decoded).every((v) => v === 0)).toBe(true);
        });

        it("should decode sequential (512 values)", () => {
            const tv = loadTestVector("sequential");
            const encoded = new Uint8Array(tv.encodedBytes);

            const decoded = decodeFastPfor(encoded, tv.numValues, tv.byteLength, new IntWrapper(0));

            expect(decoded.length).toBe(512);

            // Sequential values 0-511
            for (let i = 0; i < 512; i++) {
                expect(decoded[i]).toBe(i);
            }
        });
    });

    describe("Offset Handling", () => {
        it("should handle non-zero initial offset", () => {
            const tv = loadTestVector("small_values");
            const encoded = new Uint8Array(tv.encodedBytes);

            // Add padding before the actual data
            const padding = new Uint8Array(32);
            const combined = new Uint8Array(padding.length + encoded.length);
            combined.set(padding, 0);
            combined.set(encoded, padding.length);

            const offset = new IntWrapper(32);
            const decoded = decodeFastPfor(combined, tv.numValues, tv.byteLength, offset);

            expect(decoded.length).toBe(tv.numValues);
            expect(offset.get()).toBe(32 + Math.ceil(tv.byteLength / 4) * 4);

            // Verify first few values
            for (let i = 0; i < 10; i++) {
                expect(decoded[i]).toBe(i % 8);
            }
        });

        it("should advance offset correctly for sequential decoding", () => {
            const tv1 = loadTestVector("zeros");
            const tv2 = loadTestVector("small_values");

            const encoded1 = new Uint8Array(tv1.encodedBytes);
            const encoded2 = new Uint8Array(tv2.encodedBytes);

            // Combine two encoded streams
            const combined = new Uint8Array(Math.ceil(tv1.byteLength / 4) * 4 + Math.ceil(tv2.byteLength / 4) * 4);
            combined.set(encoded1, 0);
            combined.set(encoded2, Math.ceil(tv1.byteLength / 4) * 4);

            const offset = new IntWrapper(0);

            // Decode first stream
            const decoded1 = decodeFastPfor(combined, tv1.numValues, tv1.byteLength, offset);
            expect(decoded1.length).toBe(tv1.numValues);
            expect(Array.from(decoded1).every((v) => v === 0)).toBe(true);

            // Decode second stream
            const decoded2 = decodeFastPfor(combined, tv2.numValues, tv2.byteLength, offset);
            expect(decoded2.length).toBe(tv2.numValues);
            for (let i = 0; i < 10; i++) {
                expect(decoded2[i]).toBe(i % 8);
            }
        });
    });
});
