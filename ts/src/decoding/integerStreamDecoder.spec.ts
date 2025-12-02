import { describe, it, expect } from "vitest";
import { getVectorType, decodeLongStream, decodeNullableLongStream } from "./integerStreamDecoder";
import { PhysicalStreamType } from "../metadata/tile/physicalStreamType";
import { LogicalStreamType } from "../metadata/tile/logicalStreamType";
import { LogicalLevelTechnique } from "../metadata/tile/logicalLevelTechnique";
import { PhysicalLevelTechnique } from "../metadata/tile/physicalLevelTechnique";
import { VectorType } from "../vector/vectorType";
import { DictionaryType } from "../metadata/tile/dictionaryType";
import IntWrapper from "./intWrapper";
import BitVector from "../vector/flat/bitVector";
import { type RleEncodedStreamMetadata } from "../metadata/tile/rleEncodedStreamMetadata";
import { type StreamMetadata } from "../metadata/tile/streamMetadata";

/**
 * Helper function to create StreamMetadata
 */
function createStreamMetadata(
    logicalTechnique1: LogicalLevelTechnique,
    logicalTechnique2: LogicalLevelTechnique = LogicalLevelTechnique.NONE,
    numValues: number = 3,
): StreamMetadata {
    return {
        physicalStreamType: PhysicalStreamType.DATA,
        logicalStreamType: new LogicalStreamType(DictionaryType.NONE),
        logicalLevelTechnique1: logicalTechnique1,
        logicalLevelTechnique2: logicalTechnique2,
        physicalLevelTechnique: PhysicalLevelTechnique.VARINT,
        numValues,
        byteLength: 10,
        decompressedCount: numValues,
    };
}

/**
 * Helper function to create RleEncodedStreamMetadata
 */
function createRleMetadata(
    logicalTechnique1: LogicalLevelTechnique,
    logicalTechnique2: LogicalLevelTechnique,
    runs: number,
    numRleValues: number,
): RleEncodedStreamMetadata {
    return {
        physicalStreamType: PhysicalStreamType.DATA,
        logicalStreamType: new LogicalStreamType(DictionaryType.NONE),
        logicalLevelTechnique1: logicalTechnique1,
        logicalLevelTechnique2: logicalTechnique2,
        physicalLevelTechnique: PhysicalLevelTechnique.VARINT,
        numValues: runs * 2,
        byteLength: 10,
        decompressedCount: numRleValues,
        runs,
        numRleValues,
    };
}

describe("getVectorType", () => {
    it("Delta-RLE with single run should return SEQUENCE for 1 run", () => {
        const metadata = createRleMetadata(LogicalLevelTechnique.DELTA, LogicalLevelTechnique.RLE, 1, 5);
        const data = new Uint8Array([5, 2]);
        const offset = new IntWrapper(0);
        const result = getVectorType(metadata, 5, data, offset);
        expect(result).toBe(VectorType.SEQUENCE);
    });

    it("Delta-RLE with 2 runs should return SEQUENCE when both deltas equal 1 (zigzag=2)", () => {
        const metadata = createRleMetadata(LogicalLevelTechnique.DELTA, LogicalLevelTechnique.RLE, 2, 5);
        const data = new Uint8Array([1, 4, 2, 2]);
        const offset = new IntWrapper(0);
        const result = getVectorType(metadata, 5, data, offset);
        expect(result).toBe(VectorType.SEQUENCE);
    });
});

