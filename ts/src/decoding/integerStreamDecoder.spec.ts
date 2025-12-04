import { describe, it, expect } from "vitest";
import {
    getVectorType,
    decodeLongStream,
    decodeNullableLongStream,
    decodeIntStream,
    decodeFloat64Buffer,
    decodeNullableIntStream,
} from "./integerStreamDecoder";
import { LogicalLevelTechnique } from "../metadata/tile/logicalLevelTechnique";
import { PhysicalLevelTechnique } from "../metadata/tile/physicalLevelTechnique";
import { VectorType } from "../vector/vectorType";
import IntWrapper from "./intWrapper";
import BitVector from "../vector/flat/bitVector";
import {
    createStreamMetadata,
    createRleMetadata,
    encodeVarintInt64Array,
    encodeZigZag64,
    encodeVarintInt32Array,
    encodeDelta,
} from "./decodingTestUtils";

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

describe("decodeIntStream", () => {
    it("should decode with PhysicalLevelTechnique.NONE", () => {
        const metadata = {
            ...createStreamMetadata(LogicalLevelTechnique.NONE),
            physicalLevelTechnique: PhysicalLevelTechnique.NONE,
            numValues: 3,
            byteLength: 3,
        };
        const data = new Uint8Array([10, 20, 30]);
        const offset = new IntWrapper(0);

        const result = decodeIntStream(data, offset, metadata, false);

        expect(result).toEqual(new Int32Array([10, 20, 30]));
    });

    it("should throw for unsupported PhysicalLevelTechnique", () => {
        const metadata = {
            ...createStreamMetadata(LogicalLevelTechnique.NONE),
            physicalLevelTechnique: PhysicalLevelTechnique.ALP,
            numValues: 3,
            byteLength: 3,
        };
        const data = new Uint8Array([10, 20, 30]);
        const offset = new IntWrapper(0);

        expect(() => decodeIntStream(data, offset, metadata, false)).toThrow(
            "Specified physicalLevelTechnique is not supported (yet).",
        );
    });

    it("should decode MORTON", () => {
        const metadata = createStreamMetadata(LogicalLevelTechnique.MORTON, LogicalLevelTechnique.NONE, 4);
        // Morton encoding uses delta encoding (sorted data, no zigzag needed)
        const expectedValues = new Int32Array([10, 15, 18, 20]);
        const deltaEncoded = encodeDelta(expectedValues);
        const data = encodeVarintInt32Array(deltaEncoded);
        const offset = new IntWrapper(0);

        const result = decodeIntStream(data, offset, metadata, false);

        expect(result).toEqual(expectedValues);
    });

    it("should decode COMPONENTWISE_DELTA with scalingData", () => {
        const metadata = createStreamMetadata(
            LogicalLevelTechnique.COMPONENTWISE_DELTA,
            LogicalLevelTechnique.NONE,
            6,
        );
        const encoded = new Int32Array([4, 6, 2, 4, 2, 4]);
        const data = encodeVarintInt32Array(encoded);
        const offset = new IntWrapper(0);
        const scalingData = { extent: 100, min: 0, max: 100, scale: 2 };

        const result = decodeIntStream(data, offset, metadata, false, scalingData);

        expect(result).toEqual(new Int32Array([4, 6, 6, 10, 8, 14]));
    });
});

describe("decodeFloat64Buffer", () => {
    it("should decode NONE unsigned", () => {
        const metadata = createStreamMetadata(LogicalLevelTechnique.NONE);
        const values = new Float64Array([1.5, 2.5, 3.5]);

        const result = decodeFloat64Buffer(values, metadata, false);

        expect(result).toEqual(new Float64Array([1.5, 2.5, 3.5]));
    });

    it("should decode NONE signed", () => {
        const metadata = createStreamMetadata(LogicalLevelTechnique.NONE);
        // ZigZag encoded: positive values are multiplied by 2
        const values = new Float64Array([4, 10, 6]); // zigzag encoded [2, 5, 3]

        const result = decodeFloat64Buffer(values, metadata, true);

        expect(result).toEqual(new Float64Array([2, 5, 3]));
    });

    it("should decode RLE unsigned", () => {
        const numRleValues = 5;
        const runs = 2;
        const metadata = createRleMetadata(LogicalLevelTechnique.RLE, LogicalLevelTechnique.NONE, runs, numRleValues);
        const values = new Float64Array([3, 2, 10.5, 20.5]);

        const result = decodeFloat64Buffer(values, metadata, false);

        expect(result).toEqual(new Float64Array([10.5, 10.5, 10.5, 20.5, 20.5]));
    });

    it("should decode RLE signed", () => {
        const numRleValues = 5;
        const runs = 2;
        const metadata = createRleMetadata(LogicalLevelTechnique.RLE, LogicalLevelTechnique.NONE, runs, numRleValues);
        const values = new Float64Array([3, 2, 20, 40]); // zigzag encoded [10, 20]

        const result = decodeFloat64Buffer(values, metadata, true);

        expect(result).toEqual(new Float64Array([10, 10, 10, 20, 20]));
    });

    it("should decode DELTA without RLE", () => {
        const metadata = createStreamMetadata(LogicalLevelTechnique.DELTA);
        const values = new Float64Array([4, 4, 4]);

        const result = decodeFloat64Buffer(values, metadata, true);

        expect(result).toEqual(new Float64Array([2, 4, 6]));
    });

    it("should decode DELTA with RLE", () => {
        const numRleValues = 5;
        const runs = 2;
        const metadata = createRleMetadata(LogicalLevelTechnique.DELTA, LogicalLevelTechnique.RLE, runs, numRleValues);
        const values = new Float64Array([1, 4, 20, 4]);

        const result = decodeFloat64Buffer(values, metadata, true);

        expect(result).toEqual(new Float64Array([10, 12, 14, 16, 18]));
    });

    it("should throw for unsupported technique", () => {
        const metadata = createStreamMetadata(LogicalLevelTechnique.MORTON);
        const values = new Float64Array([1, 2, 3]);

        expect(() => decodeFloat64Buffer(values, metadata, true)).toThrow(
            "The specified Logical level technique is not supported: MORTON",
        );
    });
});

