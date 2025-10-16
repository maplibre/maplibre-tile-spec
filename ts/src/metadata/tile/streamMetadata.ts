import { PhysicalStreamType } from "./physicalStreamType";
import { LogicalStreamType } from "./logicalStreamType";
import { LogicalLevelTechnique } from "./logicalLevelTechnique";
import { PhysicalLevelTechnique } from "./physicalLevelTechnique";
import { DictionaryType } from "./dictionaryType";
import { OffsetType } from "./offsetType";
import { LengthType } from "./lengthType";
import type IntWrapper from "../../encodings/intWrapper";
import { decodeVarintInt32 } from "../../encodings/integerDecodingUtils";

export class StreamMetadata {
    constructor(
        private readonly _physicalStreamType: PhysicalStreamType,
        private readonly _logicalStreamType: LogicalStreamType,
        private readonly _logicalLevelTechnique1: LogicalLevelTechnique,
        private readonly _logicalLevelTechnique2: LogicalLevelTechnique,
        private readonly _physicalLevelTechnique: PhysicalLevelTechnique,
        private readonly _numValues: number,
        private readonly _byteLength: number,
    ) {}

    public static decode(tile: Uint8Array, offset: IntWrapper): StreamMetadata {
        const stream_type = tile[offset.get()];
        const physicalStreamType = Object.values(PhysicalStreamType)[stream_type >> 4] as PhysicalStreamType;
        let logicalStreamType: LogicalStreamType | null = null;

        switch (physicalStreamType) {
            case PhysicalStreamType.DATA:
                logicalStreamType = new LogicalStreamType(
                    Object.values(DictionaryType)[stream_type & 0xf] as DictionaryType,
                );
                break;
            case PhysicalStreamType.OFFSET:
                logicalStreamType = new LogicalStreamType(
                    null,
                    Object.values(OffsetType)[stream_type & 0xf] as OffsetType,
                );
                break;
            case PhysicalStreamType.LENGTH:
                logicalStreamType = new LogicalStreamType(
                    null,
                    null,
                    Object.values(LengthType)[stream_type & 0xf] as LengthType,
                );
                break;
        }
        offset.increment();

        const encodings_header = tile[offset.get()];
        const llt1 = Object.values(LogicalLevelTechnique)[encodings_header >> 5] as LogicalLevelTechnique;
        const llt2 = Object.values(LogicalLevelTechnique)[(encodings_header >> 2) & 0x7] as LogicalLevelTechnique;
        const plt = Object.values(PhysicalLevelTechnique)[encodings_header & 0x3] as PhysicalLevelTechnique;
        offset.increment();

        const sizeInfo = decodeVarintInt32(tile, offset, 2);
        const numValues = sizeInfo[0];
        const byteLength = sizeInfo[1];

        return new StreamMetadata(physicalStreamType, logicalStreamType, llt1, llt2, plt, numValues, byteLength);
    }

    get physicalStreamType(): PhysicalStreamType {
        return this._physicalStreamType;
    }

    get logicalStreamType(): LogicalStreamType {
        return this._logicalStreamType;
    }

    get logicalLevelTechnique1(): LogicalLevelTechnique {
        return this._logicalLevelTechnique1;
    }

    get logicalLevelTechnique2(): LogicalLevelTechnique {
        return this._logicalLevelTechnique2;
    }

    get physicalLevelTechnique(): PhysicalLevelTechnique {
        return this._physicalLevelTechnique;
    }

    get numValues(): number {
        return this._numValues;
    }

    get byteLength(): number {
        return this._byteLength;
    }
}
