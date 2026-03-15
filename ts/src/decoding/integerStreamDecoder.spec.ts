import { describe, expect, it } from "vitest";
import {
    decodeSignedInt32Stream,
    decodeSignedInt64AsFloat64Stream,
    decodeSignedInt64Stream,
    decodeSignedConstInt32Stream,
    decodeSignedConstInt64Stream,
    decodeUnsignedInt32Stream,
    decodeUnsignedConstInt32Stream,
    decodeUnsignedConstInt64Stream,
    decodeUnsignedInt64AsFloat64Stream,
    decodeUnsignedInt64Stream,
    getVectorType,
} from "./integerStreamDecoder";
import { LogicalLevelTechnique } from "../metadata/tile/logicalLevelTechnique";
import { PhysicalLevelTechnique } from "../metadata/tile/physicalLevelTechnique";
import { VectorType } from "../vector/vectorType";
import IntWrapper from "./intWrapper";
import BitVector from "../vector/flat/bitVector";
import { createRleMetadata, createStreamMetadata } from "./decodingTestUtils";
import {
    encodeFloat64,
    encodeSignedInt32Stream,
    encodeInt64SignedDelta,
    encodeInt64SignedDeltaRle,
    encodeInt64SignedNone,
    encodeInt64SignedRle,
    encodeInt64UnsignedNone,
    encodeUnsignedInt32Stream,
} from "../encoding/integerStreamEncoder";
import {
    encodeDeltaRleInt32,
    encodeVarintFloat64,
    encodeVarintInt64,
    encodeZigZagInt32Value,
    encodeZigZagInt64Value,
} from "../encoding/integerEncodingUtils";

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
        const twoRunUnitDeltaVarintPayload = new Uint8Array([1, 4, 2, 2]); // Can't achieve this array using the encoding method...
        const result = getVectorType(metadata, 5, twoRunUnitDeltaVarintPayload, new IntWrapper(0));
        expect(result).toBe(VectorType.SEQUENCE);
    });

    it("should probe 64-bit varints without throwing for large DELTA+RLE base values", () => {
        const metadata = createRleMetadata(LogicalLevelTechnique.DELTA, LogicalLevelTechnique.RLE, 2, 4);
        const data = encodeInt64SignedDeltaRle([
            [1, 9_234_567_890n],
            [3, 0n],
        ]);

        const result = getVectorType(metadata, 4, data, new IntWrapper(0), "int64");

        expect(result).toBe(VectorType.FLAT);
    });

    it("should detect SEQUENCE for DELTA+RLE direct int32 payloads with unit deltas", () => {
        const unitDeltaEncodedValue = encodeZigZagInt32Value(1);
        const twoRunUnitDeltaWords = new Uint32Array([1, 4, unitDeltaEncodedValue, unitDeltaEncodedValue]);
        const twoRunUnitDeltaPayload = new Uint8Array(twoRunUnitDeltaWords.buffer.slice(0));
        const metadata = {
            ...createRleMetadata(LogicalLevelTechnique.DELTA, LogicalLevelTechnique.RLE, 2, 5),
            physicalLevelTechnique: PhysicalLevelTechnique.NONE,
            byteLength: twoRunUnitDeltaPayload.byteLength,
        };

        const result = getVectorType(metadata, 5, twoRunUnitDeltaPayload, new IntWrapper(0));

        expect(result).toBe(VectorType.SEQUENCE);
    });

    it("should return FLAT for DELTA+RLE direct int32 payloads with non-unit deltas", () => {
        const increasingOddValues = new Int32Array([1, 3, 5, 7, 9]);
        const { data: encodedWords, runs: deltaRleRunCount } = encodeDeltaRleInt32(increasingOddValues);
        const twoRunMixedDeltaPayload = new Uint8Array(encodedWords.buffer.slice(0));
        const metadata = {
            ...createRleMetadata(LogicalLevelTechnique.DELTA, LogicalLevelTechnique.RLE, deltaRleRunCount, 5),
            physicalLevelTechnique: PhysicalLevelTechnique.NONE,
            byteLength: twoRunMixedDeltaPayload.byteLength,
        };

        const result = getVectorType(metadata, 5, twoRunMixedDeltaPayload, new IntWrapper(0));

        expect(result).toBe(VectorType.FLAT);
    });
});