describe("decodeNullableIntStream", () => {
    it("should decode MORTON", () => {
        const metadata = createStreamMetadata(LogicalLevelTechnique.MORTON, LogicalLevelTechnique.NONE, 4);
        // Morton encoding uses delta encoding (sorted data, no zigzag needed)
        const expectedValues = new Int32Array([10, 15, 18, 20]);
        const deltaEncoded = encodeDelta(expectedValues);
        const data = encodeVarintInt32Array(deltaEncoded);
        const offset = new IntWrapper(0);
        const bitVector = new BitVector(new Uint8Array([0b00001111]), 4); // All non-null

        const result = decodeNullableIntStream(data, offset, metadata, false, bitVector);

        expect(result).toEqual(expectedValues);
    });

    it("should decode COMPONENTWISE_DELTA", () => {
        const metadata = createStreamMetadata(
            LogicalLevelTechnique.COMPONENTWISE_DELTA,
            LogicalLevelTechnique.NONE,
            6,
        );
        // COMPONENTWISE_DELTA for Vec2 (x, y pairs)
        // Zigzag encoded deltas: [4, 6, 2, 4, 2, 4] (encodes [2, 3, 1, 2, 1, 2])
        // After componentwise delta decode: [2, 3, 3, 5, 4, 7]
        const encoded = new Int32Array([4, 6, 2, 4, 2, 4]);
        const data = encodeVarintInt32Array(encoded);
        const offset = new IntWrapper(0);
        const bitVector = new BitVector(new Uint8Array([0b00111111]), 6); // All non-null

        const result = decodeNullableIntStream(data, offset, metadata, false, bitVector);

        expect(result).toEqual(new Int32Array([2, 3, 3, 5, 4, 7]));
    });

    it("should throw for unsupported technique", () => {
        const metadata = createStreamMetadata(LogicalLevelTechnique.PDE, LogicalLevelTechnique.NONE, 3);
        const values = new Int32Array([1, 2, 3]);
        const data = encodeVarintInt32Array(values);
        const offset = new IntWrapper(0);
        const bitVector = new BitVector(new Uint8Array([0b00000111]), 3);

        expect(() => decodeNullableIntStream(data, offset, metadata, false, bitVector)).toThrow(
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
        const rleValues = new BigInt64Array([1n, 2n, 2n, encodeZigZag64(10n), encodeZigZag64(2n), encodeZigZag64(1n)]);
        const data = encodeVarintInt64Array(rleValues);
        const offset = new IntWrapper(0);

        const result = decodeLongStream(data, offset, metadata, true);

        expect(result).toEqual(expectedValues);
    });

    it("should decode DELTA without RLE", () => {
        const metadata = createStreamMetadata(LogicalLevelTechnique.DELTA);
        const expectedValues = new BigInt64Array([2n, 4n, 6n]);
        const deltaEncoded = new BigInt64Array([2n, 4n - 2n, 6n - 4n]);
        const zigzagEncoded = new BigInt64Array(deltaEncoded.length);
        for (let i = 0; i < deltaEncoded.length; i++) {
            zigzagEncoded[i] = encodeZigZag64(deltaEncoded[i]);
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
        const rleValues = new BigInt64Array([3n, 2n, encodeZigZag64(100n), encodeZigZag64(-50n)]);
        const data = encodeVarintInt64Array(rleValues);
        const offset = new IntWrapper(0);

        const result = decodeLongStream(data, offset, metadata, true);

        expect(result).toEqual(expectedValues);
    });

    it("should decode NONE signed", () => {
        const metadata = createStreamMetadata(LogicalLevelTechnique.NONE);
        const expectedValues = new BigInt64Array([2n, -4n, 6n]);
        const zigzagEncoded = new BigInt64Array(Array.from(expectedValues, (val) => encodeZigZag64(val)));
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
        const rleValues = new BigInt64Array([1n, 2n, 2n, encodeZigZag64(10n), encodeZigZag64(2n), encodeZigZag64(1n)]);
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
        const rleValues = new BigInt64Array([1n, 2n, encodeZigZag64(10n), encodeZigZag64(2n)]);
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
            zigzagEncoded[i] = encodeZigZag64(deltaEncoded[i]);
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
            zigzagEncoded[i] = encodeZigZag64(deltaEncoded[i]);
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
        const rleValues = new BigInt64Array([3n, 2n, encodeZigZag64(100n), encodeZigZag64(-50n)]);
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
        const rleValues = new BigInt64Array([2n, 1n, encodeZigZag64(100n), encodeZigZag64(-50n)]);
        const data = encodeVarintInt64Array(rleValues);
        const offset = new IntWrapper(0);
        const bitVector = new BitVector(new Uint8Array([0b00010101]), 5); // positions 0, 2, 4 are non-null

        const result = decodeNullableLongStream(data, offset, metadata, true, bitVector);

        expect(result).toEqual(expectedValues);
    });

    it("should decode NONE signed with all non-null values", () => {
        const metadata = createStreamMetadata(LogicalLevelTechnique.NONE);
        const expectedValues = new BigInt64Array([2n, -4n, 6n]);
        const zigzagEncoded = new BigInt64Array(Array.from(expectedValues, (val) => encodeZigZag64(val)));
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
        const zigzagEncoded = new BigInt64Array(Array.from(nonNullValues, (val) => encodeZigZag64(val)));
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
