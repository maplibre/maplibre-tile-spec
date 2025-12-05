import { describe, it, expect } from "vitest";
import IntWrapper from "./intWrapper";
import { decodeString, decodeSharedDictionary } from "./stringDecoder";
import { encodePlainStrings, encodeDictionaryStrings, encodeStructField } from "../encoding/stringEncoder";
import {
    concatenateBuffers,
    createColumnMetadataForStruct,
    createStream,
    encodeFsstStrings,
    encodeSharedDictionary,
} from "./decodingTestUtils";
import { StringFlatVector } from "../vector/flat/stringFlatVector";
import { StringDictionaryVector } from "../vector/dictionary/stringDictionaryVector";
import { StringFsstDictionaryVector } from "../vector/fsst-dictionary/stringFsstDictionaryVector";
import { ScalarType } from "../metadata/tileset/tilesetMetadata";
import { PhysicalStreamType } from "../metadata/tile/physicalStreamType";
import { LengthType } from "../metadata/tile/lengthType";
import { LogicalStreamType } from "../metadata/tile/logicalStreamType";

describe("decodeString - Plain String Decoder", () => {
    it("should decode plain strings with simple ASCII values", () => {
        const expectedStrings = ["hello", "world", "test"];
        const encodedStrings = encodePlainStrings(expectedStrings);
        const offset = new IntWrapper(0);

        const result = decodeString("testColumn", encodedStrings, offset, 2);

        expect(result).toBeInstanceOf(StringFlatVector);
        const resultVec = result as StringFlatVector;
        for (let i = 0; i < expectedStrings.length; i++) {
            expect(resultVec.getValue(i)).toBe(expectedStrings[i]);
        }
    });

    it("should decode plain strings with varying lengths", () => {
        const expectedStrings = ["a", "abc", "hello world"];
        const encodedStrings = encodePlainStrings(expectedStrings);
        const offset = new IntWrapper(0);
        const result = decodeString("testColumn", encodedStrings, offset, 2);

        const resultVec = result as StringFlatVector;
        for (let i = 0; i < expectedStrings.length; i++) {
            expect(resultVec.getValue(i)).toBe(expectedStrings[i]);
        }
    });

    it("should decode plain strings with empty strings", () => {
        const expectedStrings = ["", "encodedStrings", "", "more"];
        const encodedStrings = encodePlainStrings(expectedStrings);
        const offset = new IntWrapper(0);
        const result = decodeString("testColumn", encodedStrings, offset, 2);

        expect(result).toBeInstanceOf(StringFlatVector);
        const resultVec = result as StringFlatVector;
        for (let i = 0; i < expectedStrings.length; i++) {
            expect(resultVec.getValue(i)).toBe(expectedStrings[i]);
        }
    });

    it("should decode mixed null and empty strings", () => {
        const expectedStrings = [null, "", "encodedStrings", null, ""];
        const encodedStrings = encodePlainStrings(expectedStrings);
        const offset = new IntWrapper(0);
        const result = decodeString("testColumn", encodedStrings, offset, 3);

        expect(result).not.toBeNull();
        for (let i = 0; i < expectedStrings.length; i++) {
            expect(result.getValue(i)).toBe(expectedStrings[i]);
        }
    });

    it("should decode mixed ASCII and UTF-8 strings", () => {
        const expectedStrings = ["hello", "Привет", "world", "日本"];
        const encodedStrings = encodePlainStrings(expectedStrings);
        const offset = new IntWrapper(0);
        const result = decodeString("testColumn", encodedStrings, offset, 2);

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
        const encodedStrings = encodeDictionaryStrings(expectedStrings);
        const offset = new IntWrapper(0);
        const result = decodeString("testColumn", encodedStrings, offset, 3);

        expect(result).toBeInstanceOf(StringDictionaryVector);
        const resultVec = result as StringDictionaryVector;
        for (let i = 0; i < expectedStrings.length; i++) {
            expect(resultVec.getValue(i)).toBe(expectedStrings[i]);
        }
    });

    it("should decode dictionary with single repeated string", () => {
        const expectedStrings = ["same", "same", "same"];
        const encodedStrings = encodeDictionaryStrings(expectedStrings);
        const offset = new IntWrapper(0);
        const result = decodeString("testColumn", encodedStrings, offset, 3);

        expect(result).toBeInstanceOf(StringDictionaryVector);
        for (let i = 0; i < expectedStrings.length; i++) {
            expect(result.getValue(i)).toBe(expectedStrings[i]);
        }
    });

    it("should decode dictionary with UTF-8 strings", () => {
        const expectedStrings = ["café", "日本", "café", "日本"];
        const encodedStrings = encodeDictionaryStrings(expectedStrings);
        const offset = new IntWrapper(0);
        const result = decodeString("testColumn", encodedStrings, offset, 3);

        expect(result).toBeInstanceOf(StringDictionaryVector);
        for (let i = 0; i < expectedStrings.length; i++) {
            expect(result.getValue(i)).toBe(expectedStrings[i]);
        }
    });

    it("should decode dictionary with all unique strings", () => {
        const expectedStrings = ["unique1", "unique2", "unique3", "unique4"];
        const encodedStrings = encodeDictionaryStrings(expectedStrings);
        const offset = new IntWrapper(0);
        const result = decodeString("testColumn", encodedStrings, offset, 3);

        expect(result).toBeInstanceOf(StringDictionaryVector);
        for (let i = 0; i < expectedStrings.length; i++) {
            expect(result.getValue(i)).toBe(expectedStrings[i]);
        }
    });

    it("should decode nullable dictionary strings", () => {
        const expectedStrings = [null, "", "encodedStrings", "", null];
        const encodedStrings = encodeDictionaryStrings(expectedStrings);
        const offset = new IntWrapper(0);
        const result = decodeString("testColumn", encodedStrings, offset, 4);

        expect(result).toBeInstanceOf(StringDictionaryVector);
        for (let i = 0; i < expectedStrings.length; i++) {
            expect(result.getValue(i)).toBe(expectedStrings[i]);
        }
    });
});

