import { describe, it, expect } from "vitest";
import { decodePropertyColumn } from "./propertyDecoder";
import IntWrapper from "./intWrapper";
import { ScalarType, type Column } from "../metadata/tileset/tilesetMetadata";
import { IntFlatVector } from "../vector/flat/intFlatVector";
import { LongFlatVector } from "../vector/flat/longFlatVector";
import { FloatFlatVector } from "../vector/flat/floatFlatVector";
import { DoubleFlatVector } from "../vector/flat/doubleFlatVector";
import { BooleanFlatVector } from "../vector/flat/booleanFlatVector";
import { IntSequenceVector } from "../vector/sequence/intSequenceVector";
import { LongSequenceVector } from "../vector/sequence/longSequenceVector";
import { IntConstVector } from "../vector/constant/intConstVector";
import { LongConstVector } from "../vector/constant/longConstVector";
import { StringDictionaryVector } from "../vector/dictionary/stringDictionaryVector";
import { createColumnMetadataForStruct, encodeSharedDictionary, encodeStructField } from "./decodingTestUtils";
import { concatenateBuffers } from "../encoding/encodingUtils";
import {
    encodeInt32NoneColumn,
    encodeInt32DeltaColumn,
    encodeInt32RleColumn,
    encodeInt32DeltaRleColumn,
    encodeUint32Column,
    encodeInt64NoneColumn,
    encodeInt64DeltaColumn,
    encodeInt64RleColumn,
    encodeInt64DeltaRleColumn,
    encodeInt64NullableColumn,
    encodeUint64Column,
    encodeUint64NullableColumn,
    encodeFloatColumn,
    encodeFloatNullableColumn,
    encodeDoubleColumn,
    encodeDoubleNullableColumn,
    encodeBooleanColumn,
    encodeBooleanNullableColumn,
    encodeInt32NullableColumn,
} from "../encoding/propertyEncoder";

function createColumnMetadata(name: string, scalarType: number, nullable: boolean = false): Column {
    return {
        name: name,
        nullable: nullable,
        type: "scalarType",
        scalarType: {
            physicalType: scalarType,
            type: "physicalType",
        },
    };
}

