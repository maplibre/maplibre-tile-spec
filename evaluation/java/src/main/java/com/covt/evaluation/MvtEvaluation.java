package com.covt.evaluation;

import com.covt.evaluation.compression.IntegerCompression;
import com.google.common.collect.Iterables;
import io.github.sebasbaumh.mapbox.vectortile.adapt.jts.MvtReader;
import io.github.sebasbaumh.mapbox.vectortile.adapt.jts.TagKeyValueMapConverter;
import org.davidmoten.hilbert.HilbertCurve;
import org.davidmoten.hilbert.SmallHilbertCurve;
import org.locationtech.jts.geom.*;
import org.locationtech.jts.geom.impl.PackedCoordinateSequenceFactory;

import java.io.ByteArrayInputStream;
import java.io.IOException;
import java.sql.*;
import java.util.*;
import java.util.stream.Collectors;
import java.util.stream.IntStream;
import java.util.stream.Stream;
import java.util.zip.GZIPInputStream;

public class MvtEvaluation {
    private static final String ID_KEY = "id";

    private static final Map<String, Integer> GEOMETRY_TYPES = Map.of("Point", 0,
        "LineString", 1, "Polygon", 2, "MultiPoint", 3,
            "MultiLineString", 4, "MultiPolygon", 5
    );

    public static void main(String[] args) throws IOException, SQLException, ClassNotFoundException {
        var layers = parseMvt();
        convertLayers(layers);
    }

    private static void convertLayers(List<Layer> layers) throws IOException {
        for(var layer : layers){
            //analyzeIds(layer);
            //analyzeGeometryTypes(layer);
            //analyzeTopology(layer);
            analyzeGeometry(layer);
        }
    }

    private static void analyzeGeometry(Layer layer) throws IOException {
        var name = layer.name();
        if(!name.equals("transportation")){
            return;
        }

        /*
         * -> Sort Vertices on Hilbert Curve
         * -> Iterate of the coordinates of the geometries and compare with the geometries in the vertex buffer
         *    and save the index
         * -> Delta Encode Vertex Buffer
         * -> Delta Encode Index Buffer?
         * */

        //14 bits -> 8192 in two directions
        var numCoordinatesPerQuadrant = 8192;
        SmallHilbertCurve hilbertCurve =
                HilbertCurve.small().bits(14).dimensions(2);
        var vertexMap = new TreeMap<Integer, Vertex>();
        var features = layer.features();
        for(var feature : features){
            //var geometryType = GEOMETRY_TYPES.get(feature.geometry().getGeometryType());
            var geometryType = feature.geometry().getGeometryType();

            switch(geometryType){
                case "Point":
                    break;
                case "LineString": {
                    var lineString = (LineString) feature.geometry();
                    var vertices = lineString.getCoordinates();
                    for (var vertex : vertices) {
                        /* shift origin to have no negative coordinates */
                        var x = numCoordinatesPerQuadrant + (int) vertex.x;
                        var y = numCoordinatesPerQuadrant + (int) vertex.y;
                        var index = (int) hilbertCurve.index(x, y);
                        if (!vertexMap.containsKey(index)) {
                            vertexMap.put(index, new Vertex(x, y));
                        }
                    }

                    break;
                }
                case "Polygon":
                    break;
                case "MultiPoint":
                    throw new IllegalArgumentException("Geometry type MultiPoint is not supported yet.");
                case "MultiLineString":{
                    var multiLineString = ((MultiLineString)feature.geometry());
                    var numLineStrings = multiLineString.getNumGeometries();
                    for(var i = 0; i < numLineStrings; i++){
                        var lineString =  (LineString)multiLineString.getGeometryN(i);
                        var vertices = lineString.getCoordinates();
                        for (var vertex : vertices) {
                            /* shift origin to have no negative coordinates */
                            var x = numCoordinatesPerQuadrant + (int) vertex.x;
                            var y = numCoordinatesPerQuadrant + (int) vertex.y;
                            var index = (int) hilbertCurve.index(x, y);
                            if (!vertexMap.containsKey(index)) {
                                vertexMap.put(index, new Vertex(x, y));
                            }
                        }
                    }
                    break;
                }
                case "MultiPolygon":
                    break;
                default:
                    throw new IllegalArgumentException("GeometryCollection not supported.");
            }
        }


        Set<Map.Entry<Integer, Vertex>> vertexSet = vertexMap.entrySet();
        var vertexOffsets = new ArrayList<Integer>();
        for(var feature : features){
            var geometryType = feature.geometry().getGeometryType();
            switch(geometryType){
                case "LineString": {
                    var lineString = (LineString) feature.geometry();
                    var vertices = lineString.getCoordinates();
                    for (var vertex : vertices) {
                        var x = numCoordinatesPerQuadrant + (int) vertex.x;
                        var y = numCoordinatesPerQuadrant + (int) vertex.y;
                        var hilbertIndex = (int) hilbertCurve.index(x, y);
                        var vertexOffset = Iterables.indexOf(vertexSet,v -> v.getKey().equals(hilbertIndex));
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
                            /* shift origin to have no negative coordinates */
                            var x = numCoordinatesPerQuadrant + (int) vertex.x;
                            var y = numCoordinatesPerQuadrant + (int) vertex.y;
                            var hilbertIndex = (int) hilbertCurve.index(x, y);
                            var vertexOffset = Iterables.indexOf(vertexSet,v -> v.getKey().equals(hilbertIndex));
                            vertexOffsets.add(vertexOffset);
                        }
                    }
                    break;
                }
                default:
                    throw new IllegalArgumentException("Geometry type not supported.");
            }
        }

