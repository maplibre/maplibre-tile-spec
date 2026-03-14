import { PhysicalLevelTechnique } from "../metadata/tile/physicalLevelTechnique";
import type IntWrapper from "./intWrapper";
import {
    decodeComponentwiseDeltaVec2,
    decodeComponentwiseDeltaVec2Scaled,
    decodeDeltaRleInt32,
    decodeDeltaRleInt64,
    decodeFastPfor,
    decodeUnsignedComponentwiseDeltaVec2,
    decodeUnsignedComponentwiseDeltaVec2Scaled,
    decodeUnsignedConstRleInt32,
    decodeUnsignedConstRleInt64,
    decodeUnsignedRleInt32,
    decodeUnsignedRleInt64,
    decodeUnsignedRleFloat64,
    decodeUnsignedZigZagDeltaInt32,
    decodeUnsignedZigZagDeltaInt64,
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
    decodeRleDeltaInt32,
    decodeZigZagDeltaOfDeltaInt32,
    decodeZigZagRleDeltaInt32,
    decodeZigZagRleInt32,
    decodeZigZagRleInt64,
    decodeZigZagRleFloat64,
} from "./integerDecodingUtils";
import { LogicalLevelTechnique } from "../metadata/tile/logicalLevelTechnique";
import type { StreamMetadata, RleEncodedStreamMetadata } from "../metadata/tile/streamMetadataDecoder";
import BitVector from "../vector/flat/bitVector";
import { VectorType } from "../vector/vectorType";
import type GeometryScaling from "./geometryScaling";
import { unpackNullable } from "./unpackNullableUtils";

export function decodeSignedInt32Stream(
    data: Uint8Array,
    offset: IntWrapper,
    streamMetadata: StreamMetadata,
    scalingData?: GeometryScaling,
    nullabilityBuffer?: BitVector,
): Int32Array {
    const values = decodePhysicalLevelTechnique(data, offset, streamMetadata);
    return decodeSignedInt32(values, streamMetadata, scalingData, nullabilityBuffer);
}

export function decodeUnsignedInt32Stream(
    data: Uint8Array,
    offset: IntWrapper,
    streamMetadata: StreamMetadata,
    scalingData?: GeometryScaling,
    nullabilityBuffer?: BitVector,
): Uint32Array {
    const values = decodePhysicalLevelTechnique(data, offset, streamMetadata);
    return decodeUnsignedInt32(values, streamMetadata, scalingData, nullabilityBuffer);
}

export function decodeLengthStreamToOffsetBuffer(
    data: Uint8Array,
    offset: IntWrapper,
    streamMetadata: StreamMetadata,
): Uint32Array {
    const values = decodePhysicalLevelTechnique(data, offset, streamMetadata);
    return decodeLengthToOffsetBuffer(values, streamMetadata);
}

function decodePhysicalLevelTechnique(
    data: Uint8Array,
    offset: IntWrapper,
    streamMetadata: StreamMetadata,
): Uint32Array {
    const physicalLevelTechnique = streamMetadata.physicalLevelTechnique;
    switch (physicalLevelTechnique) {
        case PhysicalLevelTechnique.FAST_PFOR:
            return decodeFastPfor(data, streamMetadata.numValues, streamMetadata.byteLength, offset);
        case PhysicalLevelTechnique.VARINT:
            return decodeVarintInt32(data, offset, streamMetadata.numValues);
        case PhysicalLevelTechnique.NONE: {
            const dataOffset = offset.get();
            const byteLength = streamMetadata.byteLength;
            offset.add(byteLength);
            const slice = data.subarray(dataOffset, offset.get());
            return new Uint32Array(slice);
        }
        default:
            throw new Error(`Specified physicalLevelTechnique ${physicalLevelTechnique} is not supported (yet).`);
    }
}

export function decodeSignedConstInt32Stream(
    data: Uint8Array,
    offset: IntWrapper,
    streamMetadata: StreamMetadata,
): number {
    const values = decodePhysicalLevelTechnique(data, offset, streamMetadata);

    if (values.length === 1) {
        return decodeZigZagInt32Value(values[0]);
    }

    return decodeZigZagConstRleInt32(values);
}

export function decodeUnsignedConstInt32Stream(
    data: Uint8Array,
    offset: IntWrapper,
    streamMetadata: StreamMetadata,
): number {
    const values = decodePhysicalLevelTechnique(data, offset, streamMetadata);

    if (values.length === 1) {
        return values[0];
    }

    return decodeUnsignedConstRleInt32(values);
}

