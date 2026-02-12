import { PhysicalStreamType } from "../metadata/tile/physicalStreamType";
import { LogicalStreamType } from "../metadata/tile/logicalStreamType";
import { DictionaryType } from "../metadata/tile/dictionaryType";
import { LengthType } from "../metadata/tile/lengthType";
import { OffsetType } from "../metadata/tile/offsetType";
import { PhysicalLevelTechnique } from "../metadata/tile/physicalLevelTechnique";
import { LogicalLevelTechnique } from "../metadata/tile/logicalLevelTechnique";
import type { StreamMetadata } from "../metadata/tile/streamMetadataDecoder";
import IntWrapper from "../decoding/intWrapper";
import { encodeBooleanRle, encodeStrings, createStringLengths, concatenateBuffers } from "./encodingUtils";
import { encodeVarintInt32Value, encodeVarintInt32 } from "./integerEncodingUtils";

/**
 * Encodes plain strings into a complete stream with PRESENT (if needed), LENGTH, and DATA streams.
 * @param strings - Array of strings (can include null values)
 * @returns Encoded Uint8Array that can be passed to decodeString
 */
export function encodePlainStrings(strings: (string | null)[]): Uint8Array {
    const hasNull = strings.some((s) => s === null);
    const nonNullStrings = strings.filter((s): s is string => s !== null);
    const stringBytes = encodeStrings(nonNullStrings);

    const streams: Uint8Array[] = [];

    // Add PRESENT stream if nulls exist
    if (hasNull) {
        const nullabilityValues = strings.map((s) => s !== null);
        streams.push(
            createStream(PhysicalStreamType.PRESENT, encodeBooleanRle(nullabilityValues), {
                technique: PhysicalLevelTechnique.VARINT,
                count: nullabilityValues.length,
            }),
        );
    }

    // Add LENGTH stream
    const lengths = createStringLengths(nonNullStrings);
    streams.push(
        createStream(PhysicalStreamType.LENGTH, encodeVarintInt32(new Int32Array(lengths)), {
            logical: new LogicalStreamType(undefined, undefined, LengthType.VAR_BINARY),
            technique: PhysicalLevelTechnique.VARINT,
            count: lengths.length,
        }),
    );

    // Add DATA stream
    streams.push(
        createStream(PhysicalStreamType.DATA, stringBytes, {
            logical: new LogicalStreamType(DictionaryType.NONE),
        }),
    );

    return concatenateBuffers(...streams);
}

/**
 * Encodes dictionary-compressed strings into a complete stream.
 * @param strings - Array of strings (can include null values)
 * @returns Encoded Uint8Array that can be passed to decodeString
 */
export function encodeDictionaryStrings(strings: (string | null)[]): Uint8Array {
    const hasNull = strings.some((s) => s === null);
    const nonNullStrings = strings.filter((s): s is string => s !== null);

    // Create dictionary of unique strings
    const uniqueStrings = Array.from(new Set(nonNullStrings));
    const stringMap = new Map(uniqueStrings.map((s, i) => [s, i]));
    const offsets = nonNullStrings.map((s) => {
        const offset = stringMap.get(s);
        if (offset === undefined) {
            throw new Error(`String not found in dictionary: ${s}`);
        }
        return offset;
    });

    const stringBytes = encodeStrings(uniqueStrings);
    const lengths = createStringLengths(uniqueStrings);

    const streams: Uint8Array[] = [];

    // Add PRESENT stream if nulls exist
    if (hasNull) {
        const nullabilityValues = strings.map((s) => s !== null);
        streams.push(
            createStream(PhysicalStreamType.PRESENT, encodeBooleanRle(nullabilityValues), {
                technique: PhysicalLevelTechnique.VARINT,
                count: nullabilityValues.length,
            }),
        );
    }

    // Add OFFSET stream
    streams.push(
        createStream(PhysicalStreamType.OFFSET, encodeVarintInt32(new Int32Array(offsets)), {
            logical: new LogicalStreamType(undefined, OffsetType.STRING),
            technique: PhysicalLevelTechnique.VARINT,
            count: offsets.length,
        }),
    );

    // Add LENGTH stream (for dictionary)
    streams.push(
        createStream(PhysicalStreamType.LENGTH, encodeVarintInt32(new Int32Array(lengths)), {
            logical: new LogicalStreamType(undefined, undefined, LengthType.DICTIONARY),
            technique: PhysicalLevelTechnique.VARINT,
            count: lengths.length,
        }),
    );

    // Add DATA stream
    streams.push(
        createStream(PhysicalStreamType.DATA, stringBytes, {
            logical: new LogicalStreamType(DictionaryType.SINGLE),
        }),
    );

    return concatenateBuffers(...streams);
}

