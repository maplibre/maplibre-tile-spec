import IntWrapper from "./intWrapper";
import {Column, ScalarColumn, ScalarType} from "../metadata/tileset/tilesetMetadata";
import Vector from "../vector/vector";
import BitVector from "../vector/flat/bitVector";
import {StreamMetadataDecoder} from "../metadata/tile/streamMetadataDecoder";
import {VectorType} from "../vector/vectorType";
import {BooleanFlatVector} from "../vector/flat/booleanFlatVector";
import {DoubleFlatVector} from "../vector/flat/doubleFlatVector";
import {FloatFlatVector} from "../vector/flat/floatFlatVector";
import {LongConstVector} from "../vector/constant/longConstVector";
import {LongFlatVector} from "../vector/flat/longFlatVector";
import {IntFlatVector} from "../vector/flat/intFlatVector";
import {IntConstVector} from "../vector/constant/intConstVector";
import {
    decodeBooleanRle, decodeDoublesLE,
    decodeFloatsLE,
    decodeNullableBooleanRle, decodeNullableDoublesLE,
    decodeNullableFloatsLE,
    skipColumn
} from "./decodingUtils";
import IntegerStreamDecoder from "./integerStreamDecoder";
import {StringDecoder} from "./stringDecoder";


export function decodePropertyColumn(data: Uint8Array, offset: IntWrapper, columnMetadata: Column, numStreams: number,
                                     numFeatures: number, propertyColumnNames?: Set<string>): Vector | Vector[] {
        const column = columnMetadata.type.value;
        if (column instanceof ScalarColumn) {
            if(propertyColumnNames && !propertyColumnNames.has(columnMetadata.name)){
                skipColumn(numStreams, data, offset);
                return null;
            }

            return decodeScalarPropertyColumn(numStreams, data, offset, numFeatures, column, columnMetadata);
        }

        if(numStreams != 1){
            return null;
        }

        return StringDecoder.decodeSharedDictionary(data, offset, columnMetadata, numFeatures, propertyColumnNames);
 }

