import { LogicalLevelTechnique } from "../metadata/tile/logicalLevelTechnique";
import { PhysicalLevelTechnique } from "../metadata/tile/physicalLevelTechnique";
import { PhysicalStreamType } from "../metadata/tile/physicalStreamType";
import { LogicalStreamType } from "../metadata/tile/logicalStreamType";
import { DictionaryType } from "../metadata/tile/dictionaryType";
import type { StreamMetadata, RleEncodedStreamMetadata } from "../metadata/tile/streamMetadataDecoder";
import IntWrapper from "../decoding/intWrapper";
import {
    encodeVarintInt32Array,
    encodeVarintInt64Array,
    encodeSingleVarintInt32,
    encodeZigZag32,
    encodeZigZag64,
    encodeBooleanRle,
    encodeFloatsLE,
    encodeDoubleLE,
} from "./encodingUtils";

/**
 * Encodes INT_32 values with NONE encoding (no delta, no RLE)
 */
export function encodeInt32None(values: Int32Array): Uint8Array {
    const zigzagEncoded = new Int32Array(values.length);
    for (let i = 0; i < values.length; i++) {
        zigzagEncoded[i] = encodeZigZag32(values[i]);
    }
    const encodedData = encodeVarintInt32Array(zigzagEncoded);
    const streamMetadata = createStreamMetadata(LogicalLevelTechnique.NONE, LogicalLevelTechnique.NONE, values.length);
    return buildEncodedStream(streamMetadata, encodedData);
}

/**
 * Encodes INT_32 values with DELTA encoding
 */
export function encodeInt32Delta(values: Int32Array): Uint8Array {
    // Delta encode: store deltas
    const deltaEncoded = new Int32Array(values.length);
    deltaEncoded[0] = values[0];
    for (let i = 1; i < values.length; i++) {
        deltaEncoded[i] = values[i] - values[i - 1];
    }

    const zigzagEncoded = new Int32Array(deltaEncoded.length);
    for (let i = 0; i < deltaEncoded.length; i++) {
        zigzagEncoded[i] = encodeZigZag32(deltaEncoded[i]);
    }
    const encodedData = encodeVarintInt32Array(zigzagEncoded);
    const streamMetadata = createStreamMetadata(LogicalLevelTechnique.DELTA, LogicalLevelTechnique.NONE, values.length);
    return buildEncodedStream(streamMetadata, encodedData);
}

/**
 * Encodes INT_32 values with RLE encoding
 * @param runs - Array of [runLength, value] pairs
 */
export function encodeInt32Rle(runs: Array<[number, number]>): Uint8Array {
    const runLengths: number[] = [];
    const values: number[] = [];
    let totalValues = 0;

    for (const [runLength, value] of runs) {
        runLengths.push(runLength);
        values.push(encodeZigZag32(value));
        totalValues += runLength;
    }

    const rleValues = [...runLengths, ...values];
    const encodedData = encodeVarintInt32Array(new Int32Array(rleValues));
    const streamMetadata = createRleMetadata(
        LogicalLevelTechnique.RLE,
        LogicalLevelTechnique.NONE,
        runs.length,
        totalValues,
    );
    return buildEncodedStream(streamMetadata, encodedData);
}

/**
 * Encodes INT_32 values with DELTA+RLE encoding
 * @param runs - Array of [runLength, deltaValue] pairs, where first value is the base
 */
export function encodeInt32DeltaRle(runs: Array<[number, number]>): Uint8Array {
    const runLengths: number[] = [];
    const values: number[] = [];
    let totalValues = 0;

    for (const [runLength, value] of runs) {
        runLengths.push(runLength);
        values.push(encodeZigZag32(value));
        totalValues += runLength;
    }

    const rleValues = [...runLengths, ...values];
    const encodedData = encodeVarintInt32Array(new Int32Array(rleValues));
    const streamMetadata = createRleMetadata(
        LogicalLevelTechnique.DELTA,
        LogicalLevelTechnique.RLE,
        runs.length,
        totalValues,
    );
    return buildEncodedStream(streamMetadata, encodedData);
}

/**
 * Encodes nullable INT_32 values
 */
