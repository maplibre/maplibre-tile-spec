import { PhysicalLevelTechnique } from "../metadata/tile/physicalLevelTechnique";
import type IntWrapper from "./intWrapper";
import {
    decodeComponentwiseDeltaVec2,
    decodeComponentwiseDeltaVec2Scaled,
    decodeDeltaRleInt32,
    decodeDeltaRleInt64,
    decodeFastPfor,
    decodeNullableZigZagDeltaInt32,
    decodeNullableZigZagDeltaInt64,
    decodeUnsignedConstRleInt32,
    decodeUnsignedConstRleInt64,
    decodeUnsignedRleInt32,
    decodeUnsignedRleInt64,
    decodeUnsignedRleFloat64,
    decodeVarintInt32,
    decodeVarintInt64,
    decodeVarintFloat64,
    decodeZigZagInt32,
    decodeZigZagInt64,
    decodeZigZagFloat64,
    decodeZigZagConstRleInt32,
    decodeZigZagConstRleInt64,
    decodeZigZagDeltaInt32,
    decodeZigZagDeltaInt64,
    decodeZigZagDeltaFloat64,
    decodeZigZagSequenceRleInt32,
    decodeZigZagSequenceRleInt64,
    decodeZigZagInt32Value,
    decodeZigZagInt64Value,
    fastInverseDelta,
    inverseDelta,
    padWithZerosInt32,
    padWithZerosInt64,
    padZigZagWithZerosInt32,
    padZigZagWithZerosInt64,
    rleDeltaDecoding,
    zigZagDeltaOfDeltaDecoding,
    decodeZigZagRleDeltaInt32,
    decodeZigZagRleInt32,
    decodeZigZagRleInt64,
    decodeZigZagRleFloat64,
    decodeNullableZigZagRleInt32,
    decodeNullableUnsignedRleInt32,
    decodeNullableZigZagRleInt64,
    decodeNullableUnsignedRleInt64,
} from "./integerDecodingUtils";
import { LogicalLevelTechnique } from "../metadata/tile/logicalLevelTechnique";
import { type StreamMetadata, type RleEncodedStreamMetadata } from "../metadata/tile/streamMetadataDecoder";
import BitVector from "../vector/flat/bitVector";
import { VectorType } from "../vector/vectorType";
import type GeometryScaling from "./geometryScaling";

export function decodeIntStream(
    data: Uint8Array,
    offset: IntWrapper,
    streamMetadata: StreamMetadata,
    isSigned: boolean,
    scalingData?: GeometryScaling,
): Int32Array {
    const values = decodePhysicalLevelTechnique(data, offset, streamMetadata);
    return decodeInt32(values, streamMetadata, isSigned, scalingData);
}

export function decodeLengthStreamToOffsetBuffer(
    data: Uint8Array,
    offset: IntWrapper,
    streamMetadata: StreamMetadata,
): Int32Array {
    const values = decodePhysicalLevelTechnique(data, offset, streamMetadata);
    return decodeLengthToOffsetBuffer(values, streamMetadata);
}

function decodePhysicalLevelTechnique(
    data: Uint8Array,
    offset: IntWrapper,
    streamMetadata: StreamMetadata,
): Int32Array {
    const physicalLevelTechnique = streamMetadata.physicalLevelTechnique;
    if (physicalLevelTechnique === PhysicalLevelTechnique.FAST_PFOR) {
        return decodeFastPfor(data, streamMetadata.numValues, streamMetadata.byteLength, offset);
    }
    if (physicalLevelTechnique === PhysicalLevelTechnique.VARINT) {
        return decodeVarintInt32(data, offset, streamMetadata.numValues);
    }

    if (physicalLevelTechnique === PhysicalLevelTechnique.NONE) {
        const dataOffset = offset.get();
        const byteLength = streamMetadata.byteLength;
        offset.add(byteLength);
        //TODO: use Byte Rle for geometry type encoding
        const slice = data.subarray(dataOffset, offset.get());
        return new Int32Array(slice);
    }

    throw new Error("Specified physicalLevelTechnique is not supported (yet).");
}

