import { IntWrapper } from '../../decoder/IntWrapper';
import { StreamMetadata } from './StreamMetadata';
import { LogicalLevelTechnique } from './LogicalLevelTechnique';
import { PhysicalLevelTechnique } from './PhysicalLevelTechnique';
import { MortonEncodedStreamMetadata } from './MortonEncodedStreamMetadata';
import { RleEncodedStreamMetadata } from './RleEncodedStreamMetadata';

export class StreamMetadataDecoder {
    public static decode(tile: Uint8Array, offset: IntWrapper): StreamMetadata {
        const streamMetadata = StreamMetadata.decode(tile, offset);
        if (streamMetadata.logicalLevelTechnique1() === LogicalLevelTechnique.MORTON) {
            return MortonEncodedStreamMetadata.decodePartial(streamMetadata, tile, offset);
        } else if (
            (LogicalLevelTechnique.RLE === streamMetadata.logicalLevelTechnique1() ||
                LogicalLevelTechnique.RLE === streamMetadata.logicalLevelTechnique2()) &&
            PhysicalLevelTechnique.NONE !== streamMetadata.physicalLevelTechnique()
        ) {
            return RleEncodedStreamMetadata.decodePartial(streamMetadata, tile, offset);
        }

        return streamMetadata;
    }
}