export function encodeInt32NullableColumn(values: (number | null)[]): Uint8Array {
    const nonNullValues = values.filter((v): v is number => v !== null);
    const zigzagEncoded = new Int32Array(nonNullValues.map((v) => encodeZigZag32(v)));
    const encodedData = encodeVarintInt32Array(zigzagEncoded);
    const dataStreamMetadata = createStreamMetadata(
        LogicalLevelTechnique.NONE,
        LogicalLevelTechnique.NONE,
        nonNullValues.length,
    );
    const dataStream = buildEncodedStream(dataStreamMetadata, encodedData);

    // Nullability stream
    const nullabilityValues = values.map((v) => v !== null);
    const nullabilityEncoded = encodeBooleanRle(nullabilityValues);
    const nullabilityMetadata = createStreamMetadata(
        LogicalLevelTechnique.NONE,
        LogicalLevelTechnique.NONE,
        nullabilityValues.length,
    );
    const nullabilityStream = buildEncodedStream(nullabilityMetadata, nullabilityEncoded);

    return concatenateBuffers(nullabilityStream, dataStream);
}

/**
 * Encodes UINT_32 values (no zigzag encoding)
 */
export function encodeUint32Column(values: Uint32Array): Uint8Array {
    const encodedData = encodeVarintInt32Array(new Int32Array(values));
    const streamMetadata = createStreamMetadata(LogicalLevelTechnique.NONE, LogicalLevelTechnique.NONE, values.length);
    return buildEncodedStream(streamMetadata, encodedData);
}

/**
 * Encodes INT_64 values with NONE encoding
 */
export function encodeInt64NoneColumn(values: BigInt64Array): Uint8Array {
    const zigzagEncoded = new BigInt64Array(Array.from(values, (val) => encodeZigZag64(val)));
    const encodedData = encodeVarintInt64Array(zigzagEncoded);
    const streamMetadata = createStreamMetadata(LogicalLevelTechnique.NONE, LogicalLevelTechnique.NONE, values.length);
    return buildEncodedStream(streamMetadata, encodedData);
}

/**
 * Encodes INT_64 values with DELTA encoding
 */
export function encodeInt64DeltaColumn(values: BigInt64Array): Uint8Array {
    const deltaEncoded = new BigInt64Array(values.length);
    deltaEncoded[0] = values[0];
    for (let i = 1; i < values.length; i++) {
        deltaEncoded[i] = values[i] - values[i - 1];
    }

    const zigzagEncoded = new BigInt64Array(deltaEncoded.length);
    for (let i = 0; i < deltaEncoded.length; i++) {
        zigzagEncoded[i] = encodeZigZag64(deltaEncoded[i]);
    }
    const encodedData = encodeVarintInt64Array(zigzagEncoded);
    const streamMetadata = createStreamMetadata(LogicalLevelTechnique.DELTA, LogicalLevelTechnique.NONE, values.length);
    return buildEncodedStream(streamMetadata, encodedData);
}

/**
 * Encodes INT_64 values with RLE encoding
 */
export function encodeInt64RleColumn(runs: Array<[number, bigint]>): Uint8Array {
    const runLengths: bigint[] = [];
    const values: bigint[] = [];
    let totalValues = 0;

    for (const [runLength, value] of runs) {
        runLengths.push(BigInt(runLength));
        values.push(encodeZigZag64(value));
        totalValues += runLength;
    }

    const rleValues = [...runLengths, ...values];
    const encodedData = encodeVarintInt64Array(new BigInt64Array(rleValues));
    const streamMetadata = createRleMetadata(
        LogicalLevelTechnique.RLE,
        LogicalLevelTechnique.NONE,
        runs.length,
        totalValues,
    );
    return buildEncodedStream(streamMetadata, encodedData);
}

/**
 * Encodes INT_64 values with DELTA+RLE encoding
 */
export function encodeInt64DeltaRleColumn(runs: Array<[number, bigint]>): Uint8Array {
    const runLengths: bigint[] = [];
    const values: bigint[] = [];
    let totalValues = 0;

    for (const [runLength, value] of runs) {
        runLengths.push(BigInt(runLength));
        values.push(encodeZigZag64(value));
        totalValues += runLength;
    }

    const rleValues = [...runLengths, ...values];
    const encodedData = encodeVarintInt64Array(new BigInt64Array(rleValues));
    const streamMetadata = createRleMetadata(
        LogicalLevelTechnique.DELTA,
        LogicalLevelTechnique.RLE,
        runs.length,
        totalValues,
    );
    return buildEncodedStream(streamMetadata, encodedData);
}

/**
 * Encodes nullable INT_64 values
 */
