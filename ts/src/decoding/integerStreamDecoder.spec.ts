import { describe, it, expect } from "vitest";
import { getVectorType, decodeLongStream, decodeNullableLongStream } from "./integerStreamDecoder";
import { RleEncodedStreamMetadata } from "../metadata/tile/rleEncodedStreamMetadata";
import { PhysicalStreamType } from "../metadata/tile/physicalStreamType";
import { LogicalStreamType } from "../metadata/tile/logicalStreamType";
import { LogicalLevelTechnique } from "../metadata/tile/logicalLevelTechnique";
import { PhysicalLevelTechnique } from "../metadata/tile/physicalLevelTechnique";
import { VectorType } from "../vector/vectorType";
import { DictionaryType } from "../metadata/tile/dictionaryType";
import IntWrapper from "./intWrapper";
import BitVector from "../vector/flat/bitVector";
import { StreamMetadata } from "../metadata/tile/streamMetadata";

/**
 * Helper function to create StreamMetadata
 */
function createStreamMetadata(
    logicalTechnique1: LogicalLevelTechnique,
    logicalTechnique2: LogicalLevelTechnique = LogicalLevelTechnique.NONE,
    numValues: number = 3,
): StreamMetadata {
    return new StreamMetadata(
        PhysicalStreamType.DATA,
        new LogicalStreamType(DictionaryType.NONE),
        logicalTechnique1,
        logicalTechnique2,
        PhysicalLevelTechnique.VARINT,
        numValues,
        10,
    );
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
    return new RleEncodedStreamMetadata(
        PhysicalStreamType.DATA,
        new LogicalStreamType(DictionaryType.NONE),
        logicalTechnique1,
        logicalTechnique2,
        PhysicalLevelTechnique.VARINT,
        runs * 2,
        10,
        runs,
        numRleValues,
    );
}

function encodeSingleVarintInt64(value: bigint, dst: Uint8Array, offset: IntWrapper): void {
    let v = value;
    while (v > 0x7fn) {
        dst[offset.get()] = Number(v & 0x7fn) | 0x80;
        offset.increment();
        v >>= 7n;
    }
    dst[offset.get()] = Number(v & 0x7fn);
    offset.increment();
}

function encodeVarintInt64Array(values: BigInt64Array): Uint8Array {
    const buffer = new Uint8Array(values.length * 10);
    const offset = new IntWrapper(0);

    for (const value of values) {
        encodeSingleVarintInt64(value, buffer, offset);
    }
    return buffer.slice(0, offset.get());
}

