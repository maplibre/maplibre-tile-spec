import { PhysicalStreamType } from '../metadata/stream/PhysicalStreamType';
import { DictionaryType } from '../metadata/stream/DictionaryType';
import { LengthType } from '../metadata/stream/LengthType';
import { MortonEncodedStreamMetadata } from '../metadata/stream/MortonEncodedStreamMetadata';
import { IntegerDecoder } from './IntegerDecoder';
import { IntWrapper } from './IntWrapper';
import { StreamMetadataDecoder } from '../metadata/stream/StreamMetadataDecoder';
import { PhysicalLevelTechnique } from '../metadata/stream/PhysicalLevelTechnique';

export class GeometryDecoder {
    public static decodeGeometryColumn(tile: Uint8Array, numStreams: number, offset: IntWrapper): any {
        const geometryTypeMetadata = StreamMetadataDecoder.decode(tile, offset);
        const geometryTypes = IntegerDecoder.decodeIntStream(tile, offset, geometryTypeMetadata, false);
        let numGeometries = null;
        let numParts = null;
        let numRings = null;
        let vertexOffsets = null;
        let mortonVertexBuffer = null;
        const vertexBuffer = null;
        for(let i = 0; i < numStreams - 1; i++) {
            const geometryStreamMetadata = StreamMetadataDecoder.decode(tile, offset);
            const physicalStreamType = geometryStreamMetadata.physicalStreamType();
            switch (physicalStreamType) {
                case PhysicalStreamType.LENGTH: {
                    switch (geometryStreamMetadata.logicalStreamType().lengthType()){
                        case LengthType.GEOMETRIES:
                            numGeometries = IntegerDecoder.decodeIntStream(tile, offset, geometryStreamMetadata, false);
                            break;
                        case LengthType.PARTS:
                            numParts = IntegerDecoder.decodeIntStream(tile, offset, geometryStreamMetadata, false);
                            break;
                        case LengthType.RINGS:
                            numRings = IntegerDecoder.decodeIntStream(tile, offset, geometryStreamMetadata, false);
                            break;
                        case LengthType.TRIANGLES:
                            throw new Error("Not implemented yet.");
                    }
                    break;
                }
                case PhysicalStreamType.OFFSET: {
                    vertexOffsets = IntegerDecoder.decodeIntStream(tile, offset, geometryStreamMetadata, false);
                    break;
                }
                case PhysicalStreamType.DATA: {
                    if(DictionaryType.VERTEX === geometryStreamMetadata.logicalStreamType().dictionaryType()){
                        //TODO: add Varint decoding
                        if(geometryStreamMetadata.physicalLevelTechnique() != PhysicalLevelTechnique.FAST_PFOR){
                            throw new Error("Currently only FastPfor encoding supported for the VertexBuffer.");
                        }
                        // vertexBuffer = DecodingUtils.decodeFastPfor128DeltaCoordinates(tile, geometryStreamMetadata.numValues(),
                        //     geometryStreamMetadata.byteLength(), offset);
                        // TODO: implement decodeFastPfor128DeltaCoordinates
                        offset.set(offset.get() + geometryStreamMetadata.byteLength());
                    }
                    else {
                        mortonVertexBuffer = IntegerDecoder.decodeMortonStream(tile, offset, geometryStreamMetadata as MortonEncodedStreamMetadata);
                    }
                    break;
                }
            }
        }

        // TODO: return Geometry Column
        // return new GeometryColumn(geometryTypes, numGeometries, numParts, numRings, vertexOffsets, vertexBuffer, mortonVertexBuffer);
    }

}
