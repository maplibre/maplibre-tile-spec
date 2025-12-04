import { describe, it, expect } from "vitest";
import { decodePropertyColumn } from "./propertyDecoder";
import IntWrapper from "./intWrapper";
import { ScalarType, type Column } from "../metadata/tileset/tilesetMetadata";
import { LogicalLevelTechnique } from "../metadata/tile/logicalLevelTechnique";
import { IntFlatVector } from "../vector/flat/intFlatVector";
import { LongFlatVector } from "../vector/flat/longFlatVector";
import { FloatFlatVector } from "../vector/flat/floatFlatVector";
import { DoubleFlatVector } from "../vector/flat/doubleFlatVector";
import { BooleanFlatVector } from "../vector/flat/booleanFlatVector";
import { IntSequenceVector } from "../vector/sequence/intSequenceVector";
import { LongSequenceVector } from "../vector/sequence/longSequenceVector";
import { IntConstVector } from "../vector/constant/intConstVector";
import { LongConstVector } from "../vector/constant/longConstVector";
import {
    createStreamMetadata,
    createRleMetadata,
    buildEncodedStream,
    encodeVarintInt32Array,
    encodeVarintInt64Array,
    encodeZigZag32,
    encodeZigZag64,
    encodeFloatsLE,
    encodeBooleanRle,
    concatenateBuffers,
    encodeDoubleLE,
} from "./decodingTestUtils";

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
        const zigzagEncoded = new Int32Array(expectedValues.length);
        for (let i = 0; i < expectedValues.length; i++) {
            zigzagEncoded[i] = encodeZigZag32(expectedValues[i]);
        }
        const encodedData = encodeVarintInt32Array(zigzagEncoded);
        const streamMetadata = createStreamMetadata(
            LogicalLevelTechnique.NONE,
            LogicalLevelTechnique.NONE,
            expectedValues.length,
        );
        const fullStream = buildEncodedStream(streamMetadata, encodedData);
        const offset = new IntWrapper(0);

        const result = decodePropertyColumn(fullStream, offset, columnMetadata, 1, expectedValues.length);

        expect(result).toBeInstanceOf(IntFlatVector);
        const resultVec = result as IntFlatVector;
        for (let i = 0; i < expectedValues.length; i++) {
            expect(resultVec.getValue(i)).toBe(expectedValues[i]);
        }
    });

    it("should decode INT_32 column with DELTA encoding", () => {
        const expectedValues = new Int32Array([2, 4, 6]);
        const columnMetadata = createColumnMetadata("testColumn", ScalarType.INT_32, false);
        // Delta encode: store deltas
        const deltaEncoded = new Int32Array([2, 4 - 2, 6 - 4]);
        const zigzagEncoded = new Int32Array(deltaEncoded.length);
        for (let i = 0; i < deltaEncoded.length; i++) {
            zigzagEncoded[i] = encodeZigZag32(deltaEncoded[i]);
        }
        const encodedData = encodeVarintInt32Array(zigzagEncoded);
        const streamMetadata = createStreamMetadata(
            LogicalLevelTechnique.DELTA,
            LogicalLevelTechnique.NONE,
            expectedValues.length,
        );
        const fullStream = buildEncodedStream(streamMetadata, encodedData);
        const offset = new IntWrapper(0);

        const result = decodePropertyColumn(fullStream, offset, columnMetadata, 1, expectedValues.length);

        expect(result).toBeInstanceOf(IntFlatVector);
        const resultVec = result as IntFlatVector;
        for (let i = 0; i < expectedValues.length; i++) {
            expect(resultVec.getValue(i)).toBe(expectedValues[i]);
        }
    });

    it("should decode INT_32 column with RLE encoding", () => {
        const expectedValues = new Int32Array([100, 100, 100, -50, -50]);
        const columnMetadata = createColumnMetadata("testColumn", ScalarType.INT_32, false);
        const runs = 2;
        const rleValues = new Int32Array([3, 2, encodeZigZag32(100), encodeZigZag32(-50)]);
        const encodedData = encodeVarintInt32Array(rleValues);
        const streamMetadata = createRleMetadata(
            LogicalLevelTechnique.RLE,
            LogicalLevelTechnique.NONE,
            runs,
            expectedValues.length,
        );
        const fullStream = buildEncodedStream(streamMetadata, encodedData);
        const offset = new IntWrapper(0);

        const result = decodePropertyColumn(fullStream, offset, columnMetadata, 1, expectedValues.length);

        expect(result).toBeInstanceOf(IntFlatVector);
        const resultVec = result as IntFlatVector;
        for (let i = 0; i < expectedValues.length; i++) {
            expect(resultVec.getValue(i)).toBe(expectedValues[i]);
        }
    });

    it("should decode INT_32 column with DELTA+RLE encoding", () => {
        const expectedValues = new Int32Array([10, 12, 14, 15, 16]);
        const columnMetadata = createColumnMetadata("testColumn", ScalarType.INT_32, false);
        const runs = 3;
        const rleValues = new Int32Array([1, 2, 2, encodeZigZag32(10), encodeZigZag32(2), encodeZigZag32(1)]);
        const encodedData = encodeVarintInt32Array(rleValues);
        const streamMetadata = createRleMetadata(
            LogicalLevelTechnique.DELTA,
            LogicalLevelTechnique.RLE,
            runs,
            expectedValues.length,
        );
        const fullStream = buildEncodedStream(streamMetadata, encodedData);
        const offset = new IntWrapper(0);

        const result = decodePropertyColumn(fullStream, offset, columnMetadata, 1, expectedValues.length);

        expect(result).toBeInstanceOf(IntFlatVector);
        const resultVec = result as IntFlatVector;
        for (let i = 0; i < expectedValues.length; i++) {
            expect(resultVec.getValue(i)).toBe(expectedValues[i]);
        }
    });

    it("should decode nullable INT_32 column with null values", () => {
        const expectedValues = [2, null, -4, null, 6]; // positions 1 and 3 are null
        const columnMetadata = createColumnMetadata("testColumn", ScalarType.INT_32, true);
        // Only non-null values: [2, -4, 6]
        const nonNullValues = [2, -4, 6];
        const zigzagEncoded = new Int32Array(nonNullValues.map((v) => encodeZigZag32(v)));
        const encodedData = encodeVarintInt32Array(zigzagEncoded);
        const streamMetadata = createStreamMetadata(
            LogicalLevelTechnique.NONE,
            LogicalLevelTechnique.NONE,
            nonNullValues.length,
        );
        const dataStream = buildEncodedStream(streamMetadata, encodedData);
        // Nullability stream: positions 0, 2, 4 are non-null
        const nullabilityValues = [true, false, true, false, true];
        const nullabilityEncoded = encodeBooleanRle(nullabilityValues);
        const nullabilityMetadata = createStreamMetadata(
            LogicalLevelTechnique.NONE,
            LogicalLevelTechnique.NONE,
            nullabilityValues.length,
        );
        const nullabilityStream = buildEncodedStream(nullabilityMetadata, nullabilityEncoded);
        const fullData = concatenateBuffers(nullabilityStream, dataStream);
        const offset = new IntWrapper(0);

        const result = decodePropertyColumn(fullData, offset, columnMetadata, 2, expectedValues.length);

        expect(result).toBeInstanceOf(IntFlatVector);
        const resultVec = result as IntFlatVector;
        for (let i = 0; i < expectedValues.length; i++) {
            expect(resultVec.getValue(i)).toBe(expectedValues[i]);
        }
    });

    it("should decode INT_32 SEQUENCE vector", () => {
        const numValues = 5;
        const value = 10; // For single-run SEQUENCE, base and delta must be equal
        const columnMetadata = createColumnMetadata("testColumn", ScalarType.INT_32, false);
        // SEQUENCE: single run where base === delta (e.g., 10, 20, 30, 40, 50)
        const runs = 1;
        const rleValues = new Int32Array([numValues, encodeZigZag32(value)]);
        const encodedData = encodeVarintInt32Array(rleValues);
        const streamMetadata = createRleMetadata(
            LogicalLevelTechnique.DELTA,
            LogicalLevelTechnique.RLE,
            runs,
            numValues,
        );
        const fullStream = buildEncodedStream(streamMetadata, encodedData);
        const offset = new IntWrapper(0);

        const result = decodePropertyColumn(fullStream, offset, columnMetadata, 1, numValues);

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
        const runs = 1;
        const rleValues = new Int32Array([numValues, encodeZigZag32(constValue)]);
        const encodedData = encodeVarintInt32Array(rleValues);
        const streamMetadata = createRleMetadata(
            LogicalLevelTechnique.RLE,
            LogicalLevelTechnique.NONE,
            runs,
            numValues,
        );
        const fullStream = buildEncodedStream(streamMetadata, encodedData);
        const offset = new IntWrapper(0);

        const result = decodePropertyColumn(fullStream, offset, columnMetadata, 1, numValues);

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
        // No zigzag encoding for unsigned integers
        const encodedData = encodeVarintInt32Array(new Int32Array(expectedValues));
        const streamMetadata = createStreamMetadata(
            LogicalLevelTechnique.NONE,
            LogicalLevelTechnique.NONE,
            expectedValues.length,
        );
        const fullStream = buildEncodedStream(streamMetadata, encodedData);
        const offset = new IntWrapper(0);

        const result = decodePropertyColumn(fullStream, offset, columnMetadata, 1, expectedValues.length);

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
        const zigzagEncoded = new BigInt64Array(Array.from(expectedValues, (val) => encodeZigZag64(val)));
        const encodedData = encodeVarintInt64Array(zigzagEncoded);
        const streamMetadata = createStreamMetadata(
            LogicalLevelTechnique.NONE,
            LogicalLevelTechnique.NONE,
            expectedValues.length,
        );
        const fullStream = buildEncodedStream(streamMetadata, encodedData);
        const offset = new IntWrapper(0);

        const result = decodePropertyColumn(fullStream, offset, columnMetadata, 1, expectedValues.length);

        expect(result).toBeInstanceOf(LongFlatVector);
        const resultVec = result as LongFlatVector;
        for (let i = 0; i < expectedValues.length; i++) {
            expect(resultVec.getValue(i)).toBe(expectedValues[i]);
        }
    });

    it("should decode INT_64 column with DELTA encoding", () => {
        const expectedValues = new BigInt64Array([2n, 4n, 6n]);
        const columnMetadata = createColumnMetadata("testColumn", ScalarType.INT_64, false);
        const deltaEncoded = new BigInt64Array([2n, 4n - 2n, 6n - 4n]);
        const zigzagEncoded = new BigInt64Array(deltaEncoded.length);
        for (let i = 0; i < deltaEncoded.length; i++) {
            zigzagEncoded[i] = encodeZigZag64(deltaEncoded[i]);
        }
        const encodedData = encodeVarintInt64Array(zigzagEncoded);
        const streamMetadata = createStreamMetadata(
            LogicalLevelTechnique.DELTA,
            LogicalLevelTechnique.NONE,
            expectedValues.length,
        );
        const fullStream = buildEncodedStream(streamMetadata, encodedData);
        const offset = new IntWrapper(0);

        const result = decodePropertyColumn(fullStream, offset, columnMetadata, 1, expectedValues.length);

        expect(result).toBeInstanceOf(LongFlatVector);
        const resultVec = result as LongFlatVector;
        for (let i = 0; i < expectedValues.length; i++) {
            expect(resultVec.getValue(i)).toBe(expectedValues[i]);
        }
    });

    it("should decode INT_64 column with RLE encoding", () => {
        const expectedValues = new BigInt64Array([100n, 100n, 100n, -50n, -50n]);
        const columnMetadata = createColumnMetadata("testColumn", ScalarType.INT_64, false);
        const runs = 2;
        const rleValues = new BigInt64Array([3n, 2n, encodeZigZag64(100n), encodeZigZag64(-50n)]);
        const encodedData = encodeVarintInt64Array(rleValues);
        const streamMetadata = createRleMetadata(
            LogicalLevelTechnique.RLE,
            LogicalLevelTechnique.NONE,
            runs,
            expectedValues.length,
        );
        const fullStream = buildEncodedStream(streamMetadata, encodedData);
        const offset = new IntWrapper(0);

        const result = decodePropertyColumn(fullStream, offset, columnMetadata, 1, expectedValues.length);

        expect(result).toBeInstanceOf(LongFlatVector);
        const resultVec = result as LongFlatVector;
        for (let i = 0; i < expectedValues.length; i++) {
            expect(resultVec.getValue(i)).toBe(expectedValues[i]);
        }
    });

    it("should decode INT_64 column with DELTA+RLE encoding", () => {
        const expectedValues = new BigInt64Array([10n, 12n, 14n, 15n, 16n]);
        const columnMetadata = createColumnMetadata("testColumn", ScalarType.INT_64, false);
        const runs = 3;
        const rleValues = new BigInt64Array([1n, 2n, 2n, encodeZigZag64(10n), encodeZigZag64(2n), encodeZigZag64(1n)]);
        const encodedData = encodeVarintInt64Array(rleValues);
        const streamMetadata = createRleMetadata(
            LogicalLevelTechnique.DELTA,
            LogicalLevelTechnique.RLE,
            runs,
            expectedValues.length,
        );
        const fullStream = buildEncodedStream(streamMetadata, encodedData);
        const offset = new IntWrapper(0);

        const result = decodePropertyColumn(fullStream, offset, columnMetadata, 1, expectedValues.length);

        expect(result).toBeInstanceOf(LongFlatVector);
        const resultVec = result as LongFlatVector;
        for (let i = 0; i < expectedValues.length; i++) {
            expect(resultVec.getValue(i)).toBe(expectedValues[i]);
        }
    });

    it("should decode nullable INT_64 column with null values", () => {
        const expectedValues = [2n, null, -4n, null, 6n];
        const columnMetadata = createColumnMetadata("testColumn", ScalarType.INT_64, true);
        const nonNullValues = [2n, -4n, 6n];
        const zigzagEncoded = new BigInt64Array(nonNullValues.map((v) => encodeZigZag64(v)));
        const encodedData = encodeVarintInt64Array(zigzagEncoded);
        const streamMetadata = createStreamMetadata(
            LogicalLevelTechnique.NONE,
            LogicalLevelTechnique.NONE,
            nonNullValues.length,
        );
        const dataStream = buildEncodedStream(streamMetadata, encodedData);
        const nullabilityValues = [true, false, true, false, true];
        const nullabilityEncoded = encodeBooleanRle(nullabilityValues);
        const nullabilityMetadata = createStreamMetadata(
            LogicalLevelTechnique.NONE,
            LogicalLevelTechnique.NONE,
            nullabilityValues.length,
        );
        const nullabilityStream = buildEncodedStream(nullabilityMetadata, nullabilityEncoded);
        const fullData = concatenateBuffers(nullabilityStream, dataStream);
        const offset = new IntWrapper(0);

        const result = decodePropertyColumn(fullData, offset, columnMetadata, 2, expectedValues.length);

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
        // SEQUENCE: single run where base === delta (e.g., 10, 20, 30, 40, 50)
        const runs = 1;
        const rleValues = new BigInt64Array([BigInt(numValues), encodeZigZag64(value)]);
        const encodedData = encodeVarintInt64Array(rleValues);
        const streamMetadata = createRleMetadata(
            LogicalLevelTechnique.DELTA,
            LogicalLevelTechnique.RLE,
            runs,
            numValues,
        );
        const fullStream = buildEncodedStream(streamMetadata, encodedData);
        const offset = new IntWrapper(0);

        const result = decodePropertyColumn(fullStream, offset, columnMetadata, 1, numValues);

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
        const runs = 1;
        const rleValues = new BigInt64Array([BigInt(numValues), encodeZigZag64(constValue)]);
        const encodedData = encodeVarintInt64Array(rleValues);
        const streamMetadata = createRleMetadata(
            LogicalLevelTechnique.RLE,
            LogicalLevelTechnique.NONE,
            runs,
            numValues,
        );
        const fullStream = buildEncodedStream(streamMetadata, encodedData);
        const offset = new IntWrapper(0);

        const result = decodePropertyColumn(fullStream, offset, columnMetadata, 1, numValues);

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
        // No zigzag encoding for unsigned integers
        const encodedData = encodeVarintInt64Array(new BigInt64Array(expectedValues));
        const streamMetadata = createStreamMetadata(
            LogicalLevelTechnique.NONE,
            LogicalLevelTechnique.NONE,
            expectedValues.length,
        );
        const fullStream = buildEncodedStream(streamMetadata, encodedData);
        const offset = new IntWrapper(0);

        const result = decodePropertyColumn(fullStream, offset, columnMetadata, 1, expectedValues.length);

        expect(result).toBeInstanceOf(LongFlatVector);
        const resultVec = result as LongFlatVector;
        for (let i = 0; i < expectedValues.length; i++) {
            expect(resultVec.getValue(i)).toBe(expectedValues[i]);
        }
    });

    it("should decode nullable UINT_64 column with null values", () => {
        const expectedValues = [2n, null, 4n, null, 6n];
        const columnMetadata = createColumnMetadata("testColumn", ScalarType.UINT_64, true);
        const nonNullValues = [2n, 4n, 6n];
        const encodedData = encodeVarintInt64Array(new BigInt64Array(nonNullValues));
        const streamMetadata = createStreamMetadata(
            LogicalLevelTechnique.NONE,
            LogicalLevelTechnique.NONE,
            nonNullValues.length,
        );
        const dataStream = buildEncodedStream(streamMetadata, encodedData);
        const nullabilityValues = [true, false, true, false, true];
        const nullabilityEncoded = encodeBooleanRle(nullabilityValues);
        const nullabilityMetadata = createStreamMetadata(
            LogicalLevelTechnique.NONE,
            LogicalLevelTechnique.NONE,
            nullabilityValues.length,
        );
        const nullabilityStream = buildEncodedStream(nullabilityMetadata, nullabilityEncoded);
        const fullData = concatenateBuffers(nullabilityStream, dataStream);
        const offset = new IntWrapper(0);

        const result = decodePropertyColumn(fullData, offset, columnMetadata, 2, expectedValues.length);

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
        // Encode as little-endian floats
        const encodedData = encodeFloatsLE(expectedValues);
        const streamMetadata = createStreamMetadata(
            LogicalLevelTechnique.NONE,
            LogicalLevelTechnique.NONE,
            expectedValues.length,
        );
        const fullStream = buildEncodedStream(streamMetadata, encodedData);
        const offset = new IntWrapper(0);

        const result = decodePropertyColumn(fullStream, offset, columnMetadata, 1, expectedValues.length);

        expect(result).toBeInstanceOf(FloatFlatVector);
        const resultVec = result as FloatFlatVector;
        expect(resultVec.size).toBe(expectedValues.length);
        for (let i = 0; i < expectedValues.length; i++) {
            expect(resultVec.getValue(i)).toBeCloseTo(expectedValues[i], 5);
        }
    });

    it("should decode nullable FLOAT column with null values", () => {
        const columnMetadata = createColumnMetadata("testColumn", ScalarType.FLOAT, true);
        const nonNullValues = new Float32Array([1.5, 2.7, 3.14]);
        const encodedData = encodeFloatsLE(nonNullValues);
        const streamMetadata = createStreamMetadata(
            LogicalLevelTechnique.NONE,
            LogicalLevelTechnique.NONE,
            nonNullValues.length,
        );
        const dataStream = buildEncodedStream(streamMetadata, encodedData);
        const nullabilityValues = [true, false, true, false, true];
        const nullabilityEncoded = encodeBooleanRle(nullabilityValues);
        const nullabilityMetadata = createStreamMetadata(
            LogicalLevelTechnique.NONE,
            LogicalLevelTechnique.NONE,
            nullabilityValues.length,
        );
        const nullabilityStream = buildEncodedStream(nullabilityMetadata, nullabilityEncoded);
        const fullData = concatenateBuffers(nullabilityStream, dataStream);
        const offset = new IntWrapper(0);

        const result = decodePropertyColumn(fullData, offset, columnMetadata, 2, nullabilityValues.length);

        expect(result).toBeInstanceOf(FloatFlatVector);
        const resultVec = result as FloatFlatVector;
        expect(resultVec.size).toBe(nullabilityValues.length);
        expect(resultVec.getValue(0)).toBeCloseTo(1.5, 5);
        expect(resultVec.getValue(1)).toBe(null); // null value
        expect(resultVec.getValue(2)).toBeCloseTo(2.7, 5);
        expect(resultVec.getValue(3)).toBe(null); // null value
        expect(resultVec.getValue(4)).toBeCloseTo(3.14, 5);
    });

    it("should handle offset correctly after decoding FLOAT column", () => {
        const expectedValues = new Float32Array([1.0, 2.0, 3.0]);
        const columnMetadata = createColumnMetadata("testColumn", ScalarType.FLOAT, false);
        const encodedData = encodeFloatsLE(expectedValues);
        const streamMetadata = createStreamMetadata(
            LogicalLevelTechnique.NONE,
            LogicalLevelTechnique.NONE,
            expectedValues.length,
        );
        const fullStream = buildEncodedStream(streamMetadata, encodedData);
        const offset = new IntWrapper(0);

        decodePropertyColumn(fullStream, offset, columnMetadata, 1, expectedValues.length);

        // Verify offset was advanced correctly
        expect(offset.get()).toBe(fullStream.length);
    });
});

