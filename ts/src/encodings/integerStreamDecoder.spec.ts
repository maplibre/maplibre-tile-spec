import {describe, it, expect} from "vitest";
import IntegerStreamDecoder from "./integerStreamDecoder";
import {RleEncodedStreamMetadata} from "../metadata/tile/rleEncodedStreamMetadata";
import {PhysicalStreamType} from "../metadata/tile/physicalStreamType";
import {LogicalStreamType} from "../metadata/tile/logicalStreamType";
import {LogicalLevelTechnique} from "../metadata/tile/logicalLevelTechnique";
import {PhysicalLevelTechnique} from "../metadata/tile/physicalLevelTechnique";
import {VectorType} from "../vector/vectorType";
import {DictionaryType} from "../metadata/tile/dictionaryType";
import IntWrapper from "./intWrapper";

/**
 * Helper function to create RleEncodedStreamMetadata for Delta-RLE tests with VARINT encoding
 */
function createDeltaRleMetadata(runs: number, numRleValues: number): RleEncodedStreamMetadata {
    return new RleEncodedStreamMetadata(
        PhysicalStreamType.DATA,
        new LogicalStreamType(DictionaryType.NONE),
        LogicalLevelTechnique.DELTA,
        LogicalLevelTechnique.RLE,
        PhysicalLevelTechnique.VARINT,
        runs * 2, // numValues: runs + deltas (both varint encoded)
        10, // byteLength
        runs,
        numRleValues,
    );
}

describe("IntegerStreamDecoder.getVectorType", () => {
    describe("Delta-RLE with single run", () => {
        it("should return SEQUENCE for 1 run", () => {
            const metadata = createDeltaRleMetadata(1, 5);
            const data = new Uint8Array([5, 2]);
            const offset = new IntWrapper(0);
            const result = IntegerStreamDecoder.getVectorType(metadata, 5, data, offset);
            expect(result).toBe(VectorType.SEQUENCE);
        });
    });

    describe("Delta-RLE with 2 runs", () => {
        it("should return SEQUENCE when both deltas equal 1 (zigzag=2)", () => {
            // Sequence [2,3,4,5,6] encodes as [1, 4, 2, 2]
            const metadata = createDeltaRleMetadata(2, 5);
            const data = new Uint8Array([1, 4, 2, 2]);
            const offset = new IntWrapper(0);
            const result = IntegerStreamDecoder.getVectorType(metadata, 5, data, offset);
            expect(result).toBe(VectorType.SEQUENCE);
        });
    });

    describe("Delta-RLE with null values", () => {
        it("should return FLAT when numRleValues != numFeatures", () => {
            const metadata = createDeltaRleMetadata(1, 5);
            const data = new Uint8Array([5, 2]);
            const offset = new IntWrapper(0);
            const result = IntegerStreamDecoder.getVectorType(metadata, 10, data, offset);
            expect(result).toBe(VectorType.FLAT);
        });
    });
});
