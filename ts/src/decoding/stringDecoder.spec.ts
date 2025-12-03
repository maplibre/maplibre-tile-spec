import { describe, it, expect } from "vitest";
import { LogicalLevelTechnique } from "../metadata/tile/logicalLevelTechnique";
import { PhysicalStreamType } from "../metadata/tile/physicalStreamType";
import { DictionaryType } from "../metadata/tile/dictionaryType";
import { LengthType } from "../metadata/tile/lengthType";
import { LogicalStreamType } from "../metadata/tile/logicalStreamType";
import { PhysicalLevelTechnique } from "../metadata/tile/physicalLevelTechnique";
import { OffsetType } from "../metadata/tile/offsetType";
import IntWrapper from "./intWrapper";
import { decodeString } from "./stringDecoder";
import {
    buildEncodedStream,
    encodeVarintInt32Array,
    encodeStrings,
    createStringLengths,
    concatenateBuffers,
    encodeBooleanRle,
} from "./decodingTestUtils";
import { StringFlatVector } from "../vector/flat/stringFlatVector";
import { StringDictionaryVector } from "../vector/dictionary/stringDictionaryVector";
import { StringFsstDictionaryVector } from "../vector/fsst-dictionary/stringFsstDictionaryVector";

function createStream(
    physicalType: PhysicalStreamType,
    data: Uint8Array,
    options: {
        logical?: LogicalStreamType;
        technique?: PhysicalLevelTechnique;
        count?: number;
    } = {},
): Uint8Array {
    const count = options.count ?? 0;
    return buildEncodedStream(
        {
            physicalStreamType: physicalType,
            logicalStreamType: options.logical ?? new LogicalStreamType(),
            logicalLevelTechnique1: LogicalLevelTechnique.NONE,
            logicalLevelTechnique2: LogicalLevelTechnique.NONE,
            physicalLevelTechnique: options.technique ?? PhysicalLevelTechnique.NONE,
            numValues: count,
            byteLength: data.length,
            decompressedCount: count,
        },
        data,
    );
}

function createStringStreams(
    strings: (string | null)[],
    encoding: "plain" | "dictionary" | "fsst" = "plain",
): Uint8Array {
    if (encoding === "fsst") return createFsstDictionaryStringStreams();

    const hasNull = strings.some((s) => s === null);
    const nonNullStrings = strings.filter((s): s is string => s !== null);

    const stringBytes = encodeStrings(
        encoding === "dictionary"
            ? Array.from(new Set(nonNullStrings)) // Unique strings for dictionary
            : nonNullStrings,
    );

    const streams: Uint8Array[] = [];
    if (hasNull) {
        const nullabilityValues = strings.map((s) => s !== null);
        streams.push(
            createStream(PhysicalStreamType.PRESENT, encodeBooleanRle(nullabilityValues), {
                technique: PhysicalLevelTechnique.VARINT,
                count: nullabilityValues.length,
            }),
        );
    }

    if (encoding === "plain") {
        const lengths = createStringLengths(nonNullStrings);
        streams.push(
            createStream(PhysicalStreamType.LENGTH, encodeVarintInt32Array(new Int32Array(lengths)), {
                logical: new LogicalStreamType(undefined, undefined, LengthType.VAR_BINARY),
                technique: PhysicalLevelTechnique.VARINT,
                count: lengths.length,
            }),
            createStream(PhysicalStreamType.DATA, stringBytes, { logical: new LogicalStreamType(DictionaryType.NONE) }),
        );
    } else {
        const uniqueStrings = Array.from(new Set(nonNullStrings));
        const stringMap = new Map(uniqueStrings.map((s, i) => [s, i]));
        const offsets = nonNullStrings.map((s) => stringMap.get(s));
        const dictLengths = createStringLengths(uniqueStrings);

        streams.push(
            createStream(PhysicalStreamType.OFFSET, encodeVarintInt32Array(new Int32Array(offsets)), {
                logical: new LogicalStreamType(undefined, OffsetType.STRING),
                technique: PhysicalLevelTechnique.VARINT,
                count: offsets.length,
            }),
            createStream(PhysicalStreamType.LENGTH, encodeVarintInt32Array(new Int32Array(dictLengths)), {
                logical: new LogicalStreamType(undefined, undefined, LengthType.DICTIONARY),
                technique: PhysicalLevelTechnique.VARINT,
                count: dictLengths.length,
            }),
            createStream(PhysicalStreamType.DATA, stringBytes, {
                logical: new LogicalStreamType(DictionaryType.SINGLE),
            }),
        );
    }

    return concatenateBuffers(...streams);
}