export function decodeConstIntStream(
    data: Uint8Array,
    offset: IntWrapper,
    streamMetadata: StreamMetadata,
    isSigned: boolean,
): number {
    const values = decodePhysicalLevelTechnique(data, offset, streamMetadata);

    if (values.length === 1) {
        const value = values[0];
        return isSigned ? decodeZigZagInt32Value(value) : value;
    }

    return isSigned ? decodeZigZagConstRleInt32(values) : decodeUnsignedConstRleInt32(values);
}

export function decodeSequenceIntStream(
    data: Uint8Array,
    offset: IntWrapper,
    streamMetadata: StreamMetadata,
): [baseValue: number, delta: number] {
    const values = decodePhysicalLevelTechnique(data, offset, streamMetadata);
    return decodeZigZagSequenceRleInt32(values);
}

export function decodeSequenceLongStream(
    data: Uint8Array,
    offset: IntWrapper,
    streamMetadata: StreamMetadata,
): [baseValue: bigint, delta: bigint] {
    const values = decodeVarintInt64(data, offset, streamMetadata.numValues);
    return decodeZigZagSequenceRleInt64(values);
}

export function decodeLongStream(
    data: Uint8Array,
    offset: IntWrapper,
    streamMetadata: StreamMetadata,
    isSigned: boolean,
): BigInt64Array {
    const values = decodeVarintInt64(data, offset, streamMetadata.numValues);
    return decodeInt64(values, streamMetadata, isSigned);
}

export function decodeLongFloat64Stream(
    data: Uint8Array,
    offset: IntWrapper,
    streamMetadata: StreamMetadata,
    isSigned: boolean,
): Float64Array {
    const values = decodeVarintFloat64(data, offset, streamMetadata.numValues);
    return decodeFloat64(values, streamMetadata, isSigned);
}

export function decodeConstLongStream(
    data: Uint8Array,
    offset: IntWrapper,
    streamMetadata: StreamMetadata,
    isSigned: boolean,
): bigint {
    const values = decodeVarintInt64(data, offset, streamMetadata.numValues);

    if (values.length === 1) {
        const value = values[0];
        return isSigned ? decodeZigZagInt64Value(value) : value;
    }

    return isSigned ? decodeZigZagConstRleInt64(values) : decodeUnsignedConstRleInt64(values);
}

/*
 * Currently the encoder uses only fixed combinations of encodings.
 * For performance reasons it is also used a fixed combination of the encodings on the decoding side.
 * The following encodings and combinations are used:
 *   - Morton Delta -> always sorted so not ZigZag encoding needed
 *   - Delta -> currently always in combination with ZigZag encoding
 *   - Rle -> in combination with ZigZag encoding if data type is signed
 *   - Delta Rle
 *   - Componentwise Delta -> always ZigZag encoding is used
 * */
function decodeInt32(
    values: Int32Array,
    streamMetadata: StreamMetadata,
    isSigned: boolean,
    scalingData?: GeometryScaling,
): Int32Array {
    switch (streamMetadata.logicalLevelTechnique1) {
        case LogicalLevelTechnique.DELTA:
            if (streamMetadata.logicalLevelTechnique2 === LogicalLevelTechnique.RLE) {
                const rleMetadata = streamMetadata as RleEncodedStreamMetadata;
                return decodeDeltaRleInt32(values, rleMetadata.runs, rleMetadata.numRleValues);
            }
            decodeZigZagDeltaInt32(values);
            return values;
        case LogicalLevelTechnique.RLE:
            return decodeRleInt32(values, streamMetadata as RleEncodedStreamMetadata, isSigned);
        case LogicalLevelTechnique.MORTON:
            fastInverseDelta(values);
            return values;
        case LogicalLevelTechnique.COMPONENTWISE_DELTA:
            if (scalingData) {
                decodeComponentwiseDeltaVec2Scaled(values, scalingData.scale, scalingData.min, scalingData.max);
                return values;
            }

            decodeComponentwiseDeltaVec2(values);
            return values;
        case LogicalLevelTechnique.NONE:
            if (isSigned) {
                decodeZigZagInt32(values);
            }
            return values;
        default:
            throw new Error(
                `The specified Logical level technique is not supported: ${streamMetadata.logicalLevelTechnique1}`,
            );
    }
}