export function encodeInt64NullableColumn(values: (bigint | null)[]): Uint8Array {
    const nonNullValues = values.filter((v): v is bigint => v !== null);
    const zigzagEncoded = new BigInt64Array(Array.from(nonNullValues, (val) => encodeZigZag64(val)));
    const encodedData = encodeVarintInt64Array(zigzagEncoded);
    const dataStreamMetadata = createStreamMetadata(
        LogicalLevelTechnique.NONE,
        LogicalLevelTechnique.NONE,
        nonNullValues.length,
    );
    const dataStream = buildEncodedStream(dataStreamMetadata, encodedData);

    const nullabilityValues = values.map((v) => v !== null);
    const nullabilityEncoded = encodeBooleanRle(nullabilityValues);
    const nullabilityMetadata = createStreamMetadata(
        LogicalLevelTechnique.NONE,
        LogicalLevelTechnique.NONE,
        nullabilityValues.length,
    );
    const nullabilityStream = buildEncodedStream(nullabilityMetadata, nullabilityEncoded);

    return concatenateBuffers(nullabilityStream, dataStream);
}

/**
 * Encodes UINT_64 values (no zigzag encoding)
 */
export function encodeUint64Column(values: BigUint64Array): Uint8Array {
    const encodedData = encodeVarintInt64Array(new BigInt64Array(values));
    const streamMetadata = createStreamMetadata(LogicalLevelTechnique.NONE, LogicalLevelTechnique.NONE, values.length);
    return buildEncodedStream(streamMetadata, encodedData);
}

/**
 * Encodes nullable UINT_64 values
 */
export function encodeUint64NullableColumn(values: (bigint | null)[]): Uint8Array {
    const nonNullValues = values.filter((v): v is bigint => v !== null);
    const encodedData = encodeVarintInt64Array(new BigInt64Array(nonNullValues));
    const dataStreamMetadata = createStreamMetadata(
        LogicalLevelTechnique.NONE,
        LogicalLevelTechnique.NONE,
        nonNullValues.length,
    );
    const dataStream = buildEncodedStream(dataStreamMetadata, encodedData);

    const nullabilityValues = values.map((v) => v !== null);
    const nullabilityEncoded = encodeBooleanRle(nullabilityValues);
    const nullabilityMetadata = createStreamMetadata(
        LogicalLevelTechnique.NONE,
        LogicalLevelTechnique.NONE,
        nullabilityValues.length,
    );
    const nullabilityStream = buildEncodedStream(nullabilityMetadata, nullabilityEncoded);

    return concatenateBuffers(nullabilityStream, dataStream);
}

/**
 * Encodes FLOAT values
 */
export function encodeFloatColumn(values: Float32Array): Uint8Array {
    const encodedData = encodeFloatsLE(values);
    const streamMetadata = createStreamMetadata(LogicalLevelTechnique.NONE, LogicalLevelTechnique.NONE, values.length);
    return buildEncodedStream(streamMetadata, encodedData);
}

/**
 * Encodes nullable FLOAT values
 */
export function encodeFloatNullableColumn(values: (number | null)[]): Uint8Array {
    const nonNullValues = values.filter((v): v is number => v !== null);
    const encodedData = encodeFloatsLE(new Float32Array(nonNullValues));
    const dataStreamMetadata = createStreamMetadata(
        LogicalLevelTechnique.NONE,
        LogicalLevelTechnique.NONE,
        nonNullValues.length,
    );
    const dataStream = buildEncodedStream(dataStreamMetadata, encodedData);

    const nullabilityValues = values.map((v) => v !== null);
    const nullabilityEncoded = encodeBooleanRle(nullabilityValues);
    const nullabilityMetadata = createStreamMetadata(
        LogicalLevelTechnique.NONE,
        LogicalLevelTechnique.NONE,
        nullabilityValues.length,
    );
    const nullabilityStream = buildEncodedStream(nullabilityMetadata, nullabilityEncoded);

    return concatenateBuffers(nullabilityStream, dataStream);
}

/**
 * Encodes DOUBLE values
 */
export function encodeDoubleColumn(values: Float32Array): Uint8Array {
    const encodedData = encodeDoubleLE(values);
    const streamMetadata = createStreamMetadata(LogicalLevelTechnique.NONE, LogicalLevelTechnique.NONE, values.length);
    return buildEncodedStream(streamMetadata, encodedData);
}

/**
 * Encodes nullable DOUBLE values
 */
