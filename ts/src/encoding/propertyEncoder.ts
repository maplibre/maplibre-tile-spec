import { LogicalLevelTechnique } from "../metadata/tile/logicalLevelTechnique";
import { PhysicalLevelTechnique } from "../metadata/tile/physicalLevelTechnique";
import { PhysicalStreamType } from "../metadata/tile/physicalStreamType";
import { DictionaryType } from "../metadata/tile/dictionaryType";
import type { StreamMetadata, RleEncodedStreamMetadata } from "../metadata/tile/streamMetadataDecoder";
import IntWrapper from "../decoding/intWrapper";
import { encodeBooleanRle, encodeFloatsLE, encodeDoubleLE } from "./encodingUtils";
import {
    encodeVarintInt32Value,
    encodeVarintInt32,
    encodeVarintInt64,
    encodeZigZagInt32Value,
    encodeZigZagInt64Value,
    encodeZigZagInt32,
} from "./integerEncodingUtils";

/**
 * Encodes INT_32 values with NONE encoding (no delta, no RLE)
 */
export function encodeInt32NoneColumn(values: Int32Array): Uint8Array {
    const zigzagEncoded = encodeZigZagInt32(values);
    const encodedData = encodeVarintInt32(zigzagEncoded);
    const streamMetadata = createStreamMetadata(LogicalLevelTechnique.NONE, LogicalLevelTechnique.NONE, values.length);
    return buildEncodedStream(streamMetadata, encodedData);
}

/**
 * Encodes INT_32 values with DELTA encoding
 */
export function encodeInt32DeltaColumn(values: Int32Array): Uint8Array {
    // Delta encode: store deltas
    const deltaEncoded = new Int32Array(values.length);
    deltaEncoded[0] = values[0];
    for (let i = 1; i < values.length; i++) {
        deltaEncoded[i] = values[i] - values[i - 1];
    }

    const zigzagEncoded = encodeZigZagInt32(deltaEncoded);
    const encodedData = encodeVarintInt32(zigzagEncoded);
    const streamMetadata = createStreamMetadata(LogicalLevelTechnique.DELTA, LogicalLevelTechnique.NONE, values.length);
    return buildEncodedStream(streamMetadata, encodedData);
}

/**
 * Encodes INT_32 values with RLE encoding
 * @param runs - Array of [runLength, value] pairs
 */
export function encodeInt32RleColumn(runs: Array<[number, number]>): Uint8Array {
    const runLengths: number[] = [];
    const values: number[] = [];
    let totalValues = 0;

    for (const [runLength, value] of runs) {
        runLengths.push(runLength);
        values.push(encodeZigZagInt32Value(value));
        totalValues += runLength;
    }

    const rleValues = [...runLengths, ...values];
    const encodedData = encodeVarintInt32(new Uint32Array(rleValues));
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
export function encodeInt32DeltaRleColumn(runs: Array<[number, number]>): Uint8Array {
    const runLengths: number[] = [];
    const values: number[] = [];
    let totalValues = 0;

    for (const [runLength, value] of runs) {
        runLengths.push(runLength);
        values.push(encodeZigZagInt32Value(value));
        totalValues += runLength;
    }

    const rleValues = [...runLengths, ...values];
    const encodedData = encodeVarintInt32(new Uint32Array(rleValues));
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
    return encodeNullableColumn(values, (present) => encodeInt32NoneColumn(Int32Array.from(present)));
}

/**
 * Encodes UINT_32 values (no zigzag encoding)
 */
export function encodeUint32Column(values: Uint32Array): Uint8Array {
    const encodedData = encodeVarintInt32(values);
    const streamMetadata = createStreamMetadata(LogicalLevelTechnique.NONE, LogicalLevelTechnique.NONE, values.length);
    return buildEncodedStream(streamMetadata, encodedData);
}

/**
 * Encodes INT_64 values with NONE encoding
 */
export function encodeInt64NoneColumn(values: BigInt64Array): Uint8Array {
    const zigzagEncoded = new BigUint64Array(Array.from(values, (val) => encodeZigZagInt64Value(val)));
    const encodedData = encodeVarintInt64(zigzagEncoded);
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

    const zigzagEncoded = new BigUint64Array(deltaEncoded.length);
    for (let i = 0; i < deltaEncoded.length; i++) {
        zigzagEncoded[i] = encodeZigZagInt64Value(deltaEncoded[i]);
    }
    const encodedData = encodeVarintInt64(zigzagEncoded);
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
        values.push(encodeZigZagInt64Value(value));
        totalValues += runLength;
    }

    const rleValues = [...runLengths, ...values];
    const encodedData = encodeVarintInt64(new BigUint64Array(rleValues));
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
        values.push(encodeZigZagInt64Value(value));
        totalValues += runLength;
    }

    const rleValues = [...runLengths, ...values];
    const encodedData = encodeVarintInt64(new BigUint64Array(rleValues));
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
    return encodeNullableColumn(values, (present) => encodeInt64NoneColumn(BigInt64Array.from(present)));
}

/**
 * Encodes UINT_64 values (no zigzag encoding)
 */
export function encodeUint64Column(values: BigUint64Array): Uint8Array {
    const encodedData = encodeVarintInt64(values);
    const streamMetadata = createStreamMetadata(LogicalLevelTechnique.NONE, LogicalLevelTechnique.NONE, values.length);
    return buildEncodedStream(streamMetadata, encodedData);
}

