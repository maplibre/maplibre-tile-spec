package com.covt.evaluation;

import com.covt.evaluation.compression.IntegerCompression;
import com.google.common.collect.Iterables;
import io.github.sebasbaumh.mapbox.vectortile.adapt.jts.MvtReader;
import io.github.sebasbaumh.mapbox.vectortile.adapt.jts.TagKeyValueMapConverter;
import org.apache.commons.lang3.ArrayUtils;
import org.davidmoten.hilbert.HilbertCurve;
import org.davidmoten.hilbert.SmallHilbertCurve;
import org.locationtech.jts.geom.*;
import org.locationtech.jts.geom.impl.PackedCoordinateSequenceFactory;

import java.io.ByteArrayInputStream;
import java.io.ByteArrayOutputStream;
import java.io.IOException;
import java.nio.charset.StandardCharsets;
import java.sql.DriverManager;
import java.sql.ResultSet;
import java.sql.SQLException;
import java.util.*;
import java.util.stream.Collectors;
import java.util.stream.Stream;
import java.util.zip.GZIPInputStream;

enum ColumnDataType {
    STRING,
    BOOLEAN,
    INTEGER,
    LONG,
    FLOAT,
    DOUBLE,
}

public class MvtConverter {
    private static final String ID_KEY = "id";
    private static final String TILE_DATA_KEY = "tile_data";
    /* 14 bits -> 8192 in two directions */
    private static final int NUM_COORDINATES_PER_QUADRANT = 8192;

    private static final Map<String, Integer> GEOMETRY_TYPES = Map.of("Point", 0,"LineString", 1, "Polygon", 2, "MultiPoint", 3,
            "MultiLineString", 4, "MultiPolygon", 5
    );

    public static void main(String[] args) throws IOException, SQLException, ClassNotFoundException {
        var mbTilesFileName = "C:\\mapdata\\europe.mbtiles";
        var layers = getMVT(mbTilesFileName, 5, 16, 21);
        createCovtTile(layers);
    }

    /*
    * TODOs:
    * - Replace Varint encoding with Patched bitpacking for coordinates
    *   -> Compare Parquet Delta with other delta encodings based on patched bitpacking
    * - Test ORC RLE encodings for vertexOffsets -> add zigZag encoding for delta values?
    * - Compare wins for sorting point layers based on id or geometry
    * - Test for POI layer if hilbert delta coding is working
    * - Check if use RLE encoding or BitVector or BitVector with RLE for present stream
    * */
    private static void createCovtTile(List<Layer> layers) throws IOException {
        /*
        * - Header
        *   -> Version
        *   -> Num Layers
        *   -> Layer names
        * - Layer 1
        *   - Extent
        *   - Num Features
        *   - Id Column
        *       -> Delta encoded and RLE/Bitpacking Hybrid when sored
        *   - Geometry Column
        *       - GeometryType Stream -> RLE / Bit-Packing Hybrid
        *       - Topology Stream
        *       - Index Stream
        *       - Vertex Stream
        *   - Properties Columns
        *       - Long -> Plain encoding
        *           - Present Stream
        *           - Value Stream -> Varint for quick and dirty
        *       - String -> Dictionary encoding
        *           - Present Stream -> Boolean RLE?
        *           - Length Stream -> ORC RLE V1 or V2? -> Delta encode?
        *           - Dictionary Stream -> UTF-8 String
        *           - Data Stream -> ORC RLE V1 encoded
        * */
        ByteArrayOutputStream stream = new ByteArrayOutputStream();
        for(var layer : layers){
            //TODO: add encoding type to each column
            //TODO: sort points on Hilbert curve

            /* Either sort point layer based on id or geometry */
            //TODO: add other point layers
            var features = layer.name() == "poi" ? sortPointFeaturesOnHilbertCurve(layer.features())
                    : layer.features();

            var ids = getIds(features);
            stream.write(ids);
            var useIceEncoding = layer.name().equals("transportation");
            //var geometries2 = getGeometryColumn(layer, false);
            //transportation without ICE encoding -> 453kb -> save 174 kb (38%) with ICE?
            var geometries = getGeometryColumn(features, useIceEncoding);
            stream.write(geometries);

            //TODO: store data type for property column
            var propertyColumns = getPropertyColumns(features, true);
            stream.write(propertyColumns);
            var covtBuffer = stream.toByteArray();
            System.out.println("COVT size: " + covtBuffer.length);
        }
    }