describe("decodeLongStream", () => {
    it("should decode DELTA with RLE", () => {
        const metadata = createRleMetadata(LogicalLevelTechnique.DELTA, LogicalLevelTechnique.RLE, 2, 5);
        const values = new BigInt64Array([3n, 2n, 0n, 2n]);
        const data = new Uint8Array(values.buffer);
        const offset = new IntWrapper(0);
        const result = decodeLongStream(data, offset, metadata, true);
        expect(result).toBeInstanceOf(BigInt64Array);
    });

    it.skip("should decode DELTA without RLE", () => {
        const metadata = createStreamMetadata(LogicalLevelTechnique.DELTA);
        const values = new BigInt64Array([2n, 4n, 6n]);
        const data = new Uint8Array(values.buffer);
        const offset = new IntWrapper(0);
        const result = decodeLongStream(data, offset, metadata, true);
        expect(result).toBe(values);
    });

    it("should decode RLE", () => {
        const metadata = createRleMetadata(LogicalLevelTechnique.RLE, LogicalLevelTechnique.NONE, 2, 5);
        const values = new BigInt64Array([3n, 2n, 2n, 4n]);
        const data = new Uint8Array(values.buffer);
        const offset = new IntWrapper(0);
        const result = decodeLongStream(data, offset, metadata, true);
        expect(result).toBeInstanceOf(BigInt64Array);
    });

    it.skip("should decode NONE signed", () => {
        const metadata = createStreamMetadata(LogicalLevelTechnique.NONE);
        const values = new BigInt64Array([2n, 4n, 6n]);
        const data = new Uint8Array(values.buffer);
        const offset = new IntWrapper(0);
        const result = decodeLongStream(data, offset, metadata, true);
        expect(result).toBe(values);
    });

    it.skip("should decode NONE unsigned", () => {
        const metadata = createStreamMetadata(LogicalLevelTechnique.NONE);
        const values = new BigInt64Array([1n, 2n, 3n]);
        const data = new Uint8Array(values.buffer);
        const offset = new IntWrapper(0);
        const result = decodeLongStream(data, offset, metadata, false);
        expect(result).toBe(values);
    });

    it("should throw for unsupported technique", () => {
        const metadata = createStreamMetadata(LogicalLevelTechnique.MORTON);
        const values = new BigInt64Array([1n, 2n, 3n]);
        const data = new Uint8Array(values.buffer);
        const offset = new IntWrapper(0);
        expect(() => decodeLongStream(data, offset, metadata, true)).toThrow();
    });
});
describe("decodeNullableLongStream", () => {
    it("should decode DELTA with RLE", () => {
        const metadata = createRleMetadata(LogicalLevelTechnique.DELTA, LogicalLevelTechnique.RLE, 2, 3);
        const values = new BigInt64Array([2n, 1n, 0n, 2n]);
        const data = new Uint8Array(values.buffer);
        const offset = new IntWrapper(0);
        const bitVector = new BitVector(new Uint8Array([0b00000111]), 5);
        const result = decodeNullableLongStream(data, offset, metadata, true, bitVector);
        expect(result).toBeInstanceOf(BigInt64Array);
    });

    it("should decode DELTA without RLE", () => {
        const metadata = createStreamMetadata(LogicalLevelTechnique.DELTA);
        const values = new BigInt64Array([2n, 4n, 6n]);
        const data = new Uint8Array(values.buffer);
        const offset = new IntWrapper(0);
        const bitVector = new BitVector(new Uint8Array([0b00000111]), 3);
        const result = decodeNullableLongStream(data, offset, metadata, true, bitVector);
        expect(result).toBeInstanceOf(BigInt64Array);
    });

    it("should decode RLE", () => {
        const metadata = createRleMetadata(LogicalLevelTechnique.RLE, LogicalLevelTechnique.NONE, 2, 3);
        const values = new BigInt64Array([2n, 1n, 2n, 4n]);
        const data = new Uint8Array(values.buffer);
        const offset = new IntWrapper(0);
        const bitVector = new BitVector(new Uint8Array([0b00000111]), 5);
        const result = decodeNullableLongStream(data, offset, metadata, true, bitVector);
        expect(result).toBeInstanceOf(BigInt64Array);
    });

    it("should decode NONE signed", () => {
        const metadata = createStreamMetadata(LogicalLevelTechnique.NONE);
        const values = new BigInt64Array([2n, 4n, 6n]);
        const data = new Uint8Array(values.buffer);
        const offset = new IntWrapper(0);
        const bitVector = new BitVector(new Uint8Array([0b00000111]), 3);
        const result = decodeNullableLongStream(data, offset, metadata, true, bitVector);
        expect(result).toBeInstanceOf(BigInt64Array);
    });

    it("should decode NONE unsigned", () => {
        const metadata = createStreamMetadata(LogicalLevelTechnique.NONE);
        const values = new BigInt64Array([1n, 2n, 3n]);
        const data = new Uint8Array(values.buffer);
        const offset = new IntWrapper(0);
        const bitVector = new BitVector(new Uint8Array([0b00000111]), 3);
        const result = decodeNullableLongStream(data, offset, metadata, false, bitVector);
        expect(result).toBeInstanceOf(BigInt64Array);
    });

    it("should throw for unsupported technique", () => {
        const metadata = createStreamMetadata(LogicalLevelTechnique.COMPONENTWISE_DELTA);
        const values = new BigInt64Array([1n, 2n, 3n]);
        const data = new Uint8Array(values.buffer);
        const offset = new IntWrapper(0);
        const bitVector = new BitVector(new Uint8Array([0b00000111]), 3);
        expect(() => decodeNullableLongStream(data, offset, metadata, true, bitVector)).toThrow();
    });
});
