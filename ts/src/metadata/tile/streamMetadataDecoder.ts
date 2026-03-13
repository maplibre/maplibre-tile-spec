import { LogicalLevelTechnique } from "./logicalLevelTechnique";
import { PhysicalLevelTechnique } from "./physicalLevelTechnique";
import { decodeVarintInt32Value } from "../../decoding/integerDecodingUtils";
import { PhysicalStreamType } from "./physicalStreamType";
import { LogicalStreamType } from "./logicalStreamType";
import { DictionaryType } from "./dictionaryType";
import { OffsetType } from "./offsetType";
import { LengthType } from "./lengthType";
import type IntWrapper from "../../decoding/intWrapper";

export type StreamMetadata = {
    readonly physicalStreamType: PhysicalStreamType;
    readonly logicalStreamType: LogicalStreamType;
    readonly logicalLevelTechnique1: LogicalLevelTechnique;
    readonly logicalLevelTechnique2: LogicalLevelTechnique;
    readonly physicalLevelTechnique: PhysicalLevelTechnique;
    readonly numValues: number;
    readonly byteLength: number;
    /**
     * Returns the number of decompressed values.
     * For non-RLE streams, this is the same as numValues.
     * For RLE streams, this is overridden to return numRleValues.
     */
    readonly decompressedCount: number;
};

export type MortonEncodedStreamMetadata = StreamMetadata & {
    readonly numBits: number;
    readonly coordinateShift: number;
};

export type RleEncodedStreamMetadata = StreamMetadata & {
    readonly runs: number;
    readonly numRleValues: number;
};

const PHYSICAL_STREAM_TYPE_BY_ID: readonly PhysicalStreamType[] = [
    PhysicalStreamType.PRESENT,
    PhysicalStreamType.DATA,
    PhysicalStreamType.OFFSET,
    PhysicalStreamType.LENGTH,
];

const LOGICAL_LEVEL_TECHNIQUE_BY_ID: readonly LogicalLevelTechnique[] = [
    LogicalLevelTechnique.NONE,
    LogicalLevelTechnique.DELTA,
    LogicalLevelTechnique.COMPONENTWISE_DELTA,
    LogicalLevelTechnique.RLE,
    LogicalLevelTechnique.MORTON,
    LogicalLevelTechnique.PDE,
];

const PHYSICAL_LEVEL_TECHNIQUE_BY_ID: readonly PhysicalLevelTechnique[] = [
    PhysicalLevelTechnique.NONE,
    PhysicalLevelTechnique.FAST_PFOR,
    PhysicalLevelTechnique.VARINT,
    PhysicalLevelTechnique.ALP,
];

const DICTIONARY_TYPE_BY_ID: readonly DictionaryType[] = [
    DictionaryType.NONE,
    DictionaryType.SINGLE,
    DictionaryType.SHARED,
    DictionaryType.VERTEX,
    DictionaryType.MORTON,
    DictionaryType.FSST,
];

const OFFSET_TYPE_BY_ID: readonly OffsetType[] = [OffsetType.VERTEX, OffsetType.INDEX, OffsetType.STRING, OffsetType.KEY];

const LENGTH_TYPE_BY_ID: readonly LengthType[] = [
    LengthType.VAR_BINARY,
    LengthType.GEOMETRIES,
    LengthType.PARTS,
    LengthType.RINGS,
    LengthType.TRIANGLES,
    LengthType.SYMBOL,
    LengthType.DICTIONARY,
];

const DEFAULT_LOGICAL_STREAM_TYPE = new LogicalStreamType();
const DATA_LOGICAL_STREAM_TYPES: readonly LogicalStreamType[] = DICTIONARY_TYPE_BY_ID.map((type) => new LogicalStreamType(type));
const OFFSET_LOGICAL_STREAM_TYPES: readonly LogicalStreamType[] = OFFSET_TYPE_BY_ID.map(
    (type) => new LogicalStreamType(undefined, type),
);
const LENGTH_LOGICAL_STREAM_TYPES: readonly LogicalStreamType[] = LENGTH_TYPE_BY_ID.map(
    (type) => new LogicalStreamType(undefined, undefined, type),
);

export function decodeStreamMetadata(tile: Uint8Array, offset: IntWrapper): StreamMetadata {
    const startOffset = offset.get();
    try {
        const streamMetadata = decodeStreamMetadataInternal(tile, offset);
        if (streamMetadata.logicalLevelTechnique1 === LogicalLevelTechnique.MORTON) {
            return decodePartialMortonEncodedStreamMetadata(streamMetadata, tile, offset);
        }

        if (
            (LogicalLevelTechnique.RLE === streamMetadata.logicalLevelTechnique1 ||
                LogicalLevelTechnique.RLE === streamMetadata.logicalLevelTechnique2) &&
            PhysicalLevelTechnique.NONE !== streamMetadata.physicalLevelTechnique
        ) {
            return decodePartialRleEncodedStreamMetadata(streamMetadata, tile, offset);
        }

        return streamMetadata;
    } catch (error) {
        offset.set(startOffset);
        throw error;
    }
}