    private static byte[] getPropertyColumns(List<Feature> features, boolean dictionaryCodeStrings) throws IOException {
        /*
        * - String
        *   -> Plain coding -> Present stream, length stream, data stream
        *   -> Dictionary coding -> Present stream, length stream, dictionary stream, data stream
        * Integer
        *   -> plain coding
        * */

        /*
         * 1. Build up all columns to also set present stream
         *  -> Map with name of property/column and the data type
         * 2. (optional)
         *  -> depending on the data type if string type is used and dictionary coding should be used
         *     build up dictionary_data -> array of dictionary_data
         * 3.
         *  -> iterate over features -> iterate over columnMap from step 1
         *  -> a. encode present, length data stream when plain encoding is used
         *  -> b. use dictionary_data from step 2 to build up dictionary_data and data streams
         * 4.
         *  -> encode present, length, data and dictionary_data stream
         *      -> present -> Boolean RLE -> byte rle
         *      -> length  -> 	Unsigned Integer RLE v1
         *      -> data -> UTF-8 encoded
         *      -> dictionary_data -> Unsigned Integer RLE v1
         * -> brunnel, class
        * */
        var columnMetadata = new HashMap<String, ColumnDataType>();
        for(var feature : features){
            var properties = feature.properties();
            for(var property : properties.entrySet()){
                var propertyName = property.getKey();
                if(columnMetadata.containsKey(propertyName)){
                    continue;
                }

                var propertyValue = property.getValue();
                if(propertyValue instanceof String){
                    columnMetadata.put(propertyName, ColumnDataType.STRING);
                }
                else if(propertyValue instanceof Boolean){
                    throw new IllegalArgumentException("Boolean currently not supported as property data type.");
                }
                else if(propertyValue instanceof Integer || propertyValue instanceof  Long){
                    if((long)propertyValue >= Math.pow(2, 32)|| (long)propertyValue <= -Math.pow(2, 32)){
                        throw new IllegalArgumentException("Property value to large to fit into a integer.");
                    }

                    //throw new IllegalArgumentException("Integer currently not supported as property data type.");
                    columnMetadata.put(propertyName, ColumnDataType.INTEGER);
                }
                else if(propertyValue instanceof Long){
                    //columnMetadata.put(propertyName, ColumnDataType.LONG);
                    throw new IllegalArgumentException("Integer currently not supported as property data type.");
                }
                else if(propertyValue instanceof Float){
                    throw new IllegalArgumentException("Float currently not supported as property data type.");
                }
                else if(propertyValue instanceof Double){
                    throw new IllegalArgumentException("Double currently not supported as property data type.");
                }
                else{
                    throw new IllegalArgumentException("Specified data type currently not supported.");
                }
            }
        }

        var presentStreams = new ArrayList<List<Boolean>>();
        var lengthStreams = new ArrayList<List<Integer>>();
        var dataStreams = new ArrayList<List<byte[]>>();
        var dataDictionarySets = new ArrayList<LinkedHashSet<String>>();
        for(var metadata : columnMetadata.entrySet()){
            var presentStream = new ArrayList<Boolean>();
            var lengthStream = new ArrayList<Integer>();
            var dataStream = new ArrayList<byte[]>();
            var dataDictionarySet = new LinkedHashSet<String>();

            var columnName = metadata.getKey();
            var columnDataType = metadata.getValue();
            for(var feature : features){
                var properties = feature.properties();
                if(!properties.containsKey(columnName)){
                    presentStream.add(false);
                    continue;
                }

                presentStream.add(true);
                var propertyValue = properties.get(columnName);
                if(columnDataType == ColumnDataType.STRING){
                   var stringValue = (String)propertyValue;
                   var utf8EncodedData = stringValue.getBytes(StandardCharsets.UTF_8);
                   if(dictionaryCodeStrings){
                        if(!dataDictionarySet.contains(stringValue)){
                            dataDictionarySet.add(stringValue);
                        }
                   }
                   else{
                        lengthStream.add(utf8EncodedData.length);
                        dataStream.add(utf8EncodedData);
                   }
               }
                else if(columnDataType == ColumnDataType.INTEGER){
                   var intValue = ((Long) propertyValue).intValue();
                   var zigZagInt = (intValue >> 31) ^ (intValue << 1);
                   var varIntEncodedValue = IntegerCompression.varintEncode(new int[]{zigZagInt});
                   dataStream.add(varIntEncodedValue);
               }
               else{
                   throw new IllegalArgumentException("Column data type currently not supported.");
               }
            }

            presentStreams.add(presentStream);
            lengthStreams.add(lengthStream);
            dataStreams.add(dataStream);
            dataDictionarySets.add(dataDictionarySet);
        }

        /*
        * Encode columns
        * - Present
        *   -> ORC RLE V1 -> Boolean RLE V1
        * - String
        *   - Length -> ORC RLE V1
        *   - Plain
        *       - Data -> concat byte arrays
        *   - Dictionary
        *       - dictionary_data -> Concat dictionary byte arrays
        *       - data -> ORC RLE V1
        * - Int
        *   - Data -> Varint -> already encoded -> concat byte arrays
        * */
        var i = 0;
        var columnBuffer = new byte[0];
        for(var column : columnMetadata.entrySet()){
                var present = presentStreams.get(i);
                BitSet bitSet = new BitSet(present.size());
                var j = 0;
                for(var p : present){
                    bitSet.set(j++, p);
                }
                var compressedPresentStream = IntegerCompression.orcRleByteEncodingV1(bitSet.toByteArray());
                columnBuffer = ArrayUtils.addAll(columnBuffer, compressedPresentStream);

                if(column.getValue() == ColumnDataType.INTEGER){
                    var varintEncodedDataStream = concatByteArrays(dataStreams.get(i));
                    columnBuffer = ArrayUtils.addAll(columnBuffer, varintEncodedDataStream);
                }
                else if(column.getValue() == ColumnDataType.STRING){
                    if(dictionaryCodeStrings){
                        var dataDictionary = dataDictionarySets.get(i);
                        var lengthStream = new long[0];
                        var dataStream = new long[0];
                        var dataDictionaryStream = new byte[0];
                        for(var feature : features){
                            var value = (String)feature.properties().get(column.getKey());
                            if(value == null){
                                continue;
                            }

                            var index = Iterables.indexOf(dataDictionary, v -> v.equals(value));
                            var utf8Value = value.getBytes(StandardCharsets.UTF_8);
                            lengthStream = ArrayUtils.addAll(lengthStream, utf8Value.length);
                            dataDictionaryStream = ArrayUtils.addAll(dataDictionaryStream, utf8Value);
                            dataStream = ArrayUtils.addAll(dataStream, index);
                        }
                        var encodedLengthStream = IntegerCompression.orcRleEncodingV1(lengthStream);
                        var encodedDataStream = IntegerCompression.orcRleEncodingV1(dataStream);
                        columnBuffer = ArrayUtils.addAll(columnBuffer, encodedLengthStream);
                        columnBuffer = ArrayUtils.addAll(columnBuffer, dataDictionaryStream);
                        columnBuffer = ArrayUtils.addAll(columnBuffer, encodedDataStream);
                    }
                    else{
                        var lengthStream = lengthStreams.get(i).stream().mapToLong(l -> (long)l).toArray();
                        var encodedLengthStream = IntegerCompression.orcRleEncodingV1(lengthStream);
                        var dataStream = concatByteArrays(dataStreams.get(i));
                        columnBuffer = ArrayUtils.addAll(columnBuffer, encodedLengthStream);
                        columnBuffer = ArrayUtils.addAll(columnBuffer, dataStream);
                    }
                }
                else{
                    throw new IllegalArgumentException("Data type not supported");
                }

                i++;
            }


        return columnBuffer;
    }