describe("decodeUnsignedInt32Stream", () => {
    it("should decode with PhysicalLevelTechnique.NONE", () => {
        const expectedValues = new Uint32Array([10, 20, 30]);
        const metadata = createStreamMetadata(LogicalLevelTechnique.NONE);
        const data = encodeUnsignedInt32Stream(expectedValues, metadata);
        const result = decodeUnsignedInt32Stream(data, new IntWrapper(0), metadata);
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
        expect(() => decodeUnsignedInt32Stream(data, new IntWrapper(0), metadata)).toThrow(
            "Specified physicalLevelTechnique ALP is not supported (yet).",
        );
    });

    it("should decode MORTON", () => {
        const expectedValues = new Uint32Array([10, 15, 18, 20]);
        const metadata = createStreamMetadata(LogicalLevelTechnique.MORTON, LogicalLevelTechnique.NONE, 4);
        const data = encodeUnsignedInt32Stream(expectedValues, metadata);
        const result = decodeUnsignedInt32Stream(data, new IntWrapper(0), metadata);

        expect(result).toEqual(expectedValues);
    });

    it("should decode nullable MORTON fully populated", () => {
        const metadata = createStreamMetadata(LogicalLevelTechnique.MORTON, LogicalLevelTechnique.NONE, 4);
        const expectedValues = new Uint32Array([10, 15, 18, 20]);
        const bitVector = new BitVector(new Uint8Array([0b1111]), 4);
        const data = encodeUnsignedInt32Stream(expectedValues, metadata, bitVector);
        const offset = new IntWrapper(0);

        const result = decodeUnsignedInt32Stream(data, offset, metadata, undefined, bitVector);

        expect(result).toEqual(expectedValues);
    });

    it("should decode nullable MORTON null values", () => {
        const metadata = createStreamMetadata(LogicalLevelTechnique.MORTON, LogicalLevelTechnique.NONE, 3);
        const expectedValues = new Uint32Array([10, 0, 15, 0, 18]);
        const bitVector = new BitVector(new Uint8Array([0b10101]), 5);
        const data = encodeUnsignedInt32Stream(expectedValues, metadata, bitVector);

        const result = decodeUnsignedInt32Stream(data, new IntWrapper(0), metadata, undefined, bitVector);

        expect(result).toEqual(expectedValues);
    });

    it("should decode DELTA with RLE", () => {
        const expectedValues = new Uint32Array([10, 12, 14, 15, 16]);
        const metadata = createRleMetadata(
            LogicalLevelTechnique.DELTA,
            LogicalLevelTechnique.RLE,
            3,
            expectedValues.length,
        );
        const data = encodeUnsignedInt32Stream(expectedValues, metadata);

        const result = decodeUnsignedInt32Stream(data, new IntWrapper(0), metadata);

        expect(result).toEqual(expectedValues);
    });
});

