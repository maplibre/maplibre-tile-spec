import IntWrapper from "./intWrapper";
import { Column, ScalarColumn, ScalarType } from "../metadata/tileset/tilesetMetadata.g";
import Vector from "../vector/vector";
import BitVector from "../vector/flat/bitVector";
import { StreamMetadataDecoder } from "../metadata/tile/streamMetadataDecoder";
import { VectorType } from "../vector/vectorType";
import { BooleanFlatVector } from "../vector/flat/booleanFlatVector";
import { DoubleFlatVector } from "../vector/flat/doubleFlatVector";
import { FloatFlatVector } from "../vector/flat/floatFlatVector";
import { LongConstVector } from "../vector/constant/longConstVector";
import { LongFlatVector } from "../vector/flat/longFlatVector";
import { IntFlatVector } from "../vector/flat/intFlatVector";
import { IntConstVector } from "../vector/constant/intConstVector";
import {
    decodeBooleanRle,
    decodeDoublesLE,
    decodeFloatsLE,
    decodeNullableBooleanRle,
    decodeNullableDoublesLE,
    decodeNullableFloatsLE,
    skipColumn,
} from "./decodingUtils";
import IntegerStreamDecoder from "./integerStreamDecoder";
import { StringDecoder } from "./stringDecoder";
import { IntSequenceVector } from "../vector/sequence/intSequenceVector";
import { RleEncodedStreamMetadata } from "../metadata/tile/rleEncodedStreamMetadata";
import { LongSequenceVector } from "../vector/sequence/longSequenceVector";

export function decodePropertyColumn(
    data: Uint8Array,
    offset: IntWrapper,
    columnMetadata: Column,
    numStreams: number,
    numFeatures: number,
    propertyColumnNames?: Set<string>,
): Vector | Vector[] {
    if (columnMetadata.type.case === "scalarType") {
        if (propertyColumnNames && !propertyColumnNames.has(columnMetadata.name)) {
            skipColumn(numStreams, data, offset);
            return null;
        }

        return decodeScalarPropertyColumn(numStreams, data, offset, numFeatures, columnMetadata.type.value, columnMetadata);
    }

    if (numStreams != 1) {
        return null;
    }

    return StringDecoder.decodeSharedDictionary(data, offset, columnMetadata, numFeatures, propertyColumnNames);
}

function decodeScalarPropertyColumn(
    numStreams: number,
    data: Uint8Array,
    offset: IntWrapper,
    numFeatures: number,
    column: ScalarColumn,
    columnMetadata: Column,
) {
    let nullabilityBuffer: BitVector = null;
    let numValues = 0;
    if (numStreams === 0) {
        /* Skip since this column has no values */
        return null;
    }

    // Read nullability stream if column is nullable
    if (columnMetadata.nullable) {
        const presentStreamMetadata = StreamMetadataDecoder.decode(data, offset);
        numValues = presentStreamMetadata.numValues;
        const streamDataStart = offset.get();
        // Decode the RLE boolean data
        const presentVector = decodeBooleanRle(data, numValues, offset);
        // FIX: decodeBooleanRle doesn't consume all bytes!
        // We must advance to the end of the stream using byteLength from metadata
        offset.set(streamDataStart + presentStreamMetadata.byteLength);
        nullabilityBuffer = new BitVector(presentVector, presentStreamMetadata.numValues);
    }

    const sizeOrNullabilityBuffer = nullabilityBuffer ?? numFeatures;
    const scalarType = column.type.value as ScalarType;
    switch (scalarType) {
        case ScalarType.UINT_32:
        case ScalarType.INT_32:
            return decodeIntColumn(data, offset, columnMetadata, column, sizeOrNullabilityBuffer);
        case ScalarType.STRING:
            // In embedded format: numStreams includes nullability stream if column is nullable
            const stringDataStreams = columnMetadata.nullable ? numStreams - 1 : numStreams;
            return StringDecoder.decode(columnMetadata.name, data, offset, stringDataStreams, nullabilityBuffer);
        case ScalarType.BOOLEAN:
            return decodeBooleanColumn(data, offset, columnMetadata, numFeatures, sizeOrNullabilityBuffer);
        case ScalarType.UINT_64:
        case ScalarType.INT_64:
            return decodeLongColumn(data, offset, columnMetadata, sizeOrNullabilityBuffer, column);
        case ScalarType.FLOAT:
            return decodeFloatColumn(data, offset, columnMetadata, sizeOrNullabilityBuffer);
        case ScalarType.DOUBLE:
            return decodeDoubleColumn(data, offset, columnMetadata, sizeOrNullabilityBuffer);
        default:
            throw new Error(`The specified data type for the field is currently not supported: ${column}`);
    }
}

function decodeBooleanColumn(
    data: Uint8Array,
    offset: IntWrapper,
    column: Column,
    numFeatures: number,
    sizeOrNullabilityBuffer: number | BitVector,
): BooleanFlatVector {
    const dataStreamMetadata = StreamMetadataDecoder.decode(data, offset);
    const numValues = dataStreamMetadata.numValues;
    const streamDataStart = offset.get();
    const dataStream = isNullabilityBuffer(sizeOrNullabilityBuffer)
        ? decodeNullableBooleanRle(data, numValues, offset, sizeOrNullabilityBuffer)
        : decodeBooleanRle(data, numValues, offset);
    // TODO: refactor decodeNullableBooleanRle
    // Fix offset: RLE decoders don't consume all compressed bytes
    offset.set(streamDataStart + dataStreamMetadata.byteLength);
    const dataVector = new BitVector(dataStream, numValues);
    return new BooleanFlatVector(column.name, dataVector, sizeOrNullabilityBuffer);
}