describe("decodeString - FSST Dictionary Decoder (Basic Coverage)", () => {
    it("should decode FSST-compressed strings with simple symbol table", () => {
        const encodedStrings = encodeFsstStrings();
        const offset = new IntWrapper(0);

        const result = decodeString("testColumn", encodedStrings, offset, 6);

        expect(result).toBeInstanceOf(StringFsstDictionaryVector);
        const resultVec = result as StringFsstDictionaryVector;

        const expectedValues = ["cat", "dog", "cat"];
        for (let i = 0; i < expectedValues.length; i++) {
            expect(resultVec.getValue(i)).toBe(expectedValues[i]);
        }
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
        const emptyStream = createStream(PhysicalStreamType.LENGTH, new Uint8Array([]), {
            logical: new LogicalStreamType(undefined, undefined, LengthType.VAR_BINARY),
        });
        const offset = new IntWrapper(0);
        const result = decodeString("testColumn", emptyStream, offset, 1);
        expect(result).toBeNull();
    });

    it("should handle single value plain string column", () => {
        const strings = ["single"];
        const encodedStrings = encodePlainStrings(strings);
        const offset = new IntWrapper(0);
        const result = decodeString("testColumn", encodedStrings, offset, 2);

        expect(result).toBeInstanceOf(StringFlatVector);
        expect((result as StringFlatVector).getValue(0)).toBe("single");
    });

    it("should handle single null value in plain string column (returns null)", () => {
        const strings = [null];
        const encodedStrings = encodePlainStrings(strings);
        const offset = new IntWrapper(0);
        const result = decodeString("testColumn", encodedStrings, offset, 3);
        expect(result).toBeNull();
    });
});

describe("decodeString - Integration Tests", () => {
    it("should correctly track offset through multiple streams", () => {
        const strings = ["hello", "world"];
        const encodedStrings = encodePlainStrings(strings);
        const offset = new IntWrapper(0);
        const initialOffset = offset.get();

        const result = decodeString("testColumn", encodedStrings, offset, 2);

        expect(result).toBeInstanceOf(StringFlatVector);
        expect(offset.get()).toBeGreaterThan(initialOffset);
        expect(offset.get()).toBe(encodedStrings.length);
    });

    it("should correctly track offset through nullable streams", () => {
        const strings = ["test", null, "encodedStrings"];
        const encodedStrings = encodePlainStrings(strings);
        const offset = new IntWrapper(0);
        const initialOffset = offset.get();

        const result = decodeString("testColumn", encodedStrings, offset, 3);

        expect(result).not.toBeNull();
        expect(offset.get()).toBeGreaterThan(initialOffset);
        expect(offset.get()).toBe(encodedStrings.length);
    });

    it("should correctly track offset through FSST dictionary streams", () => {
        const encodedStrings = encodeFsstStrings();
        const offset = new IntWrapper(0);
        const initialOffset = offset.get();

        const result = decodeString("testColumn", encodedStrings, offset, 6);

        expect(result).toBeInstanceOf(StringFsstDictionaryVector);
        expect(offset.get()).toBeGreaterThan(initialOffset);
        expect(offset.get()).toBe(encodedStrings.length);
    });

    it("should handle consecutive decoding operations with shared offset tracker", () => {
        const stream1 = encodePlainStrings(["first"]);
        const stream2 = encodePlainStrings(["second"]);
        const combinedStream = concatenateBuffers(stream1, stream2);

        const offset = new IntWrapper(0);

        const result1 = decodeString("column1", combinedStream, offset, 2);
        expect((result1 as StringFlatVector).getValue(0)).toBe("first");

        const offsetAfterFirst = offset.get();

        const result2 = decodeString("column2", combinedStream, offset, 2);
        expect((result2 as StringFlatVector).getValue(0)).toBe("second");

        expect(offset.get()).toBeGreaterThan(offsetAfterFirst);
        expect(offset.get()).toBe(combinedStream.length);
    });
});