    private static byte [] concatByteArrays(List<byte[]> values){
        var buffer = new byte[0];
        for(var value : values){
            buffer = ArrayUtils.addAll(buffer, value);
        }
        return buffer;
    }

    private static List<Feature> sortPointFeaturesOnHilbertCurve(List<Feature> features){
        SmallHilbertCurve hilbertCurve =
                HilbertCurve.small().bits(14).dimensions(2);

        var sortedFeatures = new ArrayList<HilbertFeature>();
        for(var feature : features){
            var point = (Point)feature.geometry();
            var x = NUM_COORDINATES_PER_QUADRANT + (int) point.getX();
            var y = NUM_COORDINATES_PER_QUADRANT + (int) point.getY();
            var hilbertIndex = (int) hilbertCurve.index(x, y);
            var hilbertFeature = new HilbertFeature(feature.id(), feature.geometry(),
                    feature.properties(), hilbertIndex);
            sortedFeatures.add(hilbertFeature);
        }

        sortedFeatures.sort(Comparator.comparingInt(HilbertFeature::hilbertId));
        return sortedFeatures.stream().map(f -> new Feature(f.id(),
                f.geometry(), f.properties())).collect(Collectors.toList());
    }

    private static byte[] getIds(List<Feature> features) throws IOException {
        var ids = features.stream().map(feature -> feature.id()).mapToLong(i -> i).toArray();
        return IntegerCompression.orcRleEncodingV1(ids);
    }

