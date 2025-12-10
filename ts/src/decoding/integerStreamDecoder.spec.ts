import { describe, it, expect } from "vitest";
import {
    getVectorType,
    decodeIntStream,
    decodeFloat64,
    decodeLongStream,
    decodeNullableLongStream,
    decodeNullableIntStream,
} from "./integerStreamDecoder";
import { LogicalLevelTechnique } from "../metadata/tile/logicalLevelTechnique";
import { PhysicalLevelTechnique } from "../metadata/tile/physicalLevelTechnique";
import { VectorType } from "../vector/vectorType";
import IntWrapper from "./intWrapper";
import BitVector from "../vector/flat/bitVector";
import { createStreamMetadata, createRleMetadata } from "./decodingTestUtils";
import {
    encodeInt32Morton,
    encodeInt64SignedNone,
    encodeInt64SignedDelta,
    encodeInt64SignedRle,
    encodeInt64SignedDeltaRle,
    encodeInt64UnsignedNone,
    encodeIntStream,
    encodeFloat64,
} from "../encoding/integerStreamEncoder";

describe("getVectorType", () => {
    it("should return FLAT for RLE with 0 runs", () => {
        const metadata = createRleMetadata(LogicalLevelTechnique.RLE, LogicalLevelTechnique.RLE, 0, 0);
        const result = getVectorType(metadata, 0, new Uint8Array(), new IntWrapper(0));
        expect(result).toBe(VectorType.FLAT);
    });

    it("should return CONST for single run RLE", () => {
        const metadata = createRleMetadata(LogicalLevelTechnique.RLE, LogicalLevelTechnique.RLE, 1, 0);
        const result = getVectorType(metadata, 0, new Uint8Array(), new IntWrapper(0));
        expect(result).toBe(VectorType.CONST);
    });

    it("should return FLAT for NONE with 0 runs", () => {
        const metadata = createStreamMetadata(LogicalLevelTechnique.NONE, LogicalLevelTechnique.NONE, 0);
        const result = getVectorType(metadata, 0, new Uint8Array(), new IntWrapper(0));
        expect(result).toBe(VectorType.FLAT);
    });

    it("should return CONST for NONE with single run", () => {
        const metadata = createStreamMetadata(LogicalLevelTechnique.NONE, LogicalLevelTechnique.NONE, 1);
        const result = getVectorType(metadata, 0, new Uint8Array(), new IntWrapper(0));
        expect(result).toBe(VectorType.CONST);
    });

    it("should return FLAT for features and values mismatch", () => {
        const metadata = createStreamMetadata(LogicalLevelTechnique.DELTA, LogicalLevelTechnique.RLE, 1);
        const result = getVectorType(metadata, 2, new Uint8Array(), new IntWrapper(0));
        expect(result).toBe(VectorType.FLAT);
    });

    it("should return SEQUENCE for single RLE run", () => {
        const metadata = createRleMetadata(LogicalLevelTechnique.DELTA, LogicalLevelTechnique.RLE, 1, 1);
        const result = getVectorType(metadata, 1, new Uint8Array(), new IntWrapper(0));
        expect(result).toBe(VectorType.SEQUENCE);
    });

    it("should return SEQUENCE for RLE run with 2 runs", () => {
        const metadata = createRleMetadata(LogicalLevelTechnique.DELTA, LogicalLevelTechnique.RLE, 2, 5);
        const data = new Uint8Array([1, 4, 2, 2]); // Can't achieve this array using the encoding method...
        const result = getVectorType(metadata, 5, data, new IntWrapper(0));
        expect(result).toBe(VectorType.SEQUENCE);
    });
});

