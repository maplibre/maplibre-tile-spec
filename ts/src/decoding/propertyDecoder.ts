import type IntWrapper from "./intWrapper";
import {
  type Column,
  type ScalarColumn,
  ScalarType,
} from "../metadata/tileset/tilesetMetadata";
import type Vector from "../vector/vector";
import BitVector from "../vector/flat/bitVector";
import {
  decodeStreamMetadata,
  type RleEncodedStreamMetadata,
} from "../metadata/tile/streamMetadataDecoder";
import { VectorType } from "../vector/vectorType";
import { BooleanFlatVector } from "../vector/flat/booleanFlatVector";
import { DoubleFlatVector } from "../vector/flat/doubleFlatVector";
import { FloatFlatVector } from "../vector/flat/floatFlatVector";
import { Int64ConstVector } from "../vector/constant/int64ConstVector";
import { Int64FlatVector } from "../vector/flat/int64FlatVector";
import { Int32FlatVector } from "../vector/flat/int32FlatVector";
import { Int32ConstVector } from "../vector/constant/int32ConstVector";
import {
  decodeBooleanRle,
  decodeDoublesLE,
  decodeFloatsLE,
  skipColumn,
} from "./decodingUtils";
import {
  decodeSignedConstInt32Stream,
  decodeSignedConstInt64Stream,
  decodeSignedInt32Stream,
  decodeSignedInt64Stream,
  decodeUnsignedInt32Stream,
  decodeUnsignedConstInt32Stream,
  decodeUnsignedConstInt64Stream,
  decodeUnsignedInt64Stream,
  decodeSequenceInt32Stream,
  decodeSequenceInt64Stream,
  getVectorType,
} from "./integerStreamDecoder";
import { Int32SequenceVector } from "../vector/sequence/int32SequenceVector";
import { Int64SequenceVector } from "../vector/sequence/int64SequenceVector";
import { decodeSharedDictionary, decodeString } from "./stringDecoder";

export function decodePropertyColumn(
  data: Uint8Array,
  offset: IntWrapper,
  columnMetadata: Column,
  numStreams: number,
  numFeatures: number,
  propertyColumnNames?: Set<string>,
): Vector | Vector[] {
  if (columnMetadata.type === "scalarType") {
    if (propertyColumnNames && !propertyColumnNames.has(columnMetadata.name)) {
      skipColumn(numStreams, data, offset);
      return null;
    }

    return decodeScalarPropertyColumn(
      numStreams,
      data,
      offset,
      numFeatures,
      columnMetadata.scalarType,
      columnMetadata,
    );
  }

  if (numStreams === 0) {
    return null;
  }

  return decodeSharedDictionary(
    data,
    offset,
    columnMetadata,
    numFeatures,
    propertyColumnNames,
  );
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
  if (numStreams === 0) {
    return null;
  }

  if (columnMetadata.nullable) {
    const presentStreamMetadata = decodeStreamMetadata(data, offset);
    const numValues = presentStreamMetadata.numValues;
    const streamDataStart = offset.get();
    const presentVector = decodeBooleanRle(
      data,
      numValues,
      presentStreamMetadata.byteLength,
      offset,
    );
    offset.set(streamDataStart + presentStreamMetadata.byteLength);
    nullabilityBuffer = new BitVector(
      presentVector,
      presentStreamMetadata.numValues,
    );
  }

  const sizeOrNullabilityBuffer = nullabilityBuffer ?? numFeatures;
  const scalarType = column.physicalType;
  switch (scalarType) {
    case ScalarType.UINT_32:
    case ScalarType.INT_32:
      return decodeInt32Column(
        data,
        offset,
        columnMetadata,
        column,
        sizeOrNullabilityBuffer,
      );
    case ScalarType.STRING: {
      // In embedded format: numStreams includes nullability stream if column is nullable
      const stringDataStreams = columnMetadata.nullable
        ? numStreams - 1
        : numStreams;
      return decodeString(
        columnMetadata.name,
        data,
        offset,
        stringDataStreams,
        nullabilityBuffer,
      );
    }
    case ScalarType.BOOLEAN:
      return decodeBooleanColumn(
        data,
        offset,
        columnMetadata,
        numFeatures,
        sizeOrNullabilityBuffer,
      );
    case ScalarType.UINT_64:
    case ScalarType.INT_64:
      return decodeInt64Column(
        data,
        offset,
        columnMetadata,
        sizeOrNullabilityBuffer,
        column,
      );
    case ScalarType.FLOAT:
      return decodeFloatColumn(
        data,
        offset,
        columnMetadata,
        sizeOrNullabilityBuffer,
      );
    case ScalarType.DOUBLE:
      return decodeDoubleColumn(
        data,
        offset,
        columnMetadata,
        sizeOrNullabilityBuffer,
      );
    default:
      throw new Error(
        `The specified data type for the field is currently not supported: ${column}`,
      );
  }
}

function decodeBooleanColumn(
  data: Uint8Array,
  offset: IntWrapper,
  column: Column,
  _numFeatures: number,
  sizeOrNullabilityBuffer: number | BitVector,
): BooleanFlatVector {
  const dataStreamMetadata = decodeStreamMetadata(data, offset);
  const numValues = dataStreamMetadata.numValues;
  const streamDataStart = offset.get();
  const nullabilityBuffer = isNullabilityBuffer(sizeOrNullabilityBuffer)
    ? sizeOrNullabilityBuffer
    : undefined;
  const dataStream = decodeBooleanRle(
    data,
    numValues,
    dataStreamMetadata.byteLength,
    offset,
    nullabilityBuffer,
  );
  offset.set(streamDataStart + dataStreamMetadata.byteLength);
  const dataVector = new BitVector(dataStream, numValues);
  return new BooleanFlatVector(
    column.name,
    dataVector,
    sizeOrNullabilityBuffer,
  );
}