describe("decodePropertyColumn - BOOLEAN", () => {
    it("should decode non-nullable BOOLEAN column with RLE", () => {
        const booleanValues = [true, false, true, true, false, false, false, true];
        const columnMetadata = createColumnMetadata("testColumn", ScalarType.BOOLEAN, false);
        const encodedData = encodeBooleanRle(booleanValues);
        const streamMetadata = createStreamMetadata(
            LogicalLevelTechnique.NONE,
            LogicalLevelTechnique.NONE,
            booleanValues.length,
        );
        const fullStream = buildEncodedStream(streamMetadata, encodedData);
        const offset = new IntWrapper(0);

        const result = decodePropertyColumn(fullStream, offset, columnMetadata, 1, booleanValues.length);

        expect(result).toBeInstanceOf(BooleanFlatVector);
        const boolVec = result as BooleanFlatVector;
        for (let i = 0; i < booleanValues.length; i++) {
            expect(boolVec.getValue(i)).toBe(booleanValues[i]);
        }
    });

    it("should decode nullable BOOLEAN column with RLE and present stream", () => {
        const columnMetadata = createColumnMetadata("testColumn", ScalarType.BOOLEAN, true);
        const nonNullBooleanValues = [true, false, true];
        const encodedData = encodeBooleanRle(nonNullBooleanValues);
        const streamMetadata = createStreamMetadata(
            LogicalLevelTechnique.NONE,
            LogicalLevelTechnique.NONE,
            nonNullBooleanValues.length,
        );
        const dataStream = buildEncodedStream(streamMetadata, encodedData);
        const nullabilityValues = [true, false, true, false, true];
        const nullabilityEncoded = encodeBooleanRle(nullabilityValues);
        const nullabilityMetadata = createStreamMetadata(
            LogicalLevelTechnique.NONE,
            LogicalLevelTechnique.NONE,
            nullabilityValues.length,
        );
        const nullabilityStream = buildEncodedStream(nullabilityMetadata, nullabilityEncoded);
        const fullData = concatenateBuffers(nullabilityStream, dataStream);
        const offset = new IntWrapper(0);

        const result = decodePropertyColumn(fullData, offset, columnMetadata, 2, nullabilityValues.length);

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
        // Encode as little-endian double
        const encodedData = encodeDoubleLE(expectedValues);
        const streamMetadata = createStreamMetadata(
            LogicalLevelTechnique.NONE,
            LogicalLevelTechnique.NONE,
            expectedValues.length,
        );
        const fullStream = buildEncodedStream(streamMetadata, encodedData);
        const offset = new IntWrapper(0);

        const result = decodePropertyColumn(fullStream, offset, columnMetadata, 1, expectedValues.length);

        expect(result).toBeInstanceOf(DoubleFlatVector);
        const resultVec = result as DoubleFlatVector;
        expect(resultVec.size).toBe(expectedValues.length);
        for (let i = 0; i < expectedValues.length; i++) {
            expect(resultVec.getValue(i)).toBeCloseTo(expectedValues[i], 5);
        }
    });

    it("should decode nullable DOUBLE column with null values", () => {
        const columnMetadata = createColumnMetadata("testColumn", ScalarType.DOUBLE, true);
        const nonNullValues = new Float32Array([1.5, 2.7, 3.14159]);
        const encodedData = encodeDoubleLE(nonNullValues);
        const streamMetadata = createStreamMetadata(
            LogicalLevelTechnique.NONE,
            LogicalLevelTechnique.NONE,
            nonNullValues.length,
        );
        const dataStream = buildEncodedStream(streamMetadata, encodedData);
        const nullabilityValues = [true, false, true, false, true];
        const nullabilityEncoded = encodeBooleanRle(nullabilityValues);
        const nullabilityMetadata = createStreamMetadata(
            LogicalLevelTechnique.NONE,
            LogicalLevelTechnique.NONE,
            nullabilityValues.length,
        );
        const nullabilityStream = buildEncodedStream(nullabilityMetadata, nullabilityEncoded);
        const fullData = concatenateBuffers(nullabilityStream, dataStream);
        const offset = new IntWrapper(0);

        const result = decodePropertyColumn(fullData, offset, columnMetadata, 2, nullabilityValues.length);

        expect(result).toBeInstanceOf(DoubleFlatVector);
        const resultVec = result as DoubleFlatVector;
        expect(resultVec.size).toBe(nullabilityValues.length);
        expect(resultVec.getValue(0)).toBeCloseTo(1.5, 5);
        expect(resultVec.getValue(1)).toBe(null); // null value
        expect(resultVec.getValue(2)).toBeCloseTo(2.7, 5);
        expect(resultVec.getValue(3)).toBe(null); // null value
        expect(resultVec.getValue(4)).toBeCloseTo(3.14159, 5);
    });

    it("should handle offset correctly after decoding DOUBLE column", () => {
        const expectedValues = new Float32Array([1.33742, 1.2345, 5.4321]);
        const columnMetadata = createColumnMetadata("testColumn", ScalarType.DOUBLE, false);
        const encodedData = encodeDoubleLE(expectedValues);
        const streamMetadata = createStreamMetadata(
            LogicalLevelTechnique.NONE,
            LogicalLevelTechnique.NONE,
            expectedValues.length,
        );
        const fullStream = buildEncodedStream(streamMetadata, encodedData);
        const offset = new IntWrapper(0);

        decodePropertyColumn(fullStream, offset, columnMetadata, 1, expectedValues.length);

        // Verify offset was advanced correctly
        expect(offset.get()).toBe(fullStream.length);
    });
});