describe("decodeSharedDictionary", () => {
    describe("basic functionality", () => {
        it("should decode single field with shared dictionary", () => {
            const dictionaryStrings = ["apple", "banana", "peach", "date"];
            const { lengthStream, dataStream } = encodeSharedDictionary(dictionaryStrings);

            const fieldStreams = encodeStructField([0, 1, 2, 3], [true, true, true, true]);
            const completeencodedStrings = concatenateBuffers(lengthStream, dataStream, fieldStreams);
            const columnMetaencodedStrings = createColumnMetadataForStruct("address", [{ name: "street" }]);

            const result = decodeSharedDictionary(
                completeencodedStrings,
                new IntWrapper(0),
                columnMetaencodedStrings,
                4,
            );

            expect(result).toHaveLength(1);
            expect(result[0]).toBeInstanceOf(StringDictionaryVector);
            expect(result[0].name).toBe("address:street");
            for (let i = 0; i < dictionaryStrings.length; i++) {
                expect(result[0].getValue(i)).toBe(dictionaryStrings[i]);
            }
        });
    });

    describe("nullability", () => {
        it("should handle nullable fields with PRESENT stream", () => {
            const dictionaryStrings = ["red", "green", "blue"];
            const { lengthStream, dataStream } = encodeSharedDictionary(dictionaryStrings);

            const fieldStreams = encodeStructField([0, 2], [true, false, true, false]);
            const completeencodedStrings = concatenateBuffers(lengthStream, dataStream, fieldStreams);
            const columnMetaencodedStrings = createColumnMetadataForStruct("colors", [{ name: "primary" }]);

            const result = decodeSharedDictionary(
                completeencodedStrings,
                new IntWrapper(0),
                columnMetaencodedStrings,
                4,
            );

            expect(result).toHaveLength(1);
            const expected = ["red", null, "blue", null];
            for (let i = 0; i < expected.length; i++) {
                expect(result[0].getValue(i)).toBe(expected[i]);
            }
        });

        it("should detect nullable fields when offsetCount < numFeatures", () => {
            const dictionaryStrings = ["alpha", "beta"];
            const { lengthStream, dataStream } = encodeSharedDictionary(dictionaryStrings);

            // Simulating implicit nullability by mismatched counts
            const fieldStreams = encodeStructField([0, 1], [true, false, false, true]);
            const completeencodedStrings = concatenateBuffers(lengthStream, dataStream, fieldStreams);
            const columnMetaencodedStrings = createColumnMetadataForStruct("greek", [{ name: "letter" }]);

            const result = decodeSharedDictionary(
                completeencodedStrings,
                new IntWrapper(0),
                columnMetaencodedStrings,
                4,
            );

            expect(result).toHaveLength(1);
            const expected = ["alpha", null, null, "beta"];
            for (let i = 0; i < expected.length; i++) {
                expect(result[0].getValue(i)).toBe(expected[i]);
            }
        });
    });

    describe("FSST encoding", () => {
        it("should decode FSST-compressed shared dictionary", () => {
            const dictionaryStrings = ["compressed1", "compressed2"];
            const { lengthStream, dataStream, symbolLengthStream, symbolDataStream } = encodeSharedDictionary(
                dictionaryStrings,
                { useFsst: true },
            );

            const fieldStreams = encodeStructField([0, 1], [true, true]);
            const completeencodedStrings = concatenateBuffers(
                lengthStream,
                symbolLengthStream,
                symbolDataStream,
                dataStream,
                fieldStreams,
            );
            const columnMetaencodedStrings = createColumnMetadataForStruct("encodedStrings", [{ name: "value" }]);

            const result = decodeSharedDictionary(
                completeencodedStrings,
                new IntWrapper(0),
                columnMetaencodedStrings,
                2,
            );

            expect(result).toHaveLength(1);
            expect(result[0]).toBeInstanceOf(StringFsstDictionaryVector);
            expect(result[0].name).toBe("encodedStrings:value");
        });
    });

    describe("field filtering", () => {
        it("should filter fields by propertyColumnNames", () => {
            const dictionaryStrings = ["val1", "val2"];
            const { lengthStream, dataStream } = encodeSharedDictionary(dictionaryStrings);

            const field1Streams = encodeStructField([0], [true]);
            const field2Streams = encodeStructField([1], [true]);
            const field3Streams = encodeStructField([0], [true]);

            const completeencodedStrings = concatenateBuffers(
                lengthStream,
                dataStream,
                field1Streams,
                field2Streams,
                field3Streams,
            );
            const columnMetaencodedStrings = createColumnMetadataForStruct("multi", [
                { name: "field1" },
                { name: "field2" },
                { name: "field3" },
            ]);

            const propertyColumnNames = new Set(["multi:field1", "multi:field3"]);
            const result = decodeSharedDictionary(
                completeencodedStrings,
                new IntWrapper(0),
                columnMetaencodedStrings,
                1,
                propertyColumnNames,
            );

            expect(result).toHaveLength(2);
            expect(result[0].name).toBe("multi:field1");
            expect(result[1].name).toBe("multi:field3");
        });

        it("should skip fields with numStreams=0", () => {
            const dictionaryStrings = ["present"];
            const { lengthStream, dataStream } = encodeSharedDictionary(dictionaryStrings);

            const field1Streams = encodeStructField([0], [true], true);
            const field2Streams = encodeStructField([], [], false); // numStreams=0
            const field3Streams = encodeStructField([0], [true], true);

            const completeencodedStrings = concatenateBuffers(
                lengthStream,
                dataStream,
                field1Streams,
                field2Streams,
                field3Streams,
            );
            const columnMetaencodedStrings = createColumnMetadataForStruct("test", [
                { name: "field1" },
                { name: "field2" },
                { name: "field3" },
            ]);

            const result = decodeSharedDictionary(
                completeencodedStrings,
                new IntWrapper(0),
                columnMetaencodedStrings,
                1,
            );

            expect(result).toHaveLength(2);
            expect(result[0].name).toBe("test:field1");
            expect(result[1].name).toBe("test:field3");
        });

        it("should handle mixed present and filtered fields", () => {
            const dictionaryStrings = ["encodedStrings"];
            const { lengthStream, dataStream } = encodeSharedDictionary(dictionaryStrings);

            const field1Streams = encodeStructField([0], [true], true);
            const field2Streams = encodeStructField([], [], false);
            const field3Streams = encodeStructField([0], [true], true);

            const completeencodedStrings = concatenateBuffers(
                lengthStream,
                dataStream,
                field1Streams,
                field2Streams,
                field3Streams,
            );
            const columnMetaencodedStrings = createColumnMetadataForStruct("mixed", [
                { name: "field1" },
                { name: "field2" },
                { name: "field3" },
            ]);

            const propertyColumnNames = new Set(["mixed:field3"]);
            const result = decodeSharedDictionary(
                completeencodedStrings,
                new IntWrapper(0),
                columnMetaencodedStrings,
                1,
                propertyColumnNames,
            );

            expect(result).toHaveLength(1);
            expect(result[0].name).toBe("mixed:field3");
        });
    });

    describe("error handling", () => {
        it("should throw error for non-string field types", () => {
            const dictionaryStrings = ["value"];
            const { lengthStream, dataStream } = encodeSharedDictionary(dictionaryStrings);
            const fieldStreams = encodeStructField([0], [true]);
            const completeencodedStrings = concatenateBuffers(lengthStream, dataStream, fieldStreams);

            const columnMetaencodedStrings = createColumnMetadataForStruct("invalid", [
                { name: "field1", type: ScalarType.INT_32 },
            ]);

            expect(() => {
                decodeSharedDictionary(completeencodedStrings, new IntWrapper(0), columnMetaencodedStrings, 1);
            }).toThrow("Currently only optional string fields are implemented for a struct.");
        });
    });

    describe("offset tracking", () => {
        it("should correctly advance offset through all streams", () => {
            const dictionaryStrings = ["a", "b", "c"];
            const { lengthStream, dataStream } = encodeSharedDictionary(dictionaryStrings);

            const field1Streams = encodeStructField([0, 1], [true, true]);
            const field2Streams = encodeStructField([1, 2], [true, true]);

            const completeencodedStrings = concatenateBuffers(lengthStream, dataStream, field1Streams, field2Streams);
            const columnMetaencodedStrings = createColumnMetadataForStruct("track", [
                { name: "field1" },
                { name: "field2" },
            ]);

            const offset = new IntWrapper(0);
            const initialOffset = offset.get();
            const result = decodeSharedDictionary(completeencodedStrings, offset, columnMetaencodedStrings, 2);

            expect(result).toHaveLength(2);
            expect(offset.get()).toBeGreaterThan(initialOffset);
            expect(offset.get()).toBe(completeencodedStrings.length);
        });
    });
});
