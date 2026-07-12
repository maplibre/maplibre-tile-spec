import { bench, describe } from "vitest";
import { createSymbolTable, encodeFsst } from "../encoding/fsstEncoder";
import { decodeFsst, FsstDecoder } from "./fsstDecoder";

const textEncoder = new TextEncoder();
const phrases = [
    "international hiking network;",
    "regional hiking route;",
    "OpenStreetMap contributors;",
    "relation_id_tokens;",
    "Zürich;",
    "Paris;",
    "network;nwn;rwn;lwn;iwn;",
];
const { symbols, symbolLengths } = createSymbolTable(phrases);
const original = textEncoder.encode(phrases.join("").repeat(25_000));
const compressed = encodeFsst(symbols, symbolLengths, original);
let checksum = 0;

function referenceDecodeFsst(): Uint8Array {
    const decoded: number[] = [];
    const symbolOffsets = new Array<number>(symbolLengths.length).fill(0);
    for (let i = 1; i < symbolLengths.length; i++) {
        symbolOffsets[i] = symbolOffsets[i - 1] + symbolLengths[i - 1];
    }
    for (let i = 0; i < compressed.length; i++) {
        const symbolIndex = compressed[i];
        if (symbolIndex === 255) {
            decoded.push(compressed[++i]);
        } else {
            const length = symbolLengths[symbolIndex];
            const offset = symbolOffsets[symbolIndex];
            for (let j = 0; j < length; j++) decoded.push(symbols[offset + j]);
        }
    }
    return new Uint8Array(decoded);
}

const options = { warmupTime: 500, time: 2_000 } as const;

describe(`FSST ${compressed.length} compressed bytes to ${original.length} decoded bytes`, () => {
    bench(
        "reference whole dictionary",
        () => {
            checksum += referenceDecodeFsst().length;
        },
        options,
    );

    bench(
        "optimized whole dictionary",
        () => {
            checksum += decodeFsst(symbols, symbolLengths, compressed).length;
        },
        options,
    );

    bench(
        "one sparse range",
        () => {
            const decoder = new FsstDecoder(symbols, symbolLengths, compressed);
            checksum += decoder.decodeRange(original.length / 2, original.length / 2 + 32).length;
        },
        options,
    );

    bench(
        "ten sparse ranges",
        () => {
            const decoder = new FsstDecoder(symbols, symbolLengths, compressed);
            for (let i = 1; i <= 10; i++) {
                const start = Math.floor((original.length * i) / 11);
                checksum += decoder.decodeRange(start, start + 32).length;
            }
        },
        options,
    );
});
