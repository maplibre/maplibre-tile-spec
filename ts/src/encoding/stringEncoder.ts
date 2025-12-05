import { PhysicalStreamType } from "../metadata/tile/physicalStreamType";
import { LogicalStreamType } from "../metadata/tile/logicalStreamType";
import { DictionaryType } from "../metadata/tile/dictionaryType";
import { LengthType } from "../metadata/tile/lengthType";
import { OffsetType } from "../metadata/tile/offsetType";
import { PhysicalLevelTechnique } from "../metadata/tile/physicalLevelTechnique";
import { LogicalLevelTechnique } from "../metadata/tile/logicalLevelTechnique";
import type { StreamMetadata } from "../metadata/tile/streamMetadataDecoder";
import IntWrapper from "../decoding/intWrapper";
import {
    encodeVarintInt32Array,
    encodeSingleVarintInt32,
    encodeBooleanRle,
    encodeStrings,
    createStringLengths,
    concatenateBuffers,
} from "./encodingUtils";

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
        createStream(PhysicalStreamType.LENGTH, encodeVarintInt32Array(new Int32Array(lengths)), {
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
        createStream(PhysicalStreamType.OFFSET, encodeVarintInt32Array(new Int32Array(offsets)), {
            logical: new LogicalStreamType(undefined, OffsetType.STRING),
            technique: PhysicalLevelTechnique.VARINT,
            count: offsets.length,
        }),
    );

    // Add LENGTH stream (for dictionary)
    streams.push(
        createStream(PhysicalStreamType.LENGTH, encodeVarintInt32Array(new Int32Array(lengths)), {
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

/**
 * Encodes FSST-compressed strings into a complete stream.
 * This uses hardcoded test data: ["cat", "dog", "cat"]
 * @returns Encoded Uint8Array that can be passed to decodeString
 */
export function encodeFsstStrings(): Uint8Array {
    const symbolTable = new Uint8Array([99, 97, 116, 100, 111, 103]); // "catdog"
    const symbolLengths = new Int32Array([3, 3]);
    const compressedDictionary = new Uint8Array([0, 1]);
    const dictionaryLengths = new Int32Array([3, 3]);
    const offsets = new Int32Array([0, 1, 0]); // "cat", "dog", "cat"
    const numValues = 3;

    return concatenateBuffers(
        createStream(PhysicalStreamType.PRESENT, encodeBooleanRle(new Array(numValues).fill(true)), {
            technique: PhysicalLevelTechnique.VARINT,
            count: numValues,
        }),
        createStream(PhysicalStreamType.DATA, symbolTable, {
            logical: new LogicalStreamType(DictionaryType.FSST),
        }),
        createStream(PhysicalStreamType.LENGTH, encodeVarintInt32Array(symbolLengths), {
            logical: new LogicalStreamType(undefined, undefined, LengthType.SYMBOL),
            technique: PhysicalLevelTechnique.VARINT,
            count: symbolLengths.length,
        }),
        createStream(PhysicalStreamType.OFFSET, encodeVarintInt32Array(offsets), {
            logical: new LogicalStreamType(undefined, OffsetType.STRING),
            technique: PhysicalLevelTechnique.VARINT,
            count: offsets.length,
        }),
        createStream(PhysicalStreamType.LENGTH, encodeVarintInt32Array(dictionaryLengths), {
            logical: new LogicalStreamType(undefined, undefined, LengthType.DICTIONARY),
            technique: PhysicalLevelTechnique.VARINT,
            count: dictionaryLengths.length,
        }),
        createStream(PhysicalStreamType.DATA, compressedDictionary, {
            logical: new LogicalStreamType(DictionaryType.SINGLE),
        }),
    );
}

/**
 * Encodes a shared dictionary for struct fields.
 * @param dictionaryStrings - Array of unique strings in the dictionary
 * @param options - Encoding options
 * @returns Object containing length and data streams
 */
export function encodeSharedDictionary(
    dictionaryStrings: string[],
    options: { useFsst?: boolean; dictionaryType?: DictionaryType } = {},
): {
    lengthStream: Uint8Array;
    dataStream: Uint8Array;
    symbolLengthStream?: Uint8Array;
    symbolDataStream?: Uint8Array;
} {
    const { useFsst = false, dictionaryType = DictionaryType.SHARED } = options;

    const encodedDictionary = encodeStrings(dictionaryStrings);
    const dictionaryLengths = createStringLengths(dictionaryStrings);

    const lengthStream = createStream(
        PhysicalStreamType.LENGTH,
        encodeVarintInt32Array(new Int32Array(dictionaryLengths)),
        {
            logical: new LogicalStreamType(undefined, undefined, LengthType.DICTIONARY),
            technique: PhysicalLevelTechnique.VARINT,
            count: dictionaryLengths.length,
        },
    );

    const dataStream = createStream(PhysicalStreamType.DATA, encodedDictionary, {
        logical: new LogicalStreamType(dictionaryType),
        count: encodedDictionary.length,
    });

    if (useFsst) {
        const symbolTable = new Uint8Array([99, 97, 116, 100, 111, 103]); // "catdog"
        const symbolLengths = new Int32Array([3, 3]);

        const symbolLengthStream = createStream(PhysicalStreamType.LENGTH, encodeVarintInt32Array(symbolLengths), {
            logical: new LogicalStreamType(undefined, undefined, LengthType.SYMBOL),
            technique: PhysicalLevelTechnique.VARINT,
            count: symbolLengths.length,
        });

        const symbolDataStream = createStream(PhysicalStreamType.DATA, symbolTable, {
            logical: new LogicalStreamType(DictionaryType.FSST),
            count: symbolTable.length,
        });

        return { lengthStream, dataStream, symbolLengthStream, symbolDataStream };
    }

    return { lengthStream, dataStream };
}

/**
 * Encodes streams for a struct field.
 * @param offsetIndices - Indices into the shared dictionary
 * @param presentValues - Boolean array indicating which values are present
 * @param isPresent - Whether the field itself is present
 * @returns Encoded streams for the field
 */
export function encodeStructField(
    offsetIndices: number[],
    presentValues: boolean[],
    isPresent: boolean = true,
): Uint8Array {
    if (!isPresent) {
        return encodeNumStreams(0);
    }

    const numStreamsEncoded = encodeNumStreams(2);
    const encodedPresent = createPresentStream(presentValues);
    const encodedOffsets = createOffsetStream(offsetIndices);

    return concatenateBuffers(numStreamsEncoded, encodedPresent, encodedOffsets);
}

// Helper functions

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
    encodeSingleVarintInt32(metadata.numValues, buffer, offset);
    encodeSingleVarintInt32(metadata.byteLength, buffer, offset);

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
    encodeSingleVarintInt32(numStreams, buffer, offset);
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
    return buildEncodedStream(metadata, encodeVarintInt32Array(new Int32Array(offsetIndices)));
}