describe("decodePropertyColumn - INT_32", () => {
    it("should decode INT_32 column with NONE encoding (signed)", () => {
        const expectedValues = new Int32Array([2, -4, 6]);
        const columnMetadata = createColumnMetadata("testColumn", ScalarType.INT_32, false);
        const encodedData = encodeInt32NoneColumn(expectedValues);
        const offset = new IntWrapper(0);

        const result = decodePropertyColumn(encodedData, offset, columnMetadata, 1, expectedValues.length);

        expect(result).toBeInstanceOf(IntFlatVector);
        const resultVec = result as IntFlatVector;
        for (let i = 0; i < expectedValues.length; i++) {
            expect(resultVec.getValue(i)).toBe(expectedValues[i]);
        }
    });

    it("should decode INT_32 column with DELTA encoding", () => {
        const expectedValues = new Int32Array([2, 4, 6]);
        const columnMetadata = createColumnMetadata("testColumn", ScalarType.INT_32, false);
        const encodedData = encodeInt32DeltaColumn(expectedValues);
        const offset = new IntWrapper(0);

        const result = decodePropertyColumn(encodedData, offset, columnMetadata, 1, expectedValues.length);

        expect(result).toBeInstanceOf(IntFlatVector);
        const resultVec = result as IntFlatVector;
        for (let i = 0; i < expectedValues.length; i++) {
            expect(resultVec.getValue(i)).toBe(expectedValues[i]);
        }
    });

    it("should decode INT_32 column with RLE encoding", () => {
        const expectedValues = new Int32Array([100, 100, 100, -50, -50]);
        const columnMetadata = createColumnMetadata("testColumn", ScalarType.INT_32, false);
        const encodedData = encodeInt32RleColumn([
            [3, 100],
            [2, -50],
        ]);
        const offset = new IntWrapper(0);

        const result = decodePropertyColumn(encodedData, offset, columnMetadata, 1, expectedValues.length);

        expect(result).toBeInstanceOf(IntFlatVector);
        const resultVec = result as IntFlatVector;
        for (let i = 0; i < expectedValues.length; i++) {
            expect(resultVec.getValue(i)).toBe(expectedValues[i]);
        }
    });

    it("should decode INT_32 column with DELTA+RLE encoding", () => {
        const expectedValues = new Int32Array([10, 12, 14, 15, 16]);
        const columnMetadata = createColumnMetadata("testColumn", ScalarType.INT_32, false);
        const encodedData = encodeInt32DeltaRleColumn([
            [1, 10],
            [2, 2],
            [2, 1],
        ]);
        const offset = new IntWrapper(0);

        const result = decodePropertyColumn(encodedData, offset, columnMetadata, 1, expectedValues.length);

        expect(result).toBeInstanceOf(IntFlatVector);
        const resultVec = result as IntFlatVector;
        for (let i = 0; i < expectedValues.length; i++) {
            expect(resultVec.getValue(i)).toBe(expectedValues[i]);
        }
    });

    it("should decode nullable INT_32 column with null values", () => {
        const expectedValues = [2, null, -4, null, 6];
        const columnMetadata = createColumnMetadata("testColumn", ScalarType.INT_32, true);
        const encodedData = encodeInt32NullableColumn(expectedValues);
        const offset = new IntWrapper(0);

        const result = decodePropertyColumn(encodedData, offset, columnMetadata, 2, expectedValues.length);

        expect(result).toBeInstanceOf(IntFlatVector);
        const resultVec = result as IntFlatVector;
        for (let i = 0; i < expectedValues.length; i++) {
            expect(resultVec.getValue(i)).toBe(expectedValues[i]);
        }
    });

    it("should decode INT_32 SEQUENCE vector", () => {
        const numValues = 5;
        const value = 10;
        const columnMetadata = createColumnMetadata("testColumn", ScalarType.INT_32, false);
        const encodedData = encodeInt32DeltaRleColumn([[numValues, value]]);
        const offset = new IntWrapper(0);

        const result = decodePropertyColumn(encodedData, offset, columnMetadata, 1, numValues);

        expect(result).toBeInstanceOf(IntSequenceVector);
        const seqVec = result as IntSequenceVector;
        expect(seqVec.getValue(0)).toBe(value);
        expect(seqVec.getValue(1)).toBe(value + value);
        expect(seqVec.getValue(2)).toBe(value + value * 2);
    });

    it("should decode INT_32 CONST vector", () => {
        const numValues = 5;
        const constValue = 42;
        const columnMetadata = createColumnMetadata("testColumn", ScalarType.INT_32, false);
        const encodedData = encodeInt32RleColumn([[numValues, constValue]]);
        const offset = new IntWrapper(0);

        const result = decodePropertyColumn(encodedData, offset, columnMetadata, 1, numValues);

        expect(result).toBeInstanceOf(IntConstVector);
        const constVec = result as IntConstVector;
        expect(constVec.getValue(0)).toBe(constValue);
        expect(constVec.getValue(4)).toBe(constValue);
    });
});

describe("decodePropertyColumn - UINT_32", () => {
    it("should decode UINT_32 column with NONE encoding (unsigned)", () => {
        const expectedValues = new Uint32Array([2, 4, 6, 100]);
        const columnMetadata = createColumnMetadata("testColumn", ScalarType.UINT_32, false);
        const encodedData = encodeUint32Column(expectedValues);
        const offset = new IntWrapper(0);

        const result = decodePropertyColumn(encodedData, offset, columnMetadata, 1, expectedValues.length);

        expect(result).toBeInstanceOf(IntFlatVector);
        const resultVec = result as IntFlatVector;
        for (let i = 0; i < expectedValues.length; i++) {
            expect(resultVec.getValue(i)).toBe(expectedValues[i]);
        }
    });
});

