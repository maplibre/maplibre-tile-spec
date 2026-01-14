import { describe, it, expect, beforeEach } from "vitest";
import BitVector from "../flat/bitVector";
import { StringFsstDictionaryVector } from "./stringFsstDictionaryVector";

describe("StringFsstDictionaryVector", () => {
    let indexBuffer: Int32Array;
    let offsetBuffer: Uint32Array;
    let dictionaryBuffer: Uint8Array;
    let symbolOffsetBuffer: Uint32Array;
    let symbolTableBuffer: Uint8Array;
    let nullabilityBuffer: BitVector;

    beforeEach(() => {
        indexBuffer = new Int32Array([0, 1, 2]);
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
});