/**
 * Encodes nullable UINT_64 values
 */
export function encodeUint64NullableColumn(values: (bigint | null)[]): Uint8Array {
    return encodeNullableColumn(values, (present) => encodeUint64Column(BigUint64Array.from(present)));
}

/**
 * Encodes FLOAT values
 */
export function encodeFloatColumn(values: Float32Array): Uint8Array {
    const encodedData = encodeFloatsLE(values);
    const streamMetadata = createFixedWidthStreamMetadata(values.length);
    return buildEncodedStream(streamMetadata, encodedData);
}

/**
 * Encodes nullable FLOAT values
 */
export function encodeFloatNullableColumn(values: (number | null)[]): Uint8Array {
    return encodeNullableColumn(values, (present) => encodeFloatColumn(Float32Array.from(present)));
}

/**
 * Encodes DOUBLE values
 */
export function encodeDoubleColumn(values: Float64Array): Uint8Array {
    const encodedData = encodeDoubleLE(values);
    const streamMetadata = createFixedWidthStreamMetadata(values.length);
    return buildEncodedStream(streamMetadata, encodedData);
}

/**
 * Encodes nullable DOUBLE values
 */
export function encodeDoubleNullableColumn(values: (number | null)[]): Uint8Array {
    return encodeNullableColumn(values, (present) => encodeDoubleColumn(Float64Array.from(present)));
}

/**
 * Encodes BOOLEAN values
 */
export function encodeBooleanColumn(values: boolean[]): Uint8Array {
    const encodedData = encodeBooleanRle(values);
    const streamMetadata = createBooleanStreamMetadata(values.length);
    return buildEncodedStream(streamMetadata, encodedData);
}

/**
 * Encodes nullable BOOLEAN values
 */
export function encodeBooleanNullableColumn(values: (boolean | null)[]): Uint8Array {
    return encodeNullableColumn(values, encodeBooleanColumn);
}

/** A nullable column: its `Present` stream, then the column its present values make. */
function encodeNullableColumn<T>(values: (T | null)[], encodeColumn: (present: T[]) => Uint8Array): Uint8Array {
    const presentStream = encodePresentStream(values.map((value) => value !== null));
    const presentValues = values.filter((value): value is T => value !== null);
    return concatenateBuffers(presentStream, encodeColumn(presentValues));
}

/** The `Present` stream of a nullable column: one bit per value, set where the value is present. */
export function encodePresentStream(present: boolean[]): Uint8Array {
    const metadata: StreamMetadata = {
        ...createBooleanStreamMetadata(present.length),
        physicalStreamType: PhysicalStreamType.PRESENT,
        logicalStreamType: {},
    };
    return buildEncodedStream(metadata, encodeBooleanRle(present));
}

/**
 * The metadata of the data stream of a boolean column, which a `Present` stream shares apart from its type. The
 * bits are packed into bytes and the bytes run-length encoded. The header has no run counts, unlike that of an
 * integer RLE stream: they follow from the number of values.
 */
function createBooleanStreamMetadata(numValues: number): StreamMetadata {
    return createStreamMetadata(
        LogicalLevelTechnique.RLE,
        LogicalLevelTechnique.NONE,
        numValues,
        PhysicalLevelTechnique.NONE,
    );
}

/** The metadata of a stream of fixed-width words, which is how float and double columns store their values. */
function createFixedWidthStreamMetadata(numValues: number): StreamMetadata {
    return createStreamMetadata(
        LogicalLevelTechnique.NONE,
        LogicalLevelTechnique.NONE,
        numValues,
        PhysicalLevelTechnique.NONE,
    );
}

function createStreamMetadata(
    logicalTechnique1: LogicalLevelTechnique,
    logicalTechnique2: LogicalLevelTechnique = LogicalLevelTechnique.NONE,
    numValues = 3,
    physicalLevelTechnique: PhysicalLevelTechnique = PhysicalLevelTechnique.VARINT,
): StreamMetadata {
    return {
        physicalStreamType: PhysicalStreamType.DATA,
        logicalStreamType: { dictionaryType: DictionaryType.NONE },
        logicalLevelTechnique1: logicalTechnique1,
        logicalLevelTechnique2: logicalTechnique2,
        physicalLevelTechnique,
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
        logicalStreamType: { dictionaryType: DictionaryType.NONE },
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
    const lowerNibble = 0; // The subtype: a Present stream has none, a Data stream no dictionary
    buffer[writeOffset++] = (physicalTypeIndex << 4) | lowerNibble;

    // Byte 2: Encoding techniques
    const llt1Index = Object.values(LogicalLevelTechnique).indexOf(metadata.logicalLevelTechnique1);
    const llt2Index = Object.values(LogicalLevelTechnique).indexOf(metadata.logicalLevelTechnique2);
    const pltIndex = Object.values(PhysicalLevelTechnique).indexOf(metadata.physicalLevelTechnique);
    buffer[writeOffset++] = (llt1Index << 5) | (llt2Index << 2) | pltIndex;

    // Variable-length fields
    const offset = new IntWrapper(writeOffset);
    encodeVarintInt32Value(metadata.numValues, buffer, offset);
    encodeVarintInt32Value(metadata.byteLength, buffer, offset);

    // RLE-specific fields
    if (isRleMetadata(metadata)) {
        encodeVarintInt32Value(metadata.runs, buffer, offset);
        encodeVarintInt32Value(metadata.numRleValues, buffer, offset);
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
