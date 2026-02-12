import { describe, it, expect } from "vitest";
import { decodeFsst } from "./fsstDecoder";
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
});
