import { PhysicalStreamType } from "./physicalStreamType";
import { LogicalStreamType } from "./logicalStreamType";
import { LogicalLevelTechnique } from "./logicalLevelTechnique";
import { PhysicalLevelTechnique } from "./physicalLevelTechnique";
import { DictionaryType } from "./dictionaryType";
import { OffsetType } from "./offsetType";
import { LengthType } from "./lengthType";
import { decodeVarintInt32 } from "../../decoding/integerDecodingUtils";
import type IntWrapper from "../../decoding/intWrapper";

export function decodeStreamMetadata(tile: Uint8Array, offset: IntWrapper): StreamMetadata {
    const stream_type = tile[offset.get()];
    const physicalStreamType = Object.values(PhysicalStreamType)[stream_type >> 4] as PhysicalStreamType;
    let logicalStreamType: LogicalStreamType | null = null;

    switch (physicalStreamType) {
        case PhysicalStreamType.DATA:
            logicalStreamType = new LogicalStreamType(
                Object.values(DictionaryType)[stream_type & 0xf] as DictionaryType,
            );
            break;
        case PhysicalStreamType.OFFSET:
            logicalStreamType = new LogicalStreamType(null, Object.values(OffsetType)[stream_type & 0xf] as OffsetType);
            break;
        case PhysicalStreamType.LENGTH:
            logicalStreamType = new LogicalStreamType(
                null,
                null,
                Object.values(LengthType)[stream_type & 0xf] as LengthType,
            );
            break;
    }
    offset.increment();

    const encodings_header = tile[offset.get()];
    const llt1 = Object.values(LogicalLevelTechnique)[encodings_header >> 5] as LogicalLevelTechnique;
    const llt2 = Object.values(LogicalLevelTechnique)[(encodings_header >> 2) & 0x7] as LogicalLevelTechnique;
    const plt = Object.values(PhysicalLevelTechnique)[encodings_header & 0x3] as PhysicalLevelTechnique;
    offset.increment();

    const sizeInfo = decodeVarintInt32(tile, offset, 2);
    const numValues = sizeInfo[0];
    const byteLength = sizeInfo[1];

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