describe("decodeIntStream", () => {
    it("should decode with PhysicalLevelTechnique.NONE", () => {
        const expectedValues = new Int32Array([10, 20, 30]);
        const metadata = createStreamMetadata(LogicalLevelTechnique.NONE);
        const data = encodeIntStream(expectedValues, metadata, false);
        const result = decodeIntStream(data, new IntWrapper(0), metadata, false);
        expect(result).toEqual(expectedValues);
    });

    it("should throw for unsupported PhysicalLevelTechnique", () => {
        const data = new Uint8Array([10, 20, 30]);
        const metadata = {
            ...createStreamMetadata(LogicalLevelTechnique.NONE),
            physicalLevelTechnique: PhysicalLevelTechnique.ALP,
            numValues: 3,
            byteLength: 3,
        };
        expect(() => decodeIntStream(data, new IntWrapper(0), metadata, false)).toThrow(
            "Specified physicalLevelTechnique is not supported (yet).",
        );
    });

    it("should decode MORTON", () => {
        // Morton encoding uses delta encoding (sorted data, no zigzag needed)
        const expectedValues = new Int32Array([10, 15, 18, 20]);
        const metadata = createStreamMetadata(LogicalLevelTechnique.MORTON, LogicalLevelTechnique.NONE, 4);
        const data = encodeIntStream(expectedValues, metadata, false);
        const result = decodeIntStream(data, new IntWrapper(0), metadata, false);

        expect(result).toEqual(expectedValues);
    });

    it("should decode NONE signed with Int32", () => {
        const expectedValues = new Int32Array([2, -4, 6, -8]);
        const metadata = createStreamMetadata(
            LogicalLevelTechnique.NONE,
            LogicalLevelTechnique.NONE,
            expectedValues.length,
        );
        const data = encodeIntStream(expectedValues, metadata, true);
        const result = decodeIntStream(data, new IntWrapper(0), metadata, true);

        expect(result).toEqual(expectedValues);
    });

    it("should decode DELTA signed with Int32", () => {
        const expectedValues = new Int32Array([10, 12, 14, 16]);
        const metadata = createStreamMetadata(
            LogicalLevelTechnique.DELTA,
            LogicalLevelTechnique.NONE,
            expectedValues.length,
        );
        const data = encodeIntStream(expectedValues, metadata, true);
        const result = decodeIntStream(data, new IntWrapper(0), metadata, true);

        expect(result).toEqual(expectedValues);
    });

    it("should decode RLE signed with Int32", () => {
        const expectedValues = new Int32Array([100, 100, 100, -50, -50]);
        const runs = 2;
        const metadata = createRleMetadata(
            LogicalLevelTechnique.RLE,
            LogicalLevelTechnique.NONE,
            runs,
            expectedValues.length,
        );
        const data = encodeIntStream(expectedValues, metadata, true);
        const result = decodeIntStream(data, new IntWrapper(0), metadata, true);

        expect(result).toEqual(expectedValues);
    });
});

describe("decodeFloat64Buffer", () => {
    it("should decode NONE unsigned", () => {
        const metadata = createStreamMetadata(LogicalLevelTechnique.NONE);
        const expectedValues = new Float64Array([1.5, 2.5, 3.5]);
        const data = encodeFloat64(expectedValues, metadata, false);
        const result = decodeFloat64(data, metadata, false);

        expect(result).toEqual(expectedValues);
    });

    it("should decode NONE signed", () => {
        const metadata = createStreamMetadata(LogicalLevelTechnique.NONE);
        const expectedValues = new Float64Array([2, 5, 3]);
        const data = encodeFloat64(expectedValues, metadata, true);
        const result = decodeFloat64(data, metadata, true);

        expect(result).toEqual(expectedValues);
    });

    it("should decode RLE unsigned", () => {
        const expectedValues = new Float64Array([10.5, 10.5, 10.5, 20.5, 20.5]);
        const runs = 2;
        const metadata = createRleMetadata(
            LogicalLevelTechnique.RLE,
            LogicalLevelTechnique.NONE,
            runs,
            expectedValues.length,
        );
        const data = encodeFloat64(expectedValues, metadata, false);
        const result = decodeFloat64(data, metadata, false);

        expect(result).toEqual(expectedValues);
    });

    it("should decode RLE signed", () => {
        const expectedValues = new Float64Array([10, 10, 10, 20, 20]);
        const runs = 2;
        const metadata = createRleMetadata(
            LogicalLevelTechnique.RLE,
            LogicalLevelTechnique.NONE,
            runs,
            expectedValues.length,
        );
        const data = encodeFloat64(expectedValues, metadata, true);
        const result = decodeFloat64(data, metadata, true);

        expect(result).toEqual(expectedValues);
    });

    it("should decode DELTA without RLE", () => {
        const metadata = createStreamMetadata(LogicalLevelTechnique.DELTA);
        const expectedValues = new Float64Array([2, 4, 6]);
        const data = encodeFloat64(expectedValues, metadata, true);
        const result = decodeFloat64(data, metadata, true);

        expect(result).toEqual(expectedValues);
    });

    it("should decode DELTA with RLE", () => {
        const expectedValues = new Float64Array([10, 12, 14, 16, 18]);
        const runs = 2;
        const metadata = createRleMetadata(
            LogicalLevelTechnique.DELTA,
            LogicalLevelTechnique.RLE,
            runs,
            expectedValues.length,
        );
        const data = encodeFloat64(expectedValues, metadata, true);
        const result = decodeFloat64(data, metadata, true);

        expect(result).toEqual(new Float64Array([10, 12, 14, 16, 18]));
    });

    it("should throw for unsupported technique", () => {
        const metadata = createStreamMetadata(LogicalLevelTechnique.MORTON);
        const values = new Float64Array([1, 2, 3]);

        expect(() => decodeFloat64(values, metadata, true)).toThrow(
            "The specified Logical level technique is not supported: MORTON",
        );
    });
});

