import { DecodingUtils } from '../../decoder/DecodingUtils';
import { IntWrapper } from '../../decoder/IntWrapper';
import { StreamMetadata } from './StreamMetadata';
import { PhysicalStreamType } from './PhysicalStreamType';
import { LogicalStreamType } from './LogicalStreamType';
import { LogicalLevelTechnique } from './LogicalLevelTechnique';
import { PhysicalLevelTechnique } from './PhysicalLevelTechnique';

export class RleEncodedStreamMetadata extends StreamMetadata {
    private runCount: number;
    private num_rle_values: number;

    constructor(
        physicalStreamType: PhysicalStreamType,
        logicalStreamType: LogicalStreamType,
        logicalLevelTechnique1: LogicalLevelTechnique,
        logicalLevelTechnique2: LogicalLevelTechnique,
        physicalLevelTechnique: PhysicalLevelTechnique,
        numValues: number,
        byteLength: number,
        runs: number,
        numRleValues: number
    ) {
        super(
            physicalStreamType,
            logicalStreamType,
            logicalLevelTechnique1,
            logicalLevelTechnique2,
            physicalLevelTechnique,
            numValues,
            byteLength
        );
        this.runCount = runs;
        this.num_rle_values = numRleValues;
    }

    public static decode(tile: Uint8Array, offset: IntWrapper): RleEncodedStreamMetadata {
        const streamMetadata = StreamMetadata.decode(tile, offset);
        const rleInfo = DecodingUtils.decodeVarint(tile, offset, 2);
        return new RleEncodedStreamMetadata(
            streamMetadata.physicalStreamType(),
            streamMetadata.logicalStreamType(),
            streamMetadata.logicalLevelTechnique1(),
            streamMetadata.logicalLevelTechnique2(),
            streamMetadata.physicalLevelTechnique(),
            streamMetadata.numValues(),
            streamMetadata.byteLength(),
            rleInfo[0],
            rleInfo[1]
        );
    }

    public static decodePartial(
        streamMetadata: StreamMetadata,
        tile: Uint8Array,
        offset: IntWrapper
    ): RleEncodedStreamMetadata {
        const rleInfo = DecodingUtils.decodeVarint(tile, offset, 2);
        return new RleEncodedStreamMetadata(
            streamMetadata.physicalStreamType(),
            streamMetadata.logicalStreamType(),
            streamMetadata.logicalLevelTechnique1(),
            streamMetadata.logicalLevelTechnique2(),
            streamMetadata.physicalLevelTechnique(),
            streamMetadata.numValues(),
            streamMetadata.byteLength(),
            rleInfo[0],
            rleInfo[1]
        );
    }

    public runs(): number {
        return this.runCount;
    }

    public numRleValues(): number {
        return this.num_rle_values;
    }
}
