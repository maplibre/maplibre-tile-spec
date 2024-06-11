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
    }

    /*
    public static Geometry[] decodeGeometry(any : geometryColumn){
        var geometries = new Geometry[geometryColumn.geometryTypes.size()];
        var partOffsetCounter = 0;
        var ringOffsetsCounter = 0;
        var geometryOffsetsCounter = 0;
        var geometryCounter = 0;
        var geometryFactory = new GeometryFactory();
        var vertexBufferOffset = 0;
        var vertexOffsetsOffset = 0;

        var geometryTypes = geometryColumn.geometryTypes();
        var geometryOffsets = geometryColumn.numGeometries();
        var partOffsets = geometryColumn.numParts();
        var ringOffsets = geometryColumn.numRings();
        var vertexOffsets =  geometryColumn.vertexOffsets() != null?
                geometryColumn.vertexOffsets().stream().mapToInt(i -> i).toArray() : null;

        var vertexBuffer = geometryColumn.mortonVertexBuffer != null?
                geometryColumn.mortonVertexBuffer.stream().mapToInt(i -> i).toArray():
                geometryColumn.vertexBuffer();

        //TODO: refactor redundant code
        for(var geometryType : geometryTypes){
            if(geometryType.equals(GeometryType.POINT.ordinal())){
                if(vertexOffsets == null || vertexOffsets.length == 0){
                    var x = vertexBuffer[vertexBufferOffset++];
                    var y = vertexBuffer[vertexBufferOffset++];
                    var coordinate = new Coordinate(x, y);
                    geometries[geometryCounter++] = geometryFactory.createPoint(coordinate);
                }
                else{
                    var offset = vertexOffsets[vertexOffsetsOffset++] * 2;
                    var x = vertexBuffer[offset];
                    var y = vertexBuffer[offset+1];
                    var coordinate = new Coordinate(x, y);
                    geometries[geometryCounter++] = geometryFactory.createPoint(coordinate);
                }
            }
            else if(geometryType.equals(GeometryType.LINESTRING.ordinal())){
                if(vertexOffsets == null || vertexOffsets.length == 0){
                    var numVertices = partOffsets.get(partOffsetCounter++);
                    var vertices = getLineString(vertexBuffer, vertexBufferOffset, numVertices, false);
                    vertexBufferOffset += numVertices * 2;
                    geometries[geometryCounter++] = geometryFactory.createLineString(vertices);
                }
                else{
                    var numVertices = partOffsets.get(partOffsetCounter++);
                    var vertices = decodeDictionaryEncodedLineString(vertexBuffer, vertexOffsets, vertexOffsetsOffset, numVertices, false);
                    vertexOffsetsOffset += numVertices;
                    geometries[geometryCounter++] = geometryFactory.createLineString(vertices);
                }
            }
            else if(geometryType.equals(GeometryType.POLYGON.ordinal())){
                var numRings = partOffsets.get(partOffsetCounter++);
                var rings = new LinearRing[numRings - 1];
                var numVertices= ringOffsets.get(ringOffsetsCounter++);
                if(vertexOffsets == null || vertexOffsets.length == 0){
                    LinearRing shell = getLinearRing(vertexBuffer, vertexBufferOffset, numVertices, geometryFactory);
                    vertexBufferOffset += numVertices * 2;
                    for(var i = 0; i < rings.length; i++){
                        numVertices = ringOffsets.get(ringOffsetsCounter++);
                        rings[i] = getLinearRing(vertexBuffer, vertexBufferOffset, numVertices, geometryFactory);
                        vertexBufferOffset += numVertices * 2;
                    }
                    geometries[geometryCounter++] = geometryFactory.createPolygon(shell, rings);
                }
                else{
                    LinearRing shell = decodeDictionaryEncodedLinearRing(vertexBuffer, vertexOffsets, vertexOffsetsOffset, numVertices, geometryFactory);
                    vertexOffsetsOffset += numVertices;
                    for(var i = 0; i < rings.length; i++){
                        numVertices = ringOffsets.get(ringOffsetsCounter++);
                        rings[i] = decodeDictionaryEncodedLinearRing(vertexBuffer, vertexOffsets, vertexOffsetsOffset, numVertices, geometryFactory);
                        vertexOffsetsOffset += numVertices;
                    }
                    geometries[geometryCounter++] = geometryFactory.createPolygon(shell, rings);
                }
            }
            else if(geometryType.equals(GeometryType.MULTILINESTRING.ordinal())){
                var numLineStrings = geometryOffsets.get(geometryOffsetsCounter++);
                var lineStrings = new LineString[numLineStrings];
                if(vertexOffsets == null || vertexOffsets.length == 0){
                    for(var i = 0; i < numLineStrings; i++){
                        var numVertices = partOffsets.get(partOffsetCounter++);
                        var vertices = getLineString(vertexBuffer, vertexBufferOffset, numVertices, false);
                        lineStrings[i] = geometryFactory.createLineString(vertices);
                        vertexBufferOffset += numVertices * 2;
                    }
                    geometries[geometryCounter++] = geometryFactory.createMultiLineString(lineStrings);
                }
                else{
                    for(var i = 0; i < numLineStrings; i++){
                        var numVertices = partOffsets.get(partOffsetCounter++);
                        var vertices = decodeDictionaryEncodedLineString(vertexBuffer, vertexOffsets, vertexOffsetsOffset, numVertices, false);
                        lineStrings[i] = geometryFactory.createLineString(vertices);
                        vertexOffsetsOffset += numVertices;
                    }
                    geometries[geometryCounter++] = geometryFactory.createMultiLineString(lineStrings);
                }
            }
            else if(geometryType.equals(GeometryType.MULTIPOLYGON.ordinal())){
                var numPolygons = geometryOffsets.get(geometryOffsetsCounter++);
                var polygons = new Polygon[numPolygons];
                var numVertices = 0;
                if(vertexOffsets == null || vertexOffsets.length == 0){
                    for(var i = 0; i < numPolygons; i++){
                        var numRings = partOffsets.get(partOffsetCounter++);
                        var rings = new LinearRing[numRings - 1];
                        numVertices += ringOffsets.get(ringOffsetsCounter++);
                        LinearRing shell = getLinearRing(vertexBuffer, vertexBufferOffset, numVertices, geometryFactory);
                        vertexBufferOffset += numVertices * 2;
                        for(var j = 0; j < rings.length; j++){
                            var numRingVertices = ringOffsets.get(ringOffsetsCounter++);
                            rings[i] = getLinearRing(vertexBuffer, vertexBufferOffset, numRingVertices, geometryFactory);
                            vertexBufferOffset += numVertices * 2;
                        }

                        polygons[i] = geometryFactory.createPolygon(shell, rings);
                    }
                    geometries[geometryCounter++] = geometryFactory.createMultiPolygon(polygons);
                }
                else{
                    for(var i = 0; i < numPolygons; i++){
                        var numRings = partOffsets.get(partOffsetCounter++);
                        var rings = new LinearRing[numRings - 1];
                        numVertices += ringOffsets.get(ringOffsetsCounter++);
                        LinearRing shell = decodeDictionaryEncodedLinearRing(vertexBuffer, vertexOffsets, vertexOffsetsOffset, numVertices, geometryFactory);
                        vertexOffsetsOffset += numVertices;
                        for(var j = 0; j < rings.length; j++){
                            numVertices = ringOffsets.get(ringOffsetsCounter++);
                            rings[i] = decodeDictionaryEncodedLinearRing(vertexBuffer, vertexOffsets, vertexOffsetsOffset, numVertices, geometryFactory);
                            vertexOffsetsOffset += numVertices;
                        }

                        polygons[i] = geometryFactory.createPolygon(shell, rings);
                        geometries[geometryCounter++] = geometryFactory.createMultiPolygon(polygons);
                    }
                }
            }
            else{
                throw new Error("The specified geometry type is currently not supported.");
            }
        }

        return geometries;
    }
    */
}