    /*
    * allowGeometriesToBeSorted
    * -> specifies if the geometries ca be ordered
    * -> this can be the case when the layer is not sored after the Id
    * -> start with only sorting Points on a Hilbert curve
    * */
    private static byte[] getGeometryColumn(List<Feature> features, boolean useIceEncoding) throws IOException {
        /*
        * - GeometryType
        * - Topology
        *   -> start with only supporting the same geometry types per layer e.g. LineString and MultiLineString
        *   -> Add additional VertexOffset for a LineString when Indexed Coordinate Encoding (ICE) is used.
        *   -> LineString -> Part offsets, Vertex offsets
        *   -> MultiLineString -> Geometry offsets, Part offsets, Vertex Offsets
        *   -> No fixed structure in the physical representation
        *       -> e.g. MultiLineString and LineString
        *           -> LineString has no entry in GeometryOffset as it can be derived from the geometry type
        * - VertexBuffer
        * */

        /*var isMultiPartGeometry = layer.features().stream().anyMatch(f -> {
           var geometryType =  f.geometry().getGeometryType();
           return geometryType.equals("MultiPoint") || geometryType.equals("MultiLineString") ||
                   geometryType.equals("MultiPolygon");
        });*/

        var geometryTypes = new ArrayList<Integer>();
        var vertexOffsets = new ArrayList<Integer>();
        var partOffsets = new ArrayList<Integer>();
        var ringOffsets = new ArrayList<Integer>();
        var geometryOffsets = new ArrayList<Integer>();
        var plainVertexBuffer = new ArrayList<Vertex>();
        var iceEncodedVertexBuffer = new TreeMap<Integer, Vertex>();

        SmallHilbertCurve hilbertCurve =
                HilbertCurve.small().bits(14).dimensions(2);
        for(var feature : features){
            var geometryType = feature.geometry().getGeometryType();

            /*
            * Depending on the geometry type the topology column has the following streams:
            * Logical Representation
            * - Point: no stream
            * - LineString: Part offsets
            * - Polygon: Part offsets (Polygon), Ring offsets (LinearRing)
            * - MultiPoint: Geometry offsets -> array of offsets indicate where the vertices of each MultiPoint start
            * - MultiLineString: Geometry offsets, Part offsets (LineString)
            * - MultiPolygon -> Geometry offsets, Part offsets (Polygon), Ring offsets (LinearRing)
            * -> In addition when Indexed Coordinate Encoding (ICE) is used Vertex offsets stream is added
            * Physical Representation
            * - LineString: Num vertices
            * */
            if(geometryType.equals("Point")){
                    geometryTypes.add(GEOMETRY_TYPES.get("Point"));

                    var point = (Point)feature.geometry();
                    plainVertexBuffer.add(new Vertex((int)point.getX(), (int)point.getY()));
            }
           else if(geometryType.equals("MultiPoint")) {
                    throw new IllegalArgumentException("MultiPoint currently not supported.");
           }
            else if(geometryType.equals("LineString")){
                    geometryTypes.add(GEOMETRY_TYPES.get("LineString"));

                    var lineString = (LineString) feature.geometry();
                    /*
                    *
                    * -> sort vertex buffer on hilbert curve
                    * -> add index to vertex offset
                    * */
                    if(useIceEncoding){
                        iceDeltaCodeLineString(lineString, iceEncodedVertexBuffer, hilbertCurve);
                    }
                    else{
                        zigZagDeltaCodeLineString(lineString, plainVertexBuffer);
                    }
                    partOffsets.add(lineString.getCoordinates().length);
            }
            else if(geometryType.equals("MultiLineString")){
                    geometryTypes.add(GEOMETRY_TYPES.get("MultiLineString"));

                    var multiLineString = ((MultiLineString)feature.geometry());
                    var numLineStrings = multiLineString.getNumGeometries();
                    for(var i = 0; i < numLineStrings; i++){
                        var lineString =  (LineString)multiLineString.getGeometryN(i);
                        if(useIceEncoding){
                            iceDeltaCodeLineString(lineString, iceEncodedVertexBuffer, hilbertCurve);
                        }
                        else{
                            zigZagDeltaCodeLineString(lineString, plainVertexBuffer);
                        }

                        /*
                         * Logical Representation
                         * -> Offsets
                         * Physical Representation
                         * -> Offsets delta encoded -> number of vertices
                         * */
                        geometryOffsets.add(numLineStrings);
                    }
            }
            else if(geometryType.equals("Polygon")){
                    geometryTypes.add(GEOMETRY_TYPES.get("Polygon"));

                    /*
                    * Polygon topology ->  Part offsets (Polygon), Ring offsets (LinearRing)
                    * */
                    var polygon = (Polygon)feature.geometry();
                    deltaCodePolygon(polygon, plainVertexBuffer, partOffsets, ringOffsets);
            }
            else if(geometryType.equals("MultiPolygon")){
                    geometryTypes.add(GEOMETRY_TYPES.get("MultiPolygon"));

                    /*
                    * MultiPolygon -> Geometry offsets, Part offsets (Polygon), Ring offsets (LinearRing)
                    * */
                    var multiPolygon = ((MultiPolygon) feature.geometry());
                    var numPolygons = multiPolygon.getNumGeometries();
                    geometryOffsets.add(numPolygons);
                    for(var i = 0; i < numPolygons; i++){
                        var polygon = (Polygon)multiPolygon.getGeometryN(i);
                        deltaCodePolygon(polygon, plainVertexBuffer, partOffsets, ringOffsets);
                    }
            }
             else{
                    throw new IllegalArgumentException("Geometry type not supported.");
            }
        }

        /* ICE and delta encode LineString and MultiLineString */
        var deltaVertexBuffer = new int[iceEncodedVertexBuffer.size() * 2];
        if(useIceEncoding){
            for(var feature : features){
                var geometryType = feature.geometry().getGeometryType();
                switch(geometryType){
                    case "LineString": {
                        var lineString = (LineString) feature.geometry();
                        var vertices = lineString.getCoordinates();
                        for (var vertex : vertices) {
                            var vertexOffset = getVertexOffset(iceEncodedVertexBuffer.entrySet(), vertex, hilbertCurve);
                            vertexOffsets.add(vertexOffset);
                        }

                        break;
                    }
                    case "MultiLineString":{
                        var multiLineString = ((MultiLineString)feature.geometry());
                        var numLineStrings = multiLineString.getNumGeometries();
                        for(var i = 0; i < numLineStrings; i++){
                            var lineString =  (LineString)multiLineString.getGeometryN(i);
                            var vertices = lineString.getCoordinates();
                            for (var vertex : vertices) {
                                var vertexOffset = getVertexOffset(iceEncodedVertexBuffer.entrySet(), vertex, hilbertCurve);
                                vertexOffsets.add(vertexOffset);
                            }
                        }
                        break;
                    }
                    default:
                        throw new IllegalArgumentException("Geometry type not supported.");
                }
            }

            var previousX = 0;
            var previousY = 0;
            var i = 0;
            for(var entry : iceEncodedVertexBuffer.entrySet()){
                var vertex = entry.getValue();
                var deltaX = vertex.x() - previousX;
                var deltaY = vertex.y() - previousY;
                deltaVertexBuffer[i++] = deltaX;
                deltaVertexBuffer[i++] = deltaY;

                previousX = vertex.x();
                previousY = vertex.y();
            }
        }

        var vertexBuffer = useIceEncoding ? deltaVertexBuffer :
                plainVertexBuffer.stream().flatMap(v -> Stream.of(v.x(), v.y())).mapToInt(i -> i).toArray();

        /*
        * Pre-process depending on the encoding
        * -> Points
        *   -> no encoding
        *   -> sort on Hilbert curve and delta encode points -> also the features has to be sored e.g. Ids
        * -> LineString
        *   -> no encodig
        *   -> Indexed Coordinate Encoding (ICE)
        * -> Polygon
        *   -> no encoding
        *
        * -> Delta encode topology streams
        *  */
        /*
         * -> GeometryTypes
         *   -> Parquet RLE Bitpacking
         * -> GeometryOffsets, PartOffsets, RingOffsets
         *   -> depending on the flag plain or ORC RLE V1
         * -> VertexBuffer
         *   -> x,y -> Start with Varint
         *   -> LineString and Polygon already delta coded
         *   -> Point
         *       -> if allowGeometriesToBeSorted sort on hilbert and delta encode
         *       -> return sorted feature ids
         * */
        byte [] geometryTypeStream;
        try{
            geometryTypeStream = IntegerCompression.parquetRLEBitpackingHybridEncoding(geometryTypes.stream().mapToInt(i -> i).toArray());
        }
        catch(Exception e){
            System.out.println("Error while compressing geometry types. Length: " + geometryTypes.size());
            geometryTypeStream = new byte[0];
        }
        var geometryOffsetsStream = IntegerCompression.orcRleEncodingV1(geometryOffsets.stream().mapToLong(i -> i).toArray());
        var partOffsetsStream = IntegerCompression.orcRleEncodingV1(partOffsets.stream().mapToLong(i -> i).toArray());
        var ringOffsetsStream = IntegerCompression.orcRleEncodingV1(ringOffsets.stream().mapToLong(i -> i).toArray());

        var geometryColumn = ArrayUtils.addAll(geometryTypeStream, geometryOffsetsStream);
        geometryColumn = ArrayUtils.addAll(geometryColumn, partOffsetsStream);
        geometryColumn = ArrayUtils.addAll(geometryColumn, ringOffsetsStream);

        if(useIceEncoding){
            var vertexOffsetsBuffer = vertexOffsets.stream().mapToInt(i -> i).toArray();
            var parquetDeltaCodedVertexOffsets = IntegerCompression.parquetDeltaEncoding(vertexOffsetsBuffer);
            geometryColumn = ArrayUtils.addAll(geometryColumn, parquetDeltaCodedVertexOffsets);

            //TODO: Test ORC RLE V1, ORC RLE V2 and Parquet Delta encoding
            var parquetTest = IntegerCompression.parquetDeltaEncoding(vertexOffsetsBuffer);
            var deltaCodedVertexOffsets = zigZagDeltaEncodeValues(vertexOffsetsBuffer);
            var varintZigZagDeltaCodedValues = IntegerCompression.varintEncode(deltaCodedVertexOffsets);
            System.out.println("Parquet Delta: " + parquetTest.length + " Varint Delta: " + varintZigZagDeltaCodedValues.length);
            geometryColumn = ArrayUtils.addAll(geometryColumn, varintZigZagDeltaCodedValues);
        }

        var vertexBufferStream = IntegerCompression.varintEncode(vertexBuffer);
        return ArrayUtils.addAll(geometryColumn, vertexBufferStream);
    }

