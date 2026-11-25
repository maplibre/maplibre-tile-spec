import { PhysicalStreamType } from "../metadata/tile/physicalStreamType";
import { DictionaryType } from "../metadata/tile/dictionaryType";
import { LengthType } from "../metadata/tile/lengthType";
import { OffsetType } from "../metadata/tile/offsetType";
import { PhysicalLevelTechnique } from "../metadata/tile/physicalLevelTechnique";
import { LogicalLevelTechnique } from "../metadata/tile/logicalLevelTechnique";
import IntWrapper from "../decoding/intWrapper";
import { encodeUtf8Strings, concatenateBuffers } from "./encodingUtils";
import { encodeVarintInt32Value, encodeVarintInt32 } from "./integerEncodingUtils";
import { encodePresentStream } from "./propertyEncoder";
import type { StreamMetadata } from "../metadata/tile/streamMetadataDecoder";
import type { LogicalStreamType } from "../metadata/tile/logicalStreamType";

/**
 * Encodes plain strings into a complete stream with PRESENT (if needed), LENGTH, and DATA streams.
 * @param strings - Array of strings (can include null values)
 * @returns Encoded Uint8Array that can be passed to decodeString
 */
export function encodePlainStrings(strings: (string | null)[]): Uint8Array {
    const hasNull = strings.some((s) => s === null);
    const nonNullStrings = strings.filter((s): s is string => s !== null);
    const { bytes: stringBytes, lengths } = encodeUtf8Strings(nonNullStrings);

    const streams: Uint8Array[] = [];

    // Add PRESENT stream if nulls exist
    if (hasNull) {
        streams.push(encodePresentStream(strings.map((s) => s !== null)));
    }

    // Add LENGTH stream
    streams.push(
        createStream(PhysicalStreamType.LENGTH, encodeVarintInt32(lengths), {
            logical: { lengthType: LengthType.VAR_BINARY },
            technique: PhysicalLevelTechnique.VARINT,
            count: lengths.length,
        }),
    );

    // Add DATA stream
    streams.push(
        createStream(PhysicalStreamType.DATA, stringBytes, {
            logical: { dictionaryType: DictionaryType.NONE },
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
    const codes = new Map<string, number>();
    const uniqueStrings: string[] = [];
    const offsets = new Uint32Array(strings.length);
    let count = 0;
    for (const string of strings) {
        if (string === null) continue;
        let code = codes.get(string);
        if (code === undefined) {
            code = uniqueStrings.length;
            codes.set(string, code);
            uniqueStrings.push(string);
        }
        offsets[count++] = code;
    }
    const hasNull = count < strings.length;
    const { bytes: stringBytes, lengths } = encodeUtf8Strings(uniqueStrings);

    const streams: Uint8Array[] = [];

    // Add PRESENT stream if nulls exist
    if (hasNull) {
        streams.push(encodePresentStream(strings.map((s) => s !== null)));
    }

    // Add LENGTH stream (for dictionary)
    streams.push(
        createStream(PhysicalStreamType.LENGTH, encodeVarintInt32(lengths), {
            logical: { lengthType: LengthType.DICTIONARY },
            technique: PhysicalLevelTechnique.VARINT,
            count: lengths.length,
        }),
    );

    // Add OFFSET stream
    streams.push(
        createStream(PhysicalStreamType.OFFSET, encodeVarintInt32(offsets.subarray(0, count)), {
            logical: { offsetType: OffsetType.STRING },
            technique: PhysicalLevelTechnique.VARINT,
            count,
        }),
    );

    // Add DATA stream
    streams.push(
        createStream(PhysicalStreamType.DATA, stringBytes, {
            logical: { dictionaryType: DictionaryType.SINGLE },
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
            logicalStreamType: options.logical ?? {},
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
