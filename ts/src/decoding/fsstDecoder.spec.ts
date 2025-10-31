import {describe, it, expect, beforeEach} from "vitest";
import { decodeFsst } from "./fsstDecoder";


describe("Test fsstDecoder", () => {
    it('should decode FSST compressed data correctly', () => {
        // Setup: Create symbol table
        // Symbol 0: "Hello" (5 bytes)
        // Symbol 1: "World" (5 bytes)
        // Symbol 2: "!" (1 byte)
        const symbols = new Uint8Array([
            72, 101, 108, 108, 111,
            87, 111, 114, 108, 100,
            33
        ]);

        const symbolLengths = new Uint32Array([5, 5, 1]);

        // Compressed data:
        // 0 = Symbol 0 ("Hello")
        // 1 = Symbol 1 ("World")
        // 255, 32 = Literal byte 32 (space character)
        // 2 = Symbol 2 ("!")
        const compressedData = new Uint8Array([0, 1, 255, 32, 2]);

        // Act
        const result = decodeFsst(symbols, symbolLengths, compressedData);

        // Assert
        const expected = new Uint8Array([
            72, 101, 108, 108, 111, // "Hello"
            87, 111, 114, 108, 100, // "World"
            32,                      // " " (space)
            33                       // "!"
        ]);

        expect(result).toEqual(expected);

        // Verify decoded string
        const decodedString = new TextDecoder().decode(result);
        expect(decodedString).toBe('HelloWorld !');
    });


})