function decodeFloatColumn(
  data: Uint8Array,
  offset: IntWrapper,
  column: Column,
  sizeOrNullabilityBuffer: number | BitVector,
): FloatFlatVector {
  const dataStreamMetadata = decodeStreamMetadata(data, offset);
  const nullabilityBuffer = isNullabilityBuffer(sizeOrNullabilityBuffer)
    ? sizeOrNullabilityBuffer
    : undefined;
  const dataStream = decodeFloatsLE(
    data,
    offset,
    dataStreamMetadata.numValues,
    nullabilityBuffer,
  );
  return new FloatFlatVector(column.name, dataStream, sizeOrNullabilityBuffer);
}

function decodeDoubleColumn(
  data: Uint8Array,
  offset: IntWrapper,
  column: Column,
  sizeOrNullabilityBuffer: number | BitVector,
): DoubleFlatVector {
  const dataStreamMetadata = decodeStreamMetadata(data, offset);
  const nullabilityBuffer = isNullabilityBuffer(sizeOrNullabilityBuffer)
    ? sizeOrNullabilityBuffer
    : undefined;
  const dataStream = decodeDoublesLE(
    data,
    offset,
    dataStreamMetadata.numValues,
    nullabilityBuffer,
  );
  return new DoubleFlatVector(column.name, dataStream, sizeOrNullabilityBuffer);
}

function decodeInt64Column(
  data: Uint8Array,
  offset: IntWrapper,
  column: Column,
  sizeOrNullabilityBuffer: number | BitVector,
  scalarColumn: ScalarColumn,
): Vector<BigInt64Array | BigUint64Array, bigint> {
  const dataStreamMetadata = decodeStreamMetadata(data, offset);
  const vectorType = getVectorType(
    dataStreamMetadata,
    sizeOrNullabilityBuffer,
    data,
    offset,
  );
  const isSigned = scalarColumn.physicalType === ScalarType.INT_64;
  if (vectorType === VectorType.FLAT) {
    const nullabilityBuffer = isNullabilityBuffer(sizeOrNullabilityBuffer)
      ? sizeOrNullabilityBuffer
      : undefined;
    const dataStream = isSigned
      ? decodeSignedInt64Stream(
          data,
          offset,
          dataStreamMetadata,
          nullabilityBuffer,
        )
      : decodeUnsignedInt64Stream(
          data,
          offset,
          dataStreamMetadata,
          nullabilityBuffer,
        );
    return new Int64FlatVector(
      column.name,
      dataStream,
      sizeOrNullabilityBuffer,
    );
  }
  if (vectorType === VectorType.SEQUENCE) {
    const id = decodeSequenceInt64Stream(data, offset, dataStreamMetadata);
    return new Int64SequenceVector(
      column.name,
      id[0],
      id[1],
      (dataStreamMetadata as RleEncodedStreamMetadata).numRleValues,
    );
  }
  const constValue = isSigned
    ? decodeSignedConstInt64Stream(data, offset, dataStreamMetadata)
    : decodeUnsignedConstInt64Stream(data, offset, dataStreamMetadata);
  return new Int64ConstVector(
    column.name,
    constValue,
    sizeOrNullabilityBuffer,
    isSigned,
  );
}

function decodeInt32Column(
  data: Uint8Array,
  offset: IntWrapper,
  column: Column,
  scalarColumn: ScalarColumn,
  sizeOrNullabilityBuffer: number | BitVector,
): Vector<Int32Array | Uint32Array, number> {
  const dataStreamMetadata = decodeStreamMetadata(data, offset);
  const vectorType = getVectorType(
    dataStreamMetadata,
    sizeOrNullabilityBuffer,
    data,
    offset,
  );
  const isSigned = scalarColumn.physicalType === ScalarType.INT_32;

  if (vectorType === VectorType.FLAT) {
    const nullabilityBuffer = isNullabilityBuffer(sizeOrNullabilityBuffer)
      ? sizeOrNullabilityBuffer
      : undefined;
    const dataStream = isSigned
      ? decodeSignedInt32Stream(
          data,
          offset,
          dataStreamMetadata,
          undefined,
          nullabilityBuffer,
        )
      : decodeUnsignedInt32Stream(
          data,
          offset,
          dataStreamMetadata,
          undefined,
          nullabilityBuffer,
        );
    return new Int32FlatVector(
      column.name,
      dataStream,
      sizeOrNullabilityBuffer,
    );
  }
  if (vectorType === VectorType.SEQUENCE) {
    const id = decodeSequenceInt32Stream(data, offset, dataStreamMetadata);
    return new Int32SequenceVector(
      column.name,
      id[0],
      id[1],
      (dataStreamMetadata as RleEncodedStreamMetadata).numRleValues,
    );
  }
  const constValue = isSigned
    ? decodeSignedConstInt32Stream(data, offset, dataStreamMetadata)
    : decodeUnsignedConstInt32Stream(data, offset, dataStreamMetadata);
  return new Int32ConstVector(
    column.name,
    constValue,
    sizeOrNullabilityBuffer,
    isSigned,
  );
}

function isNullabilityBuffer(
  sizeOrNullabilityBuffer: number | BitVector,
): sizeOrNullabilityBuffer is BitVector {
  return sizeOrNullabilityBuffer instanceof BitVector;
}
