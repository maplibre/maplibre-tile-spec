import { describe, it, expect } from "vitest";
import { decodeFsst, FsstDecoder } from "./fsstDecoder";
import { encodeFsst, createSymbolTable } from "../encoding/fsstEncoder";

const textEncoder = new TextEncoder();

describe("decodeFsst", () => {
    describe("basic functionality", () => {
        it("should decode FSST compressed string data", () => {
            const inputString = "HelloWorld !";
            const originalBytes = textEncoder.encode(inputString);

            const { symbols, symbolLengths } = createSymbolTable(["Hello", "World", "!"]);
            const encoded = encodeFsst(symbols, symbolLengths, originalBytes);
            const decoded = decodeFsst(symbols, symbolLengths, encoded);

            expect(decoded).toEqual(originalBytes);
            expect(new TextDecoder().decode(decoded)).toBe(inputString);
        });

        it("should handle empty string", () => {
            const inputString = "";
            const originalBytes = textEncoder.encode(inputString);

            const { symbols, symbolLengths } = createSymbolTable(["A"]);
            const encoded = encodeFsst(symbols, symbolLengths, originalBytes);
            const decoded = decodeFsst(symbols, symbolLengths, encoded);

            expect(decoded).toEqual(originalBytes);
            expect(decoded.length).toBe(0);
        });

        it("should handle string with no matching symbols", () => {
            const inputString = "12345";
            const originalBytes = textEncoder.encode(inputString);

            const { symbols, symbolLengths } = createSymbolTable(["abc", "def", "xyz"]);
            const encoded = encodeFsst(symbols, symbolLengths, originalBytes);
            const decoded = decodeFsst(symbols, symbolLengths, encoded);

            expect(decoded).toEqual(originalBytes);
            expect(new TextDecoder().decode(decoded)).toBe(inputString);
        });

        it("should handle consecutive escaped bytes", () => {
            const symbols = new Uint8Array([65]);
            const symbolLengths = new Uint32Array([1]);
            const encoded = new Uint8Array([255, 1, 255, 2, 0, 255, 3]);

            expect(decodeFsst(symbols, symbolLengths, encoded)).toEqual(new Uint8Array([1, 2, 65, 3]));
        });

        it("should handle string with all matching symbols", () => {
            const inputString = "AAAA";
            const originalBytes = textEncoder.encode(inputString);

            const { symbols, symbolLengths } = createSymbolTable(["A"]);
            const encoded = encodeFsst(symbols, symbolLengths, originalBytes);
            const decoded = decodeFsst(symbols, symbolLengths, encoded);

            expect(decoded).toEqual(originalBytes);
            expect(new TextDecoder().decode(decoded)).toBe(inputString);
        });
    });

    describe("compression verification", () => {
        it("should handle repeated strings efficiently", () => {
            const inputString = "HelloWorldHelloWorld";
            const originalBytes = textEncoder.encode(inputString);

            const { symbols, symbolLengths } = createSymbolTable(["Hello", "World"]);
            const encoded = encodeFsst(symbols, symbolLengths, originalBytes);
            const decoded = decodeFsst(symbols, symbolLengths, encoded);

            expect(decoded).toEqual(originalBytes);
            expect(new TextDecoder().decode(decoded)).toBe(inputString);
            expect(encoded.length).toBeLessThan(originalBytes.length);
        });
    });

    it("decodes individual byte ranges", () => {
        const originalBytes = textEncoder.encode("HelloWorld escaped: \u0001");
        const { symbols, symbolLengths } = createSymbolTable(["Hello", "World", " escaped: "]);
        const encoded = encodeFsst(symbols, symbolLengths, originalBytes);
        const decoder = new FsstDecoder(symbols, symbolLengths, encoded);

        for (let start = 0; start < originalBytes.length; start++) {
            for (let end = start; end <= originalBytes.length; end++) {
                expect(decoder.decodeRange(start, end)).toEqual(originalBytes.subarray(start, end));
            }
        }
        expect(decoder.decode()).toEqual(originalBytes);
    });

    it("decodes ranges across lazily built checkpoints", () => {
        const originalBytes = textEncoder.encode("abcdefgh escaped:\u0001 multilingual:Zürich;".repeat(5_000));
        const { symbols, symbolLengths } = createSymbolTable(["abcdefgh", " escaped:", " multilingual:", "Zürich;"]);
        const encoded = encodeFsst(symbols, symbolLengths, originalBytes);
        const expected = decodeFsst(symbols, symbolLengths, encoded);
        const decoder = new FsstDecoder(symbols, symbolLengths, encoded);
        const ranges = [
            [16_380, 16_405],
            [65_530, 65_550],
            [131_068, 131_100],
            [2, 19],
            [originalBytes.length - 31, originalBytes.length],
        ];

        let random = 0x12345678;
        for (let i = 0; i < 100; i++) {
            random = (Math.imul(random, 1_664_525) + 1_013_904_223) >>> 0;
            const start = random % (originalBytes.length - 64);
            ranges.push([start, start + 64]);
        }

        for (const [start, end] of ranges) {
            expect(decoder.decodeRange(start, end)).toEqual(expected.subarray(start, end));
        }
        expect(decoder.decodeRange(0, 1).buffer.byteLength).toBe(expected.length);
    });

    it("clamps sparse and full ranges to the decoded length", () => {
        const originalBytes = textEncoder.encode("HelloWorld");
        const { symbols, symbolLengths } = createSymbolTable(["Hello", "World"]);
        const encoded = encodeFsst(symbols, symbolLengths, originalBytes);
        const sparseDecoder = new FsstDecoder(symbols, symbolLengths, encoded);
        const fullDecoder = new FsstDecoder(symbols, symbolLengths, encoded);
        fullDecoder.decode();

        expect(sparseDecoder.decodeRange(8, 20)).toEqual(originalBytes.subarray(8, 20));
        expect(fullDecoder.decodeRange(8, 20)).toEqual(originalBytes.subarray(8, 20));
        expect(sparseDecoder.decodeRange(20, 30)).toEqual(new Uint8Array());
        expect(fullDecoder.decodeRange(20, 30)).toEqual(new Uint8Array());
    });
});