function createStream(
    physicalType: PhysicalStreamType,
    data: Uint8Array,
    options: {
        logical?: LogicalStreamType;
        technique?: PhysicalLevelTechnique;
        count?: number;
    } = {},
): Uint8Array {
    const count = options.count ?? 0;
    return buildEncodedStream(
        {
            physicalStreamType: physicalType,
            logicalStreamType: options.logical ?? new LogicalStreamType(),
            logicalLevelTechnique1: LogicalLevelTechnique.NONE,
            logicalLevelTechnique2: LogicalLevelTechnique.NONE,
            physicalLevelTechnique: options.technique ?? PhysicalLevelTechnique.NONE,
            numValues: count,
            byteLength: data.length,
            decompressedCount: count,
        },
        data,
    );
}

function buildEncodedStream(streamMetadata: StreamMetadata, encodedData: Uint8Array): Uint8Array {
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

function encodeStreamMetadata(metadata: StreamMetadata): Uint8Array {
    const buffer = new Uint8Array(100);
    let writeOffset = 0;

    // Byte 1: Stream type
    const physicalTypeIndex = Object.values(PhysicalStreamType).indexOf(metadata.physicalStreamType);
    const lowerNibble = getLogicalSubtypeValue(metadata);
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

    return buffer.slice(0, offset.get());
}

function getLogicalSubtypeValue(metadata: StreamMetadata): number {
    const { physicalStreamType, logicalStreamType } = metadata;

    switch (physicalStreamType) {
        case PhysicalStreamType.DATA:
            return logicalStreamType.dictionaryType !== undefined
                ? Object.values(DictionaryType).indexOf(logicalStreamType.dictionaryType)
                : 0;
        case PhysicalStreamType.OFFSET:
            return logicalStreamType.offsetType !== undefined
                ? Object.values(OffsetType).indexOf(logicalStreamType.offsetType)
                : 0;
        case PhysicalStreamType.LENGTH:
            return logicalStreamType.lengthType !== undefined
                ? Object.values(LengthType).indexOf(logicalStreamType.lengthType)
                : 0;
        default:
            return 0;
    }
}

function encodeNumStreams(numStreams: number): Uint8Array {
    const buffer = new Uint8Array(5);
    const offset = new IntWrapper(0);
    encodeVarintInt32Value(numStreams, buffer, offset);
    return buffer.slice(0, offset.get());
}

function createPresentStream(presentValues: boolean[]): Uint8Array {
    const metadata: StreamMetadata = {
        physicalStreamType: PhysicalStreamType.PRESENT,
        logicalStreamType: new LogicalStreamType(DictionaryType.NONE),
        logicalLevelTechnique1: LogicalLevelTechnique.NONE,
        logicalLevelTechnique2: LogicalLevelTechnique.NONE,
        physicalLevelTechnique: PhysicalLevelTechnique.VARINT,
        numValues: presentValues.length,
        byteLength: 0,
        decompressedCount: presentValues.length,
    };
    return buildEncodedStream(metadata, encodeBooleanRle(presentValues));
}

function createOffsetStream(offsetIndices: number[]): Uint8Array {
    const metadata: StreamMetadata = {
        physicalStreamType: PhysicalStreamType.OFFSET,
        logicalStreamType: new LogicalStreamType(undefined, OffsetType.STRING),
        logicalLevelTechnique1: LogicalLevelTechnique.NONE,
        logicalLevelTechnique2: LogicalLevelTechnique.NONE,
        physicalLevelTechnique: PhysicalLevelTechnique.VARINT,
        numValues: offsetIndices.length,
        byteLength: 0,
        decompressedCount: offsetIndices.length,
    };
    return buildEncodedStream(metadata, encodeVarintInt32(new Int32Array(offsetIndices)));
}