function decodeInt64(values: BigInt64Array, streamMetadata: StreamMetadata, isSigned: boolean): BigInt64Array {
    switch (streamMetadata.logicalLevelTechnique1) {
        case LogicalLevelTechnique.DELTA:
            if (streamMetadata.logicalLevelTechnique2 === LogicalLevelTechnique.RLE) {
                const rleMetadata = streamMetadata as RleEncodedStreamMetadata;
                return decodeDeltaRleInt64(values, rleMetadata.runs, rleMetadata.numRleValues);
            }
            decodeZigZagDeltaInt64(values);
            return values;
        case LogicalLevelTechnique.RLE:
            return decodeRleInt64(values, streamMetadata as RleEncodedStreamMetadata, isSigned);
        case LogicalLevelTechnique.NONE:
            if (isSigned) {
                decodeZigZagInt64(values);
            }
            return values;
        default:
            throw new Error(
                `The specified Logical level technique is not supported: ${streamMetadata.logicalLevelTechnique1}`,
            );
    }
}

export function decodeFloat64(values: Float64Array, streamMetadata: StreamMetadata, isSigned: boolean): Float64Array {
    switch (streamMetadata.logicalLevelTechnique1) {
        case LogicalLevelTechnique.DELTA:
            if (streamMetadata.logicalLevelTechnique2 === LogicalLevelTechnique.RLE) {
                const rleMetadata = streamMetadata as RleEncodedStreamMetadata;
                values = decodeUnsignedRleFloat64(values, rleMetadata.runs, rleMetadata.numRleValues);
            }
            decodeZigZagDeltaFloat64(values);
            return values;
        case LogicalLevelTechnique.RLE:
            return decodeRleFloat64(values, streamMetadata as RleEncodedStreamMetadata, isSigned);
        case LogicalLevelTechnique.NONE:
            if (isSigned) {
                decodeZigZagFloat64(values);
            }
            return values;
        default:
            throw new Error(
                `The specified Logical level technique is not supported: ${streamMetadata.logicalLevelTechnique1}`,
            );
    }
}

function decodeLengthToOffsetBuffer(values: Int32Array, streamMetadata: StreamMetadata): Int32Array {
    if (
        streamMetadata.logicalLevelTechnique1 === LogicalLevelTechnique.DELTA &&
        streamMetadata.logicalLevelTechnique2 === LogicalLevelTechnique.NONE
    ) {
        const decodedValues = zigZagDeltaOfDeltaDecoding(values);
        return decodedValues;
    }

    if (
        streamMetadata.logicalLevelTechnique1 === LogicalLevelTechnique.RLE &&
        streamMetadata.logicalLevelTechnique2 === LogicalLevelTechnique.NONE
    ) {
        const rleMetadata = streamMetadata as RleEncodedStreamMetadata;
        const decodedValues = rleDeltaDecoding(values, rleMetadata.runs, rleMetadata.numRleValues);
        return decodedValues;
    }

    if (
        streamMetadata.logicalLevelTechnique1 === LogicalLevelTechnique.NONE &&
        streamMetadata.logicalLevelTechnique2 === LogicalLevelTechnique.NONE
    ) {
        //TODO: use fastInverseDelta again and check what are the performance problems in zoom 14
        //fastInverseDelta(values);
        inverseDelta(values);
        const offsets = new Int32Array(streamMetadata.numValues + 1);
        offsets[0] = 0;
        offsets.set(values, 1);
        return offsets;
    }

    if (
        streamMetadata.logicalLevelTechnique1 === LogicalLevelTechnique.DELTA &&
        streamMetadata.logicalLevelTechnique2 === LogicalLevelTechnique.RLE
    ) {
        const rleMetadata = streamMetadata as RleEncodedStreamMetadata;
        const decodedValues = decodeZigZagRleDeltaInt32(values, rleMetadata.runs, rleMetadata.numRleValues);
        fastInverseDelta(decodedValues);
        return decodedValues;
    }

    throw new Error("Only delta encoding is supported for transforming length to offset streams yet.");
}