export function decodeSequenceInt32Stream(
    data: Uint8Array,
    offset: IntWrapper,
    streamMetadata: StreamMetadata,
): [baseValue: number, delta: number] {
    const values = decodePhysicalLevelTechnique(data, offset, streamMetadata);
    return decodeZigZagSequenceRleInt32(values);
}

export function decodeSequenceInt64Stream(
    data: Uint8Array,
    offset: IntWrapper,
    streamMetadata: StreamMetadata,
): [baseValue: bigint, delta: bigint] {
    const values = decodeVarintInt64(data, offset, streamMetadata.numValues);
    return decodeZigZagSequenceRleInt64(values);
}

export function decodeSignedInt64Stream(
    data: Uint8Array,
    offset: IntWrapper,
    streamMetadata: StreamMetadata,
    nullabilityBuffer?: BitVector,
): BigInt64Array {
    const values = decodeVarintInt64(data, offset, streamMetadata.numValues);
    return decodeSignedInt64(values, streamMetadata, nullabilityBuffer);
}

export function decodeUnsignedInt64Stream(
    data: Uint8Array,
    offset: IntWrapper,
    streamMetadata: StreamMetadata,
    nullabilityBuffer?: BitVector,
): BigUint64Array {
    const values = decodeVarintInt64(data, offset, streamMetadata.numValues);
    return decodeUnsignedInt64(values, streamMetadata, nullabilityBuffer);
}

export function decodeSignedInt64AsFloat64Stream(
    data: Uint8Array,
    offset: IntWrapper,
    streamMetadata: StreamMetadata,
): Float64Array {
    const values = decodeVarintFloat64(data, offset, streamMetadata.numValues);
    return decodeFloat64Values(values, streamMetadata, true);
}

export function decodeUnsignedInt64AsFloat64Stream(
    data: Uint8Array,
    offset: IntWrapper,
    streamMetadata: StreamMetadata,
): Float64Array {
    const values = decodeVarintFloat64(data, offset, streamMetadata.numValues);
    return decodeFloat64Values(values, streamMetadata, false);
}

export function decodeSignedConstInt64Stream(
    data: Uint8Array,
    offset: IntWrapper,
    streamMetadata: StreamMetadata,
): bigint {
    const values = decodeVarintInt64(data, offset, streamMetadata.numValues);

    if (values.length === 1) {
        return decodeZigZagInt64Value(values[0]);
    }

    return decodeZigZagConstRleInt64(values);
}

export function decodeUnsignedConstInt64Stream(
    data: Uint8Array,
    offset: IntWrapper,
    streamMetadata: StreamMetadata,
): bigint {
    const values = decodeVarintInt64(data, offset, streamMetadata.numValues);

    if (values.length === 1) {
        return values[0];
    }

    return decodeUnsignedConstRleInt64(values);
}

/**
 * This method decodes integer streams.
 * Currently the encoder uses only fixed combinations of encodings.
 * For performance reasons it is also uses a fixed combination of the encodings on the decoding side.
 * The following encodings and combinations are used:
 *   - Morton Delta -> always sorted so not ZigZag encoding needed
 *   - Delta -> currently always in combination with ZigZag encoding
 *   - Rle -> in combination with ZigZag encoding if data type is signed
 *   - Delta Rle
 *   - Componentwise Delta -> always ZigZag encoding is used
 */
function decodeSignedInt32(
    values: Uint32Array,
    streamMetadata: StreamMetadata,
    scalingData?: GeometryScaling,
    nullabilityBuffer?: BitVector,
): Int32Array {
    let decodedValues: Int32Array;
    switch (streamMetadata.logicalLevelTechnique1) {
        case LogicalLevelTechnique.DELTA:
            if (streamMetadata.logicalLevelTechnique2 === LogicalLevelTechnique.RLE) {
                const rleMetadata = streamMetadata as RleEncodedStreamMetadata;
                if (!nullabilityBuffer) {
                    return decodeDeltaRleInt32(values, rleMetadata.runs, rleMetadata.numRleValues);
                }
                values = decodeUnsignedRleInt32(values, rleMetadata.runs, rleMetadata.numRleValues);
                decodedValues = decodeZigZagDeltaInt32(values);
            } else {
                decodedValues = decodeZigZagDeltaInt32(values);
            }
            break;
        case LogicalLevelTechnique.RLE:
            decodedValues = decodeZigZagRleInt32(
                values,
                (streamMetadata as RleEncodedStreamMetadata).runs,
                (streamMetadata as RleEncodedStreamMetadata).numRleValues,
            );
            break;
        case LogicalLevelTechnique.MORTON:
            fastInverseDelta(values);
            decodedValues = new Int32Array(values);
            break;
        case LogicalLevelTechnique.COMPONENTWISE_DELTA:
            if (scalingData && !nullabilityBuffer) {
                return decodeComponentwiseDeltaVec2Scaled(values, scalingData.scale, scalingData.min, scalingData.max);
            }
            decodedValues = decodeComponentwiseDeltaVec2(values);
            break;
        case LogicalLevelTechnique.NONE:
            decodedValues = decodeZigZagInt32(values);
            break;
        default:
            throw new Error(
                `The specified Logical level technique is not supported: ${streamMetadata.logicalLevelTechnique1}`,
            );
    }

    if (nullabilityBuffer) {
        return unpackNullable(decodedValues, nullabilityBuffer, 0);
    }
    return decodedValues;
}