describe("decodePropertyColumn - INT_64", () => {
    it("should decode INT_64 column with NONE encoding (signed)", () => {
        const expectedValues = new BigInt64Array([2n, -4n, 6n]);
        const columnMetadata = createColumnMetadata("testColumn", ScalarType.INT_64, false);
        const encodedData = encodeInt64NoneColumn(expectedValues);
        const offset = new IntWrapper(0);

        const result = decodePropertyColumn(encodedData, offset, columnMetadata, 1, expectedValues.length);

        expect(result).toBeInstanceOf(LongFlatVector);
        const resultVec = result as LongFlatVector;
        for (let i = 0; i < expectedValues.length; i++) {
            expect(resultVec.getValue(i)).toBe(expectedValues[i]);
        }
    });

    it("should decode INT_64 column with DELTA encoding", () => {
        const expectedValues = new BigInt64Array([2n, 4n, 6n]);
        const columnMetadata = createColumnMetadata("testColumn", ScalarType.INT_64, false);
        const encodedData = encodeInt64DeltaColumn(expectedValues);
        const offset = new IntWrapper(0);

        const result = decodePropertyColumn(encodedData, offset, columnMetadata, 1, expectedValues.length);

        expect(result).toBeInstanceOf(LongFlatVector);
        const resultVec = result as LongFlatVector;
        for (let i = 0; i < expectedValues.length; i++) {
            expect(resultVec.getValue(i)).toBe(expectedValues[i]);
        }
    });

    it("should decode INT_64 column with RLE encoding", () => {
        const expectedValues = new BigInt64Array([100n, 100n, 100n, -50n, -50n]);
        const columnMetadata = createColumnMetadata("testColumn", ScalarType.INT_64, false);
        const encodedData = encodeInt64RleColumn([
            [3, 100n],
            [2, -50n],
        ]);
        const offset = new IntWrapper(0);

        const result = decodePropertyColumn(encodedData, offset, columnMetadata, 1, expectedValues.length);

        expect(result).toBeInstanceOf(LongFlatVector);
        const resultVec = result as LongFlatVector;
        for (let i = 0; i < expectedValues.length; i++) {
            expect(resultVec.getValue(i)).toBe(expectedValues[i]);
        }
    });

    it("should decode INT_64 column with DELTA+RLE encoding", () => {
        const expectedValues = new BigInt64Array([10n, 12n, 14n, 15n, 16n]);
        const columnMetadata = createColumnMetadata("testColumn", ScalarType.INT_64, false);
        const encodedData = encodeInt64DeltaRleColumn([
            [1, 10n],
            [2, 2n],
            [2, 1n],
        ]);
        const offset = new IntWrapper(0);

        const result = decodePropertyColumn(encodedData, offset, columnMetadata, 1, expectedValues.length);

        expect(result).toBeInstanceOf(LongFlatVector);
        const resultVec = result as LongFlatVector;
        for (let i = 0; i < expectedValues.length; i++) {
            expect(resultVec.getValue(i)).toBe(expectedValues[i]);
        }
    });

    it("should decode nullable INT_64 column with null values", () => {
        const expectedValues = [2n, null, -4n, null, 6n];
        const columnMetadata = createColumnMetadata("testColumn", ScalarType.INT_64, true);
        const encodedData = encodeInt64NullableColumn(expectedValues);
        const offset = new IntWrapper(0);

        const result = decodePropertyColumn(encodedData, offset, columnMetadata, 2, expectedValues.length);

        expect(result).toBeInstanceOf(LongFlatVector);
        const resultVec = result as LongFlatVector;
        for (let i = 0; i < expectedValues.length; i++) {
            expect(resultVec.getValue(i)).toBe(expectedValues[i]);
        }
    });

    it("should decode INT_64 SEQUENCE vector", () => {
        const numValues = 5;
        const value = 10n;
        const columnMetadata = createColumnMetadata("testColumn", ScalarType.INT_64, false);
        const encodedData = encodeInt64DeltaRleColumn([[numValues, value]]);
        const offset = new IntWrapper(0);

        const result = decodePropertyColumn(encodedData, offset, columnMetadata, 1, numValues);

        expect(result).toBeInstanceOf(LongSequenceVector);
        const seqVec = result as LongSequenceVector;
        expect(seqVec.getValue(0)).toBe(value);
        expect(seqVec.getValue(1)).toBe(value + value);
        expect(seqVec.getValue(2)).toBe(value + value * 2n);
    });

    it("should decode INT_64 CONST vector", () => {
        const numValues = 5;
        const constValue = 42n;
        const columnMetadata = createColumnMetadata("testColumn", ScalarType.INT_64, false);
        const encodedData = encodeInt64RleColumn([[numValues, constValue]]);
        const offset = new IntWrapper(0);

        const result = decodePropertyColumn(encodedData, offset, columnMetadata, 1, numValues);

        expect(result).toBeInstanceOf(LongConstVector);
        const constVec = result as LongConstVector;
        expect(constVec.getValue(0)).toBe(constValue);
        expect(constVec.getValue(4)).toBe(constValue);
    });
});