        var vertexBuffer = vertexSet.stream().flatMap(v -> {
            var coord = v.getValue();
            var x = coord.x();
            var y = coord.y();
            return Stream.of(x,y);
        }).mapToInt(i->i).toArray();
        var vertexOffsetsArr = vertexOffsets.stream().mapToInt(i->i).toArray();

        var vertexBufferParquetDelta = IntegerCompression.parquetDeltaEncoding(vertexBuffer);
        var vertexBufferORCRleV2 = IntegerCompression.orcRleEncodingV2(Arrays.stream(vertexBuffer).mapToLong(i -> i).toArray());
        var vertexOffsetsParquetDelta = IntegerCompression.parquetDeltaEncoding(vertexOffsetsArr);
        var vertexOffsetsORCRleV2 = IntegerCompression.orcRleEncodingV2(Arrays.stream(vertexOffsetsArr).mapToLong(i -> i).toArray());
        System.out.println("Ratio: " + ((double)vertexOffsetsArr.length / vertexBuffer.length));
        System.out.println("VertexBuffer Parquet Delta: " + vertexBufferParquetDelta.length / 1024);
        System.out.println("VertexBuffer ORC RLEV2: " + vertexBufferORCRleV2.length / 1024);
        System.out.println("VertexOffsets Parquet Delta: " + vertexOffsetsParquetDelta.length / 1024);
        System.out.println("VertexOffsets ORC RLEV2: " + vertexOffsetsORCRleV2.length / 1024);

        /**
         * Sorting the full vertexOffsets not working -> only the offsets of a LineString collection can be sorted
         * -> but in Transportation most of the time only 2 vertices
         * -> LineString -> PartOffsets (e.g. vertex 0 to 10) -> VertexOffsets (int to coordinates) -> VertexBuffer
         * -> LineString -> PartOffsets (e.g. vertex 0 to 10) -> HilbertIndices
         */
        Collections.sort(vertexOffsets);
        var sortedDeltaEncodedVertexOffsets = new long[vertexOffsetsArr.length];
        var previousValue = 0;
        for(var i = 0; i < vertexOffsets.size(); i++){
            var value = vertexOffsets.get(i);
            var delta = value - previousValue;
            sortedDeltaEncodedVertexOffsets[i] = delta;
            previousValue = value;
        }
        var sortedVertexOffsetsArr = vertexOffsets.stream().mapToInt(i->i).toArray();
        var sortedVertexOffsetsParquetDelta = IntegerCompression.parquetDeltaEncoding(sortedVertexOffsetsArr);
        var sortedVertexOffsetsORCRleV2 = IntegerCompression.orcRleEncodingV2(Arrays.stream(sortedVertexOffsetsArr).mapToLong(i -> i).toArray());
        var sortedDeltaVertexOffsetsParquetDelta = IntegerCompression.parquetDeltaEncoding(Arrays.stream(sortedDeltaEncodedVertexOffsets).mapToInt(i -> (int)i).toArray());
        var sortedDeltaVertexOffsetsORCRleV2 = IntegerCompression.orcRleEncodingV2(sortedDeltaEncodedVertexOffsets);
        System.out.println("Sorted VertexOffsets Parquet Delta: " + sortedVertexOffsetsParquetDelta.length / 1024);
        System.out.println("Sorted VertexOffsets ORC RLEV2: " + sortedVertexOffsetsORCRleV2.length / 1024);
        System.out.println("Delta Sorted VertexOffsets Parquet Delta: " + sortedDeltaVertexOffsetsParquetDelta.length / 1024);
        System.out.println("Delta Sorted VertexOffsets ORC RLEV2: " + sortedDeltaVertexOffsetsORCRleV2.length / 1024);

