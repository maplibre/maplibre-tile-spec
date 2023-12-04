package com.covt.decoder;

import com.covt.converter.*;
import com.covt.converter.mvt.Feature;
import com.covt.converter.mvt.Layer;
import me.lemire.integercompression.IntWrapper;
import org.locationtech.jts.geom.Coordinate;
import org.locationtech.jts.geom.Geometry;
import org.locationtech.jts.geom.GeometryFactory;
import org.locationtech.jts.geom.LineString;
import org.locationtech.jts.geom.LinearRing;
import org.locationtech.jts.geom.Polygon;
import java.io.IOException;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.BitSet;
import java.util.HashMap;
import java.util.List;
import java.util.Optional;
import java.util.LinkedHashMap;

record Header(int version, int numLayers) { }

enum GeometryType {
    POINT,
    LINESTRING,
    POLYGON,
    MULTIPOINT,
    MULTILINESTRING,
    MULTIPOLYGON
}

record GeometryColumn (
    GeometryType[] geometryTypes,
    int[] geometryOffsets,
    int[] partOffsets,
    int[] ringOffsets,
    int[] vertexOffsets,
    int[] vertexBuffer
){}


public class CovtParser {
    private static final String ID_COLUMN_NAME = "id";
    private static final String GEOMETRY_COLUMN_NAME = "geometry";
    private static final String GEOMETRY_TYPES_STREAM_NAME = "geometry_types";
    private static final String GEOMETRY_OFFSETS_STREAM_NAME = "geometry_offsets";
    private static final String PART_OFFSETS_STREAM_NAME = "part_offsets";
    private static final String RING_OFFSETS_STREAM_NAME = "ring_offsets";
    private static final String  VERTEX_OFFSETS_STREAM_NAME = "vertex_offsets";
    private static final String  VERTEX_BUFFER_STREAM_NAME = "vertex_buffer";
    public static final String PRESENT_STREAM_NAME = "present";
    private static final String DATA_STREAM_NAME = "data";
    public static final String DICTIONARY_STREAM_NAME = "dictionary";

    /*
    * COVT file structure:
    * - Header -> Version (UInt32 (Varint)) | numLayers (UInt32 (Varint))
    * - Layer metadata -> Name | Extent | numFeatures | numColumns | (columnName, columnType (byte), columnEncoding (byte))[]
    * - Feature -> Id (Varint)
    * */
    /*
    * TODO:
    *  - support not SFA conform polygon where closing vertex is missing
    * */
    public static List<Layer> decodeCovt(byte[] covtBuffer) throws IOException {
        var pos = new IntWrapper(0);
        var header = decodeHeader(covtBuffer, pos);
        var layers = new ArrayList<Layer>();
        for(var i = 0; i < header.numLayers(); i++){
            var layerMetadata = decodeLayerMetadata(covtBuffer, pos);

            var columId = 0;
            long[] ids = null;
            Geometry[] geometries = null;
            var features = new ArrayList<Feature>();
            var properties = new HashMap<String, List<Optional>>();
            for(var columnMetadataEntry : layerMetadata.columnMetadata().entrySet()){
                var columnMetadata = columnMetadataEntry.getValue();
                var columnName = columnMetadataEntry.getKey();
                if(columId++ == 0 && !columnName.equals(ID_COLUMN_NAME ) && !columnName.equals(GEOMETRY_COLUMN_NAME)){
                    throw new IllegalArgumentException("Id or geometry has to be the first column in a tile.");
                }

                if(columnName.equals(ID_COLUMN_NAME )){
                    var idDataStream = columnMetadata.streams().get(0);
                    ids = decodedIds(covtBuffer, idDataStream.numValues(), idDataStream.streamEncoding(), pos);
                }
                else if(columnName.equals(GEOMETRY_COLUMN_NAME)) {
                    var geometryColumn = decodeGeometryColumn(covtBuffer,
                            columnMetadata, pos, 32 - Integer.numberOfLeadingZeros(layerMetadata.extent()));

                    geometries = convertGeometryColumn(geometryColumn, layerMetadata.numFeatures());
                }
                else{
                    var propertyColumn = decodePropertyColumn(covtBuffer, layerMetadata.numFeatures(), columnMetadata, pos);
                    properties.put(columnName, propertyColumn);
                }
            }

            for(var j = 0; j < layerMetadata.numFeatures(); j++){
                var featureProperties = new HashMap<String, Object>();
                for(var columnMetadataEntry : layerMetadata.columnMetadata().entrySet()) {
                    var columnName = columnMetadataEntry.getKey();
                    if (!columnName.equals(ID_COLUMN_NAME) && !columnName.equals(GEOMETRY_COLUMN_NAME)) {
                        var propertyValue = properties.get(columnName).get(j);
                        featureProperties.put(columnName, propertyValue);
                    }
                }

                var id = ids == null? 0 : ids[j];
                var feature = new Feature(id, geometries[j], featureProperties);
                features.add(feature);
            }

            layers.add(new Layer(layerMetadata.layerName(), features));

            /*var columns = new HashMap<String, List<Optional>>();
                if(columnMetadata.length > 2){
                    //TODO: refactor -> remove array creation
                    var featurePropertyColumns = Arrays.copyOfRange(columnMetadata, 2, columnMetadata.length);
                    for(var column : featurePropertyColumns){
                        var values = decodePropertyColumn(covtBuffer, layerMetadata.numFeatures(), column, pos);
                        columns.put(column.columnName(), values);
                    }
                }*/

            /*var features = new ArrayList<Feature>();
            for(var j = 0; j < ids.length; j++){
                var id = ids[j];
                var geometry = geometries[j];
                var properties = new HashMap<String, Object>();
                for(var column : columns.entrySet()){
                    var propertyKey = column.getKey();
                    var propertyValue = column.getValue().get(j);
                    properties.put(propertyKey, propertyValue);
                }
                var feature = new Feature(id, geometry, properties);
                features.add(feature);
            }

            var layer = new Layer(layerMetadata.layerName(), features);
            layers.add(layer);*/
        }

        return layers;
    }

