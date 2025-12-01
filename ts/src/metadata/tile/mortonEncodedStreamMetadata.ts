import { decodeStreamMetadata, StreamMetadata } from "./streamMetadata";
import { type PhysicalStreamType } from "./physicalStreamType";
import { type LogicalStreamType } from "./logicalStreamType";
import { type LogicalLevelTechnique } from "./logicalLevelTechnique";
import { type PhysicalLevelTechnique } from "./physicalLevelTechnique";
import type IntWrapper from "../../decoding/intWrapper";
import { decodeVarintInt32 } from "../../decoding/integerDecodingUtils";

export function decodeMortonEncodedStreamMetadata(tile: Uint8Array, offset: IntWrapper): MortonEncodedStreamMetadata {
    const streamMetadata = decodeStreamMetadata(tile, offset);
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

export function decodePartialMortonEncodedStreamMetadata(
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

    public numBits(): number {
        return this.num_bits;
    }

    public coordinateShift(): number {
        return this.coordinate_shift;
    }
}