function decodeFloatColumn(
    data: Uint8Array,
    offset: IntWrapper,
    column: Column,
    sizeOrNullabilityBuffer: number | BitVector,
): FloatFlatVector {
    const dataStreamMetadata = StreamMetadataDecoder.decode(data, offset);
    const dataStream = isNullabilityBuffer(sizeOrNullabilityBuffer)
        ? decodeNullableFloatsLE(data, offset, sizeOrNullabilityBuffer, dataStreamMetadata.numValues)
        : decodeFloatsLE(data, offset, dataStreamMetadata.numValues);
    return new FloatFlatVector(column.name, dataStream, sizeOrNullabilityBuffer);
}

function decodeDoubleColumn(
    data: Uint8Array,
    offset: IntWrapper,
    column: Column,
    sizeOrNullabilityBuffer: number | BitVector,
): DoubleFlatVector {
    const dataStreamMetadata = StreamMetadataDecoder.decode(data, offset);
    const dataStream = isNullabilityBuffer(sizeOrNullabilityBuffer)
        ? decodeNullableDoublesLE(data, offset, sizeOrNullabilityBuffer, dataStreamMetadata.numValues)
        : decodeDoublesLE(data, offset, dataStreamMetadata.numValues);
    return new DoubleFlatVector(column.name, dataStream, sizeOrNullabilityBuffer);
}

function decodeLongColumn(
    data: Uint8Array,
    offset: IntWrapper,
    column: Column,
    sizeOrNullabilityBuffer: number | BitVector,
    scalarColumn: ScalarColumn,
): Vector<BigInt64Array, bigint> {
    const dataStreamMetadata = StreamMetadataDecoder.decode(data, offset);
    const vectorType = IntegerStreamDecoder.getVectorType(dataStreamMetadata, sizeOrNullabilityBuffer);
    const isSigned = scalarColumn.type.value === ScalarType.INT_64;
    if (vectorType === VectorType.FLAT) {
        const dataStream = isNullabilityBuffer(sizeOrNullabilityBuffer)
            ? IntegerStreamDecoder.decodeNullableLongStream(
                  data,
                  offset,
                  dataStreamMetadata,
                  isSigned,
                  sizeOrNullabilityBuffer,
              )
            : IntegerStreamDecoder.decodeLongStream(data, offset, dataStreamMetadata, isSigned);
        return new LongFlatVector(column.name, dataStream, sizeOrNullabilityBuffer);
    } else if (vectorType === VectorType.SEQUENCE) {
        const id = IntegerStreamDecoder.decodeSequenceLongStream(data, offset, dataStreamMetadata);
        return new LongSequenceVector(
            column.name,
            id[0],
            id[1],
            (dataStreamMetadata as RleEncodedStreamMetadata).numRleValues,
        );
    } else {
        const constValue = IntegerStreamDecoder.decodeConstLongStream(data, offset, dataStreamMetadata, isSigned);
        return new LongConstVector(column.name, constValue, sizeOrNullabilityBuffer);
    }
}

function decodeIntColumn(
    data: Uint8Array,
    offset: IntWrapper,
    column: Column,
    scalarColumn: ScalarColumn,
    sizeOrNullabilityBuffer: number | BitVector,
): Vector<Int32Array, number> {
    const dataStreamMetadata = StreamMetadataDecoder.decode(data, offset);
    const vectorType = IntegerStreamDecoder.getVectorType(dataStreamMetadata, sizeOrNullabilityBuffer);
    const isSigned = scalarColumn.type.value === ScalarType.INT_32;

    if (vectorType === VectorType.FLAT) {
        const dataStream = isNullabilityBuffer(sizeOrNullabilityBuffer)
            ? IntegerStreamDecoder.decodeNullableIntStream(
                  data,
                  offset,
                  dataStreamMetadata,
                  isSigned,
                  sizeOrNullabilityBuffer,
              )
            : IntegerStreamDecoder.decodeIntStream(data, offset, dataStreamMetadata, isSigned);
        return new IntFlatVector(column.name, dataStream, sizeOrNullabilityBuffer);
    } else if (vectorType === VectorType.SEQUENCE) {
        const id = IntegerStreamDecoder.decodeSequenceIntStream(data, offset, dataStreamMetadata);
        return new IntSequenceVector(
            column.name,
            id[0],
            id[1],
            (dataStreamMetadata as RleEncodedStreamMetadata).numRleValues,
        );
    } else {
        const constValue = IntegerStreamDecoder.decodeConstIntStream(data, offset, dataStreamMetadata, isSigned);
        return new IntConstVector(column.name, constValue, sizeOrNullabilityBuffer);
    }
}

function isNullabilityBuffer(sizeOrNullabilityBuffer: number | BitVector): sizeOrNullabilityBuffer is BitVector {
    return sizeOrNullabilityBuffer instanceof BitVector;
}