function decodeUnsignedInt32(
    values: Uint32Array,
    streamMetadata: StreamMetadata,
    scalingData?: GeometryScaling,
    nullabilityBuffer?: BitVector,
): Uint32Array {
    let decodedValues: Uint32Array;
    switch (streamMetadata.logicalLevelTechnique1) {
        case LogicalLevelTechnique.DELTA:
            if (streamMetadata.logicalLevelTechnique2 === LogicalLevelTechnique.RLE) {
                const rleMetadata = streamMetadata as RleEncodedStreamMetadata;
                const deltaValues = decodeUnsignedRleInt32(values, rleMetadata.runs, rleMetadata.numRleValues);
                decodedValues = decodeUnsignedZigZagDeltaInt32(deltaValues);
            } else {
                decodedValues = decodeUnsignedZigZagDeltaInt32(values);
            }
            break;
        case LogicalLevelTechnique.RLE:
            decodedValues = decodeUnsignedRleInt32(
                values,
                (streamMetadata as RleEncodedStreamMetadata).runs,
                (streamMetadata as RleEncodedStreamMetadata).numRleValues,
            );
            break;
        case LogicalLevelTechnique.MORTON:
            fastInverseDelta(values);
            decodedValues = values;
            break;
        case LogicalLevelTechnique.COMPONENTWISE_DELTA:
            if (scalingData && !nullabilityBuffer) {
                decodedValues = decodeUnsignedComponentwiseDeltaVec2Scaled(
                    values,
                    scalingData.scale,
                    scalingData.min,
                    scalingData.max,
                );
            } else {
                decodedValues = decodeUnsignedComponentwiseDeltaVec2(values);
            }
            break;
        case LogicalLevelTechnique.NONE:
            decodedValues = values;
            break;
        default:
            throw new Error(
                `The specified Logical level technique is not supported: ${streamMetadata.logicalLevelTechnique1}`,
            );
    }

    if (nullabilityBuffer) {
        return unpackNullable(decodedValues, nullabilityBuffer, 0);
    }
    return decodedValues;
}

function decodeSignedInt64(
    values: BigUint64Array,
    streamMetadata: StreamMetadata,
    nullabilityBuffer?: BitVector,
): BigInt64Array {
    let decodedValues: BigInt64Array;
    switch (streamMetadata.logicalLevelTechnique1) {
        case LogicalLevelTechnique.DELTA:
            if (streamMetadata.logicalLevelTechnique2 === LogicalLevelTechnique.RLE) {
                const rleMetadata = streamMetadata as RleEncodedStreamMetadata;
                if (!nullabilityBuffer) {
                    return decodeDeltaRleInt64(values, rleMetadata.runs, rleMetadata.numRleValues);
                }
                values = decodeUnsignedRleInt64(values, rleMetadata.runs, rleMetadata.numRleValues);
                decodedValues = decodeZigZagDeltaInt64(values);
            } else {
                decodedValues = decodeZigZagDeltaInt64(values);
            }
            break;
        case LogicalLevelTechnique.RLE:
            decodedValues = decodeZigZagRleInt64(
                values,
                (streamMetadata as RleEncodedStreamMetadata).runs,
                (streamMetadata as RleEncodedStreamMetadata).numRleValues,
            );
            break;
        case LogicalLevelTechnique.NONE:
            decodedValues = decodeZigZagInt64(values);
            break;
        default:
            throw new Error(
                `The specified Logical level technique is not supported: ${streamMetadata.logicalLevelTechnique1}`,
            );
    }

    if (nullabilityBuffer) {
        return unpackNullable(decodedValues, nullabilityBuffer, 0n);
    }
    return decodedValues;
}