function decodeScalarPropertyColumn(numStreams: number, data: Uint8Array, offset: IntWrapper, numFeatures: number, column: ScalarColumn, columnMetadata: Column) {
    let nullabilityBuffer: BitVector = null;
    let numValues = 0;
    if (numStreams === 0) {
        /* Skip since this column has no values */
        return null;
    } else if (numStreams > 1) {
        const presentStreamMetadata = StreamMetadataDecoder.decode(data, offset);
        numValues = presentStreamMetadata.numValues;
        const presentVector = decodeBooleanRle(data, numValues, offset);
        nullabilityBuffer = new BitVector(presentVector, presentStreamMetadata.numValues);
    }
    const sizeOrNullabilityBuffer = nullabilityBuffer ?? numFeatures;
    const scalarType = column.type.value as ScalarType;
    switch (scalarType) {
        case ScalarType.UINT_32:
        case ScalarType.INT_32:
            return decodeIntColumn(data, offset, columnMetadata, column, sizeOrNullabilityBuffer);
        case ScalarType.STRING:
            return StringDecoder.decode(columnMetadata.name, data, offset, numStreams - 1,
                nullabilityBuffer);
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

function decodeBooleanColumn(data: Uint8Array, offset: IntWrapper, column: Column, numFeatures: number,
                             sizeOrNullabilityBuffer: number | BitVector): BooleanFlatVector {
    const dataStreamMetadata = StreamMetadataDecoder.decode(data, offset);
    const numValues = dataStreamMetadata.numValues;
    const dataStream = isNullabilityBuffer(sizeOrNullabilityBuffer)?
        decodeNullableBooleanRle(data, numValues, offset, sizeOrNullabilityBuffer) :
        decodeBooleanRle(data, numValues, offset);
    const dataVector = new BitVector(dataStream, numValues);
    return new BooleanFlatVector(column.name, dataVector, sizeOrNullabilityBuffer);
}

function decodeFloatColumn(data: Uint8Array, offset: IntWrapper, column: Column,
                           sizeOrNullabilityBuffer: number | BitVector): FloatFlatVector {
    const dataStreamMetadata = StreamMetadataDecoder.decode(data, offset);
    const dataStream = isNullabilityBuffer(sizeOrNullabilityBuffer)?
        decodeNullableFloatsLE(data, offset, sizeOrNullabilityBuffer):
        decodeFloatsLE(data, offset, dataStreamMetadata.numValues);
    return new FloatFlatVector(column.name, dataStream, sizeOrNullabilityBuffer);
}

function decodeDoubleColumn(data: Uint8Array, offset: IntWrapper, column: Column,
                            sizeOrNullabilityBuffer: number | BitVector): DoubleFlatVector {
    const dataStreamMetadata = StreamMetadataDecoder.decode(data, offset);
    const dataStream = isNullabilityBuffer(sizeOrNullabilityBuffer)?
        decodeNullableDoublesLE(data, offset, sizeOrNullabilityBuffer):
        decodeDoublesLE(data, offset, dataStreamMetadata.numValues);
    return new DoubleFlatVector(column.name, dataStream, sizeOrNullabilityBuffer);
}

function decodeLongColumn(data: Uint8Array, offset: IntWrapper, column: Column,
                          sizeOrNullabilityBuffer: number | BitVector, scalarColumn: ScalarColumn): Vector<BigInt64Array, bigint> {
    const dataStreamMetadata = StreamMetadataDecoder.decode(data, offset);
    const vectorType = IntegerStreamDecoder.getVectorTypeIntStream(dataStreamMetadata);
    const isSigned = scalarColumn.type.value === ScalarType.INT_64;
    if (vectorType === VectorType.FLAT) {
        const dataStream = isNullabilityBuffer(sizeOrNullabilityBuffer) ?
            IntegerStreamDecoder.decodeNullableLongStream(data, offset, dataStreamMetadata, isSigned, sizeOrNullabilityBuffer) :
            IntegerStreamDecoder.decodeLongStream(data, offset, dataStreamMetadata, isSigned);
        return new LongFlatVector(column.name, dataStream, sizeOrNullabilityBuffer);
    } else {
        const constValue = IntegerStreamDecoder.decodeConstLongStream(data, offset, dataStreamMetadata, isSigned);
        return new LongConstVector(column.name, constValue, sizeOrNullabilityBuffer);
    }
}

function decodeIntColumn(data: Uint8Array, offset: IntWrapper, column: Column, scalarColumn: ScalarColumn,
                         sizeOrNullabilityBuffer: number | BitVector): Vector<Int32Array, number> {
    const dataStreamMetadata = StreamMetadataDecoder.decode(data, offset);
    const vectorType = IntegerStreamDecoder.getVectorTypeIntStream(dataStreamMetadata);
    const isSigned = scalarColumn.type.value === ScalarType.INT_32;

    if (vectorType === VectorType.FLAT) {
        const dataStream = isNullabilityBuffer(sizeOrNullabilityBuffer)?
            IntegerStreamDecoder.decodeNullableIntStream(data, offset, dataStreamMetadata, isSigned, sizeOrNullabilityBuffer):
            IntegerStreamDecoder.decodeIntStream(data, offset, dataStreamMetadata, isSigned);
        return new IntFlatVector(column.name, dataStream, sizeOrNullabilityBuffer);
    } else {
        const constValue = IntegerStreamDecoder.decodeConstIntStream(data, offset, dataStreamMetadata, isSigned);
        return new IntConstVector(column.name, constValue, sizeOrNullabilityBuffer);
    }
}

function isNullabilityBuffer(sizeOrNullabilityBuffer: number | BitVector): sizeOrNullabilityBuffer is BitVector{
    return sizeOrNullabilityBuffer  instanceof BitVector;
}

/*export function decodePropertyColumnSequential(data: Uint8Array, offset: IntWrapper, columnMetadata: Column, numStreams: number, numFeatures: number): Vector | Vector[] {
    let presentStreamMetadata: StreamMetadata;
    const column = columnMetadata.type.value;
    if (column instanceof ScalarColumn) {
        let nullabilityBuffer: BitVector = null;
        let numValues = 0;
        if (numStreams === 0) {
            return null;
        } else if (numStreams > 1) {
            presentStreamMetadata = StreamMetadataDecoder.decode(data, offset);
            const vectorType = getVectorTypeBooleanStream(numFeatures, presentStreamMetadata.byteLength, data, offset);
            if (vectorType === VectorType.FLAT) {
                numValues = presentStreamMetadata.numValues;
                const presentVector = decodeBooleanRle(data, numValues, offset);
                nullabilityBuffer = new BitVector(presentVector, presentStreamMetadata.numValues);
            } else {
                offset.add(presentStreamMetadata.byteLength);
            }
        }

        const scalarType = column.type.value as ScalarType;
        switch (scalarType) {
            case ScalarType.BOOLEAN:
                return decodeBooleanColumn(data, offset, columnMetadata, numFeatures, null);
            case ScalarType.UINT_32:
            case ScalarType.INT_32:
                return decodeIntColumn(data, offset, columnMetadata, column, null);
            case ScalarType.UINT_64:
            case ScalarType.INT_64:
                return decodeLongColumn(data, offset, columnMetadata, null, column);
            case ScalarType.FLOAT:
                return decodeFloatColumn(data, offset, columnMetadata, null, numFeatures);
            case ScalarType.DOUBLE:
                return decodeDoubleColumn(data, offset, columnMetadata, null, numFeatures);
            case ScalarType.STRING:
                return StringDecoder.decode(columnMetadata.name, data, offset, numStreams - 1, null);
            default:
                throw new Error(`The specified data type for the field is currently not supported: ${column}`);
        }
    }

    if (numStreams === 1) {
        throw new Error("Present stream currently not supported for Structs.");
    }

    return StringDecoder.decodeSharedDictionarySequential(data, offset, columnMetadata);
}*/