    private static Geometry[] convertGeometryColumn(GeometryColumn geometryColumn, int numFeatures){
        var geometries = new Geometry[numFeatures];
        var partOffsetCounter = 0;
        var ringOffsetsCounter = 0;
        var geometryOffsetsCounter = 0;
        var geometryCounter = 0;
        var geometryFactory = new GeometryFactory();
        var vertexBufferOffset = 0;
        var vertexOffsetsOffset = 0;

        var geometryTypes = geometryColumn.geometryTypes();
        var geometryOffsets = geometryColumn.geometryOffsets();
        var partOffsets = geometryColumn.partOffsets();
        var ringOffsets = geometryColumn.ringOffsets();
        var vertexOffsets = geometryColumn.vertexOffsets();
        var vertexBuffer = geometryColumn.vertexBuffer();
        //TODO: refactor redundant code
        for(var geometryType : geometryTypes){
            if(geometryType.equals(GeometryType.POINT)){
                if(vertexOffsets == null){
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
            else if(geometryType.equals(GeometryType.LINESTRING)){
                if(vertexOffsets == null){
                    var numVertices = partOffsets[partOffsetCounter++];
                    var vertices = getLineString(vertexBuffer, vertexBufferOffset, numVertices, false);
                    vertexBufferOffset += numVertices * 2;
                    geometries[geometryCounter++] = geometryFactory.createLineString(vertices);
                }
                else{
                    var numVertices = partOffsets[partOffsetCounter++];
                    var vertices = getICELineString(vertexBuffer, vertexOffsets, vertexOffsetsOffset, numVertices, false);
                    vertexOffsetsOffset += numVertices;
                    geometries[geometryCounter++] = geometryFactory.createLineString(vertices);
                }
            }
            else if(geometryType.equals(GeometryType.POLYGON)){
                var numRings = partOffsets[partOffsetCounter++];
                var rings = new LinearRing[numRings - 1];
                var numVertices= ringOffsets[ringOffsetsCounter++];
                if(vertexOffsets == null){
                    LinearRing shell = getLinearRing(vertexBuffer, vertexBufferOffset, numVertices, geometryFactory);
                    vertexBufferOffset += numVertices * 2;
                    for(var i = 0; i < rings.length; i++){
                        numVertices = ringOffsets[ringOffsetsCounter++];
                        rings[i] = getLinearRing(vertexBuffer, vertexBufferOffset, numVertices, geometryFactory);
                        vertexBufferOffset += numVertices * 2;
                    }
                    geometries[geometryCounter++] = geometryFactory.createPolygon(shell, rings);
                }
                else{
                    LinearRing shell = getICELinearRing(vertexBuffer, vertexOffsets, vertexOffsetsOffset, numVertices, geometryFactory);
                    vertexOffsetsOffset += numVertices;
                    for(var i = 0; i < rings.length; i++){
                        numVertices = ringOffsets[ringOffsetsCounter++];
                        rings[i] = getICELinearRing(vertexBuffer, vertexOffsets, vertexOffsetsOffset, numVertices, geometryFactory);
                        vertexOffsetsOffset += numVertices;
                    }
                    geometries[geometryCounter++] = geometryFactory.createPolygon(shell, rings);
                }
            }
            else if(geometryType.equals(GeometryType.MULTILINESTRING)){
                var numLineStrings = geometryOffsets[geometryOffsetsCounter++];
                var lineStrings = new LineString[numLineStrings];
                if(vertexOffsets == null){
                    for(var i = 0; i < numLineStrings; i++){
                        var numVertices = partOffsets[partOffsetCounter++];
                        var vertices = getLineString(vertexBuffer, vertexBufferOffset, numVertices, false);
                        lineStrings[i] = geometryFactory.createLineString(vertices);
                        vertexBufferOffset += numVertices * 2;
                    }
                    geometries[geometryCounter++] = geometryFactory.createMultiLineString(lineStrings);
                }
                else{
                    for(var i = 0; i < numLineStrings; i++){
                        var numVertices = partOffsets[partOffsetCounter++];
                        var vertices = getICELineString(vertexBuffer, vertexOffsets, vertexOffsetsOffset, numVertices, false);
                        lineStrings[i] = geometryFactory.createLineString(vertices);
                        vertexOffsetsOffset += numVertices;
                    }
                    geometries[geometryCounter++] = geometryFactory.createMultiLineString(lineStrings);
                }
            }
            else if(geometryType.equals(GeometryType.MULTIPOLYGON)){
                var numPolygons = geometryOffsets[geometryOffsetsCounter++];
                var polygons = new Polygon[numPolygons];
                var numVertices = 0;
                if(vertexOffsets == null){
                    for(var i = 0; i < numPolygons; i++){
                        var numRings = partOffsets[partOffsetCounter++];
                        var rings = new LinearRing[numRings - 1];
                        numVertices += ringOffsets[ringOffsetsCounter++];
                        LinearRing shell = getLinearRing(vertexBuffer, vertexBufferOffset, numVertices, geometryFactory);
                        vertexBufferOffset += numVertices * 2;
                        for(var j = 0; j < rings.length; j++){
                            var numRingVertices = ringOffsets[ringOffsetsCounter++];
                            rings[i] = getLinearRing(vertexBuffer, vertexBufferOffset, numRingVertices, geometryFactory);
                            vertexBufferOffset += numVertices * 2;
                        }

                        polygons[i] = geometryFactory.createPolygon(shell, rings);
                    }
                    geometries[geometryCounter++] = geometryFactory.createMultiPolygon(polygons);
                }
                else{
                    for(var i = 0; i < numPolygons; i++){
                        var numRings = partOffsets[partOffsetCounter++];
                        var rings = new LinearRing[numRings - 1];
                        numVertices += ringOffsets[ringOffsetsCounter++];
                        LinearRing shell = getICELinearRing(vertexBuffer, vertexOffsets, vertexOffsetsOffset, numVertices, geometryFactory);
                        vertexOffsetsOffset += numVertices;
                        for(var j = 0; j < rings.length; j++){
                            numVertices = ringOffsets[ringOffsetsCounter++];
                            rings[i] = getICELinearRing(vertexBuffer, vertexOffsets, vertexOffsetsOffset, numVertices, geometryFactory);
                            vertexOffsetsOffset += numVertices;
                        }

                        polygons[i] = geometryFactory.createPolygon(shell, rings);
                    }
                    geometries[geometryCounter++] = geometryFactory.createMultiPolygon(polygons);
                }
            }
            else{
                throw new IllegalArgumentException("The specified geometry type is currently not supported.");
            }
        }

        return geometries;
    }

    private static List<Optional> decodePropertyColumn(byte[] covtBuffer, int numFeatures, ColumnMetadata columnMetadata, IntWrapper pos) throws IOException {
        var propertyColumnValues = new ArrayList<Optional>();
        var dataStreamMetadata  = columnMetadata.streams().get(DATA_STREAM_NAME);
        if(columnMetadata.columnDataType() == ColumnDataType.BOOLEAN){
            var rleDecodedColumn = DecodingUtils.decodeByteRle(covtBuffer, dataStreamMetadata.numValues(), pos,
                    dataStreamMetadata.byteLength());
            var decodedColumn = BitSet.valueOf(rleDecodedColumn);
            for(var i = 0; i < numFeatures; i++){
                propertyColumnValues.add(Optional.of(decodedColumn.get(i)));
            }

            return propertyColumnValues;
        }

        /* decode present stream */
        var numBytes = (int)Math.ceil(numFeatures / 8d);
        var presentStream = DecodingUtils.decodeByteRle(covtBuffer, numBytes, pos, columnMetadata.streams().
                get(PRESENT_STREAM_NAME).byteLength());
        var bitSet = BitSet.valueOf(presentStream);
        if(columnMetadata.columnDataType() == ColumnDataType.INT_64){
            long[] decodedDataColumn;
            if(dataStreamMetadata.streamEncoding() == StreamEncoding.RLE){
                decodedDataColumn = DecodingUtils.decodeRle(covtBuffer, dataStreamMetadata.numValues(), pos, true);
            }
            else if(dataStreamMetadata.streamEncoding() == StreamEncoding.VARINT_ZIG_ZAG){
                //TODO: refactor to use long instead of int
                var values = DecodingUtils.decodeZigZagVarint(covtBuffer,  pos, dataStreamMetadata.numValues());
                decodedDataColumn = Arrays.stream(values).mapToLong(i -> i).toArray();
            }
            else if(dataStreamMetadata.streamEncoding() == StreamEncoding.VARINT_DELTA_ZIG_ZAG){
                //TODO: refactor to use long instead of int
                var values = DecodingUtils.decodeZigZagDeltaVarint(covtBuffer, pos, dataStreamMetadata.numValues());
                decodedDataColumn = Arrays.stream(values).mapToLong(i -> i).toArray();
            }
            else{
                throw new IllegalArgumentException("The specified encoding for the long data stream is not supported.");
            }

            //TODO: this evaluation should happen when accessing the properties based on random access
            var j = 0;
            for(var i = 0; i < numFeatures; i++){
                if(bitSet.get(i)){
                    propertyColumnValues.add(Optional.of(decodedDataColumn[j++]));
                }
                else{
                    propertyColumnValues.add(Optional.empty());
                }
            }
        }
        else if(columnMetadata.columnDataType() == ColumnDataType.FLOAT){
            var decodedDataColumn = DecodingUtils.decodeFloatsLE(covtBuffer, pos, dataStreamMetadata.numValues());
            var j = 0;
            for(var i = 0; i < numFeatures; i++){
                if(bitSet.get(i)){
                    propertyColumnValues.add(Optional.of(decodedDataColumn[j++]));
                }
                else{
                    propertyColumnValues.add(Optional.empty());
                }
            }
        }
        else if(columnMetadata.columnDataType() == ColumnDataType.STRING){
            //TODO: also decode localized dictionary
            /* String streams: present (BitVector), data (RLE), length (RLE), data_dictionary */
            if(!columnMetadata.columnEncoding().equals(ColumnEncoding.DICTIONARY)){
                throw new IllegalArgumentException("Currently only dictionary encoding is supported for String.");
            }

            var numDictionaryEntries = columnMetadata.streams().get(DICTIONARY_STREAM_NAME).numValues();
            var data = DecodingUtils.decodeRle(covtBuffer, dataStreamMetadata.numValues(), pos, false);
            var dictionaryData = getStringDictionary(covtBuffer, numDictionaryEntries, pos);

            var dataCounter = 0;
            for(var i = 0; i < numFeatures; i++){
                if(bitSet.get(i)){
                    var index = (int)data[dataCounter++];
                    var value = dictionaryData[index];
                    propertyColumnValues.add(Optional.of(value));
                }
                else{
                    propertyColumnValues.add(Optional.empty());
                }
            }
        }
        else{
            throw new IllegalArgumentException("Data type not supported");
        }

        return propertyColumnValues;
    }

    private static int getNumberOfPresentValues(BitSet bitSet, int numValues){
        var numPresentValues = 0;
        for(var i = 0; i < numValues; i++){
            if(bitSet.get(i) == true){
                numPresentValues++;
            }
        }

        return numPresentValues;
    }

    private static String[] getStringDictionary(byte[] covtBuffer, int numDictionaryEntries, IntWrapper pos) throws IOException {
        var lengthStream = DecodingUtils.decodeRle(covtBuffer, numDictionaryEntries, pos, false);

        var dictionaryData = new String[numDictionaryEntries];
        for(var i = 0; i < numDictionaryEntries; i++){
            var length = (int)lengthStream[i];
            dictionaryData[i] = DecodingUtils.decodeString(covtBuffer, pos, length);
        }

        return dictionaryData;
    }

    private static GeometryColumn decodeGeometryColumn(byte[] covtBuffer, ColumnMetadata columnMetadata,
                                                       IntWrapper pos, int numBits) throws IOException {
        /*
        * - Geometry column streams -> geometryType, geometryOffsets, partOffsets, ringOffsets, vertexOffsets, vertexBuffer
        * - geometryType -> Byte, start with Boolean RLE -> but in general parquetRLEBitpackingHybridEncoding
        *                -> Bitpacking Hybrid 23 bytes vs ORC RLE V1 1kb vs ORC RLE V2 370 bytes
        * - geometryOffsets, partOffsets, ringOffsets -> RLE or FastPfor Delta
        * - vertexOffsets -> Varint Delta or FastPFor Delta
        * - vertexBuffer ICE -> Varint Delta or FastPfor Delta
        * */

        /* Decode topology streams */
        //TODO: quick and dirty -> get rid of this loop and use int instead of long
        var geometryTypesMetadata = columnMetadata.streams().get(GEOMETRY_TYPES_STREAM_NAME);
        var decodedGeometryTypes = DecodingUtils.decodeByteRle(covtBuffer, geometryTypesMetadata.numValues(), pos, geometryTypesMetadata.byteLength());
        var geometryTypes = new GeometryType[geometryTypesMetadata.numValues()];
        for(var j = 0; j < geometryTypes.length; j++){
            geometryTypes[j] = GeometryType.values()[decodedGeometryTypes[j]];
        }

        var geometryOffsetsMetadata = columnMetadata.streams().get(GEOMETRY_OFFSETS_STREAM_NAME);
        //TODO: refactor -> remove that redundant code
        int[] geometryOffsets = null;
        if(geometryOffsetsMetadata != null){
            var encoding = geometryOffsetsMetadata.streamEncoding();
            if(encoding == StreamEncoding.RLE){
                //TODO: add overload to rle encoding which handles int
                geometryOffsets = Arrays.stream(DecodingUtils.decodeRle(covtBuffer, geometryOffsetsMetadata.numValues(), pos, false)).
                        mapToInt(i -> (int)i).toArray();
            }
            else if(encoding == StreamEncoding.FAST_PFOR_DELTA_ZIG_ZAG){
                geometryOffsets = DecodingUtils.decodeFastPfor128ZigZagDelta(covtBuffer, geometryOffsetsMetadata.numValues(), geometryOffsetsMetadata.byteLength(), pos);
            }
            else{
                throw new IllegalArgumentException("The specified encoding is currently not supported for a topology stream.");
            }
        }

        var partOffsetsMetadata = columnMetadata.streams().get(PART_OFFSETS_STREAM_NAME);
        int[] partOffsets = null;
        if(partOffsetsMetadata != null){
            var encoding = partOffsetsMetadata.streamEncoding();
            if(encoding == StreamEncoding.RLE){
                //TODO: add overload to rle encoding which handles int
                partOffsets = Arrays.stream(DecodingUtils.decodeRle(covtBuffer, partOffsetsMetadata.numValues(), pos, false)).
                        mapToInt(i -> (int)i).toArray();
            }
            else if(encoding == StreamEncoding.FAST_PFOR_DELTA_ZIG_ZAG){
                partOffsets = DecodingUtils.decodeFastPfor128ZigZagDelta(covtBuffer, partOffsetsMetadata.numValues(), partOffsetsMetadata.byteLength(), pos);
            }
            else{
                throw new IllegalArgumentException("The specified encoding is currently not supported for a topology stream.");
            }
        }

        var ringOffsetsMetadata = columnMetadata.streams().get(RING_OFFSETS_STREAM_NAME);
        int[] ringOffsets = null;
        if(ringOffsetsMetadata != null){
            var encoding = ringOffsetsMetadata.streamEncoding();
            if(encoding == StreamEncoding.RLE){
                //TODO: add overload to rle encoding which handles int
                ringOffsets = Arrays.stream(DecodingUtils.decodeRle(covtBuffer, ringOffsetsMetadata.numValues(), pos, false)).
                        mapToInt(i -> (int)i).toArray();
            }
            else if(encoding == StreamEncoding.FAST_PFOR_DELTA_ZIG_ZAG){
                ringOffsets = DecodingUtils.decodeFastPfor128ZigZagDelta(covtBuffer, ringOffsetsMetadata.numValues(), ringOffsetsMetadata.byteLength(), pos);
            }
            else{
                throw new IllegalArgumentException("The specified encoding is currently not supported for a topology stream.");
            }
        }

        var vertexOffsetMetadata = columnMetadata.streams().get(VERTEX_OFFSETS_STREAM_NAME);
        int[] vertexOffsets = null;
        if(vertexOffsetMetadata != null){
            var encoding = vertexOffsetMetadata.streamEncoding();
            if(encoding == StreamEncoding.VARINT_DELTA_ZIG_ZAG){
                vertexOffsets = DecodingUtils.decodeZigZagDeltaVarint(covtBuffer, pos, vertexOffsetMetadata.numValues());
            }
            else if(encoding == StreamEncoding.FAST_PFOR_DELTA_ZIG_ZAG){
                vertexOffsets = DecodingUtils.decodeFastPfor128ZigZagDelta(covtBuffer, vertexOffsetMetadata.numValues(), vertexOffsetMetadata.byteLength(), pos);
            }
            else{
                throw new IllegalArgumentException("The specified encoding is currently not supported for a topology stream.");
            }
        }

        var vertexBufferMetadata = columnMetadata.streams().get(VERTEX_BUFFER_STREAM_NAME);

        var encoding = vertexBufferMetadata.streamEncoding();
        if(columnMetadata.columnEncoding() == ColumnEncoding.ICE_MORTON_CODE){
            int[] vertexBuffer = null;
            if(encoding == StreamEncoding.VARINT_DELTA_ZIG_ZAG){
                vertexBuffer = DecodingUtils.decodeDeltaVarintMortonCodes(covtBuffer, pos,
                        vertexBufferMetadata.numValues(), numBits);
            }
            else if(encoding == StreamEncoding.FAST_PFOR_DELTA_ZIG_ZAG){
                vertexBuffer = DecodingUtils.decodeFastPfor128DeltaMortonCodes(covtBuffer, vertexBufferMetadata.numValues(),
                        vertexBufferMetadata.byteLength(), pos, numBits);
            }
            else{
                throw new IllegalArgumentException("The specified encoding is currently not supported for a topology stream.");
            }

            return new GeometryColumn(geometryTypes, geometryOffsets, partOffsets, ringOffsets, vertexOffsets, vertexBuffer);
        }

        int[] vertexBuffer = null;
        if(encoding == StreamEncoding.VARINT_DELTA_ZIG_ZAG){
            vertexBuffer = DecodingUtils.decodeZigZagDeltaVarintCoordinates(covtBuffer, pos, vertexBufferMetadata.numValues());
        }
        else if(encoding == StreamEncoding.FAST_PFOR_DELTA_ZIG_ZAG){
            vertexBuffer = DecodingUtils.decodeFastPfor128DeltaCoordinates(covtBuffer, vertexBufferMetadata.numValues(),
                    vertexBufferMetadata.byteLength(), pos);
        }
        else{
            throw new IllegalArgumentException("The specified encoding is currently not supported for a topology stream.");
        }
        return new GeometryColumn(geometryTypes, geometryOffsets, partOffsets, ringOffsets, vertexOffsets, vertexBuffer);
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

    private static long[] decodedIds(byte[] covtBuffer, int numFeatures, StreamEncoding encoding, IntWrapper pos) throws IOException {
        if(encoding == StreamEncoding.RLE){
            return DecodingUtils.decodeRle(covtBuffer, numFeatures, pos, false);
        }

        if(encoding == StreamEncoding.VARINT){
            //TODO: optimize this decoding step and test
            //TODO: use long
            return Arrays.stream(DecodingUtils.decodeVarint(covtBuffer, pos, numFeatures)).mapToLong(i -> i).toArray();
        }

        if(encoding == StreamEncoding.VARINT_DELTA_ZIG_ZAG){
            //TODO: optimize this decoding step and test
            //TODO: use long
            return Arrays.stream(DecodingUtils.decodeZigZagDeltaVarint(covtBuffer, pos, numFeatures)).mapToLong(i -> i).toArray();
        }


        //TODO: add VarintZigZagDelta encoding
        throw new IllegalArgumentException("The specified encoding for the id column is not supported (yet).");
    }

    private static LayerMetadata decodeLayerMetadata(byte[] covtBuffer, IntWrapper pos) throws IOException {
        var layerName = DecodingUtils.decodeString(covtBuffer, pos);
        var extent = DecodingUtils.decodeVarint(covtBuffer, pos, 1)[0];
        var numFeatures = DecodingUtils.decodeVarint(covtBuffer, pos, 1)[0];
        var numColumns = DecodingUtils.decodeVarint(covtBuffer, pos, 1)[0];

        var columnMetadata = new LinkedHashMap<String, ColumnMetadata>();
        for(var i = 0; i < numColumns; i++){
            var columnName = DecodingUtils.decodeString(covtBuffer, pos);
            var columnType = ColumnDataType.values()[covtBuffer[pos.get()]];
            pos.increment();
            var columnEncoding = ColumnEncoding.values()[covtBuffer[pos.get()]];
            pos.increment();
            var numStreams = DecodingUtils.decodeVarint(covtBuffer, pos, 1)[0];

            var streams = new  LinkedHashMap<String, StreamMetadata>();
            columnMetadata.put(columnName, new ColumnMetadata(columnType, columnEncoding, streams));
            for(var j = 0; j < numStreams; j++){
                var streamName = DecodingUtils.decodeString(covtBuffer, pos);
                var numValues = DecodingUtils.decodeVarint(covtBuffer, pos , 1)[0];
                var byteLength = DecodingUtils.decodeVarint(covtBuffer, pos, 1)[0];
                var intEncoding = DecodingUtils.decodeVarint(covtBuffer, pos, 1)[0];
                var encoding = Arrays.stream(StreamEncoding.values())
                        .filter(p -> p.ordinal() == intEncoding)
                        .findFirst().get();
                streams.put(streamName, new StreamMetadata(numValues, byteLength, encoding));
            }
        }

        return new LayerMetadata(layerName, extent, numFeatures, numColumns, columnMetadata);
    }

    private static Header decodeHeader(byte[] covtBuffer, IntWrapper pos){
        var version = DecodingUtils.decodeVarint(covtBuffer, pos, 1)[0];
        var numLayers = DecodingUtils.decodeVarint(covtBuffer, pos,1)[0];
        return new Header(version, numLayers);
    }

}
