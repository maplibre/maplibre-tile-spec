package com.covt.evaluation;

import com.covt.compression.IntegerCompressionEvaluation;
import com.covt.evaluation.compression.IntegerCompression;
import com.covt.evaluation.file.ColumnDataType;
import com.covt.evaluation.file.ColumnEncoding;
import com.covt.evaluation.file.GeometryType;
import com.google.common.collect.Iterables;
import me.lemire.integercompression.IntWrapper;
import org.apache.commons.lang3.ArrayUtils;
import org.davidmoten.hilbert.HilbertCurve;
import org.davidmoten.hilbert.SmallHilbertCurve;
import org.locationtech.jts.geom.*;

import java.io.ByteArrayOutputStream;
import java.io.IOException;
import java.nio.charset.StandardCharsets;
import java.util.*;
import java.util.stream.Collectors;
import java.util.stream.Stream;

enum GeometryEncoding{
    UNORDERED_GEOMETRY_ENCODING,
    ORDERED_GEOMETRY_ENCODING,
    INDEXED_COORDINATE_ENCODING
}

public class MvtConverter {
    /* 14 bits -> 8192 in two directions */
    private static final int NUM_COORDINATES_PER_QUADRANT = 8192;

    /*private static final Map<String, Integer> GEOMETRY_TYPES = Map.of("Point", 0,"LineString", 1, "Polygon", 2, "MultiPoint", 3,
            "MultiLineString", 4, "MultiPolygon", 5
    );*/

    /*
    * General
    * - All UInts in the metdata are varint encoded
    * FileLayout
    * --------------------------------------------
    * File Header
    * - Version -> UInt32 (Varint)
    * - NumLayers -> UInt32 (Varint)
    * Layer Metadata
    * - Name (String) | NumFeatures (UInt32 | Varint) | numPropertyColumns (UInt32 | Varint) | Struct Column Metadata []
    *   -> Column Metdata Struct -> Column name (String), data type (Byte), encoding (Byte)
    * Layer Data Columns
    * - Id Column
    *   -> DataType -> UInt64
    *   -> Sorted -> ORC RLE V1 encoding
    *       -> Delta with Parquet RLE Bitpacking Hybrid offers a slightly better compression ratio but would
    *          add an additional encoding to the format -> example transportation layer zoom 5 -> 1.62kb vs 0.45kb
    *   -> Unsorted -> Varint encoding
    * - Geometry Column
    *   -> GeometryTypes
    *       -> ORC RLE V1 Encoding
     *      -> Parquet RLE Bitpacking Hypbrid offers better compression ratio -> which one to use?
     *      -> Bitpacking Hybrid 23 bytes vs ORC RLE V1 1kb vs ORC RLE V2 370 bytes
    *   -> Point
    *       -> Streams -> VertexBuffer
    *       -> VertexBuffer
    *           -> unsorted -> plain -> no encoding
     *          -> sorted -> sort on Hilbert curve and delta encode points with zigZag Varint encoding -> but sorting based
     *             on the id seems to offer a better compression ratio
     *          -> coordinates -> currently delta, zigZag Varint -> use FastPfor?
    *   -> LineString
    *       -> topology streams -> partOffsets
    *           -> PartOffsets -> depending on the flag plain or ORC RLE V1 -> is plain encoding needed?
    *       -> VertexBuffer
    *           -> plain -> delta, zigZag and Varint encode between vertices of a LineString
    *           -> ICE Encoding
    *   -> Polygon
    *       -> topology streams -> partOffsets, rringOffsets
    *       -> plain -> delta, zigZag and Varint encode between vertices of a Polygon
    *       -> remove last redundant Point of a Polygon?
    *   -> MultiPoint
    *           -> topology streams -> geometryOffsets
    *   -> MultiLineString
    *           -> topolgy streams -> geometryOffsets, partOffsets
    *   -> MultiPolygon
    *       -> topology streams -> geometryOffsets, partOffsets, ringOffsets
    *   -> Geometry Data Types
    *       -> Geometry, Geometry_M, Geometry_Z, Geometry_ZM
    *   -> Encodings -> Unordered, Ordered (really needed?), ICE
    * - Properties
    *   - Boolean
    *       -> streams -> present (BitVector with ORC RLE V1 Byte), data (BitVector with ORC RLE V1 Byte)
    *   - Int_64
    *       -> streams -> present (BitVector with ORC RLE V1 Byte), data (zigZag and Varint encdoding)
    *   - UInt_64
    *       -> streams -> present (BitVector with ORC RLE V1 Byte), data (Varint encdoding)
    *   - Float
    *       -> streams -> present (BitVector with ORC RLE V1 Byte), data
    *   - Double
    *       -> streams -> present (BitVector with ORC RLE V1 Byte), data
    *   - String
    *       -> plain -> really needed?
    *           -> streams -> present (BitVector with ORC RLE V1 Byte), length (ORC RLE V1), data (UTF-8)
    *       -> dictionary
    *           -> streams -> present (BitVector with ORC RLE V1 Byte), data (ORC RLE V1), length (ORC RLE V1), data_dictionary
     *
    * */

