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

export function createStructFieldStreams(
    offsetIndices: number[],
    presentValues: boolean[],
    isPresent: boolean = true,
): Uint8Array {
    if (!isPresent) {
        // Field not present in tile: encode numStreams = 0
        const buffer = new Uint8Array(5);
        const offset = new IntWrapper(0);
        encodeSingleVarintInt32(0, buffer, offset);
        return buffer.slice(0, offset.get());
    }

    // Encode numStreams = 2 (PRESENT + OFFSET streams)
    const numStreamsBuffer = new Uint8Array(5);
    const numStreamsOffset = new IntWrapper(0);
    encodeSingleVarintInt32(2, numStreamsBuffer, numStreamsOffset);
    const numStreamsEncoded = numStreamsBuffer.slice(0, numStreamsOffset.get());

    // Encode PRESENT stream (Boolean RLE)
    const presentMetadata = {
        physicalStreamType: PhysicalStreamType.PRESENT,
        logicalStreamType: new LogicalStreamType(DictionaryType.NONE),
        logicalLevelTechnique1: LogicalLevelTechnique.NONE,
        logicalLevelTechnique2: LogicalLevelTechnique.NONE,
        physicalLevelTechnique: PhysicalLevelTechnique.VARINT,
        numValues: presentValues.length,
        byteLength: 0,
        decompressedCount: presentValues.length,
    };
    const encodedPresent = buildEncodedStream(presentMetadata, encodeBooleanRle(presentValues));

    // Encode OFFSET stream (dictionary indices)
    const offsetMetadata = {
        physicalStreamType: PhysicalStreamType.OFFSET,
        logicalStreamType: new LogicalStreamType(undefined, OffsetType.STRING),
        logicalLevelTechnique1: LogicalLevelTechnique.NONE,
        logicalLevelTechnique2: LogicalLevelTechnique.NONE,
        physicalLevelTechnique: PhysicalLevelTechnique.VARINT,
        numValues: offsetIndices.length,
        byteLength: 0,
        decompressedCount: offsetIndices.length,
    };
    const encodedOffsets = buildEncodedStream(offsetMetadata, encodeVarintInt32Array(new Int32Array(offsetIndices)));

    return concatenateBuffers(numStreamsEncoded, encodedPresent, encodedOffsets);
}

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

    if (hasNull) {
        const nullabilityValues = strings.map((s) => s !== null);
        streams.push(
            createStream(PhysicalStreamType.PRESENT, encodeBooleanRle(nullabilityValues), {
                technique: PhysicalLevelTechnique.VARINT,
                count: nullabilityValues.length,
            }),
        );
    }

    if (encoding === "plain") {
        const lengths = createStringLengths(nonNullStrings);
        streams.push(
            createStream(PhysicalStreamType.LENGTH, encodeVarintInt32Array(new Int32Array(lengths)), {
                logical: new LogicalStreamType(undefined, undefined, LengthType.VAR_BINARY),
                technique: PhysicalLevelTechnique.VARINT,
                count: lengths.length,
            }),
            createStream(PhysicalStreamType.DATA, stringBytes, {
                logical: new LogicalStreamType(DictionaryType.NONE),
            }),
        );
    } else {
        const stringMap = new Map(uniqueStrings.map((s, i) => [s, i]));
        const offsets = nonNullStrings.map((s) => stringMap.get(s));

        const { lengthStream, dataStream } = createSharedDictionaryStreams(uniqueStrings, {
            dictionaryType: DictionaryType.SINGLE,
        });

        streams.push(
            createStream(PhysicalStreamType.OFFSET, encodeVarintInt32Array(new Int32Array(offsets)), {
                logical: new LogicalStreamType(undefined, OffsetType.STRING),
                technique: PhysicalLevelTechnique.VARINT,
                count: offsets.length,
            }),
            lengthStream,
            dataStream,
        );
    }

    return concatenateBuffers(...streams);
}

