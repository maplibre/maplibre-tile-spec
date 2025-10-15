import { StreamMetadata } from './streamMetadata';
import { PhysicalStreamType } from './physicalStreamType';
import { LogicalStreamType } from './logicalStreamType';
import { LogicalLevelTechnique } from './logicalLevelTechnique';
import { PhysicalLevelTechnique } from './physicalLevelTechnique';
import IntWrapper from "../../encodings/intWrapper";
import {decodeVarintInt32} from "../../encodings/integerDecodingUtils";

export class RleEncodedStreamMetadata extends StreamMetadata {

    /**
     * @param numValues After LogicalLevelTechnique was applied -> numRuns + numValues
     * @param _runs Length of the runs array
     * @param _numRleValues Used for pre-allocating the arrays on the client for faster decoding
     */
    constructor(
        physicalStreamType: PhysicalStreamType,
        logicalStreamType: LogicalStreamType,
        logicalLevelTechnique1: LogicalLevelTechnique,
        logicalLevelTechnique2: LogicalLevelTechnique,
        physicalLevelTechnique: PhysicalLevelTechnique,
        numValues: number,
        byteLength: number,
        private readonly _runs: number,
        private readonly _numRleValues: number
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
    }

    static decode(tile: Uint8Array, offset: IntWrapper): RleEncodedStreamMetadata {
        const streamMetadata = StreamMetadata.decode(tile, offset);
        const rleInfo = decodeVarintInt32(tile, offset, 2);
        return new RleEncodedStreamMetadata(
            streamMetadata.physicalStreamType,
            streamMetadata.logicalStreamType,
            streamMetadata.logicalLevelTechnique1,
            streamMetadata.logicalLevelTechnique2,
            streamMetadata.physicalLevelTechnique,
            streamMetadata.numValues,
            streamMetadata.byteLength,
            rleInfo[0],
            rleInfo[1]
        );
    }

    static decodePartial(
        streamMetadata: StreamMetadata,
        tile: Uint8Array,
        offset: IntWrapper
    ): RleEncodedStreamMetadata {
        const rleInfo = decodeVarintInt32(tile, offset, 2);
        return new RleEncodedStreamMetadata(
            streamMetadata.physicalStreamType,
            streamMetadata.logicalStreamType,
            streamMetadata.logicalLevelTechnique1,
            streamMetadata.logicalLevelTechnique2,
            streamMetadata.physicalLevelTechnique,
            streamMetadata.numValues,
            streamMetadata.byteLength,
            rleInfo[0],
            rleInfo[1]
        );
    }

    public get runs(): number {
        return this._runs;
    }

    get numRleValues(): number {
        return this._numRleValues;
    }
}
