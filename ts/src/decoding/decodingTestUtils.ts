import { PhysicalStreamType } from "../metadata/tile/physicalStreamType";
import { LogicalStreamType } from "../metadata/tile/logicalStreamType";
import { LogicalLevelTechnique } from "../metadata/tile/logicalLevelTechnique";
import { PhysicalLevelTechnique } from "../metadata/tile/physicalLevelTechnique";
import { DictionaryType } from "../metadata/tile/dictionaryType";
import { LengthType } from "../metadata/tile/lengthType";
import { OffsetType } from "../metadata/tile/offsetType";
import { type RleEncodedStreamMetadata, type StreamMetadata } from "../metadata/tile/streamMetadataDecoder";
import IntWrapper from "./intWrapper";
import { type Column, type Field, ComplexType, ScalarType } from "../metadata/tileset/tilesetMetadata";
import {
    encodeVarintInt32Array,
    encodeSingleVarintInt32,
    encodeBooleanRle,
    encodeStrings,
    createStringLengths,
} from "../encoding/encodingUtils";

/**
 * Creates basic stream metadata with logical techniques.
 */
export function createStreamMetadata(
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

/**
 * Creates RLE-encoded stream metadata.
 */
export function createRleMetadata(
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

/**
 * Creates column metadata for STRUCT type columns.
 */
export function createColumnMetadataForStruct(
    columnName: string,
    childFields: Array<{ name: string; type?: number }>,
): Column {
    const children: Field[] = childFields.map((fieldConfig) => ({
        name: fieldConfig.name,
        nullable: true,
        scalarField: {
            physicalType: fieldConfig.type ?? ScalarType.STRING,
            type: "physicalType" as const,
        },
        type: "scalarField" as const,
    }));

    return {
        name: columnName,
        nullable: false,
        complexType: {
            physicalType: ComplexType.STRUCT,
            children,
            type: "physicalType" as const,
        },
        type: "complexType" as const,
    };
}

/**
 * Creates a single stream with metadata and data.
 */
export function createStream(
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

/**
 * Creates streams for string data with optional dictionary or FSST encoding.
 */
export function createStringStreams(
    strings: (string | null)[],
    encoding: "plain" | "dictionary" | "fsst" = "plain",
): Uint8Array {
    if (encoding === "fsst") return createFsstDictionaryStringStreams();

    const hasNull = strings.some((s) => s === null);
    const nonNullStrings = strings.filter((s): s is string => s !== null);

    const uniqueStrings = Array.from(new Set(nonNullStrings));
    const stringsToEncode = encoding === "dictionary" ? uniqueStrings : nonNullStrings;
    const stringBytes = encodeStrings(stringsToEncode);

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

    // Add encoding-specific streams
    if (encoding === "plain") {
        streams.push(...createPlainStringStreams(nonNullStrings, stringBytes));
    } else {
        streams.push(...createDictionaryStringStreams(nonNullStrings, uniqueStrings, stringBytes));
    }

    return concatenateBuffers(...streams);
}

/**
 * Creates LENGTH and DATA streams for plain string encoding.
 */
function createPlainStringStreams(strings: string[], stringBytes: Uint8Array): Uint8Array[] {
    const lengths = createStringLengths(strings);
    return [
        createStream(PhysicalStreamType.LENGTH, encodeVarintInt32Array(new Int32Array(lengths)), {
            logical: new LogicalStreamType(undefined, undefined, LengthType.VAR_BINARY),
            technique: PhysicalLevelTechnique.VARINT,
            count: lengths.length,
        }),
        createStream(PhysicalStreamType.DATA, stringBytes, {
            logical: new LogicalStreamType(DictionaryType.NONE),
        }),
    ];
}

/**
 * Creates OFFSET, LENGTH, and DATA streams for dictionary string encoding.
 */
function createDictionaryStringStreams(
    strings: string[],
    uniqueStrings: string[],
    stringBytes: Uint8Array,
): Uint8Array[] {
    const stringMap = new Map(uniqueStrings.map((s, i) => [s, i]));
    const offsets = strings.map((s) => stringMap.get(s));

    const { lengthStream, dataStream } = createSharedDictionaryStreams(uniqueStrings, {
        dictionaryType: DictionaryType.SINGLE,
    });

    return [
        createStream(PhysicalStreamType.OFFSET, encodeVarintInt32Array(new Int32Array(offsets)), {
            logical: new LogicalStreamType(undefined, OffsetType.STRING),
            technique: PhysicalLevelTechnique.VARINT,
            count: offsets.length,
        }),
        lengthStream,
        dataStream,
    ];
}

/**
 * Creates FSST dictionary streams for testing.
 * Contains test data: ["cat", "dog", "cat"]
 */
export function createFsstDictionaryStringStreams(): Uint8Array {
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
        createStream(PhysicalStreamType.DATA, symbolTable, { logical: new LogicalStreamType(DictionaryType.FSST) }),
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
 * Creates LENGTH and DATA streams for shared dictionary encoding.
 * Optionally includes FSST symbol table streams.
 */
export function createSharedDictionaryStreams(
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
        return { ...createFsstSymbolStreams(), lengthStream, dataStream };
    }

    return { lengthStream, dataStream };
}

/**
 * Creates FSST symbol table streams for testing.
 */
function createFsstSymbolStreams(): {
    symbolLengthStream: Uint8Array;
    symbolDataStream: Uint8Array;
} {
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

    return { symbolLengthStream, symbolDataStream };
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
 * Creates streams for STRUCT field data.
 */
export function createStructFieldStreams(
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

function encodeNumStreams(numStreams: number): Uint8Array {
    const buffer = new Uint8Array(5);
    const offset = new IntWrapper(0);
    encodeSingleVarintInt32(numStreams, buffer, offset);
    return buffer.slice(0, offset.get());
}

function createPresentStream(presentValues: boolean[]): Uint8Array {
    const metadata = {
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
    const metadata = {
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

/**
 * Builds a complete encoded stream by combining metadata and data.
 */
export function buildEncodedStream(
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

/**
 * Encodes stream metadata into binary format.
 * - Byte 1: Stream type (physical type in upper 4 bits, logical subtype in lower 4 bits)
 * - Byte 2: Encodings (llt1[5-7], llt2[2-4], plt[0-1])
 * - Varints: numValues, byteLength
 * - If RLE: Varints: runs, numRleValues
 */
export function encodeStreamMetadata(metadata: StreamMetadata | RleEncodedStreamMetadata): Uint8Array {
    const buffer = new Uint8Array(100);
    let writeOffset = 0;

    // Byte 1: Stream type
    buffer[writeOffset++] = encodeStreamTypeByte(metadata);

    // Byte 2: Encoding techniques
    buffer[writeOffset++] = encodeEncodingsByte(metadata);

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

function encodeStreamTypeByte(metadata: StreamMetadata | RleEncodedStreamMetadata): number {
    const physicalTypeIndex = Object.values(PhysicalStreamType).indexOf(metadata.physicalStreamType);
    const lowerNibble = getLogicalSubtypeValue(metadata);
    return (physicalTypeIndex << 4) | lowerNibble;
}

function getLogicalSubtypeValue(metadata: StreamMetadata | RleEncodedStreamMetadata): number {
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

function encodeEncodingsByte(metadata: StreamMetadata | RleEncodedStreamMetadata): number {
    const llt1Index = Object.values(LogicalLevelTechnique).indexOf(metadata.logicalLevelTechnique1);
    const llt2Index = Object.values(LogicalLevelTechnique).indexOf(metadata.logicalLevelTechnique2);
    const pltIndex = Object.values(PhysicalLevelTechnique).indexOf(metadata.physicalLevelTechnique);
    return (llt1Index << 5) | (llt2Index << 2) | pltIndex;
}

function isRleMetadata(metadata: StreamMetadata | RleEncodedStreamMetadata): metadata is RleEncodedStreamMetadata {
    return "runs" in metadata && "numRleValues" in metadata;
}

/**
 * Concatenates multiple Uint8Array buffers into a single buffer.
 */
export function concatenateBuffers(...buffers: Uint8Array[]): Uint8Array {
    const totalLength = buffers.reduce((sum, buf) => sum + buf.length, 0);
    const result = new Uint8Array(totalLength);
    let offset = 0;

    for (const buffer of buffers) {
        result.set(buffer, offset);
        offset += buffer.length;
    }

    return result;
}