describe("decodePropertyColumn - UINT_64", () => {
    it("should decode UINT_64 column with NONE encoding (unsigned)", () => {
        const expectedValues = new BigUint64Array([2n, 4n, 6n, 100n]);
        const columnMetadata = createColumnMetadata("testColumn", ScalarType.UINT_64, false);
        const encodedData = encodeUint64Column(expectedValues);
        const offset = new IntWrapper(0);

        const result = decodePropertyColumn(encodedData, offset, columnMetadata, 1, expectedValues.length);

        expect(result).toBeInstanceOf(LongFlatVector);
        const resultVec = result as LongFlatVector;
        for (let i = 0; i < expectedValues.length; i++) {
            expect(resultVec.getValue(i)).toBe(expectedValues[i]);
        }
    });

    it("should decode nullable UINT_64 column with null values", () => {
        const expectedValues = [2n, null, 4n, null, 6n];
        const columnMetadata = createColumnMetadata("testColumn", ScalarType.UINT_64, true);
        const encodedData = encodeUint64NullableColumn(expectedValues);
        const offset = new IntWrapper(0);

        const result = decodePropertyColumn(encodedData, offset, columnMetadata, 2, expectedValues.length);

        expect(result).toBeInstanceOf(LongFlatVector);
        const resultVec = result as LongFlatVector;
        for (let i = 0; i < expectedValues.length; i++) {
            expect(resultVec.getValue(i)).toBe(expectedValues[i]);
        }
    });
});

describe("decodePropertyColumn - FLOAT", () => {
    it("should decode non-nullable FLOAT column", () => {
        const expectedValues = new Float32Array([1.5, 2.7, -3.14, 4.2]);
        const columnMetadata = createColumnMetadata("testColumn", ScalarType.FLOAT, false);
        const encodedData = encodeFloatColumn(expectedValues);
        const offset = new IntWrapper(0);

        const result = decodePropertyColumn(encodedData, offset, columnMetadata, 1, expectedValues.length);

        expect(result).toBeInstanceOf(FloatFlatVector);
        const resultVec = result as FloatFlatVector;
        expect(resultVec.size).toBe(expectedValues.length);
        for (let i = 0; i < expectedValues.length; i++) {
            expect(resultVec.getValue(i)).toBeCloseTo(expectedValues[i], 5);
        }
    });

    it("should decode nullable FLOAT column with null values", () => {
        const expectedValues = [1.5, null, 2.7, null, 3.14];
        const columnMetadata = createColumnMetadata("testColumn", ScalarType.FLOAT, true);
        const encodedData = encodeFloatNullableColumn(expectedValues);
        const offset = new IntWrapper(0);

        const result = decodePropertyColumn(encodedData, offset, columnMetadata, 2, expectedValues.length);

        expect(result).toBeInstanceOf(FloatFlatVector);
        const resultVec = result as FloatFlatVector;
        expect(resultVec.size).toBe(expectedValues.length);
        expect(resultVec.getValue(0)).toBeCloseTo(1.5, 5);
        expect(resultVec.getValue(1)).toBe(null); // null value
        expect(resultVec.getValue(2)).toBeCloseTo(2.7, 5);
        expect(resultVec.getValue(3)).toBe(null); // null value
        expect(resultVec.getValue(4)).toBeCloseTo(3.14, 5);
    });

    it("should handle offset correctly after decoding FLOAT column", () => {
        const expectedValues = new Float32Array([1.0, 2.0, 3.0]);
        const columnMetadata = createColumnMetadata("testColumn", ScalarType.FLOAT, false);
        const encodedData = encodeFloatColumn(expectedValues);
        const offset = new IntWrapper(0);

        decodePropertyColumn(encodedData, offset, columnMetadata, 1, expectedValues.length);

        // Verify offset was advanced correctly
        expect(offset.get()).toBe(encodedData.length);
    });
});

