import { DecodingUtils } from '../../decoder/DecodingUtils';
import { IntWrapper } from '../../decoder/IntWrapper';
import { StreamMetadata } from './StreamMetadata';
import { PhysicalStreamType } from './PhysicalStreamType';
import { LogicalStreamType } from './LogicalStreamType';
import { LogicalLevelTechnique } from './LogicalLevelTechnique';
import { PhysicalLevelTechnique } from './PhysicalLevelTechnique';

export class MortonEncodedStreamMetadata extends StreamMetadata {
    private num_bits: number;
    private coordinate_shift: number;

    constructor(
        physicalStreamType: PhysicalStreamType,
        logicalStreamType: LogicalStreamType,
        logicalLevelTechnique1: LogicalLevelTechnique,
        logicalLevelTechnique2: LogicalLevelTechnique,
        physicalLevelTechnique: PhysicalLevelTechnique,
        numValues: number,
        byteLength: number,
        numBits: number,
        coordinateShift: number
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
        this.num_bits = numBits;
        this.coordinate_shift = coordinateShift;
    }

    public static decode(tile: Uint8Array, offset: IntWrapper): MortonEncodedStreamMetadata {
        const streamMetadata = StreamMetadata.decode(tile, offset);
        const mortonInfo = DecodingUtils.decodeVarint(tile, offset, 2);
        return new MortonEncodedStreamMetadata(
            streamMetadata.physicalStreamType(),
            streamMetadata.logicalStreamType(),
            streamMetadata.logicalLevelTechnique1(),
            streamMetadata.logicalLevelTechnique2(),
            streamMetadata.physicalLevelTechnique(),
            streamMetadata.numValues(),
            streamMetadata.byteLength(),
            mortonInfo[0],
            mortonInfo[1]
        );
    }

    public static decodePartial(streamMetadata: StreamMetadata, tile: Uint8Array, offset: IntWrapper): MortonEncodedStreamMetadata {
        const mortonInfo = DecodingUtils.decodeVarint(tile, offset, 2);
        return new MortonEncodedStreamMetadata(
            streamMetadata.physicalStreamType(),
            streamMetadata.logicalStreamType(),
            streamMetadata.logicalLevelTechnique1(),
            streamMetadata.logicalLevelTechnique2(),
            streamMetadata.physicalLevelTechnique(),
            streamMetadata.numValues(),
            streamMetadata.byteLength(),
            mortonInfo[0],
            mortonInfo[1]
        );
    }

    public numBits(): number {
        return this.num_bits;
    }

    public coordinateShift(): number {
        return this.coordinate_shift;
    }
}