function createFsstDictionaryStringStreams(): Uint8Array {
    // FSST hardcoded data for test
    const symbolTable = new Uint8Array([99, 97, 116, 100, 111, 103]); // "catdog"
    const symbolLengths = new Int32Array([3, 3]);
    const compressedDictionary = new Uint8Array([0, 1]);
    const dictionaryLengths = new Int32Array([3, 3]);
    const offsets = new Int32Array([0, 1, 0]); // "cat", "dog", "cat"
    const numValues = 3;

    return concatenateBuffers(
        createStream(PhysicalStreamType.PRESENT, encodeBooleanRle(new Array(numValues).fill(true)), {
            technique: PhysicalLevelTechnique.VARINT,
            count: numValues,
        }),
        createStream(PhysicalStreamType.DATA, symbolTable, { logical: new LogicalStreamType(DictionaryType.FSST) }),
        createStream(PhysicalStreamType.LENGTH, encodeVarintInt32Array(symbolLengths), {
            logical: new LogicalStreamType(undefined, undefined, LengthType.SYMBOL),
            technique: PhysicalLevelTechnique.VARINT,
            count: symbolLengths.length,
        }),
        createStream(PhysicalStreamType.OFFSET, encodeVarintInt32Array(offsets), {
            logical: new LogicalStreamType(undefined, OffsetType.STRING),
            technique: PhysicalLevelTechnique.VARINT,
            count: offsets.length,
        }),
        createStream(PhysicalStreamType.LENGTH, encodeVarintInt32Array(dictionaryLengths), {
            logical: new LogicalStreamType(undefined, undefined, LengthType.DICTIONARY),
            technique: PhysicalLevelTechnique.VARINT,
            count: dictionaryLengths.length,
        }),
        createStream(PhysicalStreamType.DATA, compressedDictionary, {
            logical: new LogicalStreamType(DictionaryType.SINGLE),
        }),
    );
}

describe("decodeString - Plain String Decoder", () => {
    it("should decode plain strings with simple ASCII values", () => {
        const expectedStrings = ["hello", "world", "test"];
        const fullStream = createStringStreams(expectedStrings, "plain");
        const offset = new IntWrapper(0);

        const result = decodeString("testColumn", fullStream, offset, 2);

        expect(result).toBeInstanceOf(StringFlatVector);
        const resultVec = result as StringFlatVector;
        for (let i = 0; i < expectedStrings.length; i++) {
            expect(resultVec.getValue(i)).toBe(expectedStrings[i]);
        }
    });

    it("should decode plain strings with varying lengths", () => {
        const expectedStrings = ["a", "abc", "hello world"];
        const fullStream = createStringStreams(expectedStrings, "plain");
        const offset = new IntWrapper(0);

        const result = decodeString("testColumn", fullStream, offset, 2);

        expect(result).toBeInstanceOf(StringFlatVector);
        const resultVec = result as StringFlatVector;
        for (let i = 0; i < expectedStrings.length; i++) {
            expect(resultVec.getValue(i)).toBe(expectedStrings[i]);
        }
    });

    it("should decode nullable plain strings", () => {
        const expectedStrings: (string | null)[] = ["hello", null, "world", null, "test"];
        const fullStream = createStringStreams(expectedStrings, "plain");
        const offset = new IntWrapper(0);

        const result = decodeString("testColumn", fullStream, offset, 3);

        // Nullable strings return StringDictionaryVector or StringFlatVector
        expect(result).not.toBeNull();
        for (let i = 0; i < expectedStrings.length; i++) {
            expect(result.getValue(i)).toBe(expectedStrings[i]);
        }
    });

    it("should decode plain strings with empty strings", () => {
        const expectedStrings = ["", "data", "", "more"];
        const fullStream = createStringStreams(expectedStrings, "plain");
        const offset = new IntWrapper(0);

        const result = decodeString("testColumn", fullStream, offset, 2);

        expect(result).toBeInstanceOf(StringFlatVector);
        const resultVec = result as StringFlatVector;
        for (let i = 0; i < expectedStrings.length; i++) {
            expect(resultVec.getValue(i)).toBe(expectedStrings[i]);
        }
    });

    it("should decode mixed null and empty strings", () => {
        const expectedStrings: (string | null)[] = [null, "", "data", null, ""];
        const fullStream = createStringStreams(expectedStrings, "plain");
        const offset = new IntWrapper(0);

        const result = decodeString("testColumn", fullStream, offset, 3);

        expect(result).not.toBeNull();
        for (let i = 0; i < expectedStrings.length; i++) {
            expect(result.getValue(i)).toBe(expectedStrings[i]);
        }
    });

    it("should decode mixed ASCII and UTF-8 strings", () => {
        const expectedStrings = ["hello", "Привет", "world", "日本"];
        const fullStream = createStringStreams(expectedStrings, "plain");
        const offset = new IntWrapper(0);

        const result = decodeString("testColumn", fullStream, offset, 2);

        expect(result).toBeInstanceOf(StringFlatVector);
        const resultVec = result as StringFlatVector;
        for (let i = 0; i < expectedStrings.length; i++) {
            expect(resultVec.getValue(i)).toBe(expectedStrings[i]);
        }
    });
});