describe("decodePropertyColumn - BOOLEAN", () => {
    it("should decode non-nullable BOOLEAN column with RLE", () => {
        const booleanValues = [true, false, true, true, false, false, false, true];
        const columnMetadata = createColumnMetadata("testColumn", ScalarType.BOOLEAN, false);
        const encodedData = encodeBooleanColumn(booleanValues);
        const offset = new IntWrapper(0);

        const result = decodePropertyColumn(encodedData, offset, columnMetadata, 1, booleanValues.length);

        expect(result).toBeInstanceOf(BooleanFlatVector);
        const boolVec = result as BooleanFlatVector;
        for (let i = 0; i < booleanValues.length; i++) {
            expect(boolVec.getValue(i)).toBe(booleanValues[i]);
        }
    });

    it("should decode nullable BOOLEAN column with RLE and present stream", () => {
        const expectedValues = [true, null, false, null, true];
        const columnMetadata = createColumnMetadata("testColumn", ScalarType.BOOLEAN, true);
        const encodedData = encodeBooleanNullableColumn(expectedValues);
        const offset = new IntWrapper(0);

        const result = decodePropertyColumn(encodedData, offset, columnMetadata, 2, expectedValues.length);

        expect(result).toBeInstanceOf(BooleanFlatVector);
        const boolVec = result as BooleanFlatVector;
        expect(boolVec.getValue(0)).toBe(true);
        expect(boolVec.getValue(1)).toBe(null);
        expect(boolVec.getValue(2)).toBe(false);
        expect(boolVec.getValue(3)).toBe(null);
        expect(boolVec.getValue(4)).toBe(true);
    });
});

describe("decodePropertyColumn - DOUBLE", () => {
    it("should decode non-nullable DOUBLE column", () => {
        const expectedValues = new Float32Array([1.2345, 5.4321, 1.33742]);
        const columnMetadata = createColumnMetadata("testColumn", ScalarType.DOUBLE, false);
        const encodedData = encodeDoubleColumn(expectedValues);
        const offset = new IntWrapper(0);

        const result = decodePropertyColumn(encodedData, offset, columnMetadata, 1, expectedValues.length);

        expect(result).toBeInstanceOf(DoubleFlatVector);
        const resultVec = result as DoubleFlatVector;
        expect(resultVec.size).toBe(expectedValues.length);
        for (let i = 0; i < expectedValues.length; i++) {
            expect(resultVec.getValue(i)).toBeCloseTo(expectedValues[i], 5);
        }
    });

    it("should decode nullable DOUBLE column with null values", () => {
        const expectedValues = [1.5, null, 2.7, null, 3.14159];
        const columnMetadata = createColumnMetadata("testColumn", ScalarType.DOUBLE, true);
        const encodedData = encodeDoubleNullableColumn(expectedValues);
        const offset = new IntWrapper(0);

        const result = decodePropertyColumn(encodedData, offset, columnMetadata, 2, expectedValues.length);

        expect(result).toBeInstanceOf(DoubleFlatVector);
        const resultVec = result as DoubleFlatVector;
        expect(resultVec.size).toBe(expectedValues.length);
        expect(resultVec.getValue(0)).toBeCloseTo(1.5, 5);
        expect(resultVec.getValue(1)).toBe(null); // null value
        expect(resultVec.getValue(2)).toBeCloseTo(2.7, 5);
        expect(resultVec.getValue(3)).toBe(null); // null value
        expect(resultVec.getValue(4)).toBeCloseTo(3.14159, 5);
    });

    it("should handle offset correctly after decoding DOUBLE column", () => {
        const expectedValues = new Float32Array([1.33742, 1.2345, 5.4321]);
        const columnMetadata = createColumnMetadata("testColumn", ScalarType.DOUBLE, false);
        const encodedData = encodeDoubleColumn(expectedValues);
        const offset = new IntWrapper(0);

        decodePropertyColumn(encodedData, offset, columnMetadata, 1, expectedValues.length);

        // Verify offset was advanced correctly
        expect(offset.get()).toBe(encodedData.length);
    });
});