describe("decodeSignedInt32Stream", () => {
    it("should decode NONE signed with Int32", () => {
        const expectedValues = new Int32Array([2, -4, 6, -8]);
        const metadata = createStreamMetadata(
            LogicalLevelTechnique.NONE,
            LogicalLevelTechnique.NONE,
            expectedValues.length,
        );
        const data = encodeSignedInt32Stream(expectedValues, metadata);
        const result = decodeSignedInt32Stream(data, new IntWrapper(0), metadata);

        expect(result).toEqual(expectedValues);
    });

    it("should decode nullable NONE signed Int32 partially populated", () => {
        const expectedValues = new Int32Array([0, 15, 0, 20]);
        const metadata = createStreamMetadata(LogicalLevelTechnique.NONE, LogicalLevelTechnique.NONE, 2);
        const bitVector = new BitVector(new Uint8Array([0b1010]), 4);
        const data = encodeSignedInt32Stream(expectedValues, metadata, bitVector);
        const result = decodeSignedInt32Stream(data, new IntWrapper(0), metadata, undefined, bitVector);

        expect(result).toEqual(new Int32Array([0, 15, 0, 20]));
    });

    it("should decode DELTA signed with Int32", () => {
        const expectedValues = new Int32Array([10, 12, 14, 16]);
        const metadata = createStreamMetadata(
            LogicalLevelTechnique.DELTA,
            LogicalLevelTechnique.NONE,
            expectedValues.length,
        );
        const data = encodeSignedInt32Stream(expectedValues, metadata);
        const result = decodeSignedInt32Stream(data, new IntWrapper(0), metadata);

        expect(result).toEqual(expectedValues);
    });

    it("should decode nullable DELTA signed Int32 with null values", () => {
        const logicalValueCount = 5;
        const physicalValueCount = 3;
        const metadata = createStreamMetadata(
            LogicalLevelTechnique.DELTA,
            LogicalLevelTechnique.NONE,
            physicalValueCount,
        );
        const expectedValues = new Int32Array([0, 2, 0, 4, 6]);
        const bitVector = new BitVector(new Uint8Array([0b00011010]), logicalValueCount);
        const data = encodeSignedInt32Stream(expectedValues, metadata, bitVector);

        const result = decodeSignedInt32Stream(data, new IntWrapper(0), metadata, undefined, bitVector);

        expect(result).toEqual(expectedValues);
    });

    it("should decode Componentwise Delta with Int32", () => {
        const metadata = createStreamMetadata(LogicalLevelTechnique.COMPONENTWISE_DELTA, LogicalLevelTechnique.NONE, 4);
        const expectedValues = new Int32Array([10, 20, 11, 21]);
        const data = encodeSignedInt32Stream(expectedValues, metadata);

        const result = decodeSignedInt32Stream(data, new IntWrapper(0), metadata);

        expect(result).toEqual(expectedValues);
    });

    it("should decode Componentwise Delta Scaled with Int32", () => {
        const metadata = createStreamMetadata(LogicalLevelTechnique.COMPONENTWISE_DELTA, LogicalLevelTechnique.NONE, 4);
        const expectedValues = new Int32Array([100, 200, 110, 220]);
        const scalingData = { extent: 4096, min: 0, max: 4096, scale: 2.0 };
        const data = encodeSignedInt32Stream(expectedValues, metadata, undefined, scalingData);

        const result = decodeSignedInt32Stream(data, new IntWrapper(0), metadata, scalingData);

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
        const data = encodeSignedInt32Stream(expectedValues, metadata);
        const result = decodeSignedInt32Stream(data, new IntWrapper(0), metadata);

        expect(result).toEqual(expectedValues);
    });

    it("should decode nullable RLE Int32 partially populated", () => {
        let metadata = createStreamMetadata(LogicalLevelTechnique.RLE, LogicalLevelTechnique.NONE, 2);
        const expectedValues = new Uint32Array([0, 15, 0, 20]);
        const bitVector = new BitVector(new Uint8Array([0b1010]), 4);
        const data = encodeUnsignedInt32Stream(expectedValues, metadata, bitVector);
        metadata = createRleMetadata(LogicalLevelTechnique.RLE, LogicalLevelTechnique.NONE, 2, 2);
        const result = decodeUnsignedInt32Stream(data, new IntWrapper(0), metadata, undefined, bitVector);

        expect(result).toEqual(new Uint32Array([0, 15, 0, 20]));
    });
});

describe("decodeSignedConstInt32Stream", () => {
    it("should decode signed const Int32", () => {
        const metadata = createStreamMetadata(LogicalLevelTechnique.NONE, LogicalLevelTechnique.NONE, 1);
        const data = encodeSignedInt32Stream(new Int32Array([-8]), metadata);

        const result = decodeSignedConstInt32Stream(data, new IntWrapper(0), metadata);

        expect(result).toBe(-8);
    });
});

