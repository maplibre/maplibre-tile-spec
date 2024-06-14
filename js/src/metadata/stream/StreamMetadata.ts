import { DecodingUtils } from '../../decoder/DecodingUtils';
import { IntWrapper } from '../../decoder/IntWrapper';
import { PhysicalStreamType } from './PhysicalStreamType';
import { LogicalStreamType } from './LogicalStreamType';
import { LogicalLevelTechnique } from './LogicalLevelTechnique';
import { PhysicalLevelTechnique } from './PhysicalLevelTechnique';
import { DictionaryType } from './DictionaryType';
import { OffsetType } from './OffsetType';
import { LengthType } from './LengthType';

export class StreamMetadata {
    private physical_stream_type: PhysicalStreamType;
    private logical_stream_type: LogicalStreamType;
    private logical_level_technique1: LogicalLevelTechnique;
    private logical_level_technique2: LogicalLevelTechnique;
    private physical_level_technique: PhysicalLevelTechnique;
    private num_values: number;
    private byte_length: number;

    constructor(
        physical_stream_type: PhysicalStreamType,
        logical_stream_type: LogicalStreamType,
        logical_level_technique1: LogicalLevelTechnique,
        logical_level_technique2: LogicalLevelTechnique,
        physical_level_technique: PhysicalLevelTechnique,
        num_values: number,
        byte_length: number
    ) {
        this.physical_stream_type = physical_stream_type;
        this.logical_stream_type = logical_stream_type;
        this.logical_level_technique1 = logical_level_technique1;
        this.logical_level_technique2 = logical_level_technique2;
        this.physical_level_technique = physical_level_technique;
        this.num_values = num_values;
        this.byte_length = byte_length;
    }

    public static decode(tile: Uint8Array, offset: IntWrapper): StreamMetadata {
        const stream_type = tile[offset.get()];
        const physical_stream_type = Object.values(PhysicalStreamType)[stream_type >> 4] as PhysicalStreamType;
        let logical_stream_type: LogicalStreamType | null = null;

        switch (physical_stream_type) {
            case PhysicalStreamType.DATA:
                logical_stream_type = new LogicalStreamType(Object.values(DictionaryType)[stream_type & 0xf] as DictionaryType);
                break;
            case PhysicalStreamType.OFFSET:
                logical_stream_type = new LogicalStreamType(null, Object.values(OffsetType)[stream_type & 0xf] as OffsetType);
                break;
            case PhysicalStreamType.LENGTH:
                logical_stream_type = new LogicalStreamType(null, null, Object.values(LengthType)[stream_type & 0xf] as LengthType);
                break;
        }
        offset.increment();

        const encodings_header = tile[offset.get()] & 0xFF;
        const logical_level_technique1 = Object.values(LogicalLevelTechnique)[encodings_header >> 5] as LogicalLevelTechnique;
        const logical_level_technique2 = Object.values(LogicalLevelTechnique)[encodings_header >> 2 & 0x7] as LogicalLevelTechnique;
        const physical_level_technique = Object.values(PhysicalLevelTechnique)[encodings_header & 0x3] as PhysicalLevelTechnique;
        offset.increment();

        const size_info = DecodingUtils.decodeVarint(tile, offset, 2);
        const num_values = size_info[0];
        const byte_length = size_info[1];

        return new StreamMetadata(
            physical_stream_type,
            logical_stream_type,
            logical_level_technique1,
            logical_level_technique2,
            physical_level_technique,
            num_values,
            byte_length
        );
    }

    public physicalStreamType(): PhysicalStreamType {
        return this.physical_stream_type;
    }

    public logicalStreamType(): LogicalStreamType {
        return this.logical_stream_type;
    }

    public logicalLevelTechnique1(): LogicalLevelTechnique {
        return this.logical_level_technique1;
    }

    public logicalLevelTechnique2(): LogicalLevelTechnique {
        return this.logical_level_technique2;
    }

    public physicalLevelTechnique(): PhysicalLevelTechnique {
        return this.physical_level_technique;
    }

    public numValues(): number {
        return this.num_values;
    }

    public byteLength(): number {
        return this.byte_length;
    }
}