describe("decodePropertyColumn - STRING", () => {
    describe("basic functionality", () => {
        it("should decode single field with shared dictionary", () => {
            const dictionaryStrings = ["apple", "banana", "peach", "date"];
            const { lengthStream, dataStream } = encodeSharedDictionary(dictionaryStrings);
            const fieldStreams = encodeStructField([0, 1, 2, 3], [true, true, true, true]);
            const completeData = concatenateBuffers(lengthStream, dataStream, fieldStreams);
            const columnMetadata = createColumnMetadataForStruct("address", [{ name: "street" }]);
            const offset = new IntWrapper(0);
            const result = decodePropertyColumn(completeData, offset, columnMetadata, 1, dictionaryStrings.length);

            expect(result).toHaveLength(1);
            expect(result[0]).toBeInstanceOf(StringDictionaryVector);
            expect(result[0].name).toBe("address:street");
            for (let i = 0; i < dictionaryStrings.length; i++) {
                expect(result[0].getValue(i)).toBe(dictionaryStrings[i]);
            }
        });

        it("should decode shared dictionary when numStreams matches encoder output (3 + N*2)", () => {
            const dictionaryStrings = ["apple", "banana", "peach", "date"];
            const { lengthStream, dataStream } = encodeSharedDictionary(dictionaryStrings);
            const fieldStreams = encodeStructField([0, 1, 2, 3], [true, true, true, true]);
            const completeData = concatenateBuffers(lengthStream, dataStream, fieldStreams);
            const columnMetadata = createColumnMetadataForStruct("address", [{ name: "street" }]);
            const offset = new IntWrapper(0);
            const result = decodePropertyColumn(completeData, offset, columnMetadata, 5, dictionaryStrings.length);

            expect(result).toHaveLength(1);
            expect(result[0]).toBeInstanceOf(StringDictionaryVector);
            expect(result[0].name).toBe("address:street");
            for (let i = 0; i < dictionaryStrings.length; i++) {
                expect(result[0].getValue(i)).toBe(dictionaryStrings[i]);
            }
        });
    });
});

describe("decodePropertyColumn - Edge Cases", () => {
    it("should filter columns with propertyColumnNames set", () => {
        const expectedValues = new Int32Array([1, 2, 3]);
        const columnMetadata = createColumnMetadata("includedColumn", ScalarType.INT_32, false);
        const encodedData = encodeInt32NoneColumn(expectedValues);
        const propertyColumnNames = new Set(["includedColumn"]);
        const offset = new IntWrapper(0);

        const result = decodePropertyColumn(
            encodedData,
            offset,
            columnMetadata,
            1,
            expectedValues.length,
            propertyColumnNames,
        );

        expect(result).toBeInstanceOf(IntFlatVector);
        const resultVec = result as IntFlatVector;
        for (let i = 0; i < expectedValues.length; i++) {
            expect(resultVec.getValue(i)).toBe(expectedValues[i]);
        }
    });

    it("should skip column when not in propertyColumnNames filter", () => {
        const expectedValues = new Int32Array([1, 2, 3]);
        const columnMetadata = createColumnMetadata("excludedColumn", ScalarType.INT_32, false);
        const encodedData = encodeInt32NoneColumn(expectedValues);
        const propertyColumnNames = new Set(["someOtherColumn"]);
        const offset = new IntWrapper(0);

        const result = decodePropertyColumn(
            encodedData,
            offset,
            columnMetadata,
            1,
            expectedValues.length,
            propertyColumnNames,
        );

        // Should return null and advance the offset past the skipped data
        expect(result).toBe(null);
        expect(offset.get()).toBe(encodedData.length);
    });

    it("should return null for empty columns (numStreams === 0)", () => {
        const columnMetadata = createColumnMetadata("emptyColumn", ScalarType.INT_32, false);
        const offset = new IntWrapper(0);
        const data = new Uint8Array(0);

        const result = decodePropertyColumn(data, offset, columnMetadata, 0, 0);

        expect(result).toBeNull();
    });

    it("should return null for complex type with numStreams === 0", () => {
        const columnMetadata = createColumnMetadataForStruct("structColumn", [
            { name: "field1" },
            { name: "field2" },
        ]);
        const offset = new IntWrapper(0);
        const data = new Uint8Array(0);

        const result = decodePropertyColumn(data, offset, columnMetadata, 0, 5);

        expect(result).toBeNull();
    });

    it("should throw error for unsupported data type", () => {
        const columnMetadata = createColumnMetadata("unsupportedColumn", ScalarType.INT_8, false);
        const encodedData = encodeInt32NoneColumn(new Int32Array([1, 2, 3]));
        const offset = new IntWrapper(0);

        expect(() => {
            decodePropertyColumn(encodedData, offset, columnMetadata, 1, 3);
        }).toThrow();
    });
});