describe("decodeUnsignedConstInt32Stream", () => {
    it("should decode unsigned const Int32", () => {
        const metadata = createStreamMetadata(LogicalLevelTechnique.NONE, LogicalLevelTechnique.NONE, 1);
        const data = encodeUnsignedInt32Stream(new Uint32Array([0xffffffff]), metadata);

        const result = decodeUnsignedConstInt32Stream(data, new IntWrapper(0), metadata);

        expect(result).toBe(0xffffffff);
    });

    it("should throw for unsupported technique", () => {
        const metadata = createStreamMetadata(LogicalLevelTechnique.PDE, LogicalLevelTechnique.NONE, 3);
        const offset = new IntWrapper(0);
        const bitVector = new BitVector(new Uint8Array([0b00000111]), 3);

        expect(() => decodeUnsignedInt32Stream(new Uint8Array([]), offset, metadata, undefined, bitVector)).toThrow(
            "The specified Logical level technique is not supported",
        );
    });
});

describe("decodeInt64AsFloat64Stream", () => {
    it("should decode NONE unsigned", () => {
        const metadata = createStreamMetadata(LogicalLevelTechnique.NONE);
        const expectedValues = new Float64Array([1, 2, 3]);
        const encodedValues = encodeFloat64(new Float64Array(expectedValues), metadata, false);
        const data = encodeVarintFloat64(encodedValues);
        const result = decodeUnsignedInt64AsFloat64Stream(data, new IntWrapper(0), metadata);

        expect(result).toEqual(expectedValues);
    });

    it("should decode NONE signed", () => {
        const metadata = createStreamMetadata(LogicalLevelTechnique.NONE);
        const expectedValues = new Float64Array([2, 5, 3]);
        const encodedValues = encodeFloat64(new Float64Array(expectedValues), metadata, true);
        const data = encodeVarintFloat64(encodedValues);
        const result = decodeSignedInt64AsFloat64Stream(data, new IntWrapper(0), metadata);

        expect(result).toEqual(expectedValues);
    });

    it("should decode RLE unsigned", () => {
        const expectedValues = new Float64Array([10, 10, 10, 20, 20]);
        const runs = 2;
        const metadata = createRleMetadata(
            LogicalLevelTechnique.RLE,
            LogicalLevelTechnique.NONE,
            runs,
            expectedValues.length,
        );
        const encodedValues = encodeFloat64(new Float64Array(expectedValues), metadata, false);
        const data = encodeVarintFloat64(encodedValues);
        const result = decodeUnsignedInt64AsFloat64Stream(data, new IntWrapper(0), metadata);

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
        const encodedValues = encodeFloat64(new Float64Array(expectedValues), metadata, true);
        const data = encodeVarintFloat64(encodedValues);
        const result = decodeSignedInt64AsFloat64Stream(data, new IntWrapper(0), metadata);

        expect(result).toEqual(expectedValues);
    });

    it("should decode DELTA without RLE", () => {
        const metadata = createStreamMetadata(LogicalLevelTechnique.DELTA);
        const expectedValues = new Float64Array([2, 4, 6]);
        const encodedValues = encodeFloat64(new Float64Array(expectedValues), metadata, true);
        const data = encodeVarintFloat64(encodedValues);
        const result = decodeSignedInt64AsFloat64Stream(data, new IntWrapper(0), metadata);

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
        const encodedValues = encodeFloat64(new Float64Array(expectedValues), metadata, true);
        const data = encodeVarintFloat64(encodedValues);
        const result = decodeSignedInt64AsFloat64Stream(data, new IntWrapper(0), metadata);

        expect(result).toEqual(new Float64Array([10, 12, 14, 16, 18]));
    });

    it("should throw for unsupported technique", () => {
        const metadata = createStreamMetadata(LogicalLevelTechnique.MORTON);
        const values = new Uint8Array(new Float64Array([1, 2, 3]).buffer);

        expect(() => decodeSignedInt64AsFloat64Stream(values, new IntWrapper(0), metadata)).toThrow(
            "The specified Logical level technique is not supported: MORTON",
        );
    });
});