        /*var deltaEncodedVertexOffsets = new long[vertexOffsetsArr.length];
        var previousValue = 0;
        for(var i = 0; i < vertexOffsetsArr.length; i++){
            var value = vertexOffsetsArr[i];
            var delta = value - previousValue;
            deltaEncodedVertexOffsets[i] = delta;
            previousValue = value;
        }
        var deltaEncodedVertexOffsetsParquetDelta = IntegerCompression.parquetDeltaEncoding(Arrays.stream(deltaEncodedVertexOffsets).mapToInt(i -> (int)i).toArray());
        var deltaEncodedVertexOffsetsORCRleV2 = IntegerCompression.orcRleEncodingV2(deltaEncodedVertexOffsets);
        System.out.println("Delta VertexOffsets Parquet Delta: " + deltaEncodedVertexOffsetsParquetDelta.length / 1024);
        System.out.println("Delta Vertex Offsets ORC RLEV2: " + deltaEncodedVertexOffsetsORCRleV2.length / 1024);*/
    }

    private static void analyzeTopology(Layer layer) throws IOException {
        var name = layer.name();
        if(!name.equals("transportation")){
            return;
        }

        /*
        * Depending on the geometry type the topology column has the following streams:
        * - Point: no stream
        * - LineString: Part offsets
        * - Polygon: Part offsets (Polygon), Ring offsets (LinearRing)
        * - MultiPoint: Geometry offsets -> array of offsets indicate where the vertices of each MultiPoint start
        * - MultiLineString: Geometry offsets, Part offsets (LineString)
        * - MultiPolygon -> Geometry offsets, Part offsets (Polygon), Ring offsets (LinearRing)
        * Currently for all geometry types all streams hava an entry -> later flags should be used to
        * reduce memory footprint in the client after RLE decoding
        * -> If GeometrieOffsets >= 1 -> MulitPartGeometry
        * -> If PartOffsets and RingOffsets = 0 -> Point
        * -> If PartOffsets >= 2 and RingOffsets and GeometrieOffsets = 0 -> LinearRing
        * -> If GeometrieOffsets = 0 and RingOffsets >= 1 -> Polygon -> 0,1,10
        * -> Zero indicates no offsets (PartOffsets and RingOffsets) present
        *   -> A mulit-part geometry therefore needs at least one element
        *   -> against the SFA spec where zero and empty geometries are allowed
        * How to handle if there are different geometry types per layer like Point and LineString
        * not only the geometry and multi-part version of the geometry?
        * - Point and MultiPoint -> numPoints
        * - LineString and MultiLineString -> numLineStrings, numVertices
        * - Polygon and MultiPolygon -> numPolygons, numRings, numVertices
        * - Point and LineString -> numVertices -> 1 indicates Point geometry
        * - Point and Polygon -> numRings, numVertices -> 1 for numRings and numVertices indicates Point
        * - LineString and Polygon
        * - Point and MultiLineString
        * - Point and MultiPolygon
        * - LineString and MultiPolygon
        * - LineString and MultiPoint
        * - Polygon and MultiPoint
        * - Polygon and MultiLineString
        * -> if a fixed structure for every geometry with Geometry offsets, Part offsets (Polygon) and Ring offsets
        *    is used no geometry type column is needed
        * -> via RLE encoding the sparse columns like Geometry offsets and Part offsets can be effectively compressed
        * -> or use separate flag for layer which all have the same geometry type to prune columns -> less memory
        *    needed on the client -> find the typ with the highest dimension -> dimension flag for each layer
        * -> if Indexed Coordinate Encoding (ICE) is used -> additional vertex offset stream
        *   -> GeometryOffsets, PartOffsets, RingOffsets, VertexOffsets
        *  */

        var features = layer.features();
        var geometryOffsets = new ArrayList<Integer>();
        var partOffsets = new ArrayList<Integer>();
        var ringOffsets = new ArrayList<Integer>();
        geometryOffsets.add(0);
        partOffsets.add(0);
        ringOffsets.add(0);
        for(var feature : features){
            //var geometryType = GEOMETRY_TYPES.get(feature.geometry().getGeometryType());
            var geometryType = feature.geometry().getGeometryType();

            switch(geometryType){
                case "Point":
                    partOffsets.add(0);
                    ringOffsets.add(0);
                    geometryOffsets.add(0);
                    break;
                case "LineString":
                    /*
                    *  Vertex offsets -> if Indexed Coordinate Encoding (ICE) is used
                    * */
                    ringOffsets.add(0);
                    geometryOffsets.add(0);
                    var numVertices = feature.geometry().getNumPoints();
                    var partOffset = partOffsets.get(partOffsets.size() - 1) + numVertices;
                    partOffsets.add(partOffset);
                    break;
                case "Polygon":
                    geometryOffsets.add(0);

                    var polygon = (Polygon)feature.geometry();
                    var numRings = polygon.getNumInteriorRing() + 1;
                    partOffset = partOffsets.get(partOffsets.size() - 1) + numRings;
                    partOffsets.add(partOffset);
                    numVertices = polygon.getExteriorRing().getNumPoints();
                    for(var j = 0; j < polygon.getNumInteriorRing(); j++){
                        numVertices += polygon.getInteriorRingN(j).getNumPoints();
                    }
                    var ringOffset = ringOffsets.get(ringOffsets.size()-1) + numVertices;
                    ringOffsets.add(ringOffset);
                    break;
                case "MultiPoint":
                    throw new IllegalArgumentException("Geometry type MultiPoint is not supported yet.");
                case "MultiLineString":
                    ringOffsets.add(0);

                    var multiLineString = ((MultiLineString)feature.geometry());
                    var numLineStrings = multiLineString.getNumGeometries();
                    var geometryOffset = geometryOffsets.get(geometryOffsets.size() -1) + numLineStrings;
                    geometryOffsets.add(geometryOffset);
                    for(var i = 0; i < numLineStrings; i++){
                        numVertices = multiLineString.getGeometryN(i).getNumGeometries();
                        partOffset = partOffsets.get(partOffsets.size() - 1) + numVertices;
                        partOffsets.add(partOffset);
                    }
                    //TODO: how to handle the ring offsets stream?
                    break;
                case "MultiPolygon":
                    var multiPolygon = ((MultiPolygon) feature.geometry());
                    var numPolygons = multiPolygon.getNumGeometries();
                    geometryOffset = geometryOffsets.get(geometryOffsets.size() -1) + numPolygons;
                    geometryOffsets.add(geometryOffset);
                    //geometryOffsets.add

                    for (var i = 0; i < numPolygons; i++) {
                        polygon = (Polygon) multiPolygon.getGeometryN(i);
                        numRings = polygon.getNumInteriorRing() + 1;
                        partOffset = partOffsets.get(partOffsets.size() - 1) + numRings;
                        partOffsets.add(partOffset);
                        numVertices = polygon.getExteriorRing().getNumPoints();
                        for (var j = 0; j < polygon.getNumInteriorRing(); j++) {
                            numVertices += polygon.getInteriorRingN(j).getNumPoints();
                        }
                        ringOffset = ringOffsets.get(ringOffsets.size()-1) + numVertices;
                        ringOffsets.add(ringOffset);
                    }
                    break;
                default:
                    throw new IllegalArgumentException("GeometryCollection not supported.");
            }
        }

        var geometryOffsetsArr =  geometryOffsets.stream().mapToInt(i -> i).toArray();
        var partOffsetsArr =  partOffsets.stream().mapToInt(i -> i).toArray();
        var ringOffsetsArr =  ringOffsets.stream().mapToInt(i -> i).toArray();
        var geometryOffsetsRLEV2 = IntegerCompression.orcRleEncodingV2(Arrays.stream(geometryOffsetsArr).mapToLong(i -> i).toArray());
        var partOffsetsRLEV2 = IntegerCompression.orcRleEncodingV2(Arrays.stream(partOffsetsArr).mapToLong(i -> i).toArray());
        var ringOffsetsRLEV2 = IntegerCompression.orcRleEncodingV2(Arrays.stream(ringOffsetsArr).mapToLong(i -> i).toArray());
        var geometryOffsetsRLEV1 = IntegerCompression.orcRleEncodingV1(Arrays.stream(geometryOffsetsArr).mapToLong(i -> i).toArray());
        var partOffsetsRLEV1 = IntegerCompression.orcRleEncodingV1(Arrays.stream(partOffsetsArr).mapToLong(i -> i).toArray());
        var ringOffsetsRLEV1 = IntegerCompression.orcRleEncodingV1(Arrays.stream(ringOffsetsArr).mapToLong(i -> i).toArray());
        /*var geometryOffsetsParquetRLE = IntegerCompression.parquetRLEBitpackingHybridEncoding(geometryOffsetsArr);*/
        var partOffsetsParquetRLE = IntegerCompression.parquetRLEBitpackingHybridEncoding(partOffsetsArr);
        /*var ringOffsetsParquetRLE = IntegerCompression.parquetRLEBitpackingHybridEncoding(ringOffsetsArr);*/
        var geometryOffsetsParquetDelta = IntegerCompression.parquetDeltaEncoding(geometryOffsetsArr);
        var partOffsetsParquetDelta = IntegerCompression.parquetDeltaEncoding(partOffsetsArr);
        var ringOffsetsParquetDelta = IntegerCompression.parquetDeltaEncoding(ringOffsetsArr);
        System.out.println(name + " -----------------------------------------");
        System.out.println("GeometryOffsets RLE V1: " + geometryOffsetsRLEV1.length);
        System.out.println("PartOffsets RLE V1: " + partOffsetsRLEV1.length);
        System.out.println("RingOffsets RLE V1: " + ringOffsetsRLEV1.length);
        System.out.println("GeometryOffsets RLE V2: " + geometryOffsetsRLEV2.length);
        System.out.println("PartOffsets RLE V2: " + partOffsetsRLEV2.length);
        System.out.println("RingOffsets RLE V2: " + ringOffsetsRLEV2.length);
        /*System.out.println("GeometryOffsets Parquet RLE: " + geometryOffsetsParquetRLE.length);*/
        System.out.println("PartOffsets Parquet RLE: " + partOffsetsParquetRLE.length);
        /*System.out.println("RingOffsets Parquet RLE: " + ringOffsetsParquetRLE.length);*/
        System.out.println("GeometryOffsets Parquet Delta: " + geometryOffsetsParquetDelta.length);
        System.out.println("PartOffsets Parquet Delta: " + partOffsetsParquetDelta.length);
        System.out.println("RingOffsets Parquet Delta: " + ringOffsetsParquetDelta.length);

        var deltaEncodedPartOffsetsArr = new long[partOffsets.size()];
        var previousValue = 0;
        for(var i = 0; i < partOffsets.size(); i++){
            var value = partOffsets.get(i);
            var delta = value - previousValue;
            deltaEncodedPartOffsetsArr[i] = delta;
            previousValue = value;
        }
        var partOffsetsDeltaRLEV1 = IntegerCompression.orcRleEncodingV1(deltaEncodedPartOffsetsArr);
        var partOffsetsDeltaRLEV2 = IntegerCompression.orcRleEncodingV2(deltaEncodedPartOffsetsArr);
        var partOffsetsDeltaParquetRLE = IntegerCompression.parquetRLEBitpackingHybridEncoding(Arrays.stream(deltaEncodedPartOffsetsArr).mapToInt(i -> (int)i).toArray());
        var partOffsetsDeltaParquetDelta = IntegerCompression.parquetDeltaEncoding(Arrays.stream(deltaEncodedPartOffsetsArr).mapToInt(i -> (int)i).toArray());
        System.out.println("PartOffsets Delta ORC RLE V1: " + partOffsetsDeltaRLEV1.length);
        System.out.println("PartOffsets Delta ORC RLE V2: " + partOffsetsDeltaRLEV2.length);
        System.out.println("PartOffsets Delta Parquet RLE: " + partOffsetsDeltaParquetRLE.length);
        System.out.println("PartOffsets Delta Parquet Delta: " + partOffsetsDeltaParquetDelta.length);
    }

    private static void analyzeGeometryTypes(Layer layer) throws IOException {
        var name = layer.name();
        long previousGeometryType = 0;
        var features = layer.features();
        var geometryTypes = new long[features.size()];
        var deltaGeometryTypes = new long[features.size()];
        var i = 0;
        var geometryTypeJoiner = new StringJoiner(",");
        var deltaGeometryTypeJoiner = new StringJoiner(",");
        for(var feature : features){
            var geometryType = GEOMETRY_TYPES.get(feature.geometry().getGeometryType());
            geometryTypes[i] = geometryType;
            var deltaGeometryType = geometryType - previousGeometryType;
            if(i < 25){
                geometryTypeJoiner.add(String.valueOf(geometryTypes[i]));
                deltaGeometryTypeJoiner.add(String.valueOf(deltaGeometryType));
            }
            deltaGeometryTypes[i++] = deltaGeometryType;
            previousGeometryType = geometryType;
        }

        try{
            var rleV1EncodedGeometryTypes = IntegerCompression.orcRleEncodingV1(geometryTypes);
            var rleV2EncodedGeometryTypes = IntegerCompression.orcRleEncodingV2(geometryTypes);
            var rleV1EncodedDeltaGeometryTypes = IntegerCompression.orcRleEncodingV1(deltaGeometryTypes);
            var rleV2EncodedDeltaGeometryTypes = IntegerCompression.orcRleEncodingV2(deltaGeometryTypes);
            var geometryIntTypes = Arrays.stream(geometryTypes).mapToInt(j -> (int) j).toArray();
            var deltaGeometryIntTypes = Arrays.stream(deltaGeometryTypes).mapToInt(j -> (int) j).toArray();
            var parquetDeltaEncodedGeometryTypes = IntegerCompression.parquetDeltaEncoding(geometryIntTypes);
            var parquetRLEEncodedGeometryTypes = IntegerCompression.parquetRLEBitpackingHybridEncoding(geometryIntTypes);
            var parquetDeltaEncodedDeltaGeometryTypes = IntegerCompression.parquetDeltaEncoding(deltaGeometryIntTypes);
            var parquetRLEEncodedDeltaGeometryTypes = IntegerCompression.parquetRLEBitpackingHybridEncoding(deltaGeometryIntTypes);

            //var num = Arrays.stream(geometryIntTypes).filter(a -> a != 1).boxed().collect(Collectors.toList());

            System.out.println(name + " -----------------------------------------");
            System.out.println("Num Types: " +  geometryTypes.length);
            System.out.println("Values: " + geometryTypeJoiner.toString());
            System.out.println("Delta Values: " + deltaGeometryTypeJoiner.toString());
            System.out.println("RLE V1: " + rleV1EncodedGeometryTypes.length);
            System.out.println("RLE V2: " + rleV2EncodedGeometryTypes.length);
            System.out.println("RLE V1 Delta : " + rleV1EncodedDeltaGeometryTypes.length);
            System.out.println("RLE V2 Delta : " + rleV2EncodedDeltaGeometryTypes.length);
            System.out.println("Parquet Delta: " + parquetDeltaEncodedGeometryTypes.length);
            System.out.println("Parquet RLE Bitpacking: " + parquetRLEEncodedGeometryTypes.length);
            System.out.println("Parquet  Delta Delta : " + parquetDeltaEncodedDeltaGeometryTypes.length);
            System.out.println("Parquet RLE Bitpacking Delta: " + parquetRLEEncodedDeltaGeometryTypes.length);
        }
        catch(Exception e){
            System.out.println(e);
        }
    }

    private static void analyzeIds(Layer layer) throws IOException {
        var name = layer.name();
        long previousId = 0;
        var features = layer.features();
        var ids = new long[features.size()];
        var deltaIds = new long[features.size()];
        var i = 0;
        var idJoiner = new StringJoiner(",");
        var deltaIdJoiner = new StringJoiner(",");
        for(var feature : features){
            var id = feature.id();
            ids[i] = id;
            var deltaId = id - previousId;
            if(i < 25){
                idJoiner.add(String.valueOf(id));
                deltaIdJoiner.add(String.valueOf(deltaId));
            }
            deltaIds[i++] = deltaId;
            previousId = id;
        }

        var rleV1EncodedIds = IntegerCompression.orcRleEncodingV1(ids);
        var rleV2EncodedIds = IntegerCompression.orcRleEncodingV2(ids);
        var rleV1EncodedDeltaIds = IntegerCompression.orcRleEncodingV1(deltaIds);
        var rleV2EncodedDeltaIds = IntegerCompression.orcRleEncodingV2(deltaIds);

        System.out.println(name + " -----------------------------------------");
        System.out.println("Num Ids: " +  ids.length);
        System.out.println("Values: " + idJoiner.toString());
        System.out.println("Delta Values: " + deltaIdJoiner.toString());
        System.out.println("RLE V1: " + rleV1EncodedIds.length);
        System.out.println("RLE V2: " + rleV2EncodedIds.length);
        System.out.println("RLE V1 Delta : " + rleV1EncodedDeltaIds.length);
        System.out.println("RLE V2 Delta : " + rleV2EncodedDeltaIds.length);

        if(name.equals("transportation")){
            var intIds = Arrays.stream(ids).mapToInt(j -> (int) j).toArray();
            var intDeltaIds = Arrays.stream(deltaIds).mapToInt(j -> (int) j).toArray();
            var parquetDeltaEncodedIds = IntegerCompression.parquetDeltaEncoding(intIds);
            var parquetRLEEncodedIds = IntegerCompression.parquetRLEBitpackingHybridEncoding(intIds);
            var parquetDeltaEncodedDeltaIds = IntegerCompression.parquetDeltaEncoding(intDeltaIds);
            var parquetRLEEncodedDeltaIds = IntegerCompression.parquetRLEBitpackingHybridEncoding(intDeltaIds);
            System.out.println("Parquet Delta: " + parquetDeltaEncodedIds.length);
            System.out.println("Parquet RLE Bitpacking: " + parquetRLEEncodedIds.length);
            System.out.println("Parquet  Delta Delta : " + parquetDeltaEncodedDeltaIds.length);
            System.out.println("Parquet RLE Bitpacking Delta: " + parquetRLEEncodedDeltaIds.length);
        }
    }

    private static List<Layer> parseMvt() throws SQLException, IOException, ClassNotFoundException {
        Class.forName("org.sqlite.JDBC");
        var connection = DriverManager.getConnection("jdbc:sqlite:C:\\mapdata\\europe.mbtiles");
        var stmt = connection .createStatement();
        ResultSet rs = stmt.executeQuery( "SELECT tile_data FROM tiles WHERE tile_column = 16 AND tile_row = 21 AND zoom_level = 5;");
        //ResultSet rs = stmt.executeQuery( "SELECT tile_data FROM tiles WHERE tile_column = 16 AND tile_row = 20 AND zoom_level = 5;");
        rs.next();
        var blob = rs.getBytes("tile_data");
        rs.close();
        stmt.close();
        connection.close();

        var inputStream = new ByteArrayInputStream(blob);
        var gZIPInputStream = new GZIPInputStream(inputStream);
        var mvtTile = gZIPInputStream.readAllBytes();
        var result = MvtReader.loadMvt(
                new ByteArrayInputStream(mvtTile),
                MvtEvaluation.createGeometryFactory(),
                new TagKeyValueMapConverter(false, "id"));
        final var mvtLayers = result.getLayers();

        var layers = new ArrayList<Layer>();
        for(var layer : mvtLayers){
            var name = layer.getName();
            var mvtFeatures = layer.getGeometries();
            var features = new ArrayList<Feature>();
            for(var mvtFeature : mvtFeatures){
                //var geometryType = mvtFeature.getGeometryType();
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