function encodeZigZag(value: bigint): bigint {
    return (value << 1n) ^ (value >> 63n);
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
        const numRleValues = 5;
        const runs = 3;
        const metadata = createRleMetadata(LogicalLevelTechnique.DELTA, LogicalLevelTechnique.RLE, runs, numRleValues);
        const expectedValues = new BigInt64Array([10n, 12n, 14n, 15n, 16n]);
        const rleValues = new BigInt64Array([1n, 2n, 2n, encodeZigZag(10n), encodeZigZag(2n), encodeZigZag(1n)]);
        const data = encodeVarintInt64Array(rleValues);
        const offset = new IntWrapper(0);

        const result = decodeLongStream(data, offset, metadata, true);

        expect(result).toEqual(expectedValues);
    });

    it("should decode DELTA without RLE", () => {
        const metadata = createStreamMetadata(LogicalLevelTechnique.DELTA);
        const expectedValues = new BigInt64Array([2n, 4n, 6n]);
        const deltaEncoded = new BigInt64Array([2n, 4n - 2n, 6n - 4n,]);
        const zigzagEncoded = new BigInt64Array(deltaEncoded.length);
        for (let i = 0; i < deltaEncoded.length; i++) {
            zigzagEncoded[i] = encodeZigZag(deltaEncoded[i]);
        }
        const data = encodeVarintInt64Array(zigzagEncoded);
        const offset = new IntWrapper(0);

        const result = decodeLongStream(data, offset, metadata, true);

        expect(result).toEqual(expectedValues);
    });

    it("should decode RLE", () => {
        const numRleValues = 5;
        const runs = 2;
        const metadata = createRleMetadata(LogicalLevelTechnique.RLE, LogicalLevelTechnique.NONE, runs, numRleValues);
        const expectedValues = new BigInt64Array([100n, 100n, 100n, -50n, -50n]);
        const rleValues = new BigInt64Array([3n, 2n, encodeZigZag(100n), encodeZigZag(-50n),
        ]);
        const data = encodeVarintInt64Array(rleValues);
        const offset = new IntWrapper(0);

        const result = decodeLongStream(data, offset, metadata, true);

        expect(result).toEqual(expectedValues);
    });

    it("should decode NONE signed", () => {
        const metadata = createStreamMetadata(LogicalLevelTechnique.NONE);
        const expectedValues = new BigInt64Array([2n, -4n, 6n]);
        const zigzagEncoded = new BigInt64Array(Array.from(expectedValues, (val) => encodeZigZag(val)));
        const data = encodeVarintInt64Array(zigzagEncoded);
        const offset = new IntWrapper(0);

        const result = decodeLongStream(data, offset, metadata, true);

        expect(result).toEqual(expectedValues);
    });

    it("should decode NONE unsigned", () => {
        const metadata = createStreamMetadata(LogicalLevelTechnique.NONE);
        const expectedValues = new BigInt64Array([1n, 2n, 3n]);
        // Unsigned values are not zigzag encoded, just varint encoded
        const data = encodeVarintInt64Array(expectedValues);
        const offset = new IntWrapper(0);

        const result = decodeLongStream(data, offset, metadata, false);

        expect(result).toEqual(expectedValues);
    });

    it("should throw for unsupported technique", () => {
        const metadata = createStreamMetadata(LogicalLevelTechnique.MORTON);
        const data = encodeVarintInt64Array(new BigInt64Array([1n, 2n, 3n]));
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
        const rleValues = new BigInt64Array([1n, 2n, 2n, encodeZigZag(10n), encodeZigZag(2n), encodeZigZag(1n)]);
        const data = encodeVarintInt64Array(rleValues);
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
        const rleValues = new BigInt64Array([1n, 2n, encodeZigZag(10n), encodeZigZag(2n)]);
        const data = encodeVarintInt64Array(rleValues);
        const offset = new IntWrapper(0);
        const bitVector = new BitVector(new Uint8Array([0b00010101]), 5); // positions 0, 2, 4 are non-null

        const result = decodeNullableLongStream(data, offset, metadata, true, bitVector);

        expect(result).toEqual(expectedValues);
    });

    it("should decode DELTA without RLE with all non-null values", () => {
        const metadata = createStreamMetadata(LogicalLevelTechnique.DELTA);
        const expectedValues = new BigInt64Array([2n, 4n, 6n]);
        const deltaEncoded = new BigInt64Array([2n, 4n - 2n, 6n - 4n]);
        const zigzagEncoded = new BigInt64Array(deltaEncoded.length);
        for (let i = 0; i < deltaEncoded.length; i++) {
            zigzagEncoded[i] = encodeZigZag(deltaEncoded[i]);
        }
        const data = encodeVarintInt64Array(zigzagEncoded);
        const offset = new IntWrapper(0);
        const bitVector = new BitVector(new Uint8Array([0b00000111]), 3); // All non-null

        const result = decodeNullableLongStream(data, offset, metadata, true, bitVector);

        expect(result).toEqual(expectedValues);
    });

    it("should decode DELTA without RLE with null values", () => {
        const metadata = createStreamMetadata(LogicalLevelTechnique.DELTA, LogicalLevelTechnique.NONE, 5);
        const expectedValues = new BigInt64Array([0n, 2n, 2n, 4n, 6n]); // null values repeat previous value
        // Encode only non-null values: [2n, 4n, 6n]
        const deltaEncoded = new BigInt64Array([2n, 4n - 2n, 6n - 4n]);
        const zigzagEncoded = new BigInt64Array(deltaEncoded.length);
        for (let i = 0; i < deltaEncoded.length; i++) {
            zigzagEncoded[i] = encodeZigZag(deltaEncoded[i]);
        }
        const data = encodeVarintInt64Array(zigzagEncoded);
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
        const rleValues = new BigInt64Array([3n, 2n, encodeZigZag(100n), encodeZigZag(-50n)]);
        const data = encodeVarintInt64Array(rleValues);
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
        const rleValues = new BigInt64Array([2n, 1n, encodeZigZag(100n), encodeZigZag(-50n)]);
        const data = encodeVarintInt64Array(rleValues);
        const offset = new IntWrapper(0);
        const bitVector = new BitVector(new Uint8Array([0b00010101]), 5); // positions 0, 2, 4 are non-null

        const result = decodeNullableLongStream(data, offset, metadata, true, bitVector);

        expect(result).toEqual(expectedValues);
    });

    it("should decode NONE signed with all non-null values", () => {
        const metadata = createStreamMetadata(LogicalLevelTechnique.NONE);
        const expectedValues = new BigInt64Array([2n, -4n, 6n]);
        const zigzagEncoded = new BigInt64Array(Array.from(expectedValues, (val) => encodeZigZag(val)));
        const data = encodeVarintInt64Array(zigzagEncoded);
        const offset = new IntWrapper(0);
        const bitVector = new BitVector(new Uint8Array([0b00000111]), 3); // All non-null

        const result = decodeNullableLongStream(data, offset, metadata, true, bitVector);

        expect(result).toEqual(expectedValues);
    });

    it("should decode NONE signed with null values", () => {
        const metadata = createStreamMetadata(LogicalLevelTechnique.NONE, LogicalLevelTechnique.NONE, 5);
        const expectedValues = new BigInt64Array([2n, 0n, -4n, 0n, 6n]); // null values become 0n
        // Encode only non-null values: [2n, -4n, 6n]
        const nonNullValues = [2n, -4n, 6n];
        const zigzagEncoded = new BigInt64Array(Array.from(nonNullValues, (val) => encodeZigZag(val)));
        const data = encodeVarintInt64Array(zigzagEncoded);
        const offset = new IntWrapper(0);
        const bitVector = new BitVector(new Uint8Array([0b00010101]), 5); // positions 0, 2, 4 are non-null

        const result = decodeNullableLongStream(data, offset, metadata, true, bitVector);

        expect(result).toEqual(expectedValues);
    });

    it("should decode NONE unsigned with all non-null values", () => {
        const metadata = createStreamMetadata(LogicalLevelTechnique.NONE);
        const expectedValues = new BigInt64Array([1n, 2n, 3n]);
        // Unsigned values are not zigzag encoded, just varint encoded
        const data = encodeVarintInt64Array(expectedValues);
        const offset = new IntWrapper(0);
        const bitVector = new BitVector(new Uint8Array([0b00000111]), 3); // All non-null

        const result = decodeNullableLongStream(data, offset, metadata, false, bitVector);

        expect(result).toEqual(expectedValues);
    });

    it("should decode NONE unsigned with null values", () => {
        const metadata = createStreamMetadata(LogicalLevelTechnique.NONE, LogicalLevelTechnique.NONE, 5);
        const expectedValues = new BigInt64Array([0n, 1n, 2n, 0n, 3n]); // null values become 0n
        // Encode only non-null values: [1n, 2n, 3n]
        const nonNullValues = new BigInt64Array([1n, 2n, 3n]);
        const data = encodeVarintInt64Array(nonNullValues);
        const offset = new IntWrapper(0);
        const bitVector = new BitVector(new Uint8Array([0b00010110]), 5); // positions 1, 2, 4 are non-null

        const result = decodeNullableLongStream(data, offset, metadata, false, bitVector);

        expect(result).toEqual(expectedValues);
    });

    it("should throw for unsupported technique", () => {
        const metadata = createStreamMetadata(LogicalLevelTechnique.COMPONENTWISE_DELTA);
        const values = new BigInt64Array([1n, 2n, 3n]);
        const data = encodeVarintInt64Array(values);
        const offset = new IntWrapper(0);
        const bitVector = new BitVector(new Uint8Array([0b00000111]), 3);
        expect(() => decodeNullableLongStream(data, offset, metadata, true, bitVector)).toThrow();
    });
});
