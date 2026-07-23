import { readFileSync } from "node:fs";
import { bench, describe } from "vitest";
import decodeTile from "../../mltDecoder";
import type BitVector from "../flat/bitVector";
import { type FsstDictionaryCache, StringFsstDictionaryVector } from "./stringFsstDictionaryVector";

type FsstVectorData = {
    indexBuffer: Uint32Array;
    offsetBuffer: Uint32Array;
    dataBuffer: Uint8Array;
    symbolOffsetBuffer: Uint32Array;
    symbolTableBuffer: Uint8Array;
    nullabilityBuffer: BitVector;
};

describe("cold FSST dictionary access", () => {
    const tileData = readFileSync(new URL("../../../../test/expected/tag0x01/omt/14_8299_10748.mlt", import.meta.url));
    const featureTable = decodeTile(new Uint8Array(tileData.buffer, tileData.byteOffset, tileData.byteLength)).find(
        (table) => table.name === "poi",
    );
    const dictionaryVector = featureTable?.propertyVectors.find((vector) => vector?.name === "name");
    if (!(dictionaryVector instanceof StringFsstDictionaryVector)) {
        throw new Error("FSST cache benchmark dictionary poi.name not found");
    }

    // Access internal buffers to construct cold vectors around the same production dictionary.
    const vectorData = dictionaryVector as unknown as FsstVectorData;
    const valueIndex = Array.from({ length: dictionaryVector.size }, (_, index) => index).find((index) =>
        dictionaryVector.has(index),
    );
    if (valueIndex === undefined) throw new Error("FSST cache benchmark dictionary contains no values");

    let decodedLengthChecksum = 0;
    const createVector = (dataBuffer: Uint8Array, sharedDictionaryCache?: FsstDictionaryCache) =>
        new StringFsstDictionaryVector(
            "name",
            vectorData.indexBuffer,
            vectorData.offsetBuffer,
            dataBuffer,
            vectorData.symbolOffsetBuffer,
            vectorData.symbolTableBuffer,
            vectorData.nullabilityBuffer,
            sharedDictionaryCache,
        );

    bench(
        "one ordinary FSST vector",
        () => {
            // A fresh buffer keeps every benchmark iteration cold.
            const vector = createVector(vectorData.dataBuffer.slice());
            decodedLengthChecksum = (decodedLengthChecksum + (vector.getValue(valueIndex)?.length ?? 0)) | 0;
        },
        { warmupTime: 500, time: 5_000 },
    );

    bench(
        "two ordinary FSST vectors",
        () => {
            // Ordinary vectors do not receive a SharedDict cache, so both decode independently.
            for (let i = 0; i < 2; i++) {
                const vector = createVector(vectorData.dataBuffer.slice());
                decodedLengthChecksum = (decodedLengthChecksum + (vector.getValue(valueIndex)?.length ?? 0)) | 0;
            }
        },
        { warmupTime: 500, time: 5_000 },
    );

    bench(
        "two vectors from one SharedDict",
        () => {
            // Vectors from one SharedDict receive the same cache, so the second reuses the first decoded dictionary.
            const sharedData = vectorData.dataBuffer.slice();
            const sharedDictionaryCache: FsstDictionaryCache = {};
            for (let i = 0; i < 2; i++) {
                const vector = createVector(sharedData, sharedDictionaryCache);
                decodedLengthChecksum = (decodedLengthChecksum + (vector.getValue(valueIndex)?.length ?? 0)) | 0;
            }
        },
        { warmupTime: 500, time: 5_000 },
    );

    bench(
        "five vectors from one SharedDict",
        () => {
            // Vectors from one SharedDict receive the same cache, so four reuse the first decoded dictionary.
            const sharedData = vectorData.dataBuffer.slice();
            const sharedDictionaryCache: FsstDictionaryCache = {};
            for (let i = 0; i < 5; i++) {
                const vector = createVector(sharedData, sharedDictionaryCache);
                decodedLengthChecksum = (decodedLengthChecksum + (vector.getValue(valueIndex)?.length ?? 0)) | 0;
            }
        },
        { warmupTime: 500, time: 5_000 },
    );
});