export function createFsstDictionaryStringStreams(): Uint8Array {
    // Hardcoded FSST test data
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

    // Standard Dictionary Streams
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
        // FSST Symbol Table Streams Hardcoded for test
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

export function buildEncodedStream(
    streamMetadata: StreamMetadata | RleEncodedStreamMetadata,
    encodedData: Uint8Array,
): Uint8Array {
    // Update byteLength to match actual encoded data length
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

export function encodeStreamMetadata(metadata: StreamMetadata | RleEncodedStreamMetadata): Uint8Array {
    const buffer = new Uint8Array(100); // Oversized, will trim
    let writeOffset = 0;

    // Encode stream type byte (first byte)
    // physicalStreamType in upper 4 bits, type-specific value in lower 4 bits
    const physicalTypeIndex = Object.values(PhysicalStreamType).indexOf(metadata.physicalStreamType);
    let lowerNibble = 0;

    switch (metadata.physicalStreamType) {
        case PhysicalStreamType.DATA:
            lowerNibble =
                metadata.logicalStreamType.dictionaryType !== undefined
                    ? Object.values(DictionaryType).indexOf(metadata.logicalStreamType.dictionaryType)
                    : 0;
            break;
        case PhysicalStreamType.OFFSET:
            lowerNibble =
                metadata.logicalStreamType.offsetType !== undefined
                    ? Object.values(OffsetType).indexOf(metadata.logicalStreamType.offsetType)
                    : 0;
            break;
        case PhysicalStreamType.LENGTH:
            lowerNibble =
                metadata.logicalStreamType.lengthType !== undefined
                    ? Object.values(LengthType).indexOf(metadata.logicalStreamType.lengthType)
                    : 0;
            break;
    }

    const streamTypeByte = (physicalTypeIndex << 4) | lowerNibble;
    buffer[writeOffset++] = streamTypeByte;

    // Encode encodings header byte (second byte)
    // llt1 in bits 5-7, llt2 in bits 2-4, plt in bits 0-1
    const llt1Index = Object.values(LogicalLevelTechnique).indexOf(metadata.logicalLevelTechnique1);
    const llt2Index = Object.values(LogicalLevelTechnique).indexOf(metadata.logicalLevelTechnique2);
    const pltIndex = Object.values(PhysicalLevelTechnique).indexOf(metadata.physicalLevelTechnique);
    const encodingsHeader = (llt1Index << 5) | (llt2Index << 2) | pltIndex;
    buffer[writeOffset++] = encodingsHeader;

    // Encode numValues and byteLength as varints
    const offset = new IntWrapper(writeOffset);
    encodeSingleVarintInt32(metadata.numValues, buffer, offset);
    encodeSingleVarintInt32(metadata.byteLength, buffer, offset);

    // If RLE, encode runs and numRleValues
    if ("runs" in metadata && "numRleValues" in metadata) {
        encodeSingleVarintInt32(metadata.runs, buffer, offset);
        encodeSingleVarintInt32(metadata.numRleValues, buffer, offset);
    }

    return buffer.slice(0, offset.get());
}

export function encodeSingleVarintInt32(value: number, dst: Uint8Array, offset: IntWrapper): void {
    let v = value;
    while (v > 0x7f) {
        dst[offset.get()] = (v & 0x7f) | 0x80;
        offset.increment();
        v >>>= 7;
    }
    dst[offset.get()] = v & 0x7f;
    offset.increment();
}

export function encodeVarintInt32Array(values: Int32Array): Uint8Array {
    const buffer = new Uint8Array(values.length * 5);
    const offset = new IntWrapper(0);

    for (const value of values) {
        encodeSingleVarintInt32(value, buffer, offset);
    }
    return buffer.slice(0, offset.get());
}

export function encodeZigZag32(value: number): number {
    return (value << 1) ^ (value >> 31);
}

//Used for Morton encoding
export function encodeDelta(values: Int32Array): Int32Array {
    if (values.length === 0) return new Int32Array(0);

    const result = new Int32Array(values.length);
    result[0] = values[0];

    for (let i = 1; i < values.length; i++) {
        result[i] = values[i] - values[i - 1];
    }

    return result;
}

export function encodeSingleVarintInt64(value: bigint, dst: Uint8Array, offset: IntWrapper): void {
    let v = value;
    while (v > 0x7fn) {
        dst[offset.get()] = Number(v & 0x7fn) | 0x80;
        offset.increment();
        v >>= 7n;
    }
    dst[offset.get()] = Number(v & 0x7fn);
    offset.increment();
}

export function encodeVarintInt64Array(values: BigInt64Array): Uint8Array {
    const buffer = new Uint8Array(values.length * 10);
    const offset = new IntWrapper(0);

    for (const value of values) {
        encodeSingleVarintInt64(value, buffer, offset);
    }
    return buffer.slice(0, offset.get());
}

export function encodeZigZag64(value: bigint): bigint {
    return (value << 1n) ^ (value >> 63n);
}

export function encodeFloatsLE(values: Float32Array): Uint8Array {
    const buffer = new Uint8Array(values.length * 4);
    const view = new DataView(buffer.buffer);

    for (let i = 0; i < values.length; i++) {
        view.setFloat32(i * 4, values[i], true);
    }

    return buffer;
}

export function encodeDoubleLE(values: Float32Array): Uint8Array {
    const buffer = new Uint8Array(values.length * 8);
    const view = new DataView(buffer.buffer);

    for (let i = 0; i < values.length; i++) {
        view.setFloat64(i * 8, values[i], true);
    }

    return buffer;
}

export function encodeBooleanRle(values: boolean[]): Uint8Array {
    // Pack booleans into bytes (8 booleans per byte)
    const numBytes = Math.ceil(values.length / 8);
    const packed = new Uint8Array(numBytes);

    for (let i = 0; i < values.length; i++) {
        if (values[i]) {
            const byteIndex = Math.floor(i / 8);
            const bitIndex = i % 8;
            packed[byteIndex] |= 1 << bitIndex;
        }
    }

    const result = new Uint8Array(1 + numBytes);
    result[0] = 256 - numBytes;
    result.set(packed, 1);

    return result;
}

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

export function encodeStrings(strings: string[]): Uint8Array {
    const encoder = new TextEncoder();
    const encoded = strings.map((s) => encoder.encode(s));
    const totalLength = encoded.reduce((sum, arr) => sum + arr.length, 0);
    const result = new Uint8Array(totalLength);
    let offset = 0;
    for (const arr of encoded) {
        result.set(arr, offset);
        offset += arr.length;
    }
    return result;
}

export function createStringLengths(strings: string[]): Int32Array {
    const lengths = new Int32Array(strings.length);
    const encoder = new TextEncoder();
    for (let i = 0; i < strings.length; i++) {
        lengths[i] = encoder.encode(strings[i]).length;
    }
    return lengths;
}
