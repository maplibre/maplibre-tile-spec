import { readFileSync } from "node:fs";
import { bench, describe } from "vitest";
import decodeTile from "../mltDecoder";
import { StringFsstDictionaryVector } from "../vector/fsst-dictionary/stringFsstDictionaryVector";
import { decodeFsst } from "./fsstDecoder";

type FsstDictionary = {
    symbols: Uint8Array;
    symbolLengths: Uint32Array;
    compressed: Uint8Array;
    decodedLength: number;
};

let checksum = 0;

function loadDictionary(tilePath: string, tableName: string, columnName: string): FsstDictionary {
    const tileData = readFileSync(new URL(tilePath, import.meta.url));
    const featureTable = decodeTile(new Uint8Array(tileData.buffer, tileData.byteOffset, tileData.byteLength)).find(
        (table) => table.name === tableName,
    );
    const dictionaryVector = featureTable?.propertyVectors.find((vector) => vector?.name === columnName);
    if (!(dictionaryVector instanceof StringFsstDictionaryVector)) {
        throw new Error(`FSST benchmark dictionary ${tableName}.${columnName} not found`);
    }
    // Access internal buffers so this benchmark isolates decodeFsst instead of measuring cached vector access.
    const {
        dataBuffer: compressed,
        symbolOffsetBuffer: symbolOffsets,
        symbolTableBuffer: symbols,
        offsetBuffer,
    } = dictionaryVector as unknown as {
        dataBuffer: Uint8Array;
        symbolOffsetBuffer: Uint32Array;
        symbolTableBuffer: Uint8Array;
        offsetBuffer: Uint32Array;
    };
    const symbolLengths = new Uint32Array(symbolOffsets.length - 1);
    for (let i = 0; i < symbolLengths.length; i++) {
        symbolLengths[i] = symbolOffsets[i + 1] - symbolOffsets[i];
    }
    return {
        symbols,
        symbolLengths,
        compressed,
        decodedLength: offsetBuffer[offsetBuffer.length - 1],
    };
}

function consume(decoded: Uint8Array): void {
    checksum = (checksum + decoded[0] + decoded[decoded.length >> 1] + decoded[decoded.length - 1]) | 0;
}

describe("decode whole FSST dictionary", () => {
    const typical = loadDictionary("../../../test/expected/tag0x01/amazon_here/5_16_10.mlt", "pois", "name");
    bench(
        `typical: ${typical.compressed.length} compressed bytes to ${typical.decodedLength} decoded bytes`,
        () => consume(decodeFsst(typical.symbols, typical.symbolLengths, typical.compressed)),
        { warmupTime: 500, time: 5_000 },
    );

    const large = loadDictionary("../../../test/expected/tag0x01/omt/14_8299_10748.mlt", "poi", "name");
    bench(
        `large: ${large.compressed.length} compressed bytes to ${large.decodedLength} decoded bytes`,
        () => consume(decodeFsst(large.symbols, large.symbolLengths, large.compressed)),
        { warmupTime: 500, time: 5_000 },
    );

    const xlarge = loadDictionary("../../../test/fixtures/osm/xlarge.mlt", "hiking", "name");
    bench(
        `xlarge: ${xlarge.compressed.length} compressed bytes to ${xlarge.decodedLength} decoded bytes`,
        () => consume(decodeFsst(xlarge.symbols, xlarge.symbolLengths, xlarge.compressed)),
        { warmupTime: 500, time: 5_000 },
    );
});