describe("decodeInt64Stream", () => {
    describe("unsigned DELTA with RLE", () => {
        it("should decode unsigned DELTA with RLE", () => {
            const expectedValues = new BigUint64Array([10n, 12n, 14n, 15n, 16n]);
            const metadata = createRleMetadata(
                LogicalLevelTechnique.DELTA,
                LogicalLevelTechnique.RLE,
                3,
                expectedValues.length,
            );
            const encodedValues = new BigUint64Array([
                1n,
                2n,
                2n,
                encodeZigZagInt64Value(10n),
                encodeZigZagInt64Value(2n),
                encodeZigZagInt64Value(1n),
            ]);
            const data = encodeVarintInt64(encodedValues);
            const offset = new IntWrapper(0);

            const result = decodeUnsignedInt64Stream(data, offset, metadata);

            expect(result).toEqual(expectedValues);
        });
    });

    describe("DELTA with RLE", () => {
        it("should decode DELTA with RLE", () => {
            const numRleValues = 5;
            const runs = 3;
            const metadata = createRleMetadata(
                LogicalLevelTechnique.DELTA,
                LogicalLevelTechnique.RLE,
                runs,
                numRleValues,
            );
            const expectedValues = new BigInt64Array([10n, 12n, 14n, 15n, 16n]);
            const data = encodeInt64SignedDeltaRle([
                [1, 10n],
                [2, 2n],
                [2, 1n],
            ]);
            const offset = new IntWrapper(0);

            const result = decodeSignedInt64Stream(data, offset, metadata);

            expect(result).toEqual(expectedValues);
        });

        it("should decode DELTA with RLE with all non-null values", () => {
            const numRleValues = 5;
            const runs = 3;
            const metadata = createRleMetadata(
                LogicalLevelTechnique.DELTA,
                LogicalLevelTechnique.RLE,
                runs,
                numRleValues,
            );
            const expectedValues = new BigInt64Array([10n, 12n, 14n, 15n, 16n]);
            const data = encodeInt64SignedDeltaRle([
                [1, 10n],
                [2, 2n],
                [2, 1n],
            ]);
            const offset = new IntWrapper(0);
            const bitVector = new BitVector(new Uint8Array([0b00011111]), 5);

            const result = decodeSignedInt64Stream(data, offset, metadata, bitVector);

            expect(result).toEqual(expectedValues);
        });

        it("should decode DELTA with RLE with null values", () => {
            const numRleValues = 3;
            const runs = 2;
            const metadata = createRleMetadata(
                LogicalLevelTechnique.DELTA,
                LogicalLevelTechnique.RLE,
                runs,
                numRleValues,
            );
            const expectedValues = new BigInt64Array([10n, 0n, 12n, 0n, 14n]);
            const data = encodeInt64SignedDeltaRle([
                [1, 10n],
                [2, 2n],
            ]);
            const offset = new IntWrapper(0);
            const bitVector = new BitVector(new Uint8Array([0b00010101]), 5);

            const result = decodeSignedInt64Stream(data, offset, metadata, bitVector);

            expect(result).toEqual(expectedValues);
        });
    });

    describe("DELTA without RLE", () => {
        it("should decode DELTA without RLE", () => {
            const metadata = createStreamMetadata(LogicalLevelTechnique.DELTA);
            const expectedValues = new BigInt64Array([2n, 4n, 6n]);
            const data = encodeInt64SignedDelta(expectedValues);
            const offset = new IntWrapper(0);

            const result = decodeSignedInt64Stream(data, offset, metadata);

            expect(result).toEqual(expectedValues);
        });

        it("should decode DELTA without RLE with all non-null values", () => {
            const metadata = createStreamMetadata(LogicalLevelTechnique.DELTA);
            const expectedValues = new BigInt64Array([2n, 4n, 6n]);
            const data = encodeInt64SignedDelta(expectedValues);
            const offset = new IntWrapper(0);
            const bitVector = new BitVector(new Uint8Array([0b00000111]), 3);

            const result = decodeSignedInt64Stream(data, offset, metadata, bitVector);

            expect(result).toEqual(expectedValues);
        });

        it("should decode DELTA without RLE with null values", () => {
            const metadata = createStreamMetadata(LogicalLevelTechnique.DELTA, LogicalLevelTechnique.NONE, 5);
            const expectedValues = new BigInt64Array([0n, 2n, 0n, 4n, 6n]);
            const nonNullValues = new BigInt64Array([2n, 4n, 6n]);
            const data = encodeInt64SignedDelta(nonNullValues);
            const offset = new IntWrapper(0);
            const bitVector = new BitVector(new Uint8Array([0b00011010]), 5);

            const result = decodeSignedInt64Stream(data, offset, metadata, bitVector);

            expect(result).toEqual(expectedValues);
        });
    });

    describe("RLE", () => {
        it("should decode RLE", () => {
            const numRleValues = 5;
            const runs = 2;
            const metadata = createRleMetadata(
                LogicalLevelTechnique.RLE,
                LogicalLevelTechnique.NONE,
                runs,
                numRleValues,
            );
            const expectedValues = new BigInt64Array([100n, 100n, 100n, -50n, -50n]);
            const data = encodeInt64SignedRle([
                [3, 100n],
                [2, -50n],
            ]);
            const offset = new IntWrapper(0);

            const result = decodeSignedInt64Stream(data, offset, metadata);

            expect(result).toEqual(expectedValues);
        });

        it("should decode RLE with all non-null values", () => {
            const numRleValues = 5;
            const runs = 2;
            const metadata = createRleMetadata(
                LogicalLevelTechnique.RLE,
                LogicalLevelTechnique.NONE,
                runs,
                numRleValues,
            );
            const expectedValues = new BigInt64Array([100n, 100n, 100n, -50n, -50n]);
            const data = encodeInt64SignedRle([
                [3, 100n],
                [2, -50n],
            ]);
            const offset = new IntWrapper(0);
            const bitVector = new BitVector(new Uint8Array([0b00011111]), 5);

            const result = decodeSignedInt64Stream(data, offset, metadata, bitVector);

            expect(result).toEqual(expectedValues);
        });

        it("should decode RLE with null values", () => {
            const numRleValues = 3;
            const runs = 2;
            const metadata = createRleMetadata(
                LogicalLevelTechnique.RLE,
                LogicalLevelTechnique.NONE,
                runs,
                numRleValues,
            );
            const expectedValues = new BigInt64Array([100n, 0n, 100n, 0n, -50n]);
            const data = encodeInt64SignedRle([
                [2, 100n],
                [1, -50n],
            ]);
            const offset = new IntWrapper(0);
            const bitVector = new BitVector(new Uint8Array([0b00010101]), 5);

            const result = decodeSignedInt64Stream(data, offset, metadata, bitVector);

            expect(result).toEqual(expectedValues);
        });
    });

    describe("NONE", () => {
        it("should decode NONE signed", () => {
            const metadata = createStreamMetadata(LogicalLevelTechnique.NONE);
            const expectedValues = new BigInt64Array([2n, -4n, 6n]);
            const data = encodeInt64SignedNone(expectedValues);
            const offset = new IntWrapper(0);

            const result = decodeSignedInt64Stream(data, offset, metadata);

            expect(result).toEqual(expectedValues);
        });

        it("should decode NONE signed min int64", () => {
            const metadata = createStreamMetadata(LogicalLevelTechnique.NONE, LogicalLevelTechnique.NONE, 1);
            const expectedValues = new BigInt64Array([-(2n ** 63n)]);
            const data = encodeInt64SignedNone(expectedValues);
            const offset = new IntWrapper(0);

            const result = decodeSignedInt64Stream(data, offset, metadata);

            expect(result).toEqual(expectedValues);
        });

        it("should decode NONE unsigned", () => {
            const metadata = createStreamMetadata(LogicalLevelTechnique.NONE);
            const expectedValues = new BigUint64Array([1n, 2n, 3n]);
            const data = encodeInt64UnsignedNone(new BigInt64Array(expectedValues));
            const offset = new IntWrapper(0);

            const result = decodeUnsignedInt64Stream(data, offset, metadata);

            expect(result).toEqual(expectedValues);
        });

        it("should decode signed const Int64", () => {
            const metadata = createStreamMetadata(LogicalLevelTechnique.NONE, LogicalLevelTechnique.NONE, 1);
            const data = encodeInt64SignedNone(new BigInt64Array([-8n]));

            const result = decodeSignedConstInt64Stream(data, new IntWrapper(0), metadata);

            expect(result).toBe(-8n);
        });

        it("should decode unsigned const Int64", () => {
            const metadata = createStreamMetadata(LogicalLevelTechnique.NONE, LogicalLevelTechnique.NONE, 1);
            const data = encodeInt64UnsignedNone(new BigInt64Array([0xffffffffffffffffn]));

            const result = decodeUnsignedConstInt64Stream(data, new IntWrapper(0), metadata);

            expect(result).toBe(0xffffffffffffffffn);
        });

        it("should decode NONE signed with all non-null values", () => {
            const metadata = createStreamMetadata(LogicalLevelTechnique.NONE);
            const expectedValues = new BigInt64Array([2n, -4n, 6n]);
            const data = encodeInt64SignedNone(expectedValues);
            const offset = new IntWrapper(0);
            const bitVector = new BitVector(new Uint8Array([0b00000111]), 3);

            const result = decodeSignedInt64Stream(data, offset, metadata, bitVector);

            expect(result).toEqual(expectedValues);
        });

        it("should decode NONE signed with null values", () => {
            const metadata = createStreamMetadata(LogicalLevelTechnique.NONE, LogicalLevelTechnique.NONE, 5);
            const expectedValues = new BigInt64Array([2n, 0n, -4n, 0n, 6n]);
            const nonNullValues = new BigInt64Array([2n, -4n, 6n]);
            const data = encodeInt64SignedNone(nonNullValues);
            const offset = new IntWrapper(0);
            const bitVector = new BitVector(new Uint8Array([0b00010101]), 5);

            const result = decodeSignedInt64Stream(data, offset, metadata, bitVector);

            expect(result).toEqual(expectedValues);
        });

        it("should decode NONE unsigned with all non-null values", () => {
            const metadata = createStreamMetadata(LogicalLevelTechnique.NONE);
            const expectedValues = new BigUint64Array([1n, 2n, 3n]);
            const data = encodeInt64UnsignedNone(new BigInt64Array(expectedValues));
            const offset = new IntWrapper(0);
            const bitVector = new BitVector(new Uint8Array([0b00000111]), 3);

            const result = decodeUnsignedInt64Stream(data, offset, metadata, bitVector);

            expect(result).toEqual(expectedValues);
        });

        it("should decode NONE unsigned with null values", () => {
            const metadata = createStreamMetadata(LogicalLevelTechnique.NONE, LogicalLevelTechnique.NONE, 5);
            const expectedValues = new BigUint64Array([0n, 1n, 2n, 0n, 3n]);
            const nonNullValues = new BigInt64Array([1n, 2n, 3n]);
            const data = encodeInt64UnsignedNone(nonNullValues);
            const offset = new IntWrapper(0);
            const bitVector = new BitVector(new Uint8Array([0b00010110]), 5);

            const result = decodeUnsignedInt64Stream(data, offset, metadata, bitVector);

            expect(result).toEqual(expectedValues);
        });
    });

    describe("error handling", () => {
        it("should throw for unsupported technique", () => {
            const metadata = createStreamMetadata(LogicalLevelTechnique.MORTON);
            const data = encodeInt64UnsignedNone(new BigInt64Array([1n, 2n, 3n]));
            const offset = new IntWrapper(0);
            expect(() => decodeSignedInt64Stream(data, offset, metadata)).toThrow(
                "The specified Logical level technique is not supported: MORTON",
            );
        });

        it("should throw for unsupported technique with nullable", () => {
            const metadata = createStreamMetadata(LogicalLevelTechnique.COMPONENTWISE_DELTA);
            const values = new BigInt64Array([1n, 2n, 3n]);
            const data = encodeInt64UnsignedNone(values);
            const offset = new IntWrapper(0);
            const bitVector = new BitVector(new Uint8Array([0b00000111]), 3);
            expect(() => decodeSignedInt64Stream(data, offset, metadata, bitVector)).toThrow();
        });
    });
});
