package com.mlt.decoder;

import com.mlt.converter.geometry.GeometryType;
import com.mlt.metadata.stream.*;
import me.lemire.integercompression.IntWrapper;
import org.apache.commons.lang3.NotImplementedException;
import org.locationtech.jts.geom.*;
import java.util.List;
import java.util.Optional;

public class GeometryDecoder {

    record GeometryColumn(List<Integer> geometryTypes, List<Integer> numGeometries, List<Integer> numParts, List<Integer> numRings,
                          List<Integer> vertexOffsets, List<Integer> mortonVertexBuffer, int[] vertexBuffer){}

    private GeometryDecoder(){}

    public static GeometryColumn decodeGeometryColumn(byte[] tile, int numStreams, IntWrapper offset) {
        var geometryTypeMetadata = StreamMetadataDecoder.decode(tile, offset);
        var geometryTypes = IntegerDecoder.decodeIntStream(tile, offset, geometryTypeMetadata, false);

        List<Integer> numGeometries = null;
        List<Integer> numParts = null;
        List<Integer> numRings = null;
        List<Integer> vertexOffsets = null;
        List<Integer> mortonVertexBuffer = null;
        int[] vertexBuffer = null;
        for(var i = 0; i < numStreams - 1; i++){
            var geometryStreamMetadata = StreamMetadataDecoder.decode(tile, offset);
            switch (geometryStreamMetadata.physicalStreamType()){
                case LENGTH:
                    switch (geometryStreamMetadata.logicalStreamType().lengthType()){
                        case GEOMETRIES:
                            numGeometries = IntegerDecoder.decodeIntStream(tile, offset, geometryStreamMetadata, false);
                            break;
                        case PARTS:
                            numParts = IntegerDecoder.decodeIntStream(tile, offset, geometryStreamMetadata, false);
                            break;
                        case RINGS:
                            numRings = IntegerDecoder.decodeIntStream(tile, offset, geometryStreamMetadata, false);
                            break;
                        case TRIANGLES:
                            throw new NotImplementedException("Not implemented yet.");
                    }
                    break;
                case OFFSET:
                    vertexOffsets = IntegerDecoder.decodeIntStream(tile, offset, geometryStreamMetadata, false);
                    break;
                case DATA:
                    if(DictionaryType.VERTEX.equals(geometryStreamMetadata.logicalStreamType().dictionaryType())){
                        //TODO: add Varint decoding
                        if(geometryStreamMetadata.physicalLevelTechnique() != PhysicalLevelTechnique.FAST_PFOR){
                            throw new IllegalArgumentException("Currently only FastPfor encoding supported for the VertexBuffer.");
                        }
                        vertexBuffer = DecodingUtils.decodeFastPfor128DeltaCoordinates(tile, geometryStreamMetadata.numValues(),
                            geometryStreamMetadata.byteLength(), offset);
                    }
                    else{
                        mortonVertexBuffer = IntegerDecoder.decodeMortonStream(tile, offset, (MortonEncodedStreamMetadata)geometryStreamMetadata);
                    }
                    break;
            }
        }

        return new GeometryColumn(geometryTypes, numGeometries, numParts, numRings, vertexOffsets, mortonVertexBuffer,
                vertexBuffer);
    }

    public static Geometry[] decodeGeometry(GeometryColumn geometryColumn){
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
                    var vertices = getICELineString(vertexBuffer, vertexOffsets, vertexOffsetsOffset, numVertices, false);
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
                    LinearRing shell = getICELinearRing(vertexBuffer, vertexOffsets, vertexOffsetsOffset, numVertices, geometryFactory);
                    vertexOffsetsOffset += numVertices;
                    for(var i = 0; i < rings.length; i++){
                        numVertices = ringOffsets.get(ringOffsetsCounter++);
                        rings[i] = getICELinearRing(vertexBuffer, vertexOffsets, vertexOffsetsOffset, numVertices, geometryFactory);
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
                        var vertices = getICELineString(vertexBuffer, vertexOffsets, vertexOffsetsOffset, numVertices, false);
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
                        LinearRing shell = getICELinearRing(vertexBuffer, vertexOffsets, vertexOffsetsOffset, numVertices, geometryFactory);
                        vertexOffsetsOffset += numVertices;
                        for(var j = 0; j < rings.length; j++){
                            numVertices = ringOffsets.get(ringOffsetsCounter++);
                            rings[i] = getICELinearRing(vertexBuffer, vertexOffsets, vertexOffsetsOffset, numVertices, geometryFactory);
                            vertexOffsetsOffset += numVertices;
                        }

                        polygons[i] = geometryFactory.createPolygon(shell, rings);
                        geometries[geometryCounter++] = geometryFactory.createMultiPolygon(polygons);
                    }
                }
            }
            else{
                throw new IllegalArgumentException("The specified geometry type is currently not supported.");
            }
        }

        return geometries;
    }

    private static LinearRing getLinearRing(int[] vertexBuffer, int startIndex, int numVertices, GeometryFactory geometryFactory){
        var linearRing = getLineString(vertexBuffer, startIndex, numVertices, true);
        return geometryFactory.createLinearRing(linearRing);
    }

    private static LinearRing getICELinearRing(int[] vertexBuffer, int[] vertexOffsets, int vertexOffset, int numVertices, GeometryFactory geometryFactory){
        var linearRing = getICELineString(vertexBuffer, vertexOffsets, vertexOffset, numVertices, true);
        return geometryFactory.createLinearRing(linearRing);
    }

    private static Coordinate[] getLineString(int[] vertexBuffer, int startIndex, int numVertices, boolean closeLineString){
        var vertices = new Coordinate[closeLineString? numVertices+1 : numVertices];
        for(var i = 0; i < numVertices * 2; i+=2){
            var x = vertexBuffer[startIndex + i];
            var y = vertexBuffer[startIndex + i + 1];
            vertices[i/2] = new Coordinate(x, y);
        }

        if(closeLineString){
            vertices[vertices.length -1] = vertices[0];
        }
        return vertices;
    }

    private static Coordinate[] getICELineString(int[] vertexBuffer, int[] vertexOffsets, int vertexOffset, int numVertices, boolean closeLineString){
        var vertices = new Coordinate[closeLineString? numVertices+1 : numVertices];
        for(var i = 0; i < numVertices * 2; i+=2){
            var offset = vertexOffsets[vertexOffset + i/2] * 2;
            var x = vertexBuffer[offset];
            var y = vertexBuffer[offset+1];
            vertices[i/2] = new Coordinate(x, y);
        }

        if(closeLineString){
            vertices[vertices.length -1] = vertices[0];
        }
        return vertices;
    }

}