describe("decodeString - Dictionary String Decoder", () => {
    it("should decode dictionary-compressed strings with repeated values", () => {
        const expectedStrings = ["cat", "dog", "cat", "cat", "dog"];
        const fullStream = createStringStreams(expectedStrings, "dictionary");
        const offset = new IntWrapper(0);

        const result = decodeString("testColumn", fullStream, offset, 3);

        expect(result).toBeInstanceOf(StringDictionaryVector);
        const resultVec = result as StringDictionaryVector;
        for (let i = 0; i < expectedStrings.length; i++) {
            expect(resultVec.getValue(i)).toBe(expectedStrings[i]);
        }
    });

    it("should decode dictionary with single repeated string", () => {
        const expectedStrings = ["same", "same", "same"];
        const fullStream = createStringStreams(expectedStrings, "dictionary");
        const offset = new IntWrapper(0);

        const result = decodeString("testColumn", fullStream, offset, 3);

        expect(result).toBeInstanceOf(StringDictionaryVector);
        for (let i = 0; i < expectedStrings.length; i++) {
            expect(result.getValue(i)).toBe("same");
        }
    });

    it("should decode dictionary with UTF-8 strings", () => {
        const expectedStrings = ["café", "日本", "café", "日本"];
        const fullStream = createStringStreams(expectedStrings, "dictionary");
        const offset = new IntWrapper(0);

        const result = decodeString("testColumn", fullStream, offset, 3);

        expect(result).toBeInstanceOf(StringDictionaryVector);
        for (let i = 0; i < expectedStrings.length; i++) {
            expect(result.getValue(i)).toBe(expectedStrings[i]);
        }
    });

    it("should decode dictionary with all unique strings", () => {
        const expectedStrings = ["unique1", "unique2", "unique3", "unique4"];
        const fullStream = createStringStreams(expectedStrings, "dictionary");
        const offset = new IntWrapper(0);

        const result = decodeString("testColumn", fullStream, offset, 3);

        expect(result).toBeInstanceOf(StringDictionaryVector);
        for (let i = 0; i < expectedStrings.length; i++) {
            expect(result.getValue(i)).toBe(expectedStrings[i]);
        }
    });

    it("should decode nullable dictionary strings", () => {
        const expectedStrings: (string | null)[] = [null, "", "data", "", null];
        const fullStream = createStringStreams(expectedStrings, "dictionary");
        const offset = new IntWrapper(0);

        const result = decodeString("testColumn", fullStream, offset, 4);

        expect(result).toBeInstanceOf(StringDictionaryVector);
        for (let i = 0; i < expectedStrings.length; i++) {
            expect(result.getValue(i)).toBe(expectedStrings[i]);
        }
    });

    describe("decodeString - FSST Dictionary Decoder (Basic Coverage)", () => {
        it("should decode FSST-compressed strings with simple symbol table", () => {
            const fullStream = createStringStreams([], "fsst");
            const offset = new IntWrapper(0);

            const result = decodeString("testColumn", fullStream, offset, 6);

            expect(result).toBeInstanceOf(StringFsstDictionaryVector);
            const resultVec = result as StringFsstDictionaryVector;

            // Expected: ["cat", "dog", "cat"]
            expect(resultVec.getValue(0)).toBe("cat");
            expect(resultVec.getValue(1)).toBe("dog");
            expect(resultVec.getValue(2)).toBe("cat");
        });
    });

    describe("decodeString - Empty Column Edge Cases", () => {
        it("should handle empty column with numStreams = 0 (returns null)", () => {
            const fullStream = new Uint8Array([]);
            const offset = new IntWrapper(0);
            const result = decodeString("testColumn", fullStream, offset, 0);

            expect(result).toBeNull();
        });

        it("should handle column with all zero-length streams (returns null)", () => {
            // Create a stream with metadata but zero byteLength
            const metadata = {
                physicalStreamType: PhysicalStreamType.LENGTH,
                logicalStreamType: new LogicalStreamType(undefined, undefined, LengthType.VAR_BINARY),
                logicalLevelTechnique1: LogicalLevelTechnique.NONE,
                logicalLevelTechnique2: LogicalLevelTechnique.NONE,
                physicalLevelTechnique: PhysicalLevelTechnique.VARINT,
                numValues: 0,
                byteLength: 0,
                decompressedCount: 0,
            };
            const emptyStream = buildEncodedStream(metadata, new Uint8Array([]));

            const offset = new IntWrapper(0);
            const result = decodeString("testColumn", emptyStream, offset, 1);

            expect(result).toBeNull();
        });

        it("should handle single value plain string column", () => {
            const strings = ["single"];
            const fullStream = createStringStreams(strings, "plain");
            const offset = new IntWrapper(0);
            const result = decodeString("testColumn", fullStream, offset, 2);

            expect(result).toBeInstanceOf(StringFlatVector);
            const resultVec = result as StringFlatVector;
            expect(resultVec.getValue(0)).toBe("single");
        });

        it("should handle single null value in plain string column (returns null)", () => {
            const strings: (string | null)[] = [null];
            const fullStream = createStringStreams(strings, "plain");
            const offset = new IntWrapper(0);
            const result = decodeString("testColumn", fullStream, offset, 3);

            // When there are only null values, the decoder returns null since there's no data
            expect(result).toBeNull();
        });
    });

    describe("decodeString - Integration Tests", () => {
        it("should correctly track offset through multiple streams", () => {
            const strings = ["hello", "world"];
            const fullStream = createStringStreams(strings, "plain");
            const offset = new IntWrapper(0);
            const initialOffset = offset.get();

            const result = decodeString("testColumn", fullStream, offset, 2);

            expect(result).toBeInstanceOf(StringFlatVector);
            expect(offset.get()).toBeGreaterThan(initialOffset);
            expect(offset.get()).toBe(fullStream.length);
        });

        it("should correctly track offset through nullable streams", () => {
            const strings: (string | null)[] = ["test", null, "data"];
            const fullStream = createStringStreams(strings, "plain");
            const offset = new IntWrapper(0);
            const initialOffset = offset.get();

            const result = decodeString("testColumn", fullStream, offset, 3);

            expect(result).not.toBeNull();
            // Verify offset advanced through PRESENT, LENGTH, and DATA streams
            expect(offset.get()).toBeGreaterThan(initialOffset);
            expect(offset.get()).toBe(fullStream.length);
        });

        it("should correctly track offset through FSST dictionary streams", () => {
            const fullStream = createStringStreams([], "fsst");
            const offset = new IntWrapper(0);
            const initialOffset = offset.get();

            const result = decodeString("testColumn", fullStream, offset, 6);

            expect(result).toBeInstanceOf(StringFsstDictionaryVector);
            // Verify offset advanced through all 6 streams (PRESENT, SYMBOL_TABLE, SYMBOL_LENGTH, OFFSET, DICT_LENGTH, DICT_DATA)
            expect(offset.get()).toBeGreaterThan(initialOffset);
            expect(offset.get()).toBe(fullStream.length);
        });

        it("should handle consecutive decoding operations with shared offset tracker", () => {
            // Create two separate string columns in sequence
            const stream1 = createStringStreams(["first"], "plain");
            const stream2 = createStringStreams(["second"], "plain");
            const combinedStream = concatenateBuffers(stream1, stream2);

            const offset = new IntWrapper(0);

            // Decode first column
            const result1 = decodeString("column1", combinedStream, offset, 2);
            expect(result1).toBeInstanceOf(StringFlatVector);
            const vec1 = result1 as StringFlatVector;
            expect(vec1.getValue(0)).toBe("first");

            // Offset should now point to start of second column
            const offsetAfterFirst = offset.get();

            // Decode second column
            const result2 = decodeString("column2", combinedStream, offset, 2);
            expect(result2).toBeInstanceOf(StringFlatVector);
            const vec2 = result2 as StringFlatVector;
            expect(vec2.getValue(0)).toBe("second");

            // Verify offset advanced through both columns
            expect(offset.get()).toBeGreaterThan(offsetAfterFirst);
            expect(offset.get()).toBe(combinedStream.length);
        });
    });
});
