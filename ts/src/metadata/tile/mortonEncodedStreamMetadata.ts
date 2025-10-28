import { StreamMetadata } from "./streamMetadata";
import { type PhysicalStreamType } from "./physicalStreamType";
import { type LogicalStreamType } from "./logicalStreamType";
import { type LogicalLevelTechnique } from "./logicalLevelTechnique";
import { type PhysicalLevelTechnique } from "./physicalLevelTechnique";
import type IntWrapper from "../../encodings/intWrapper";
import { decodeVarintInt32 } from "../../encodings/integerDecodingUtils";

export class MortonEncodedStreamMetadata extends StreamMetadata {
    private readonly num_bits: number;
    private readonly coordinate_shift: number;

    constructor(
        physicalStreamType: PhysicalStreamType,
        logicalStreamType: LogicalStreamType,
        logicalLevelTechnique1: LogicalLevelTechnique,
        logicalLevelTechnique2: LogicalLevelTechnique,
        physicalLevelTechnique: PhysicalLevelTechnique,
        numValues: number,
        byteLength: number,
        numBits: number,
        coordinateShift: number,
    ) {
        super(
            physicalStreamType,
            logicalStreamType,
            logicalLevelTechnique1,
            logicalLevelTechnique2,
            physicalLevelTechnique,
            numValues,
            byteLength,
        );
        this.num_bits = numBits;
        this.coordinate_shift = coordinateShift;
    }

    public static decode(tile: Uint8Array, offset: IntWrapper): MortonEncodedStreamMetadata {
        const streamMetadata = StreamMetadata.decode(tile, offset);
        const mortonInfo = decodeVarintInt32(tile, offset, 2);
        return new MortonEncodedStreamMetadata(
            streamMetadata.physicalStreamType,
            streamMetadata.logicalStreamType,
            streamMetadata.logicalLevelTechnique1,
            streamMetadata.logicalLevelTechnique2,
            streamMetadata.physicalLevelTechnique,
            streamMetadata.numValues,
            streamMetadata.byteLength,
            mortonInfo[0],
            mortonInfo[1],
        );
    }

    public static decodePartial(
        streamMetadata: StreamMetadata,
        tile: Uint8Array,
        offset: IntWrapper,
    ): MortonEncodedStreamMetadata {
        const mortonInfo = decodeVarintInt32(tile, offset, 2);
        return new MortonEncodedStreamMetadata(
            streamMetadata.physicalStreamType,
            streamMetadata.logicalStreamType,
            streamMetadata.logicalLevelTechnique1,
            streamMetadata.logicalLevelTechnique2,
            streamMetadata.physicalLevelTechnique,
            streamMetadata.numValues,
            streamMetadata.byteLength,
            mortonInfo[0],
            mortonInfo[1],
        );
    }

    public numBits(): number {
        return this.num_bits;
    }

    public coordinateShift(): number {
        return this.coordinate_shift;
    }
}
