import { decodeStreamMetadata, StreamMetadata } from "./streamMetadata";
import type IntWrapper from "../../decoding/intWrapper";
import { decodeVarintInt32 } from "../../decoding/integerDecodingUtils";

export function decodeMortonEncodedStreamMetadata(tile: Uint8Array, offset: IntWrapper): MortonEncodedStreamMetadata {
    const streamMetadata = decodeStreamMetadata(tile, offset);
    return decodePartialMortonEncodedStreamMetadata(streamMetadata, tile, offset);
}

export function decodePartialMortonEncodedStreamMetadata(
    streamMetadata: StreamMetadata,
    tile: Uint8Array,
    offset: IntWrapper,
): MortonEncodedStreamMetadata {
    const mortonInfo = decodeVarintInt32(tile, offset, 2);
    return {
        physicalStreamType: streamMetadata.physicalStreamType,
        logicalStreamType: streamMetadata.logicalStreamType,
        logicalLevelTechnique1: streamMetadata.logicalLevelTechnique1,
        logicalLevelTechnique2: streamMetadata.logicalLevelTechnique2,
        physicalLevelTechnique: streamMetadata.physicalLevelTechnique,
        numValues: streamMetadata.numValues,
        byteLength: streamMetadata.byteLength,
        decompressedCount: streamMetadata.decompressedCount,
        numBits: mortonInfo[0],
        coordinateShift: mortonInfo[1],
    };
}

export type MortonEncodedStreamMetadata = StreamMetadata & {
    readonly numBits: number;
    readonly coordinateShift: number;
};