export function encodeDoubleNullableColumn(values: (number | null)[]): Uint8Array {
    const nonNullValues = values.filter((v): v is number => v !== null);
    const encodedData = encodeDoubleLE(new Float32Array(nonNullValues));
    const dataStreamMetadata = createStreamMetadata(
        LogicalLevelTechnique.NONE,
        LogicalLevelTechnique.NONE,
        nonNullValues.length,
    );
    const dataStream = buildEncodedStream(dataStreamMetadata, encodedData);

    const nullabilityValues = values.map((v) => v !== null);
    const nullabilityEncoded = encodeBooleanRle(nullabilityValues);
    const nullabilityMetadata = createStreamMetadata(
        LogicalLevelTechnique.NONE,
        LogicalLevelTechnique.NONE,
        nullabilityValues.length,
    );
    const nullabilityStream = buildEncodedStream(nullabilityMetadata, nullabilityEncoded);

    return concatenateBuffers(nullabilityStream, dataStream);
}

/**
 * Encodes BOOLEAN values
 */
export function encodeBooleanColumn(values: boolean[]): Uint8Array {
    const encodedData = encodeBooleanRle(values);
    const streamMetadata = createStreamMetadata(LogicalLevelTechnique.NONE, LogicalLevelTechnique.NONE, values.length);
    return buildEncodedStream(streamMetadata, encodedData);
}

/**
 * Encodes nullable BOOLEAN values
 */
export function encodeBooleanNullableColumn(values: (boolean | null)[]): Uint8Array {
    const nonNullValues = values.filter((v): v is boolean => v !== null);
    const encodedData = encodeBooleanRle(nonNullValues);
    const dataStreamMetadata = createStreamMetadata(
        LogicalLevelTechnique.NONE,
        LogicalLevelTechnique.NONE,
        nonNullValues.length,
    );
    const dataStream = buildEncodedStream(dataStreamMetadata, encodedData);

    const nullabilityValues = values.map((v) => v !== null);
    const nullabilityEncoded = encodeBooleanRle(nullabilityValues);
    const nullabilityMetadata = createStreamMetadata(
        LogicalLevelTechnique.NONE,
        LogicalLevelTechnique.NONE,
        nullabilityValues.length,
    );
    const nullabilityStream = buildEncodedStream(nullabilityMetadata, nullabilityEncoded);

    return concatenateBuffers(nullabilityStream, dataStream);
}

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

function buildEncodedStream(
    streamMetadata: StreamMetadata | RleEncodedStreamMetadata,
    encodedData: Uint8Array,
): Uint8Array {
    const updatedMetadata = {
        ...streamMetadata,
        byteLength: encodedData.length,
    };

    const metadataBuffer = encodeStreamMetadata(updatedMetadata);
    const result = new Uint8Array(metadataBuffer.length + encodedData.length);
    result.set(metadataBuffer, 0);
    result.set(encodedData, metadataBuffer.length);

    return result;
}

function encodeStreamMetadata(metadata: StreamMetadata | RleEncodedStreamMetadata): Uint8Array {
    const buffer = new Uint8Array(100);
    let writeOffset = 0;

    // Byte 1: Stream type
    const physicalTypeIndex = Object.values(PhysicalStreamType).indexOf(metadata.physicalStreamType);
    const lowerNibble = 0; // For DATA stream with NONE dictionary type
    buffer[writeOffset++] = (physicalTypeIndex << 4) | lowerNibble;

    // Byte 2: Encoding techniques
    const llt1Index = Object.values(LogicalLevelTechnique).indexOf(metadata.logicalLevelTechnique1);
    const llt2Index = Object.values(LogicalLevelTechnique).indexOf(metadata.logicalLevelTechnique2);
    const pltIndex = Object.values(PhysicalLevelTechnique).indexOf(metadata.physicalLevelTechnique);
    buffer[writeOffset++] = (llt1Index << 5) | (llt2Index << 2) | pltIndex;

    // Variable-length fields
    const offset = new IntWrapper(writeOffset);
    encodeSingleVarintInt32(metadata.numValues, buffer, offset);
    encodeSingleVarintInt32(metadata.byteLength, buffer, offset);

    // RLE-specific fields
    if (isRleMetadata(metadata)) {
        encodeSingleVarintInt32(metadata.runs, buffer, offset);
        encodeSingleVarintInt32(metadata.numRleValues, buffer, offset);
    }

    return buffer.slice(0, offset.get());
}

function isRleMetadata(metadata: StreamMetadata | RleEncodedStreamMetadata): metadata is RleEncodedStreamMetadata {
    return "runs" in metadata && "numRleValues" in metadata;
}

function concatenateBuffers(...buffers: Uint8Array[]): Uint8Array {
    const totalLength = buffers.reduce((sum, buf) => sum + buf.length, 0);
    const result = new Uint8Array(totalLength);
    let offset = 0;

    for (const buffer of buffers) {
        result.set(buffer, offset);
        offset += buffer.length;
    }

    return result;
}