describe("decodePropertyColumn - Edge Cases", () => {
    it("should filter columns with propertyColumnNames set", () => {
        const expectedValues = new Int32Array([1, 2, 3]);
        const columnMetadata = createColumnMetadata("includedColumn", ScalarType.INT_32, false);
        const zigzagEncoded = new Int32Array(expectedValues.length);
        for (let i = 0; i < expectedValues.length; i++) {
            zigzagEncoded[i] = encodeZigZag32(expectedValues[i]);
        }
        const encodedData = encodeVarintInt32Array(zigzagEncoded);
        const streamMetadata = createStreamMetadata(
            LogicalLevelTechnique.NONE,
            LogicalLevelTechnique.NONE,
            expectedValues.length,
        );
        const fullStream = buildEncodedStream(streamMetadata, encodedData);
        const propertyColumnNames = new Set(["includedColumn"]);
        const offset = new IntWrapper(0);

        const result = decodePropertyColumn(
            fullStream,
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
        const zigzagEncoded = new Int32Array(expectedValues.length);
        for (let i = 0; i < expectedValues.length; i++) {
            zigzagEncoded[i] = encodeZigZag32(expectedValues[i]);
        }
        const encodedData = encodeVarintInt32Array(zigzagEncoded);
        const streamMetadata = createStreamMetadata(
            LogicalLevelTechnique.NONE,
            LogicalLevelTechnique.NONE,
            expectedValues.length,
        );
        const fullStream = buildEncodedStream(streamMetadata, encodedData);
        // Filter set does NOT include "excludedColumn", so it should be skipped
        const propertyColumnNames = new Set(["someOtherColumn"]);
        const offset = new IntWrapper(0);

        const result = decodePropertyColumn(
            fullStream,
            offset,
            columnMetadata,
            1,
            expectedValues.length,
            propertyColumnNames,
        );

        // Should return null and advance the offset past the skipped data
        expect(result).toBe(null);
        expect(offset.get()).toBe(fullStream.length);
    });

    it("should return null for empty columns (numStreams === 0)", () => {
        const columnMetadata = createColumnMetadata("emptyColumn", ScalarType.INT_32, false);
        const offset = new IntWrapper(0);
        const data = new Uint8Array(0);

        const result = decodePropertyColumn(data, offset, columnMetadata, 0, 0);

        expect(result).toBeNull();
    });

    it("should throw error for unsupported data type", () => {
        const columnMetadata = createColumnMetadata("unsupportedColumn", ScalarType.INT_8, false);
        const encodedData = new Uint8Array([1, 2, 3]);
        const streamMetadata = createStreamMetadata(LogicalLevelTechnique.NONE, LogicalLevelTechnique.NONE, 3);
        const fullStream = buildEncodedStream(streamMetadata, encodedData);
        const offset = new IntWrapper(0);

        expect(() => {
            decodePropertyColumn(fullStream, offset, columnMetadata, 1, 3);
        }).toThrow();
    });
});