    private static int[] zigZagDeltaEncodeValues(int[] values){
        var previousValue = 0;
        var deltaCodedValues = new int[values.length];
        var i = 0;
        for(var value : values){
            var deltaValue = value - previousValue;
            var zigZagDeltaValue = (deltaValue >> 31) ^ (deltaValue << 1);
            deltaCodedValues[i++] = zigZagDeltaValue;
            previousValue = value;
        }

        return deltaCodedValues;
    }

    private static int getVertexOffset(Set<Map.Entry<Integer, Vertex>> iceEncodedVertexBuffer, Coordinate vertex,
                                       SmallHilbertCurve hilbertCurve){
            var x = NUM_COORDINATES_PER_QUADRANT + (int) vertex.x;
            var y = NUM_COORDINATES_PER_QUADRANT + (int) vertex.y;
            var hilbertIndex = (int) hilbertCurve.index(x, y);
            return Iterables.indexOf(iceEncodedVertexBuffer, v -> v.getKey().equals(hilbertIndex));
    }

    private static void deltaCodePolygon(Polygon polygon, List<Vertex> vertexBuffer, List<Integer> partOffsets, List<Integer> ringOffsets){
        var numRings = polygon.getNumInteriorRing() + 1;
        partOffsets.add(numRings);
        var shell = polygon.getExteriorRing();
        zigZagDeltaCodeLineString(shell, vertexBuffer);

        for(var i = 0; i < polygon.getNumInteriorRing(); i++){
            var ring = polygon.getInteriorRingN(i);
            zigZagDeltaCodeLineString(ring, vertexBuffer);
            ringOffsets.add(ring.getNumPoints());
        }
    }