export function decodeNullableIntStream(
    data: Uint8Array,
    offset: IntWrapper,
    streamMetadata: StreamMetadata,
    isSigned: boolean,
    bitVector: BitVector,
): Int32Array {
    const values =
        streamMetadata.physicalLevelTechnique === PhysicalLevelTechnique.FAST_PFOR
            ? decodeFastPfor(data, streamMetadata.numValues, streamMetadata.byteLength, offset)
            : decodeVarintInt32(data, offset, streamMetadata.numValues);

    return decodeNullableInt32(values, streamMetadata, isSigned, bitVector);
}

export function decodeNullableLongStream(
    data: Uint8Array,
    offset: IntWrapper,
    streamMetadata: StreamMetadata,
    isSigned: boolean,
    bitVector: BitVector,
): BigInt64Array {
    const values = decodeVarintInt64(data, offset, streamMetadata.numValues);
    return decodeNullableInt64(values, streamMetadata, isSigned, bitVector);
}

function decodeNullableInt32(
    values: Int32Array,
    streamMetadata: StreamMetadata,
    isSigned: boolean,
    bitVector: BitVector,
): Int32Array {
    switch (streamMetadata.logicalLevelTechnique1) {
        case LogicalLevelTechnique.DELTA:
            if (streamMetadata.logicalLevelTechnique2 === LogicalLevelTechnique.RLE) {
                const rleMetadata = streamMetadata as RleEncodedStreamMetadata;
                values = decodeUnsignedRleInt32(values, rleMetadata.runs, rleMetadata.numRleValues);
            }
            return decodeNullableZigZagDeltaInt32(bitVector, values);
        case LogicalLevelTechnique.RLE:
            return decodeNullableRleInt32(values, streamMetadata, isSigned, bitVector);
        case LogicalLevelTechnique.MORTON:
            fastInverseDelta(values);
            return values;
        case LogicalLevelTechnique.COMPONENTWISE_DELTA:
            decodeComponentwiseDeltaVec2(values);
            return values;
        case LogicalLevelTechnique.NONE:
            values = isSigned ? padZigZagWithZerosInt32(bitVector, values) : padWithZerosInt32(bitVector, values);
            return values;
        default:
            throw new Error("The specified Logical level technique is not supported");
    }
}

function decodeNullableInt64(
    values: BigInt64Array,
    streamMetadata: StreamMetadata,
    isSigned: boolean,
    bitVector: BitVector,
): BigInt64Array {
    switch (streamMetadata.logicalLevelTechnique1) {
        case LogicalLevelTechnique.DELTA:
            if (streamMetadata.logicalLevelTechnique2 === LogicalLevelTechnique.RLE) {
                const rleMetadata = streamMetadata as RleEncodedStreamMetadata;
                values = decodeUnsignedRleInt64(values, rleMetadata.runs, rleMetadata.numRleValues);
            }
            return decodeNullableZigZagDeltaInt64(bitVector, values);
        case LogicalLevelTechnique.RLE:
            return decodeNullableRleInt64(values, streamMetadata, isSigned, bitVector);
        case LogicalLevelTechnique.NONE:
            values = isSigned ? padZigZagWithZerosInt64(bitVector, values) : padWithZerosInt64(bitVector, values);
            return values;
        default:
            throw new Error("The specified Logical level technique is not supported");
    }
}

