package com.covt.evaluation;

import com.covt.evaluation.file.*;
import me.lemire.integercompression.IntWrapper;
import org.locationtech.jts.geom.*;
import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.*;
import java.util.stream.Collectors;

public class CovtParser {

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
    public static List<Layer> parseCovt(byte[] covtBuffer, boolean useIceEncoding, boolean useFastPfor, boolean useStringDictionaryCoding) throws IOException {
        var pos = new IntWrapper(0);
        var header = decodeHeader(covtBuffer, pos);
        var layers = new ArrayList<Layer>();
        for(var i = 0; i < header.numLayers(); i++){
            var layerMetadata = decodeLayerMetadata(covtBuffer, pos);

            var columnMetadata = layerMetadata.columnMetadata();
            if(!columnMetadata[0].columnName().equals("id") || !columnMetadata[1].columnName().equals("geometry")){
                throw new IllegalArgumentException("In the current implementation id and geometry has to be the first to column in a tile.");
            }

            var ids = decodedIds(covtBuffer, layerMetadata.numFeatures(), columnMetadata[0], pos);
            var geometries = decodeGeometries(covtBuffer, layerMetadata.numFeatures(), columnMetadata[1], pos);

            var columns = new HashMap<String, List<Optional>>();
            if(columnMetadata.length > 2){
                //TODO: refactor -> remove array creation
                var featurePropertyColumns = Arrays.copyOfRange(columnMetadata, 2, columnMetadata.length);
                for(var column : featurePropertyColumns){
                    var values = decodePropertyColumn(covtBuffer, layerMetadata.numFeatures(), column, pos);
                    columns.put(column.columnName(), values);
                }
            }

            var features = new ArrayList<Feature>();
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
            layers.add(layer);
        }

        return layers;
    }