    /*
    * TODOs:
    * - Replace Varint encoding with Patched bitpacking for coordinates
    *   -> Compare Parquet Delta with other delta encodings based on patched bitpacking
    *   -> Bitpacking with 5 bits and either use Patched encoding or offset compression
    * - Test ORC RLE encodings for vertexOffsets -> add zigZag encoding for delta values?
    * - Compare wins for sorting point layers based on id or geometry
    * - Test for POI layer if hilbert delta coding is working
    * - Check if use RLE encoding or BitVector or BitVector with RLE for present stream
    * - Use a central dictionary for all string columns
    * - Remove the redundant closing vertex of a LinearRing/Polygon
    * - Sort string dictionary on data or data_dictionary
    * */
    public static byte[] createCovtTile(List<Layer> layers, boolean useIceEncoding, boolean useFastPfor, boolean useDictionaryCoding,
                                        boolean removeClosingPolygonVertex) throws IOException {
        /*
        * Design
        *   -> Layer names and size/offsets of the layers part of the header to allow random access?
        * - Header
        *   -> Version: UInt32 (Varint)
        *   -> Num Layers: UInt32 (Varint)
        *   -> Layer names: String[] -> String: length (Varint or byte?), Utf-8 buffer
        * - Layers
        *   - Questions
        *       - Is id and geometry column mandatory or optional? -> start with mandatory
        *       - Add name of id and geometry redundant?
        *       - Make datatype of id column variable?
        *       - Use Protobuf or JSON for the metadata? -> how much larger is the size
        *           - Size
        *               - General -> two Columns
        *               - Poi -> 30 Columns
        *       - Separate  columns for z and m?
        *       - Use just Blob or Json for nested properties? -> what advantage does Dremel encoding have?
        *       - Also use a additional Timestamp component for the geometry column like FlatGeobuf?
        *   - Metadata -> Name | NumFeatures | numPropertyColumns | propertyColumnName,  propertyColumnType, propertyColumnEncoding []
        *       - Name: -> Self contained when part of the layer
        *       - Extent: UInt32 (Varint)
        *       - Num Features: UInt32 (Varint)
        *       - Num Property Columns: UInt32 (Varint)
        *       - Struct Column Metadata [] -> Column name, data type, encoding
        *       - Property Column name: String
        *       - Column Data type: Byte
        *           - String -> 0
        *           - Float -> 1
        *           - Double -> 2
        *           - Int64 -> 3
        *           - UInt64 -> 4
        *           - Boolean -> 5
        *           - Geometry -> 6
        *           - GeometryM -> 7
        *           - GeometryZ-> 8
        *           - GeometryZM -> 9
        *               -> "M" dimension for additional dimensional information (commonly time, or road-mile, or upstream-distance information) for each coordinate
        *               -> JTS and SFA spec uses double for measurement
        *           - Nested
        *               - Dremel encoding
        *               - JSON
        *       - Column encoding: Byte
        *           -> Plain/Default -> 0
        *           -> Varint -> 1
        *           -> Delta Varint (ZigZag) -> 2
        *           -> ORC RLE V1 -> 3
        *           -> ORC RLE V2 -> 4
        *           -> RLE/Bitpacking Hybrid -> 5
        *           -> Delta and RLE/Bitpacking Hybrid -> 6
        *           -> Dictionary with RLE/Bitpacking Hybrid -> 7
        *           -> UnorderedGeometryEncoding -> 8
        *           -> OrderedGeometryEncoding -> 9
        *           -> IndexedCoordinateEncoding -> 10
        *           -> Id
        *           -> Geometry -> Plain or ICE
        *               -> General
        *                   -> Plain -> LineString and Polygon Delta Coded, Point not
        *                   -> VertexBufferDelta -> Point, between adjacent LineString and Polygons -> if sorted
        *                   -> ICE Encoding
        *               -> GeometryType -> always use RLE/Bitpacking Hybrid
        *               -> GeometryOffsets, PartOffsets, RingOffsets -> always ORC RLE V1 encoding?
        *               -> VertexBuffer
        *                   -> Plain -> delta encoding between Points and Vertices of a LineString or Polygon
        *                   -> ICE Encoding
        *                       -> FastPFOR128 or Varint ZigZag Delta fo VertexBuffer
        *                       -> VertexOffsets -> Varint ZigZag Delta coded
        *           -> Properties
        *               -> String -> Plain or Dictionary
        *   - Id Column
        *       -> Plain
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
        try(ByteArrayOutputStream stream = new ByteArrayOutputStream()){
            var header = getHeader(layers);
            stream.write(header);

            for(var layer : layers){
                if(layer.name().equals("building")){
                    var numTotalVertices = layer.features().stream().map(f -> f.geometry().getCoordinates().length).reduce(0, (p, f) -> p + f);
                    System.out.println("Num Buildings: " + layer.features().size() + ", Num Total Vertices: " + numTotalVertices);
                }

                var useIce = (layer.name().equals("transportation") || layer.name().equals("boundary"))&& useIceEncoding;
                var encoding = useIce ? GeometryEncoding.INDEXED_COORDINATE_ENCODING : GeometryEncoding.UNORDERED_GEOMETRY_ENCODING;
                var metadata = getLayerMetadata(layer, encoding);
                stream.write(metadata);

                System.out.println("Layer name: " + layer.name() + " ----------------------------------");
                //TODO: add other point layers
                /* Either sort point layer based on id or geometry */
                /*var features = layer.name() == "poi" ? sortPointFeaturesOnHilbertCurve(layer.features())
                    : layer.features();*/
                //TODO: sort features either based on id or geometry with hilbert id
                var features = layer.features();

                //var ids = (layer.name().equals("building") || layer.name().equals("place") || layer.name().equals("poi")) ?
                var ids = (layer.name().equals("building") || layer.name().equals("poi")) ?
                        sortDeltaVarintEncodeIds(features) : getIds(features, true);
                //var ids = sortDeltaVarintEncodeIds(features);
                stream.write(ids);

                //var plainGeometries = getGeometryColumn(features, false);
                var geometries = getGeometryColumn(features, useIce, useFastPfor && useIce, removeClosingPolygonVertex);
                /*System.out.println(String.format("Plain coding: %s, ICE coding: %s",
                    plainGeometries.length / 1000, geometries.length / 1000));*/
                stream.write(geometries);

                //TODO: store data type and encoding for property column
                var propertyColumns = getPropertyColumns(features, useDictionaryCoding, true, true);
                stream.write(propertyColumns);

                /*System.out.println(String.format("id size: %s, geometry size: %s, property size: %s",
                ids.length, geometries.length, propertyColumns.length));*/
            }