describe("decodeNullableIntStream", () => {
    it("should decode MORTON", () => {
        const metadata = createStreamMetadata(LogicalLevelTechnique.MORTON, LogicalLevelTechnique.NONE, 4);
        const expectedValues = new Int32Array([10, 15, 18, 20]);
        const data = encodeInt32Morton(expectedValues);
        const offset = new IntWrapper(0);
        const bitVector = new BitVector(new Uint8Array([0b00001111]), 4); // All non-null

        const result = decodeNullableIntStream(data, offset, metadata, false, bitVector);

        expect(result).toEqual(expectedValues);
    });

    it("should throw for unsupported technique", () => {
        const metadata = createStreamMetadata(LogicalLevelTechnique.PDE, LogicalLevelTechnique.NONE, 3);
        const offset = new IntWrapper(0);
        const bitVector = new BitVector(new Uint8Array([0b00000111]), 3);

        expect(() => decodeNullableIntStream(new Uint8Array([]), offset, metadata, false, bitVector)).toThrow(
            "The specified Logical level technique is not supported",
        );
    });
});

describe("decodeLongStream", () => {
    it("should decode DELTA with RLE", () => {
        const numRleValues = 5;
        const runs = 3;
        const metadata = createRleMetadata(LogicalLevelTechnique.DELTA, LogicalLevelTechnique.RLE, runs, numRleValues);
        const expectedValues = new BigInt64Array([10n, 12n, 14n, 15n, 16n]);
        const data = encodeInt64SignedDeltaRle([
            [1, 10n],
            [2, 2n],
            [2, 1n],
        ]);
        const offset = new IntWrapper(0);

        const result = decodeLongStream(data, offset, metadata, true);

        expect(result).toEqual(expectedValues);
    });

    it("should decode DELTA without RLE", () => {
        const metadata = createStreamMetadata(LogicalLevelTechnique.DELTA);
        const expectedValues = new BigInt64Array([2n, 4n, 6n]);
        const data = encodeInt64SignedDelta(expectedValues);
        const offset = new IntWrapper(0);

        const result = decodeLongStream(data, offset, metadata, true);

        expect(result).toEqual(expectedValues);
    });

    it("should decode RLE", () => {
        const numRleValues = 5;
        const runs = 2;
        const metadata = createRleMetadata(LogicalLevelTechnique.RLE, LogicalLevelTechnique.NONE, runs, numRleValues);
        const expectedValues = new BigInt64Array([100n, 100n, 100n, -50n, -50n]);
        const data = encodeInt64SignedRle([
            [3, 100n],
            [2, -50n],
        ]);
        const offset = new IntWrapper(0);

        const result = decodeLongStream(data, offset, metadata, true);

        expect(result).toEqual(expectedValues);
    });

    it("should decode NONE signed", () => {
        const metadata = createStreamMetadata(LogicalLevelTechnique.NONE);
        const expectedValues = new BigInt64Array([2n, -4n, 6n]);
        const data = encodeInt64SignedNone(expectedValues);
        const offset = new IntWrapper(0);

        const result = decodeLongStream(data, offset, metadata, true);

        expect(result).toEqual(expectedValues);
    });

    it("should decode NONE unsigned", () => {
        const metadata = createStreamMetadata(LogicalLevelTechnique.NONE);
        const expectedValues = new BigInt64Array([1n, 2n, 3n]);
        const data = encodeInt64UnsignedNone(expectedValues);
        const offset = new IntWrapper(0);

        const result = decodeLongStream(data, offset, metadata, false);

        expect(result).toEqual(expectedValues);
    });

    it("should throw for unsupported technique", () => {
        const metadata = createStreamMetadata(LogicalLevelTechnique.MORTON);
        const data = encodeInt64UnsignedNone(new BigInt64Array([1n, 2n, 3n]));
        const offset = new IntWrapper(0);
        expect(() => decodeLongStream(data, offset, metadata, true)).toThrow(
            "The specified Logical level technique is not supported: MORTON",
        );
    });
});
describe("decodeNullableLongStream", () => {
    it("should decode DELTA with RLE with all non-null values", () => {
        const numRleValues = 5;
        const runs = 3;
        const metadata = createRleMetadata(LogicalLevelTechnique.DELTA, LogicalLevelTechnique.RLE, runs, numRleValues);
        const expectedValues = new BigInt64Array([10n, 12n, 14n, 15n, 16n]);
        const data = encodeInt64SignedDeltaRle([
            [1, 10n],
            [2, 2n],
            [2, 1n],
        ]);
        const offset = new IntWrapper(0);
        const bitVector = new BitVector(new Uint8Array([0b00011111]), 5); // All non-null

        const result = decodeNullableLongStream(data, offset, metadata, true, bitVector);

        expect(result).toEqual(expectedValues);
    });

    it("should decode DELTA with RLE with null values", () => {
        const numRleValues = 3; // Only 3 non-null values
        const runs = 2;
        const metadata = createRleMetadata(LogicalLevelTechnique.DELTA, LogicalLevelTechnique.RLE, runs, numRleValues);
        const expectedValues = new BigInt64Array([10n, 10n, 12n, 12n, 14n]); // null values repeat previous value
        // Encode only non-null values: [10n, 12n, 14n]
        const data = encodeInt64SignedDeltaRle([
            [1, 10n],
            [2, 2n],
        ]);
        const offset = new IntWrapper(0);
        const bitVector = new BitVector(new Uint8Array([0b00010101]), 5); // positions 0, 2, 4 are non-null

        const result = decodeNullableLongStream(data, offset, metadata, true, bitVector);

        expect(result).toEqual(expectedValues);
    });

    it("should decode DELTA without RLE with all non-null values", () => {
        const metadata = createStreamMetadata(LogicalLevelTechnique.DELTA);
        const expectedValues = new BigInt64Array([2n, 4n, 6n]);
        const data = encodeInt64SignedDelta(expectedValues);
        const offset = new IntWrapper(0);
        const bitVector = new BitVector(new Uint8Array([0b00000111]), 3); // All non-null

        const result = decodeNullableLongStream(data, offset, metadata, true, bitVector);

        expect(result).toEqual(expectedValues);
    });

    it("should decode DELTA without RLE with null values", () => {
        const metadata = createStreamMetadata(LogicalLevelTechnique.DELTA, LogicalLevelTechnique.NONE, 5);
        const expectedValues = new BigInt64Array([0n, 2n, 2n, 4n, 6n]); // null values repeat previous value
        // Encode only non-null values: [2n, 4n, 6n]
        const nonNullValues = new BigInt64Array([2n, 4n, 6n]);
        const data = encodeInt64SignedDelta(nonNullValues);
        const offset = new IntWrapper(0);
        const bitVector = new BitVector(new Uint8Array([0b00011010]), 5); // positions 1, 3, 4 are non-null

        const result = decodeNullableLongStream(data, offset, metadata, true, bitVector);

        expect(result).toEqual(expectedValues);
    });

    it("should decode RLE with all non-null values", () => {
        const numRleValues = 5;
        const runs = 2;
        const metadata = createRleMetadata(LogicalLevelTechnique.RLE, LogicalLevelTechnique.NONE, runs, numRleValues);
        const expectedValues = new BigInt64Array([100n, 100n, 100n, -50n, -50n]);
        const data = encodeInt64SignedRle([
            [3, 100n],
            [2, -50n],
        ]);
        const offset = new IntWrapper(0);
        const bitVector = new BitVector(new Uint8Array([0b00011111]), 5); // All non-null

        const result = decodeNullableLongStream(data, offset, metadata, true, bitVector);

        expect(result).toEqual(expectedValues);
    });

    it("should decode RLE with null values", () => {
        const numRleValues = 3;
        const runs = 2;
        const metadata = createRleMetadata(LogicalLevelTechnique.RLE, LogicalLevelTechnique.NONE, runs, numRleValues);
        const expectedValues = new BigInt64Array([100n, 0n, 100n, 0n, -50n]); // null values become 0n
        // Encode only non-null values: [100n, 100n, -50n]
        const data = encodeInt64SignedRle([
            [2, 100n],
            [1, -50n],
        ]);
        const offset = new IntWrapper(0);
        const bitVector = new BitVector(new Uint8Array([0b00010101]), 5); // positions 0, 2, 4 are non-null

        const result = decodeNullableLongStream(data, offset, metadata, true, bitVector);

        expect(result).toEqual(expectedValues);
    });

    it("should decode NONE signed with all non-null values", () => {
        const metadata = createStreamMetadata(LogicalLevelTechnique.NONE);
        const expectedValues = new BigInt64Array([2n, -4n, 6n]);
        const data = encodeInt64SignedNone(expectedValues);
        const offset = new IntWrapper(0);
        const bitVector = new BitVector(new Uint8Array([0b00000111]), 3); // All non-null

        const result = decodeNullableLongStream(data, offset, metadata, true, bitVector);

        expect(result).toEqual(expectedValues);
    });

    it("should decode NONE signed with null values", () => {
        const metadata = createStreamMetadata(LogicalLevelTechnique.NONE, LogicalLevelTechnique.NONE, 5);
        const expectedValues = new BigInt64Array([2n, 0n, -4n, 0n, 6n]); // null values become 0n
        // Encode only non-null values: [2n, -4n, 6n]
        const nonNullValues = new BigInt64Array([2n, -4n, 6n]);
        const data = encodeInt64SignedNone(nonNullValues);
        const offset = new IntWrapper(0);
        const bitVector = new BitVector(new Uint8Array([0b00010101]), 5); // positions 0, 2, 4 are non-null

        const result = decodeNullableLongStream(data, offset, metadata, true, bitVector);

        expect(result).toEqual(expectedValues);
    });

    it("should decode NONE unsigned with all non-null values", () => {
        const metadata = createStreamMetadata(LogicalLevelTechnique.NONE);
        const expectedValues = new BigInt64Array([1n, 2n, 3n]);
        const data = encodeInt64UnsignedNone(expectedValues);
        const offset = new IntWrapper(0);
        const bitVector = new BitVector(new Uint8Array([0b00000111]), 3); // All non-null

        const result = decodeNullableLongStream(data, offset, metadata, false, bitVector);

        expect(result).toEqual(expectedValues);
    });

    it("should decode NONE unsigned with null values", () => {
        const metadata = createStreamMetadata(LogicalLevelTechnique.NONE, LogicalLevelTechnique.NONE, 5);
        const expectedValues = new BigInt64Array([0n, 1n, 2n, 0n, 3n]); // null values become 0n
        const nonNullValues = new BigInt64Array([1n, 2n, 3n]);
        const data = encodeInt64UnsignedNone(nonNullValues);
        const offset = new IntWrapper(0);
        const bitVector = new BitVector(new Uint8Array([0b00010110]), 5); // positions 1, 2, 4 are non-null

        const result = decodeNullableLongStream(data, offset, metadata, false, bitVector);

        expect(result).toEqual(expectedValues);
    });

    it("should throw for unsupported technique", () => {
        const metadata = createStreamMetadata(LogicalLevelTechnique.COMPONENTWISE_DELTA);
        const values = new BigInt64Array([1n, 2n, 3n]);
        const data = encodeInt64UnsignedNone(values);
        const offset = new IntWrapper(0);
        const bitVector = new BitVector(new Uint8Array([0b00000111]), 3);
        expect(() => decodeNullableLongStream(data, offset, metadata, true, bitVector)).toThrow();
    });
});
