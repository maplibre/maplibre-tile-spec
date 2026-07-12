import { describe, it, expect, beforeEach } from "vitest";
import BitVector from "../flat/bitVector";
import { StringFsstDictionaryVector } from "./stringFsstDictionaryVector";
import { createSymbolTable, encodeFsst } from "../../encoding/fsstEncoder";

describe("StringFsstDictionaryVector", () => {
    let indexBuffer: Uint32Array;
    let offsetBuffer: Uint32Array;
    let dictionaryBuffer: Uint8Array;
    let symbolOffsetBuffer: Uint32Array;
    let symbolTableBuffer: Uint8Array;
    let nullabilityBuffer: BitVector;

    beforeEach(() => {
        indexBuffer = new Uint32Array([0, 1, 2]);
        offsetBuffer = new Uint32Array([0, 5, 10]);
        dictionaryBuffer = new Uint8Array([
            /* mock data */
        ]);
        symbolOffsetBuffer = new Uint32Array([0, 3, 6]);
        symbolTableBuffer = new Uint8Array([
            /* mock data */
        ]);
        nullabilityBuffer = new BitVector(new Uint8Array([0b00000001]), 2);
    });

    it("should create an instance of StringFsstDictionaryVector", () => {
        const vector = new StringFsstDictionaryVector(
            "testVector",
            indexBuffer,
            offsetBuffer,
            dictionaryBuffer,
            symbolOffsetBuffer,
            symbolTableBuffer,
            nullabilityBuffer,
        );
        expect(vector).toBeInstanceOf(StringFsstDictionaryVector);
    });

    it("shares decoders only when all FSST buffers match and caches values sparsely", () => {
        const original = new TextEncoder().encode("alphabeta");
        const { symbols, symbolLengths } = createSymbolTable(["alpha", "beta"]);
        const compressed = encodeFsst(symbols, symbolLengths, original);
        const symbolOffsets = new Uint32Array(symbolLengths.length + 1);
        for (let i = 0; i < symbolLengths.length; i++) symbolOffsets[i + 1] = symbolOffsets[i] + symbolLengths[i];
        const offsets = new Uint32Array([0, 5, 9]);
        const indices = new Uint32Array([0, 1]);
        const presence = new BitVector(new Uint8Array([0b00000011]), 2);
        const createVector = (fsstOffsets: Uint32Array) =>
            new StringFsstDictionaryVector("test", indices, offsets, compressed, fsstOffsets, symbols, presence);

        const first = createVector(symbolOffsets);
        const second = createVector(symbolOffsets);
        const equivalentOffsets = new Uint32Array(symbolOffsets);
        const third = createVector(equivalentOffsets);
        const internals = (vector: StringFsstDictionaryVector) =>
            vector as unknown as {
                decodedValues?: Map<number, string> | Array<string | undefined>;
                dictionaryDecoder: object;
                symbolLengthBuffer?: Uint32Array;
            };

        expect(internals(first).decodedValues).toBeUndefined();
        expect(first.getValue(0)).toBe("alpha");
        expect(second.getValue(1)).toBe("beta");
        expect(third.getValue(0)).toBe("alpha");
        expect(internals(first).decodedValues).toBeInstanceOf(Map);
        expect(internals(first).dictionaryDecoder).toBe(internals(second).dictionaryDecoder);
        expect(internals(third).dictionaryDecoder).not.toBe(internals(first).dictionaryDecoder);
        expect(internals(first).symbolLengthBuffer).toBeInstanceOf(Uint32Array);
        expect(internals(second).symbolLengthBuffer).toBeUndefined();
    });

    it("promotes the value cache when access becomes dense", () => {
        const values = Array.from({ length: 256 }, (_, index) => `value-${index.toString().padStart(3, "0")}`);
        const original = new TextEncoder().encode(values.join(""));
        const { symbols, symbolLengths } = createSymbolTable(["value-"]);
        const compressed = encodeFsst(symbols, symbolLengths, original);
        const symbolOffsets = new Uint32Array(symbolLengths.length + 1);
        for (let i = 0; i < symbolLengths.length; i++) symbolOffsets[i + 1] = symbolOffsets[i] + symbolLengths[i];
        const offsets = new Uint32Array(values.length + 1);
        for (let i = 0; i < values.length; i++) offsets[i + 1] = offsets[i] + values[i].length;
        const vector = new StringFsstDictionaryVector(
            "test",
            new Uint32Array(values.map((_, index) => index)),
            offsets,
            compressed,
            symbolOffsets,
            symbols,
            new BitVector(new Uint8Array(values.length / 8).fill(0xff), values.length),
        );

        expect(values.map((_, index) => vector.getValue(index))).toEqual(values);
        expect((vector as unknown as { decodedValues: unknown }).decodedValues).toBeInstanceOf(Array);
    });
});
