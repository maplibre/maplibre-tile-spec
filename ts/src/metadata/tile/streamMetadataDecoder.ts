import { StreamMetadata } from "./streamMetadata";
import { LogicalLevelTechnique } from "./logicalLevelTechnique";
import { PhysicalLevelTechnique } from "./physicalLevelTechnique";
import { MortonEncodedStreamMetadata } from "./mortonEncodedStreamMetadata";
import { RleEncodedStreamMetadata } from "./rleEncodedStreamMetadata";
import type IntWrapper from "../../encodings/intWrapper";

export class StreamMetadataDecoder {
    public static decode(tile: Uint8Array, offset: IntWrapper): StreamMetadata {
        const streamMetadata = StreamMetadata.decode(tile, offset);
        if (streamMetadata.logicalLevelTechnique1 === LogicalLevelTechnique.MORTON) {
            return MortonEncodedStreamMetadata.decodePartial(streamMetadata, tile, offset);
        }

        if (
            (LogicalLevelTechnique.RLE === streamMetadata.logicalLevelTechnique1 ||
                LogicalLevelTechnique.RLE === streamMetadata.logicalLevelTechnique2) &&
            PhysicalLevelTechnique.NONE !== streamMetadata.physicalLevelTechnique
        ) {
            return RleEncodedStreamMetadata.decodePartial(streamMetadata, tile, offset);
        }

        return streamMetadata;
    }
}
