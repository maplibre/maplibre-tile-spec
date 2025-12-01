import { decodeStreamMetadata, type StreamMetadata } from "./streamMetadata";
import { LogicalLevelTechnique } from "./logicalLevelTechnique";
import { PhysicalLevelTechnique } from "./physicalLevelTechnique";
import { decodePartialMortonEncodedStreamMetadata } from "./mortonEncodedStreamMetadata";
import { decodePartialRleEncodedStreamMetadata } from "./rleEncodedStreamMetadata";
import type IntWrapper from "../../decoding/intWrapper";

export function decodeStreamMetadataExtended(tile: Uint8Array, offset: IntWrapper): StreamMetadata {
    const streamMetadata = decodeStreamMetadata(tile, offset);
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
}