    private static List<Optional> decodePropertyColumn(byte[] covtBuffer, int numFeatures, ColumnMetadata columnMetadata, IntWrapper pos) throws IOException {
        var propertyColumnValues = new ArrayList<Optional>();

        /* decode present stream */
        var numBytes = (int)Math.ceil(numFeatures / 8d);
        var presentStream = ConversionUtils.decodeOrcRleByteEncodingV1(covtBuffer, numBytes, pos);
        var bitSet = BitSet.valueOf(presentStream);
        if(columnMetadata.columnType() == ColumnDataType.INT_64){
            for(var i = 0; i < numFeatures; i++){
                if(bitSet.get(i)){
                    //TODO: replace int with long
                    /* Varint zig-zag decode */
                    var value = ConversionUtils.decodeZigZagVarint(covtBuffer, pos);
                    propertyColumnValues.add(Optional.of(value));
                }
                else{
                    propertyColumnValues.add(Optional.empty());
                }
            }
        }
        else if(columnMetadata.columnType() == ColumnDataType.STRING){
            //TODO: also decode localized dictionary
            /* String streams: present (BitVector), data (ORC RLE V1), length (ORC RLE V1), data_dictionary */
            if(!columnMetadata.columnEncoding().equals(ColumnEncoding.DICTIONARY)){
                throw new IllegalArgumentException("Currently only dictionary encoding is supported for String.");
            }

            var numPresentValues = getNumberOfPresentValues(bitSet, numFeatures);
            /*if(numFeatures == 256 && columnMetadata.columnName().equals("name:nonlatin")){
                numPresentValues = 12;
            }*/
            var data = ConversionUtils.decodeOrcRleEncodingV1(covtBuffer, numPresentValues, pos);
            var dictionaryData = getStringDictionary(covtBuffer, data, pos);

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

    private static String[] getStringDictionary(byte[] covtBuffer, long[] data, IntWrapper pos) throws IOException {
        //TODO: add numDistinctValues as Varint for faster decdoing
        var numDictionaryEntries = Arrays.stream(data).distinct().toArray().length;
        var lengthStream = ConversionUtils.decodeOrcRleEncodingV1(covtBuffer, numDictionaryEntries, pos);

        var dictionaryData = new String[numDictionaryEntries];
        for(var i = 0; i < lengthStream.length; i++){
            var length = (int)lengthStream[i];
            dictionaryData[i] = ConversionUtils.decodeString(covtBuffer, pos, length);
        }

        return dictionaryData;
    }

    private static Geometry[] decodeGeometries(byte[] covtBuffer, int numFeatures, ColumnMetadata columnMetadata, IntWrapper pos) throws IOException {
        /*
        * - Geometry column streams -> geometryType, geometryOffsets, ringOffsets, partOffsets, vertexOffsets, vertexBuffer
        * - geometryType -> Byte, for test ORC RLE V1 but in general parquetRLEBitpackingHybridEncoding -> Bitpacking Hybrid 23 bytes vs ORC RLE V1 1kb vs ORC RLE V2 370 bytes
        * - geometryOffsets, ringOffsets, partOffsets -> ORC RLE V1
        * - vertexOffsets -> delta, zig-zag, varint encoded
        * - vertexBuffer ICE -> delta, zig-zag, varint encoded
        * */

        //TODO: quick and dirty -> get rid of this loop and use a better compression algorithm
        var geometryTypes = Arrays.stream(ConversionUtils.decodeOrcRleEncodingV1(covtBuffer, numFeatures, pos))
                .mapToObj(v -> GeometryType.values()[(int)v]).collect(Collectors.toList());

        var numGeometryOffsets = 0;
        var numPartOffsets  = 0;
        //var numRingOffsets = 0;
        for(var geometryType : geometryTypes){
            /*
            * - Depending on the geometryType and encoding read geometryOffsets, partOffsets, ringOffsets and vertexOffsets
            * - geometryOffsets, partOffsets, ringOffsets and vertexOffsets streams are zigZag, Delta, Varint encoded
            * Logical Representation
            * - Point: no stream
            * - LineString: Part offsets
            * - Polygon: Part offsets (Polygon), Ring offsets (LinearRing)
            * - MultiPoint: Geometry offsets -> array of offsets indicate where the vertices of each MultiPoint start
            * - MultiLineString: Geometry offsets, Part offsets (LineString)
            * - MultiPolygon -> Geometry offsets, Part offsets (Polygon), Ring offsets (LinearRing)
            * -> In addition when Indexed Coordinate Encoding (ICE) is used Vertex offsets stream is added
            * Physical Representation
            * */

            if(geometryType.equals(GeometryType.MULTIPOINT) || geometryType.equals(GeometryType.MULTILINESTRING) ||
                    geometryType.equals(GeometryType.MULTIPOLYGON)){
                numGeometryOffsets++;
            }
            else if(geometryType.equals(GeometryType.LINESTRING) || geometryType.equals(GeometryType.MULTILINESTRING) ||
                    geometryType.equals(GeometryType.POLYGON) || geometryType.equals(GeometryType.MULTIPOLYGON)){
                /* -> For LineString
                        -> PartOffsets -> just increment
                *  -> For Polygon
                *       -> PartOffsets -> Just increment
                *       -> RingOffsets -> Add values of partOffsets
                *   -> For MultiPoint
                *       -> GeometryOffsets -> Just increment
                *   -> For MultiLineString
                *       -> GeometryOffsets -> Just increment
                *       -> PartOffsets -> Add values of GeometryOffsets
                *   -> For MultiPolygon
                *       -> GeometryOffsets -> Just increment
                *       -> PartOffsets -> Add values of GeometryOffsets
                        -> RingOffsets -> Add values of PartOffsets
                *  */
                numPartOffsets++;
            }
            /*else if(geometryType.equals(GeometryType.POLYGON) || geometryType.equals(GeometryType.MULTIPOLYGON)){
                numRingOffsets++;
            }*/
        }

        /* Decode topology streams */
        var geometryOffsets = ConversionUtils.decodeOrcRleEncodingV1(covtBuffer, numGeometryOffsets, pos);
        //TODO: refactor to improve performance
        if(numGeometryOffsets > 0 && !geometryTypes.stream().anyMatch(g -> g.equals(GeometryType.MULTIPOINT))){
            /* Case for MultiPolygon and MultiLineString
            * -> Add the Multi* count to the simple features
            * */
            numPartOffsets += (int)Arrays.stream(geometryOffsets).sum();
        }
        var partOffsets = ConversionUtils.decodeOrcRleEncodingV1(covtBuffer, numPartOffsets, pos);
        /* Case for Polygon and MultiPolygon */
        long[] ringOffsets = null;
        if(geometryTypes.stream().anyMatch(g -> g.equals(GeometryType.POLYGON) || g.equals(GeometryType.MULTIPOLYGON))){
            var numRingOffsets = (int)Arrays.stream(partOffsets).sum();
            ringOffsets = ConversionUtils.decodeOrcRleEncodingV1(covtBuffer, numRingOffsets, pos);
        }

        /*
        * Construct Geometry
        * -> Point
        *   -> numFeatures
        *   -> if delta coded
        * -> LineString
        *   -> numFeatures -> number of partOffsets
        *   -> VertexBuffer
        *   -> if ICE encoded
        * -> Polygon
        *       -> numFeatures -> number of partOffsets
        * */
        var geometries = new Geometry[numFeatures];
        var partOffsetCounter = 0;
        var ringOffsetsCounter = 0;
        var geometryOffsetsCounter = 0;
        var geometryCounter = 0;
        var geometryFactory = new GeometryFactory();

        if(columnMetadata.columnEncoding().equals(ColumnEncoding.UNORDERED_GEOMETRY_ENCODING)){
            for(var geometryType : geometryTypes){
                if(geometryType.equals(GeometryType.POINT)){
                    var coordinate = getCoordinate(covtBuffer, pos);
                    geometries[geometryCounter++] = geometryFactory.createPoint(coordinate);
                }
                else if(geometryType.equals(GeometryType.LINESTRING)){
                    var numVertices = (int)partOffsets[partOffsetCounter++];
                    var vertices = getLineString(covtBuffer, pos, numVertices, false);
                    geometries[geometryCounter++] = geometryFactory.createLineString(vertices);
                }
                else if(geometryType.equals(GeometryType.POLYGON)){
                    var numRings = (int)partOffsets[partOffsetCounter++];
                    var rings = new LinearRing[numRings - 1];
                    LinearRing shell = getLinearRing(covtBuffer, pos, (int)ringOffsets[ringOffsetsCounter++], geometryFactory);
                    for(var i = 0; i < rings.length; i++){
                        var numVertices = (int)ringOffsets[ringOffsetsCounter++];
                        rings[i] = getLinearRing(covtBuffer, pos, numVertices, geometryFactory);
                    }
                    geometries[geometryCounter++] = geometryFactory.createPolygon(shell, rings);
                }
                else if(geometryType.equals(GeometryType.MULTILINESTRING)){
                    var numLineStrings = (int)geometryOffsets[geometryOffsetsCounter++];
                    var lineStrings = new LineString[numLineStrings];
                    for(var i = 0; i < numLineStrings; i++){
                        var numVertices = (int)partOffsets[partOffsetCounter++];;
                        var vertices = getLineString(covtBuffer, pos, numVertices, false);
                        lineStrings[i] = geometryFactory.createLineString(vertices);
                    }
                    geometries[geometryCounter++] = geometryFactory.createMultiLineString(lineStrings);
                }
                else if(geometryType.equals(GeometryType.MULTIPOLYGON)){
                    var numPolygons = (int)geometryOffsets[geometryOffsetsCounter++];
                    var polygons = new Polygon[numPolygons];
                    for(var i = 0; i < numPolygons; i++){
                        var numRings = (int)partOffsets[partOffsetCounter++];
                        var rings = new LinearRing[numRings - 1];
                        LinearRing shell = getLinearRing(covtBuffer, pos, (int)ringOffsets[ringOffsetsCounter++], geometryFactory);
                        for(var j = 0; j < rings.length; j++){
                            var numVertices = (int)ringOffsets[ringOffsetsCounter++];
                            rings[j] = getLinearRing(covtBuffer, pos, numVertices, geometryFactory);
                        }
                        polygons[i] = geometryFactory.createPolygon(shell, rings);
                    }

                    geometries[geometryCounter++] = geometryFactory.createMultiPolygon(polygons);
                }
                else{
                    throw new IllegalArgumentException("The specified geometry type is currently not supported.");
                }
            }
        }
        else if(columnMetadata.columnEncoding().equals(ColumnEncoding.ORDERED_GEOMETRY_ENCODING)){
            //TODO: decode sorted geometry coding
            throw new IllegalArgumentException("Ordered geometry encoding is currently not supported");
        }
        else{
            /* ICE encoding -> add header numVertexOffsets and numVertices to allow faster decdoing */
            var totalNumPartOffsets = (int)Arrays.stream(partOffsets).sum();
            var vertexOffsets = ConversionUtils.varintZigZagDeltaDecode(covtBuffer, pos, totalNumPartOffsets);
            var numTotalVertices = Arrays.stream(vertexOffsets).max().getAsInt() + 1;
            var vertexBuffer = ConversionUtils.varintZigZagDeltaDecodeCoordinates(covtBuffer, pos, numTotalVertices * 2);

            var vertexOffsetCounter = 0;
            var geometryOffsetCounter = 0;
            for(var geometryType : geometryTypes){
                if(geometryType.equals(GeometryType.LINESTRING)){
                    var numVertices = (int)partOffsets[partOffsetCounter++];
                    var vertices = new Coordinate[numVertices];
                    for(var i = 0; i < numVertices; i++){
                        var offset = vertexOffsets[vertexOffsetCounter++];
                        var x = vertexBuffer[offset * 2];
                        var y = vertexBuffer[offset * 2 + 1];
                        vertices[i] = new Coordinate(x,y);
                    }

                    geometries[geometryCounter++] = geometryFactory.createLineString(vertices);
                }
                else if(geometryType.equals(GeometryType.MULTILINESTRING)){
                    var numLineStrings = (int)geometryOffsets[geometryOffsetCounter++];
                    var lineStrings = new LineString[numLineStrings];
                    for(var i = 0; i < numLineStrings; i++){
                        var numVertices = (int)partOffsets[partOffsetCounter++];
                        var vertices = new Coordinate[numVertices];
                        for(var j = 0; j < numVertices; j++){
                            var offset = vertexOffsets[vertexOffsetCounter++];
                            var x = vertexBuffer[offset * 2];
                            var y = vertexBuffer[offset * 2 + 1];
                            vertices[j] = new Coordinate(x,y);
                        }

                        lineStrings[i] = geometryFactory.createLineString(vertices);
                    }
                    geometries[geometryCounter++] = geometryFactory.createMultiLineString(lineStrings);
                }
                else{
                    throw new IllegalArgumentException("ICE encoding on the specified geometry type is currently not supported.");
                }
            }
        }

        return geometries;
    }

    private static LinearRing getLinearRing(byte[] covtBuffer, IntWrapper pos, int numVertices, GeometryFactory geometryFactory){
        var linearRing = getLineString(covtBuffer, pos, numVertices, true);
        return geometryFactory.createLinearRing(linearRing);
    }

    private static Coordinate[] getLineString(byte[] covtBuffer, IntWrapper pos, int numVertices, boolean closeLineString){
        var vertices = new Coordinate[closeLineString? numVertices+1 : numVertices];
        var previousX = 0;
        var previousY = 0;
        for(var i = 0; i < numVertices; i++){
            var vertex= getCoordinate(covtBuffer, pos);
            var x = previousX + (int)vertex.x;
            var y = previousY + (int)vertex.y;
            vertex.setX(x);
            vertex.setY(y);
            vertices[i] = vertex;

            previousX = x;
            previousY = y;
        }

        vertices[vertices.length -1] = vertices[0];
        return vertices;
    }

    private static Coordinate getCoordinate(byte[] covtBuffer, IntWrapper pos){
        var x = ConversionUtils.decodeZigZagVarint(covtBuffer, pos);
        var y = ConversionUtils.decodeZigZagVarint(covtBuffer, pos);
        return new Coordinate(x,y);
    }

    private static long[] decodedIds(byte[] covtBuffer, int numFeatures, ColumnMetadata columnMetadata, IntWrapper pos) throws IOException {
        if(columnMetadata.columnEncoding().equals(ColumnEncoding.ORC_RLE_V1)){
            return ConversionUtils.decodeOrcRleEncodingV1(covtBuffer, numFeatures, pos);
        }
        else{
            //TODO: optimize this decoding step and test
            return ConversionUtils.decodeVarintLong(covtBuffer, pos);
        }
    }

    private static LayerMetadata decodeLayerMetadata(byte[] covtBuffer, IntWrapper pos) throws IOException {
        var layerName = ConversionUtils.decodeString(covtBuffer, pos);
        var extent = ConversionUtils.decodeVarint(covtBuffer, pos)[0];
        var numFeatures = ConversionUtils.decodeVarint(covtBuffer, pos)[0];
        var numColumns = ConversionUtils.decodeVarint(covtBuffer, pos)[0];

        var columnMetadata = new ColumnMetadata[numColumns];
        for(var i = 0; i < numColumns; i++){
            var columnName = ConversionUtils.decodeString(covtBuffer, pos);
            var columnType = ColumnDataType.values()[covtBuffer[pos.get()]];
            pos.increment();
            var columnEncoding = ColumnEncoding.values()[covtBuffer[pos.get()]];
            pos.increment();
            columnMetadata[i] = new ColumnMetadata(columnName, columnType, columnEncoding);
        }

        return new LayerMetadata(layerName, extent, numFeatures, numColumns, columnMetadata);
    }

    private static Header decodeHeader(byte[] covtBuffer, IntWrapper pos){
        var version = ConversionUtils.decodeVarint(covtBuffer, pos)[0];
        var numLayers = ConversionUtils.decodeVarint(covtBuffer, pos)[0];
        return new Header(version, numLayers);
    }

}