function decodePartialMortonEncodedStreamMetadata(
    streamMetadata: StreamMetadata,
    tile: Uint8Array,
    offset: IntWrapper,
): MortonEncodedStreamMetadata {
    const numBits = decodeMetadataVarint(tile, offset, "morton metadata");
    const coordinateShift = decodeMetadataVarint(tile, offset, "morton metadata");
    return {
        physicalStreamType: streamMetadata.physicalStreamType,
        logicalStreamType: streamMetadata.logicalStreamType,
        logicalLevelTechnique1: streamMetadata.logicalLevelTechnique1,
        logicalLevelTechnique2: streamMetadata.logicalLevelTechnique2,
        physicalLevelTechnique: streamMetadata.physicalLevelTechnique,
        numValues: streamMetadata.numValues,
        byteLength: streamMetadata.byteLength,
        decompressedCount: streamMetadata.decompressedCount,
        numBits,
        coordinateShift,
    };
}

function decodePartialRleEncodedStreamMetadata(
    streamMetadata: StreamMetadata,
    tile: Uint8Array,
    offset: IntWrapper,
): RleEncodedStreamMetadata {
    const runs = decodeMetadataVarint(tile, offset, "rle metadata");
    const numRleValues = decodeMetadataVarint(tile, offset, "rle metadata");
    return {
        physicalStreamType: streamMetadata.physicalStreamType,
        logicalStreamType: streamMetadata.logicalStreamType,
        logicalLevelTechnique1: streamMetadata.logicalLevelTechnique1,
        logicalLevelTechnique2: streamMetadata.logicalLevelTechnique2,
        physicalLevelTechnique: streamMetadata.physicalLevelTechnique,
        numValues: streamMetadata.numValues,
        byteLength: streamMetadata.byteLength,
        decompressedCount: numRleValues,
        runs,
        numRleValues,
    };
}

function decodeStreamMetadataInternal(tile: Uint8Array, offset: IntWrapper): StreamMetadata {
    ensureRemaining(tile, offset, 1, "stream_type");
    const streamTypeByte = tile[offset.get()];
    const physicalStreamType = PHYSICAL_STREAM_TYPE_BY_ID[streamTypeByte >> 4];
    if (physicalStreamType === undefined) {
        throw new Error(`Invalid physical stream type: ${streamTypeByte >> 4}`);
    }
    let logicalStreamType = DEFAULT_LOGICAL_STREAM_TYPE;
    const logicalStreamTypeId = streamTypeByte & 0xf;

    switch (physicalStreamType) {
        case PhysicalStreamType.DATA:
            logicalStreamType = DATA_LOGICAL_STREAM_TYPES[logicalStreamTypeId];
            if (logicalStreamType === undefined) {
                throw new Error(`Invalid dictionary type: ${logicalStreamTypeId}`);
            }
            break;
        case PhysicalStreamType.OFFSET:
            logicalStreamType = OFFSET_LOGICAL_STREAM_TYPES[logicalStreamTypeId];
            if (logicalStreamType === undefined) {
                throw new Error(`Invalid offset type: ${logicalStreamTypeId}`);
            }
            break;
        case PhysicalStreamType.LENGTH:
            logicalStreamType = LENGTH_LOGICAL_STREAM_TYPES[logicalStreamTypeId];
            if (logicalStreamType === undefined) {
                throw new Error(`Invalid length type: ${logicalStreamTypeId}`);
            }
            break;
    }
    offset.increment();

    ensureRemaining(tile, offset, 1, "encodings_header");
    const encodingsHeader = tile[offset.get()];
    const llt1 = LOGICAL_LEVEL_TECHNIQUE_BY_ID[encodingsHeader >> 5];
    const llt2 = LOGICAL_LEVEL_TECHNIQUE_BY_ID[(encodingsHeader >> 2) & 0x7];
    const plt = PHYSICAL_LEVEL_TECHNIQUE_BY_ID[encodingsHeader & 0x3];
    if (llt1 === undefined || llt2 === undefined || plt === undefined) {
        throw new Error(`Invalid stream encoding header: ${encodingsHeader}`);
    }
    offset.increment();

    const numValues = decodeMetadataVarint(tile, offset, "numValues");
    const byteLength = decodeMetadataVarint(tile, offset, "byteLength");

    return {
        physicalStreamType,
        logicalStreamType,
        logicalLevelTechnique1: llt1,
        logicalLevelTechnique2: llt2,
        physicalLevelTechnique: plt,
        numValues,
        byteLength,
        decompressedCount: numValues,
    };
}

function ensureRemaining(tile: Uint8Array, offset: IntWrapper, needed: number, context: string): void {
    const currentOffset = offset.get();
    const remaining = tile.length - currentOffset;
    if (remaining < needed) {
        throw new RangeError(
            `truncated stream metadata while reading ${context} at offset=${currentOffset} (needed=${needed}, remaining=${remaining})`,
        );
    }
}

function decodeMetadataVarint(tile: Uint8Array, offset: IntWrapper, context: string): number {
    const currentOffset = offset.get();
    try {
        return decodeVarintInt32Value(tile, offset);
    } catch (error) {
        const remaining = tile.length - currentOffset;
        if (error instanceof RangeError) {
            throw new RangeError(
                `truncated stream metadata while reading ${context} at offset=${currentOffset} (remaining=${remaining})`,
            );
        }
        if (error instanceof Error) {
            throw new Error(`invalid stream metadata while reading ${context} at offset=${currentOffset}: ${error.message}`);
        }
        throw error;
    }
}
