import { decodeStreamMetadata, type StreamMetadata } from "./streamMetadata";
import { decodeVarintInt32 } from "../../decoding/integerDecodingUtils";
import type IntWrapper from "../../decoding/intWrapper";

export function decodeRleEncodedStreamMetadata(tile: Uint8Array, offset: IntWrapper): RleEncodedStreamMetadata {
    const streamMetadata = decodeStreamMetadata(tile, offset);
    return decodePartialRleEncodedStreamMetadata(streamMetadata, tile, offset);
}

export function decodePartialRleEncodedStreamMetadata(
    streamMetadata: StreamMetadata,
    tile: Uint8Array,
    offset: IntWrapper,
): RleEncodedStreamMetadata {
    const rleInfo = decodeVarintInt32(tile, offset, 2);
    return {
        physicalStreamType: streamMetadata.physicalStreamType,
        logicalStreamType: streamMetadata.logicalStreamType,
        logicalLevelTechnique1: streamMetadata.logicalLevelTechnique1,
        logicalLevelTechnique2: streamMetadata.logicalLevelTechnique2,
        physicalLevelTechnique: streamMetadata.physicalLevelTechnique,
        numValues: streamMetadata.numValues,
        byteLength: streamMetadata.byteLength,
        decompressedCount: rleInfo[1],
        runs: rleInfo[0],
        numRleValues: rleInfo[1],
    };
}

export type RleEncodedStreamMetadata = StreamMetadata & {
    readonly runs: number;
    readonly numRleValues: number;
};