function decodeUnsignedInt64(
    values: BigUint64Array,
    streamMetadata: StreamMetadata,
    nullabilityBuffer?: BitVector,
): BigUint64Array {
    let decodedValues: BigUint64Array;
    switch (streamMetadata.logicalLevelTechnique1) {
        case LogicalLevelTechnique.DELTA:
            if (streamMetadata.logicalLevelTechnique2 === LogicalLevelTechnique.RLE) {
                const rleMetadata = streamMetadata as RleEncodedStreamMetadata;
                const deltaValues = decodeUnsignedRleInt64(values, rleMetadata.runs, rleMetadata.numRleValues);
                decodedValues = decodeUnsignedZigZagDeltaInt64(deltaValues);
            } else {
                decodedValues = decodeUnsignedZigZagDeltaInt64(values);
            }
            break;
        case LogicalLevelTechnique.RLE:
            decodedValues = decodeUnsignedRleInt64(
                values,
                (streamMetadata as RleEncodedStreamMetadata).runs,
                (streamMetadata as RleEncodedStreamMetadata).numRleValues,
            );
            break;
        case LogicalLevelTechnique.NONE:
            decodedValues = values;
            break;
        default:
            throw new Error(
                `The specified Logical level technique is not supported: ${streamMetadata.logicalLevelTechnique1}`,
            );
    }

    if (nullabilityBuffer) {
        return unpackNullable(decodedValues, nullabilityBuffer, 0n);
    }
    return decodedValues;
}

function decodeFloat64Values(values: Float64Array, streamMetadata: StreamMetadata, isSigned: boolean): Float64Array {
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

function decodeLengthToOffsetBuffer(values: Uint32Array, streamMetadata: StreamMetadata): Uint32Array {
    if (
        streamMetadata.logicalLevelTechnique1 === LogicalLevelTechnique.DELTA &&
        streamMetadata.logicalLevelTechnique2 === LogicalLevelTechnique.NONE
    ) {
        return decodeZigZagDeltaOfDeltaInt32(values);
    }

    if (
        streamMetadata.logicalLevelTechnique1 === LogicalLevelTechnique.RLE &&
        streamMetadata.logicalLevelTechnique2 === LogicalLevelTechnique.NONE
    ) {
        const rleMetadata = streamMetadata as RleEncodedStreamMetadata;
        return decodeRleDeltaInt32(values, rleMetadata.runs, rleMetadata.numRleValues);
    }

    if (
        streamMetadata.logicalLevelTechnique1 === LogicalLevelTechnique.NONE &&
        streamMetadata.logicalLevelTechnique2 === LogicalLevelTechnique.NONE
    ) {
        //TODO: use fastInverseDelta again and check what are the performance problems in zoom 14
        //fastInverseDelta(values);
        inverseDelta(values);
        const offsets = new Uint32Array(streamMetadata.numValues + 1);
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
        return new Uint32Array(decodedValues);
    }

    throw new Error("Only delta encoding is supported for transforming length to offset streams yet.");
}

export function getVectorType(
    streamMetadata: StreamMetadata,
    sizeOrNullabilityBuffer: number | BitVector,
    data: Uint8Array,
    offset: IntWrapper,
    options?: { varintWidth?: "int32" | "int64" },
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

    if (streamMetadata.physicalLevelTechnique === PhysicalLevelTechnique.VARINT) {
        if (isDeltaRleSequenceVarintWidth(data, offset, options?.varintWidth ?? "int32")) {
            return VectorType.SEQUENCE;
        }
        return streamMetadata.numValues === 1 ? VectorType.CONST : VectorType.FLAT;
    }

    const byteOffset = offset.get();
    const values = new Int32Array(data.buffer, data.byteOffset + byteOffset, 4);
    offset.set(savedOffset);
    // Check if both deltas are encoded 1
    const zigZagOne = 2;
    if (values[2] === zigZagOne && values[3] === zigZagOne) {
        return VectorType.SEQUENCE;
    }
    return streamMetadata.numValues === 1 ? VectorType.CONST : VectorType.FLAT;
}

function isDeltaRleSequenceVarintWidth(data: Uint8Array, offset: IntWrapper, varintWidth: "int32" | "int64"): boolean {
    const savedOffset = offset.get();

    if (varintWidth === "int64") {
        const values = decodeVarintInt64(data, offset, 4);
        offset.set(savedOffset);
        return values[2] === 2n && values[3] === 2n;
    }

    const values = decodeVarintInt32(data, offset, 4);
    offset.set(savedOffset);
    return values[2] === 2 && values[3] === 2;
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