export function getVectorType(
    streamMetadata: StreamMetadata,
    sizeOrNullabilityBuffer: number | BitVector,
    data: Uint8Array,
    offset: IntWrapper,
): VectorType {
    const logicalLevelTechnique1 = streamMetadata.logicalLevelTechnique1;
    if (logicalLevelTechnique1 === LogicalLevelTechnique.RLE) {
        return (streamMetadata as RleEncodedStreamMetadata).runs === 1 ? VectorType.CONST : VectorType.FLAT;
    }

    if (
        logicalLevelTechnique1 !== LogicalLevelTechnique.DELTA ||
        streamMetadata.logicalLevelTechnique2 !== LogicalLevelTechnique.RLE
    ) {
        return streamMetadata.numValues === 1 ? VectorType.CONST : VectorType.FLAT;
    }
    const numFeatures =
        sizeOrNullabilityBuffer instanceof BitVector ? sizeOrNullabilityBuffer.size() : sizeOrNullabilityBuffer;
    const rleMetadata = streamMetadata as RleEncodedStreamMetadata;

    if (rleMetadata.numRleValues !== numFeatures) {
        return VectorType.FLAT;
    }
    // Single run is always a sequence
    if (rleMetadata.runs === 1) {
        return VectorType.SEQUENCE;
    }

    if (rleMetadata.runs !== 2) {
        return streamMetadata.numValues === 1 ? VectorType.CONST : VectorType.FLAT;
    }
    // Two runs can be a sequence if both deltas are equal to 1
    const savedOffset = offset.get();

    let values: Int32Array;
    if (streamMetadata.physicalLevelTechnique === PhysicalLevelTechnique.VARINT) {
        values = decodeVarintInt32(data, offset, 4);
    } else {
        const byteOffset = offset.get();
        values = new Int32Array(data.buffer, data.byteOffset + byteOffset, 4);
    }
    offset.set(savedOffset);
    // Check if both deltas are encoded 1
    const zigZagOne = 2;
    if (values[2] === zigZagOne && values[3] === zigZagOne) {
        return VectorType.SEQUENCE;
    }
    return streamMetadata.numValues === 1 ? VectorType.CONST : VectorType.FLAT;
}

function decodeRleInt32(data: Int32Array, streamMetadata: RleEncodedStreamMetadata, isSigned: boolean): Int32Array {
    return isSigned
        ? decodeZigZagRleInt32(data, streamMetadata.runs, streamMetadata.numRleValues)
        : decodeUnsignedRleInt32(data, streamMetadata.runs, streamMetadata.numRleValues);
}

function decodeRleInt64(
    data: BigInt64Array,
    streamMetadata: RleEncodedStreamMetadata,
    isSigned: boolean,
): BigInt64Array {
    return isSigned
        ? decodeZigZagRleInt64(data, streamMetadata.runs, streamMetadata.numRleValues)
        : decodeUnsignedRleInt64(data, streamMetadata.runs, streamMetadata.numRleValues);
}

function decodeRleFloat64(
    data: Float64Array,
    streamMetadata: RleEncodedStreamMetadata,
    isSigned: boolean,
): Float64Array {
    return isSigned
        ? decodeZigZagRleFloat64(data, streamMetadata.runs, streamMetadata.numRleValues)
        : decodeUnsignedRleFloat64(data, streamMetadata.runs, streamMetadata.numRleValues);
}

function decodeNullableRleInt32(
    data: Int32Array,
    streamMetadata: StreamMetadata,
    isSigned: boolean,
    bitVector: BitVector,
): Int32Array {
    const rleMetadata = streamMetadata as RleEncodedStreamMetadata;
    return isSigned
        ? decodeNullableZigZagRleInt32(bitVector, data, rleMetadata.runs)
        : decodeNullableUnsignedRleInt32(bitVector, data, rleMetadata.runs);
}

export function decodeNullableRleInt64(
    data: BigInt64Array,
    streamMetadata: StreamMetadata,
    isSigned: boolean,
    bitVector: BitVector,
): BigInt64Array {
    const rleMetadata = streamMetadata as RleEncodedStreamMetadata;
    return isSigned
        ? decodeNullableZigZagRleInt64(bitVector, data, rleMetadata.runs)
        : decodeNullableUnsignedRleInt64(bitVector, data, rleMetadata.runs);
}