    private static void iceDeltaCodeLineString(LineString lineString, TreeMap<Integer, Vertex> vertexBuffer,
                                               SmallHilbertCurve hilbertCurve) {
        var vertices = lineString.getCoordinates();
        for (var vertex : vertices) {
            var x = NUM_COORDINATES_PER_QUADRANT + (int) vertex.x;
            var y = NUM_COORDINATES_PER_QUADRANT + (int) vertex.y;
            var index = (int) hilbertCurve.index(x, y);

            if (!vertexBuffer.containsKey(index)) {
                vertexBuffer.put(index, new Vertex(x, y));
            }
        }
    }

    private static void zigZagDeltaCodeLineString(LineString lineString, List<Vertex> vertexBuffer){
        var vertices = lineString.getCoordinates();
        var deltaVertexX = 0;
        var deltaVertexY = 0;
        for (var vertex : vertices) {
            var x = (int) vertex.x - deltaVertexX;
            var y = (int) vertex.y - deltaVertexY;

            var zigZagX = (x >> 31) ^ (x << 1);
            var zigZagY = (y >> 31) ^ (y << 1);

            vertexBuffer.add(new Vertex(zigZagX, zigZagY));
            deltaVertexX = x;
            deltaVertexY = y;
        }
    }

    private static List<Layer> getMVT(String mbTilesFileName, int zoom, int x, int y) throws SQLException, IOException, ClassNotFoundException {
        Class.forName("org.sqlite.JDBC");
        byte[] blob;
        try(var connection = DriverManager.getConnection("jdbc:sqlite:" + mbTilesFileName)) {
            try(var stmt = connection.createStatement()){
                try(ResultSet rs = stmt.executeQuery(String.format("SELECT %s FROM tiles WHERE tile_column = %d AND tile_row = %d AND zoom_level = %d;",
                        TILE_DATA_KEY, x, y, zoom))){
                    rs.next();
                    blob = rs.getBytes(TILE_DATA_KEY);
                }
            }
        }

        var inputStream = new ByteArrayInputStream(blob);
        var gZIPInputStream = new GZIPInputStream(inputStream);
        var mvtTile = gZIPInputStream.readAllBytes();
        System.out.println("MVT tile size uncompressed: " + mvtTile.length);

        var result = MvtReader.loadMvt(
                new ByteArrayInputStream(mvtTile),
                MvtConverter.createGeometryFactory(),
                new TagKeyValueMapConverter(false, "id"));
        final var mvtLayers = result.getLayers();

        var layers = new ArrayList<Layer>();
        for(var layer : mvtLayers){
            var name = layer.getName();
            var mvtFeatures = layer.getGeometries();
            var features = new ArrayList<Feature>();
            for(var mvtFeature : mvtFeatures){
                var properties = ((LinkedHashMap)mvtFeature.getUserData());
                var id = (long)properties.get(ID_KEY);
                properties.remove(ID_KEY);
                var feature = new Feature(id, mvtFeature, properties);
                features.add(feature);
            }

            layers.add(new Layer(name, features));
        }

        return layers;
    }

    private static GeometryFactory createGeometryFactory() {
        final PrecisionModel precisionModel = new PrecisionModel();
        final PackedCoordinateSequenceFactory coordinateSequenceFactory =
                new PackedCoordinateSequenceFactory(PackedCoordinateSequenceFactory.DOUBLE);
        return new GeometryFactory(precisionModel, 0, coordinateSequenceFactory);
    }
}
