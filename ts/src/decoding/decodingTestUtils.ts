import { PhysicalStreamType } from "../metadata/tile/physicalStreamType";
import { LogicalStreamType } from "../metadata/tile/logicalStreamType";
import { LogicalLevelTechnique } from "../metadata/tile/logicalLevelTechnique";
import { PhysicalLevelTechnique } from "../metadata/tile/physicalLevelTechnique";
import { DictionaryType } from "../metadata/tile/dictionaryType";
import { LengthType } from "../metadata/tile/lengthType";
import { OffsetType } from "../metadata/tile/offsetType";
import { type RleEncodedStreamMetadata, type StreamMetadata } from "../metadata/tile/streamMetadataDecoder";
import IntWrapper from "./intWrapper";

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

export function buildEncodedStream(
    streamMetadata: StreamMetadata | RleEncodedStreamMetadata,
    encodedData: Uint8Array
): Uint8Array {
    // Update byteLength to match actual encoded data length
    const updatedMetadata = {
        ...streamMetadata,
        byteLength: encodedData.length
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
            lowerNibble = metadata.logicalStreamType.dictionaryType !== undefined
                ? Object.values(DictionaryType).indexOf(metadata.logicalStreamType.dictionaryType)
                : 0;
            break;
        case PhysicalStreamType.OFFSET:
            lowerNibble = metadata.logicalStreamType.offsetType !== undefined
                ? Object.values(OffsetType).indexOf(metadata.logicalStreamType.offsetType)
                : 0;
            break;
        case PhysicalStreamType.LENGTH:
            lowerNibble = metadata.logicalStreamType.lengthType !== undefined
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
    if ('runs' in metadata && 'numRleValues' in metadata) {
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

export function encodeBooleanRle(values: boolean[]): Uint8Array {
    // Pack booleans into bytes (8 booleans per byte)
    const numBytes = Math.ceil(values.length / 8);
    const packed = new Uint8Array(numBytes);

    for (let i = 0; i < values.length; i++) {
        if (values[i]) {
            const byteIndex = Math.floor(i / 8);
            const bitIndex = i % 8;
            packed[byteIndex] |= (1 << bitIndex);
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
    const encoded = strings.map(s => encoder.encode(s));
    const totalLength = encoded.reduce((sum, arr) => sum + arr.length, 0);
    const result = new Uint8Array(totalLength);
    let offset = 0;
    for (const arr of encoded) {
        result.set(arr, offset);
        offset += arr.length;
    }
    return result;
}

export function createStringOffsets(strings: string[]): Int32Array {
    const offsets = new Int32Array(strings.length + 1);
    let currentOffset = 0;
    const encoder = new TextEncoder();
    for (let i = 0; i < strings.length; i++) {
        offsets[i] = currentOffset;
        currentOffset += encoder.encode(strings[i]).length;
    }
    offsets[strings.length] = currentOffset;
    return offsets;
}

export function createStringLengths(strings: string[]): Int32Array {
    const lengths = new Int32Array(strings.length);
    const encoder = new TextEncoder();
    for (let i = 0; i < strings.length; i++) {
        lengths[i] = encoder.encode(strings[i]).length;
    }
    return lengths;
}