            return stream.toByteArray();
        }
    }

    private static byte[] getHeader(List<Layer> layers) {
        /*
        * -> Version: UInt32 (Varint)
        * -> Num Layers: UInt32 (Varint)
        * */
        var version = 1;
        var numLayers = layers.size();

        return ConversionUtils.varintEncode(new int[]{version, numLayers});
    }

    private static byte[] getLayerMetadata(Layer layer, GeometryEncoding geometryEncoding) throws IOException {
        /*
         * Metadata -> Name | NumFeatures | numPropertyColumns | propertyColumnName,  propertyColumnType, propertyColumnEncoding []
         *       - Name: -> Self contained when part of the layer
         *       - Extent: UInt32 (Varint)
         *       - Num Features: UInt32 (Varint)
         *       - Num Property Columns: UInt32 (Varint)
         *       - Struct Column Metadata [] -> Column name, data type, encoding
         *       - Property Column name: String
         *       - Column Data type: Byte
         *           - String -> 0
         *           - Float -> 1
         *           - Double -> 2
         *           - Int64 -> 3
         *           - UInt64 -> 4
         *           - Boolean -> 5
         *           - Geometry -> 6
         *           - GeometryM -> 7
         *           - GeometryZ-> 8
         *           - GeometryZM -> 9
         *               -> "M" dimension for additional dimensional information (commonly time, or road-mile, or upstream-distance information) for each coordinate
         *               -> JTS and SFA spec uses double for measurement
         *           - Nested
         *               - Dremel encoding
         *               - JSON
         *       - Column encoding: Byte
         *           -> Plain/Default -> 0
         *           -> Varint -> 1
         *           -> Delta Varint (ZigZag) -> 2
         *           -> ORC RLE V1 -> 3
         *           -> ORC RLE V2 -> 4
         *           -> RLE/Bitpacking Hybrid -> 5
         *           -> Delta and RLE/Bitpacking Hybrid -> 6
         *           -> Dictionary with RLE/Bitpacking Hybrid -> 7
         *           -> UnorderedGeometryEncoding -> 8
         *           -> OrderedGeometryEncoding -> 9
         *           -> IndexedCoordinateEncoding -> 10
         *           -> Id
         *           -> Geometry -> Plain or ICE
         *               -> General
         *                   -> Plain -> LineString and Polygon Delta Coded, Point not
         *                   -> VertexBufferDelta -> Point, between adjacent LineString and Polygons -> if sorted
         *                   -> ICE Encoding
         *               -> GeometryType -> always use RLE/Bitpacking Hybrid
         *               -> GeometryOffsets, PartOffsets, RingOffsets -> always ORC RLE V1 encoding?
         *               -> VertexBuffer
         *                   -> Plain -> delta encoding between Points and Vertices of a LineString or Polygon
         *                   -> ICE Encoding
         *                       -> FastPFOR128 or Varint ZigZag Delta fo VertexBuffer
         *                       -> VertexOffsets -> Varint ZigZag Delta coded
         *           -> Properties
         *               -> String -> Plain or Dictionary
        * */

        var name = ConversionUtils.encodeString(layer.name());
        var metadata = name;

        //TODO: get extent from layer
        var extent = ConversionUtils.varintEncode(new int[]{4096});
        metadata = ArrayUtils.addAll(metadata, extent);

        /*
        * - Num Features: UInt32 (Varint)
         *- Num Property Columns: UInt32 (Varint)
         *- Struct Column Metadata [] -> Column name, data type, encoding
        * */
        var features = layer.features();
        var numFeatures = features.size();
        metadata = ArrayUtils.addAll(metadata, ConversionUtils.varintEncode(new int[]{numFeatures}));
        var columnMetadata = getColumnMetadata(features);
        /* Also add id and geometry column */
        var numColumns = ConversionUtils.varintEncode(new int[]{columnMetadata.size() + 2});
        metadata = ArrayUtils.addAll(metadata, numColumns);


        /* Add id and geometry column */
        var idColumnName = ConversionUtils.encodeString("id");
        var idColumnDataType = (byte) ColumnDataType.UINT_64.ordinal();
        var idColumnEncoding = (byte)(geometryEncoding.equals(GeometryEncoding.ORDERED_GEOMETRY_ENCODING)
                ? ColumnEncoding.VARINT.ordinal() : ColumnEncoding.ORC_RLE_V1.ordinal());
        metadata = ArrayUtils.addAll(metadata, idColumnName);
        metadata = ArrayUtils.addAll(metadata, idColumnDataType);
        metadata = ArrayUtils.addAll(metadata, idColumnEncoding);

        var geometryColumnName = ConversionUtils.encodeString("geometry");
        var geometryColumnDataType = (byte)ColumnDataType.GEOMETRY.ordinal();
        var geometryColumnEncoding = (byte)(geometryEncoding.equals(GeometryEncoding.UNORDERED_GEOMETRY_ENCODING)
                ? ColumnEncoding.UNORDERED_GEOMETRY_ENCODING.ordinal() :
                geometryEncoding.equals(GeometryEncoding.ORDERED_GEOMETRY_ENCODING) ? ColumnEncoding.ORDERED_GEOMETRY_ENCODING.ordinal() :
                        ColumnEncoding.INDEXED_COORDINATE_ENCODING.ordinal());
        metadata = ArrayUtils.addAll(metadata, geometryColumnName);
        metadata = ArrayUtils.addAll(metadata, geometryColumnDataType);
        metadata = ArrayUtils.addAll(metadata, geometryColumnEncoding);

        /* Add property columns */
        for(var column : columnMetadata.entrySet()){
            var columnName = ConversionUtils.encodeString(column.getKey());
            var dataType = column.getValue();
            byte encoding = 0;
            if(dataType.equals(ColumnDataType.STRING)){
                encoding = (byte)ColumnEncoding.DICTIONARY.ordinal();
            }
            /*else if(dataType.equals(ColumnDataType.BOOLEAN)){

            }*/
            else if(dataType.equals(ColumnDataType.INT_64)){
                encoding = (byte)ColumnEncoding.VARINT.ordinal();
            }
            else{
                encoding = (byte)ColumnEncoding.PLAIN.ordinal();
            }

            metadata = ArrayUtils.addAll(metadata, columnName);
            metadata = ArrayUtils.addAll(metadata, (byte)dataType.ordinal());
            metadata = ArrayUtils.addAll(metadata, encoding);
        }

        return metadata;
    }

    private static byte[] getPropertyColumns(List<Feature> features, boolean dictionaryCodeStrings, boolean localizedEncoding, boolean useRleEncodingForInts) throws IOException {
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
        var columnMetadata = getColumnMetadata(features);

        var presentStreams = new ArrayList<List<Boolean>>();
        var lengthStreams = new ArrayList<List<Integer>>();
        var dataStreams = new ArrayList<List<byte[]>>();
        var dataBooleanStreams = new ArrayList<List<Boolean>>();
        var dataDictionaryStreams = new ArrayList<List<byte[]>>();
        var dataDictionarySets = new ArrayList<LinkedHashSet<String>>();
        var localizedDataDictionary = new LinkedHashSet<String>();
        var localizedDataDictionaryStream = new ArrayList<byte[]>();
        var localizedLengthStream = new ArrayList<Integer>();
        for(var metadata : columnMetadata.entrySet()){
            var presentStream = new ArrayList<Boolean>();
            var lengthStream = new ArrayList<Integer>();
            var dataStream = new ArrayList<byte[]>();
            var dataBooleanStream = new ArrayList<Boolean>();
            var dataDictionaryStream = new ArrayList<byte[]>();
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
                            lengthStream.add(utf8EncodedData.length);
                            dataDictionaryStream.add(utf8EncodedData);
                        }

                        if(columnName.contains("name") && !localizedDataDictionary.contains((stringValue))){
                            localizedLengthStream.add(utf8EncodedData.length);
                            localizedDataDictionary.add(stringValue);
                            localizedDataDictionaryStream.add(utf8EncodedData);
                        }
                   }
                   else{
                        lengthStream.add(utf8EncodedData.length);
                        dataStream.add(utf8EncodedData);
                   }
               }
                else if(columnDataType == ColumnDataType.INT_64){
                   var intValue = ((Long) propertyValue).intValue();
                   var zigZagInt = ConversionUtils.zigZagEncode(intValue);
                   var varIntEncodedValue = ConversionUtils.varintEncode(new int[]{zigZagInt});
                   dataStream.add(varIntEncodedValue);
               }
                else if(columnDataType == ColumnDataType.BOOLEAN){
                    var booleanValue = (Boolean)propertyValue;
                    dataBooleanStream.add(booleanValue);
                }
                else{
                   throw new IllegalArgumentException("Column data type currently not supported.");
               }
            }

            presentStreams.add(presentStream);

            /* For boolean values */
            if(dataBooleanStream.size() > 0){
                dataBooleanStreams.add(dataBooleanStream);
            }

            /* For int and plain string encoding */
            if(dataStream.size() > 0){
                dataStreams.add(dataStream);
            }

            /* String plain and dictionary coding */
            if(lengthStream.size() > 0){
                lengthStreams.add(lengthStream);
            }

            /* String dictionary encoding */
            if(dataDictionaryStream.size() > 0){
                dataDictionaryStreams.add(dataDictionaryStream);
                dataDictionarySets.add(dataDictionarySet);
            }
        }

        var arr = localizedDataDictionary.stream().collect(Collectors.toList());
        arr.sort(String::compareTo);
        //localizedDataDictionary.clear();
        //localizedDataDictionary.addAll(arr);

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
        * - Boolean
        *   - BitSet with ORC RLE V1
        * */
        var columnBuffer = new byte[0];
        for(var column : columnMetadata.entrySet()){
                var present = presentStreams.remove(0);
                BitSet bitSet = new BitSet(present.size());
                var j = 0;
                for(var p : present){
                    bitSet.set(j++, p);
                }

                var presentStream = bitSet.toByteArray();
                /* The BitSet only returns the bytes until the last set bit */
                var numMissingBytes = (int)Math.ceil(present.size() / 8d) - presentStream.length;
                if(numMissingBytes != 0){
                    var paddingBytes = new byte[numMissingBytes];
                    Arrays.fill(paddingBytes, (byte)0);
                    presentStream = ArrayUtils.addAll(presentStream, paddingBytes);
                }

                var compressedPresentStream = IntegerCompression.orcRleByteEncodingV1(presentStream);
                columnBuffer = ArrayUtils.addAll(columnBuffer, compressedPresentStream);

                if(column.getValue() == ColumnDataType.INT_64){
                    var varintEncodedDataStream = concatByteArrays(dataStreams.remove(0));
                    if(useRleEncodingForInts){
                        var numValues = present.stream().filter(p -> p == true).collect(Collectors.toList()).size();
                        ConversionUtils.varintZigZagDecode(varintEncodedDataStream, new IntWrapper(0), numValues);
                        var intValues = Arrays.stream(ConversionUtils.varintZigZagDecode(varintEncodedDataStream, new IntWrapper(0), numValues)).
                                mapToLong(i -> i).toArray();
                        var rleCoded = IntegerCompression.orcRleEncodingV1(intValues);
                        columnBuffer = ArrayUtils.addAll(columnBuffer, rleCoded);
                    }
                    else{
                        columnBuffer = ArrayUtils.addAll(columnBuffer, varintEncodedDataStream);
                    }
                }
                else if(column.getValue() == ColumnDataType.STRING){
                    var lengthStream = lengthStreams.remove(0).stream().mapToLong(i -> i).toArray();
                    var encodedLengthStream = IntegerCompression.orcRleEncodingV1(lengthStream);

                    if(dictionaryCodeStrings && localizedEncoding && column.getKey().contains("name")){
                        var dataStream = new long[0];
                        for(var feature : features){
                            var value = (String)feature.properties().get(column.getKey());
                            if(value == null){
                                continue;
                            }

                            var index = Iterables.indexOf(localizedDataDictionary, v -> v.equals(value));
                            dataStream = ArrayUtils.addAll(dataStream, index);
                        }

                        var encodedDataStream = IntegerCompression.orcRleEncodingV1(dataStream);
                        columnBuffer = ArrayUtils.addAll(columnBuffer, encodedDataStream);
                    }
                    else if(dictionaryCodeStrings){
                        var dataDictionary = dataDictionarySets.remove(0);
                        var dataStream = new long[0];
                        for(var feature : features){
                            var value = (String)feature.properties().get(column.getKey());
                            if(value == null){
                                continue;
                            }

                            var index = Iterables.indexOf(dataDictionary, v -> v.equals(value));
                            dataStream = ArrayUtils.addAll(dataStream, index);
                        }

                        var dataDictionaryStream = concatByteArrays(
                                dataDictionaryStreams.remove(0));
                        var encodedDataStream = IntegerCompression.orcRleEncodingV1(dataStream);
                        columnBuffer = ArrayUtils.addAll(columnBuffer, encodedDataStream);
                        columnBuffer = ArrayUtils.addAll(columnBuffer, encodedLengthStream);
                        columnBuffer = ArrayUtils.addAll(columnBuffer, dataDictionaryStream);
                    }
                    else{
                        var dataStream = concatByteArrays(dataStreams.remove(0));
                        columnBuffer = ArrayUtils.addAll(columnBuffer, encodedLengthStream);
                        columnBuffer = ArrayUtils.addAll(columnBuffer, dataStream);
                    }
                }
                else if(column.getValue() == ColumnDataType.BOOLEAN){
                    var dataBooleanStream = dataBooleanStreams.remove(0);
                    var booleanBitSet = new BitSet(dataBooleanStream.size());
                    var k = 0;
                    for(var d : dataBooleanStream){
                        booleanBitSet.set(k++, d);
                    }

                    var compressedBooleanStream = IntegerCompression.orcRleByteEncodingV1(booleanBitSet.toByteArray());
                    columnBuffer = ArrayUtils.addAll(columnBuffer, compressedBooleanStream);
                }
                else{
                    throw new IllegalArgumentException("Data type not supported");
                }
            }

        if(localizedDataDictionary.size() > 0){
            var lengthStream = localizedLengthStream.stream().mapToLong(i -> i).toArray();
            var encodedLengthStream = IntegerCompression.orcRleEncodingV1(lengthStream);
            columnBuffer = ArrayUtils.addAll(columnBuffer, encodedLengthStream);
            var encodedLocalizedDictionary = concatByteArrays(localizedDataDictionaryStream);
            //TODO: also sort indices
            //var encodedLocalizedDictionary = concatByteArrays(arr.stream().map(a -> a.getBytes(StandardCharsets.UTF_8)).collect(Collectors.toList()));
            columnBuffer = ArrayUtils.addAll(columnBuffer, encodedLocalizedDictionary);
        }

        return columnBuffer;
    }

    private static HashMap<String, ColumnDataType> getColumnMetadata(List<Feature> features){
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
                    columnMetadata.put(propertyName, ColumnDataType.BOOLEAN);
                    //throw new IllegalArgumentException("Boolean currently not supported as property data type.");
                }
                //TODO: quick and dirty -> use proper solution
                else if(propertyValue instanceof Integer || propertyValue instanceof  Long){
                    if((long)propertyValue >= Math.pow(2, 32)|| (long)propertyValue <= -Math.pow(2, 32)){
                        throw new IllegalArgumentException("Property value to large to fit into a integer.");
                    }

                    //throw new IllegalArgumentException("Integer currently not supported as property data type.");
                    columnMetadata.put(propertyName, ColumnDataType.INT_64);
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

        return columnMetadata;
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

    private static byte[] getIds(List<Feature> features, boolean useRleEncoding) throws IOException {
        //TODO: also sort if Rle encoding is used to ensure ascending order -> in transportation layer
        // this is a problem as 1,1,2,2,.... when sorted
        var ids = features.stream().map(feature -> feature.id()).mapToLong(i -> i).toArray();

        //TODO: fix plain encoding of longs
        return useRleEncoding ? IntegerCompression.orcRleEncodingV1(ids) : ConversionUtils.varintEncode(ids);
    }

    private static byte[] sortDeltaVarintEncodeIds(List<Feature> features) throws IOException {
        features.sort((p, c) -> ((Integer)(int)p.id()).compareTo((int)c.id()));

        var zigZagDeltaCodedIds = new long[features.size()];
        var previousValue = 0l;
        var i = 0;
        for(var feature : features){
            //TODO: remove conversion to int -> implement zigZag encoding for long
            zigZagDeltaCodedIds[i++] = ConversionUtils.zigZagEncode((int)(feature.id() - previousValue));
            previousValue = feature.id();
        }

        return ConversionUtils.varintEncode(zigZagDeltaCodedIds);
    }

    /*
    * allowGeometriesToBeSorted
    * -> specifies if the geometries ca be ordered
    * -> this can be the case when the layer is not sored after the Id
    * -> start with only sorting Points on a Hilbert curve
    * */
    private static byte[] getGeometryColumn(List<Feature> features, boolean useIceEncoding, boolean useFastPfor, boolean removeClosingPolygonVertex) throws IOException {
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
        var geometryFactory = new GeometryFactory();
        var deltaPointsBuffer = new ArrayList<Integer>();
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
                    geometryTypes.add(GeometryType.POINT.ordinal());

                    var point = (Point)feature.geometry();
                    var zigZagX = ConversionUtils.zigZagEncode((int)point.getX());
                    var zigZagY = ConversionUtils.zigZagEncode((int)point.getY());
                    plainVertexBuffer.add(new Vertex(zigZagX, zigZagY));
            }
            else if(geometryType.equals("MultiPoint")) {
                    throw new IllegalArgumentException("MultiPoint currently not supported.");
           }
            else if(geometryType.equals("LineString")){
                    geometryTypes.add( GeometryType.LINESTRING.ordinal());

                    var lineString = (LineString) feature.geometry();
                    partOffsets.add(lineString.getCoordinates().length);
                    /*
                    *
                    * -> sort vertex buffer on hilbert curve
                    * -> add index to vertex offset
                    * */
                    if(useIceEncoding){
                        iceLineString(lineString, iceEncodedVertexBuffer, hilbertCurve);
                    }
                    else{
                        var deltaPointBuffer = zigZagDeltaCodeLineString(lineString, plainVertexBuffer);
                        deltaPointsBuffer.addAll(deltaPointBuffer);
                    }
            }
            else if(geometryType.equals("MultiLineString")){
                    geometryTypes.add( GeometryType.MULTILINESTRING.ordinal());

                    var multiLineString = ((MultiLineString)feature.geometry());
                    var numLineStrings = multiLineString.getNumGeometries();
                    /*
                     * Logical Representation
                     * -> Offsets
                     * Physical Representation
                     * -> Offsets delta encoded -> number of vertices
                     * */
                    geometryOffsets.add(numLineStrings);
                    for(var i = 0; i < numLineStrings; i++){
                        var lineString =  (LineString)multiLineString.getGeometryN(i);
                        partOffsets.add(lineString.getCoordinates().length);
                        if(useIceEncoding){
                            iceLineString(lineString, iceEncodedVertexBuffer, hilbertCurve);
                        }
                        else{
                            zigZagDeltaCodeLineString(lineString, plainVertexBuffer);
                        }
                    }
            }
            else if(geometryType.equals("Polygon")){
                    geometryTypes.add( GeometryType.POLYGON.ordinal());

                    /*
                    * Polygon topology ->  Part offsets (Polygon), Ring offsets (LinearRing)
                    * */
                    var polygon = (Polygon)feature.geometry();
                    deltaCodePolygon(polygon, plainVertexBuffer, partOffsets, ringOffsets, removeClosingPolygonVertex, geometryFactory);
            }
            else if(geometryType.equals("MultiPolygon")){
                    geometryTypes.add( GeometryType.MULTIPOLYGON.ordinal());

                    /*
                    * MultiPolygon -> Geometry offsets, Part offsets (Polygon), Ring offsets (LinearRing)
                    * */
                    var multiPolygon = ((MultiPolygon) feature.geometry());
                    var numPolygons = multiPolygon.getNumGeometries();
                    geometryOffsets.add(numPolygons);
                    for(var i = 0; i < numPolygons; i++){
                        var polygon = (Polygon)multiPolygon.getGeometryN(i);
                        deltaCodePolygon(polygon, plainVertexBuffer, partOffsets, ringOffsets, removeClosingPolygonVertex, geometryFactory);
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
                        //TODO: fix this when a polygon geometry is part of the layer
                        System.err.println(String.format("Geometry type %s not supported for ICE Encoding.", geometryType));
                        //throw new IllegalArgumentException(String.format("Geometry type %s not supported for ICE Encoding.", geometryType));
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

        //geometryTypeStream = IntegerCompression.parquetRLEBitpackingHybridEncoding(geometryTypes.stream().mapToInt(i -> i).toArray());
        //TODO: replace with RLE Bitpacking Hybrid to reduce size of transportation layer with 1kb
        geometryTypeStream = IntegerCompression.orcRleEncodingV1(geometryTypes.stream().mapToLong(i -> i).toArray());
        //geometryTypeStream = IntegerCompression.orcRleByteEncodingV1(geometryTypes.stream()(i -> i).toArray());

        var geometryOffsetsStream = IntegerCompression.orcRleEncodingV1(geometryOffsets.stream().mapToLong(i -> i).toArray());
        var partOffsetsStream = IntegerCompression.orcRleEncodingV1(partOffsets.stream().mapToLong(i -> i).toArray());
        var ringOffsetsStream = IntegerCompression.orcRleEncodingV1(ringOffsets.stream().mapToLong(i -> i).toArray());

        /*
        * Tests -----------------------------------------
        * */
        var partOffsetsOrcV2 = IntegerCompression.orcRleEncodingV2(partOffsets.stream().mapToLong(i -> i).toArray());
        var partOffsetsParquetDelta = IntegerCompression.parquetDeltaEncoding(partOffsets.stream().mapToInt(i -> i).toArray());
        var zigZagDeltaEncoded = zigZagDeltaEncodeValues(partOffsets.stream().mapToInt(i -> i).toArray());
        var varintZigZagDeltaCoded = ConversionUtils.varintEncode(zigZagDeltaEncoded);
        var fastPforEncoded = IntegerCompressionEvaluation.fastPfor128EncodeBuffer(zigZagDeltaEncoded);
        var zigZagDeltaEncodedRingOffsets = zigZagDeltaEncodeValues(ringOffsets.stream().mapToInt(i -> i).toArray());
        var ringOffsetsFastPfor = IntegerCompressionEvaluation.fastPfor128EncodeBuffer(zigZagDeltaEncodedRingOffsets);
        /*
        * -----------------------------------------------
        * */

        var geometryColumn = ArrayUtils.addAll(geometryTypeStream, geometryOffsetsStream);
        geometryColumn = ArrayUtils.addAll(geometryColumn, partOffsetsStream);
        geometryColumn = ArrayUtils.addAll(geometryColumn, ringOffsetsStream);
        //geometryColumn = ArrayUtils.addAll(geometryColumn, fastPforEncoded);
        //geometryColumn = ArrayUtils.addAll(geometryColumn, ringOffsetsFastPfor);

        if(useIceEncoding){
            var vertexOffsetsBuffer = vertexOffsets.stream().mapToInt(i -> i).toArray();

            var deltaCodedVertexOffsets = zigZagDeltaEncodeValues(vertexOffsetsBuffer);
            var varintZigZagDeltaCodedVertexOffsets = ConversionUtils.varintEncode(deltaCodedVertexOffsets);
            var parquetDeltaCodedVertexOffsets = IntegerCompression.parquetRLEBitpackingHybridEncoding(vertexOffsetsBuffer);
            var longVertexOffsetsBuffer = Arrays.stream(vertexOffsetsBuffer).mapToLong(i -> i).toArray();
            var orcRleV1VertexOffsets = IntegerCompression.orcRleEncodingV1(longVertexOffsetsBuffer);
            var orcRleV2VertexOffsets = IntegerCompression.orcRleEncodingV2(longVertexOffsetsBuffer);
            var longOffsets = Arrays.stream(deltaCodedVertexOffsets).mapToLong(i -> i).toArray();
            var orcRleV1DeltaVertexOffsets = IntegerCompression.orcRleEncodingV1(longOffsets);
            var orcRleV2DeltaVertexOffsets = IntegerCompression.orcRleEncodingV2(longOffsets);
            var rleBitpackngHybridDeltaVertexOffsets = IntegerCompression.parquetRLEBitpackingHybridEncoding(deltaCodedVertexOffsets);
            var fastPforDeltaVertexOffsets = IntegerCompressionEvaluation.fastPfor128EncodeBuffer(deltaCodedVertexOffsets);
            System.out.println("------------- Vertex Offsets Varint: " + varintZigZagDeltaCodedVertexOffsets.length + " FastPfor: " + fastPforDeltaVertexOffsets.length);

            /*System.out.println(String.format("varint zig-zag delta offsets: %s, parquet delta offsets: %s, rle v1 offsets: %s, rle v2 offsets: %s",
                    varintZigZagDeltaCodedVertexOffsets.length / 1000, parquetDeltaCodedVertexOffsets.length / 1000,
                    orcRleV1VertexOffsets.length / 1000, orcRleV2VertexOffsets.length / 1000));*/

            //geometryColumn = ArrayUtils.addAll(geometryColumn, fastPforDeltaVertexOffsets);
            geometryColumn = ArrayUtils.addAll(geometryColumn, varintZigZagDeltaCodedVertexOffsets);
        }

        var diffVarintOffsetCompression = Arrays.stream(vertexBuffer).filter(v -> Math.abs(v) >= 64 && Math.abs(v) <= 127).toArray().length;
        //System.out.println("Diff varint and offset compression: " + vertexBuffer.length + ", " + diffVarintOffsetCompression);

        var vertexBufferStream = useIceEncoding? zigZagVarintEncode(vertexBuffer) :
                ConversionUtils.varintEncode(vertexBuffer);
        var zigZagCodedVertexBufferStream = Arrays.stream(vertexBuffer).map(value -> (value >> 31) ^ (value << 1)).toArray();
        var vertexBufferStreamFastPforEncoding = IntegerCompressionEvaluation.fastPfor128EncodeBuffer(Arrays.copyOfRange(zigZagCodedVertexBufferStream, 2, zigZagCodedVertexBufferStream.length));
        var vertexBufferStreamFastPforEncoding2 = IntegerCompressionEvaluation.fastPfor128Encode(Arrays.copyOfRange(vertexBuffer, 2, vertexBuffer.length));
        var plainVertexBufferWithoutDelta = iceEncodedVertexBuffer.entrySet().stream().flatMap(v -> Stream.of(v.getValue().x(), v.getValue().y())).mapToInt(i -> i).toArray();
        var vertexBufferStreamFastPforEncoding4 = IntegerCompressionEvaluation.fastPfor128Encode(plainVertexBufferWithoutDelta);
        var vertexBufferStreamFastPforEncoding5 = IntegerCompression.parquetDeltaEncoding(plainVertexBufferWithoutDelta);
        //var vertexBufferOrcRleV2Encoded = IntegerCompression.orcRleEncodingV2(Arrays.stream(vertexBuffer).mapToLong(i -> (long)i).toArray());
        var vertexBufferOrcRleV2Encoded = IntegerCompression.orcRleEncodingV2(Arrays.stream(zigZagCodedVertexBufferStream).mapToLong(i -> (long)i).toArray());
       var optPfdEncoded = IntegerCompressionEvaluation.optPFDEncode(zigZagCodedVertexBufferStream);
       var pfdEncoded = IntegerCompressionEvaluation.netPFDEncode(zigZagCodedVertexBufferStream);
        //System.out.println(vertexBufferStream.length + ", " + (useIceEncoding? vertexBufferStreamFastPforEncoding.length : vertexBufferStreamFastPforEncoding2));
        /*if(useIceEncoding){
            System.out.println("Varint encoded VertexBuffer: " + vertexBufferStream.length / 1024d +
                    " ORC RLE V2 encoded VertexBuffer: " + vertexBufferOrcRleV2Encoded.length / 1024d +
                    " FastPfor128 without ZigZag encoded VertexBuffer: " + vertexBufferStreamFastPforEncoding.length / 1024d +
                    " FastPfor128 encoded VertexBuffer: " + vertexBufferStreamFastPforEncoding.length / 1024d +
                    " Opt Pfd encoded VertexBuffer: "  + optPfdEncoded / 1024d +
                    " Pfd encoded VertexBuffer:" + pfdEncoded /1024d);
        }*/


        /*if(useIceEncoding){
            for(var delta : vertexBuffer){
                System.out.println(delta);
            }
        }*/


        return useFastPfor? ArrayUtils.addAll(geometryColumn, vertexBufferStreamFastPforEncoding) :
                ArrayUtils.addAll(geometryColumn, vertexBufferStream);
    }

    private static byte[] zigZagVarintEncode(int[] values) throws IOException {
        var zigZagValues = Arrays.stream(values).map(value -> (value >> 31) ^ (value << 1)).toArray();
        return ConversionUtils.varintEncode(zigZagValues);
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

    private static void deltaCodePolygon(Polygon polygon, List<Vertex> vertexBuffer, List<Integer> partOffsets, List<Integer> ringOffsets,
                                         boolean removeClosingPolygonVertex, GeometryFactory geometryFactory){
        var numRings = polygon.getNumInteriorRing() + 1;
        partOffsets.add(numRings);

        var exteriorRing = polygon.getExteriorRing();
        var shell = removeClosingPolygonVertex ?
                new GeometryFactory().createLineString(Arrays.copyOf(exteriorRing.getCoordinates(),
                        exteriorRing.getCoordinates().length - 1)) : exteriorRing;

        zigZagDeltaCodeLineString(shell, vertexBuffer);
        ringOffsets.add(shell.getNumPoints());

        for(var i = 0; i < polygon.getNumInteriorRing(); i++){
            var interiorRing = polygon.getInteriorRingN(i);
            var ring = removeClosingPolygonVertex ?
                    new GeometryFactory().createLineString(Arrays.copyOf(interiorRing.getCoordinates(),
                            interiorRing.getCoordinates().length - 1)) : interiorRing;

            zigZagDeltaCodeLineString(ring, vertexBuffer);
            ringOffsets.add(ring.getNumPoints());
        }
    }

    private static void iceLineString(LineString lineString, TreeMap<Integer, Vertex> vertexBuffer,
                                      SmallHilbertCurve hilbertCurve) {
        var vertices = lineString.getCoordinates();
        for (var vertex : vertices) {
            var x = (int)vertex.x;
            var y  = (int)vertex.y;
            var shiftedX = NUM_COORDINATES_PER_QUADRANT + x;
            var shiftedY = NUM_COORDINATES_PER_QUADRANT + y;
            var index = (int) hilbertCurve.index(shiftedX, shiftedY);

            if (!vertexBuffer.containsKey(index)) {
                vertexBuffer.put(index, new Vertex(x, y));
            }
        }
    }

    private static List<Integer> zigZagDeltaCodeLineString(LineString lineString, List<Vertex> vertexBuffer){
        var vertices = lineString.getCoordinates();
        var previousVertexX = 0;
        var previousVertexY = 0;
        var deltaPoints = new ArrayList<Integer>();
        var i = 0;
        for (var vertex : vertices) {
            var deltaX = (int) vertex.x - previousVertexX;
            var deltaY = (int) vertex.y - previousVertexY;

            var zigZagX = (deltaX >> 31) ^ (deltaX << 1);
            var zigZagY = (deltaY >> 31) ^ (deltaY << 1);

            vertexBuffer.add(new Vertex(zigZagX, zigZagY));
            previousVertexX = (int)vertex.x;
            previousVertexY = (int)vertex.y;

            if(i > 0){
                deltaPoints.add(deltaX);
                deltaPoints.add(deltaY);
            }
            i++;
        }

        return deltaPoints;
    }

}
