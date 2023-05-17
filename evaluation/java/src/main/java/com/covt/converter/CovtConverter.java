package com.covt.converter;

import com.covt.converter.geometry.GeometryType;
import com.covt.converter.geometry.Vertex;
import com.covt.converter.mvt.Feature;
import com.covt.converter.mvt.Layer;
import com.google.common.collect.Iterables;
import org.apache.commons.lang3.ArrayUtils;
import org.davidmoten.hilbert.HilbertCurve;
import org.davidmoten.hilbert.SmallHilbertCurve;
import org.locationtech.jts.geom.*;

import java.io.ByteArrayOutputStream;
import java.io.IOException;
import java.nio.charset.StandardCharsets;
import java.util.*;
import java.util.function.Function;
import java.util.stream.Collectors;

record GeometryColumData(ColumnMetadata columnMetadata, byte[] geometryColumn){}

record PropertyColumData(PropertyColumnsMetadata metadata, byte[] propertyColumns){}

/*
-> boolean -> rle encode present, boolean rle encode data
        *       -> Map<columnName, booleanColumnData> -> ColumnMetadata, PresentStream, DataStream
        *   -> int
        *       -> Map<columnName, intColumnData>
        *   -> String Dictionary
        *       -> Map<columnName, stringDictionaryColumnData>
        *        -> stringDictionaryColumnData -> ColumnMetadata, PresentStream, LengthStream, DictionaryStream
        *   -> String Localized Dictionary
        *       -> Map<columName, stringLocalizedDictionaryColumnData>
                -> ColumnMetadata, Map<LocalizedStreamData>
        *       ->
        */
record BooleanColumnData(ColumnMetadata columnMetadata, List<Boolean> presentStream, List<Boolean> dataStream){}

record LongColumnData(ColumnMetadata columnMetadata, List<Boolean> presentStream, List<Long> dataStream){}

record StringDictionaryColumnData(ColumnMetadata columnMetadata, List<Boolean> presentStream, List<Integer> dataStream,
                                  List<Integer> lengthStream, List<String> dictionaryStream){}

record StringLocalizedDictionaryColumnData(ColumnMetadata columnMetadata, List<Integer> lengthStream,
                                           List<String> dictionaryStream, Map<String, LocalizedStringDictionaryStreamData> streamData){}

record LocalizedStringDictionaryStreamData(List<Boolean> presentStream, List<Integer> dataStream){}

record PropertyColumnsMetadata(List<NamedColumnMetadata> booleanMetadata, List<NamedColumnMetadata> longMetadata,
                                List<NamedColumnMetadata> stringDictionaryMetadata,
                                List<NamedColumnMetadata> localizedStringDictionaryMetadata){}

record NamedColumnMetadata(String columnName, ColumnMetadata columnMetadata) {}

@FunctionalInterface
interface TriFunction<T, U, V, R> {

    R apply(T t, U u, V v);

    default <K> TriFunction<T, U, V, K> andThen(Function<? super R, ? extends K> after) {
        Objects.requireNonNull(after);
        return (T t, U u, V v) -> after.apply(apply(t, u, v));
    }
}

public class CovtConverter {
    /* 14 bits -> 8192 in two directions */
    private static final int NUM_COORDINATES_PER_QUADRANT = 8192;
    private static final String ID_COLUMN_NAME = "id";
    private static final String GEOMETRY_COLUMN_NAME = "geometry";

    private static final List<String> LOCALIZED_COLUM_NAME_PREFIXES = Arrays.asList("name");
    private static final Set<String> RLE_ENCODED_INT_LAYER_NAMES = new HashSet<>(List.of("building"));
    private static final Set<String> DELTA_ENCODED_ID_LAYER_NAMES = new HashSet<>(List.of("building", "poi", "place"));
    private static final Set<String>  ICE_ENCODED_LAYER_NAMES = new HashSet<>(List.of("transportation", "boundary"));
    private static final Set<String>  LOCALIZE_DELIMITER = new HashSet<>(List.of(":", "_"));
    private static String PRESENT_STREAM_NAME = "present";
    private static String DATA_STREAM_NAME = "data";
    private static String LENGTH_STREAM_NAME = "length";
    public static final String DICTIONARY_STREAM_NAME = "dictionary";
    private static String GEOMETRY_TYPES_STREAM_NAME = "geometry_types";

    public static byte[] convertMvtTile(List<Layer> layers, boolean useIce) throws IOException {
        try(ByteArrayOutputStream stream = new ByteArrayOutputStream()){
            var header = convertHeader(layers);
            stream.write(header);

            for(var layer : layers){
                var layerName = layer.name();
                var features = layer.features();
                var propertyColumnMetadata = getPropertyColumnMetadata(features);

                var idColumEncoding = DELTA_ENCODED_ID_LAYER_NAMES.contains(layerName) ?
                        ColumnEncoding.DELTA_VARINT : ColumnEncoding.RLE;
                var idColumn = convertIdColumn(features, idColumEncoding);
                //TODO: id and geometry has to be the first columns in the metadata
                var idMetadata = new ColumnMetadata(ColumnDataType.UINT_64, idColumEncoding,
                        new LinkedHashMap<>(Map.of(DATA_STREAM_NAME ,
                                new StreamMetadata(features.size(), idColumn.length))));

                var geometryColumnData = ICE_ENCODED_LAYER_NAMES.contains(layerName) && useIce ?
                        convertIceCodedGeometryColumn(features) :
                        convertUnorderedGeometryColumn(features);
                var geometryColumn = geometryColumnData.geometryColumn();
                var geometryMetadata = geometryColumnData.columnMetadata();

                var propertyColumnData = convertPropertyColumns(features, propertyColumnMetadata);
                var propertyMetadata = propertyColumnData.metadata();
                var propertyColumns = propertyColumnData.propertyColumns();

                var layerMetadata = convertLayerMetadata(layerName, idMetadata, geometryMetadata, propertyMetadata);

                stream.write(layerMetadata);
                stream.write(idColumn);
                stream.write(geometryColumn);
                stream.write(propertyColumns);
            }

            return stream.toByteArray();
        }
    }

    private static byte[] convertLayerMetadata(String layerName, ColumnMetadata idMetadata, ColumnMetadata geometryMetadata,
                                           PropertyColumnsMetadata propertyColumnData) throws IOException {
        /*
        * -> LayerHeader -> LayerName, NumFeatures, NumColumns, ColumnMetadata[]
        * -> ColumnMetadata -> ColumnName, DataType, ColumnEncoding, numStreams, GeometryMetadata | PrimitiveTypeMetadata | StringDictionaryMetadata | LocalizedStringDictionaryMetadata
        * -> StreamMetadata -> StreamName, NumValues, ByteLength
        * -> IdMetadata -> PrimitiveTypeMetadata -> StreamMetadata[1]
        * -> GeometryMetadata -> StreamMetadata[max 6],
        * -> PrimitiveTypeMetadata -> StreamMetadata[2]
        * -> StringDictionaryMetadata -> StreamMetadata[4]
        * -> LocalizedStringDictionaryMetadata -> StreamMetadata[n]
        * */
        var numFeatures = geometryMetadata.streams().get(GEOMETRY_TYPES_STREAM_NAME).numValues();
        var numColumns = propertyColumnData.booleanMetadata().size() + propertyColumnData.longMetadata().size() +
                propertyColumnData.stringDictionaryMetadata().size() + propertyColumnData.localizedStringDictionaryMetadata().size() + 2;
        var metadata = ArrayUtils.addAll(EncodingUtils.encodeString(layerName), EncodingUtils.encodeVarints(new long[]{numFeatures, numColumns}));

        metadata = ArrayUtils.addAll(metadata, EncodingUtils.encodeString(ID_COLUMN_NAME));
        metadata  = ArrayUtils.addAll(metadata, (byte)ColumnDataType.UINT_64.ordinal());
        metadata = ArrayUtils.addAll(metadata, (byte) idMetadata.columnEncoding().ordinal());
        metadata = ArrayUtils.addAll(metadata, EncodingUtils.encodeVarints(new long[]{1}));
        metadata = addStreamMetadata(metadata, DATA_STREAM_NAME,  idMetadata.streams().get(DATA_STREAM_NAME));

        var geometryStreams = geometryMetadata.streams();
        metadata = ArrayUtils.addAll(metadata, EncodingUtils.encodeString(GEOMETRY_COLUMN_NAME));
        metadata  = ArrayUtils.addAll(metadata, (byte) ColumnDataType.GEOMETRY.ordinal());
        metadata = ArrayUtils.addAll(metadata, (byte) geometryMetadata.columnEncoding().ordinal());
        metadata = ArrayUtils.addAll(metadata, EncodingUtils.encodeVarints(new long[]{geometryStreams.size()}));
        for(var geometryStream : geometryStreams.entrySet()){
            metadata = addStreamMetadata(metadata, geometryStream.getKey(), geometryStream.getValue());
        }

        metadata = addNamedColumnMetadata(metadata, propertyColumnData.booleanMetadata());
        metadata = addNamedColumnMetadata(metadata, propertyColumnData.longMetadata());
        metadata = addNamedColumnMetadata(metadata, propertyColumnData.stringDictionaryMetadata());
        return addNamedColumnMetadata(metadata, propertyColumnData.localizedStringDictionaryMetadata());
    }

    private static byte[] addNamedColumnMetadata(byte[] metadata, List<NamedColumnMetadata> namedMetadata) throws IOException {
        for(var column : namedMetadata){
            var columnMetadata = column.columnMetadata();
            metadata = ArrayUtils.addAll(metadata, EncodingUtils.encodeString(column.columnName()));
            metadata = ArrayUtils.addAll(metadata, (byte)columnMetadata.columnDataType().ordinal());
            metadata = ArrayUtils.addAll(metadata, (byte)columnMetadata.columnEncoding().ordinal());
            metadata = ArrayUtils.addAll(metadata, EncodingUtils.encodeVarints(new long[]{columnMetadata.streams().size()}));

            for(var stream : columnMetadata.streams().entrySet()){
                metadata = addStreamMetadata(metadata, stream.getKey(), stream.getValue());
            }
        }

        return metadata;
    }

    private static byte[] addStreamMetadata(byte[] metadata, String streamName, StreamMetadata streamMetadata) throws IOException {
        metadata = ArrayUtils.addAll(metadata, EncodingUtils.encodeString(streamName));
        metadata = ArrayUtils.addAll(metadata, EncodingUtils.encodeVarints(new long[]{streamMetadata.numValues()}));
        return ArrayUtils.addAll(metadata, EncodingUtils.encodeVarints(new long[]{streamMetadata.byteLength()}));
    }

    private static byte[] convertHeader(List<Layer> layers) {
        /*
         * -> Version: UInt32 (Varint)
         * -> Num Layers: UInt32 (Varint)
         * */
        var version = 1;
        var numLayers = layers.size();

        return EncodingUtils.encodeVarints(new long[]{version, numLayers});
    }

    private static LinkedHashMap<String, ColumnMetadata> getPropertyColumnMetadata(List<Feature> features){
        /*
         * Layer header:
         * -> Metadata -> Name (String) | NumFeatures (UInt32 | Varint) | NumColumns (UInt32 | Varint) | Struct Column Metadata []
         * -> Column Metadata Struct -> Column name (String), data type (Byte), encoding (Byte), Struct Stream Metadata []
         * -> Stream Metadata Struct -> name (String), numValues (UInt32 | Varint), byteLength (UInt32 | Varint)
         * -> Streams per column type
         *   -> id  -> no streams specified
         *   -> geometry -> up to 6 streams -> geometryTypes, geometryOffsets, partOffsets, ringOffsets, vertexOffsets, vertexBuffer
         *   -> properties
         *       -> boolean -> present and data stream
         *       -> int -> present and data stream
         *       -> string
         *           -> plain
         *           -> dictionary -> present, data, length, dictionary
         *           -> localized dictionary -> present, en, present, ge, ...., length, dictionary
         * */
        var columnMetadata = new LinkedHashMap<String, ColumnMetadata>();
        for(var feature : features){
            var properties = feature.properties();
            for(var property : properties.entrySet()){
                var columnName = property.getKey();
                if(columnMetadata.containsKey(columnName)){
                    continue;
                }

                var propertyValue = property.getValue();
                if(propertyValue instanceof String){
                    if(LOCALIZED_COLUM_NAME_PREFIXES.stream().anyMatch(prefix -> columnName.contains(prefix))){
                        /* a feature with localized properties has to use : or _ as seperator*/
                        var columnNameComponents = columnName.split(":|_");
                        var baseColumnName = columnNameComponents[0];
                        var streamName = columnNameComponents.length > 1 ? columnNameComponents[1] : columnName;
                        var localizedColumn = columnMetadata.get(baseColumnName);
                        /* Used as a placeholder since can only be filled after the compression of the columns */
                        var emptyStreamData = new StreamMetadata(0,0);
                        if(localizedColumn == null){
                            var metadata = new ColumnMetadata(ColumnDataType.STRING,
                                    ColumnEncoding.LOCALIZED_DICTIONARY, new LinkedHashMap<>(Map.of(streamName, emptyStreamData)));
                            columnMetadata.put(baseColumnName, metadata);
                        }
                        else{
                            if(!localizedColumn.streams().containsKey(streamName)){
                                localizedColumn.streams().put(streamName, emptyStreamData);
                            }
                        }
                    }
                    else{
                        var metadata = new ColumnMetadata(ColumnDataType.STRING, ColumnEncoding.DICTIONARY, new LinkedHashMap<>());
                        columnMetadata.put(columnName, metadata);
                    }
                }
                else if(propertyValue instanceof Boolean){
                    var metadata = new ColumnMetadata(ColumnDataType.BOOLEAN, ColumnEncoding.BOOLEAN_RLE, new LinkedHashMap<>());
                    columnMetadata.put(columnName, metadata);
                }
                //TODO: also handle unsigned int and long to avoid zigZag coding
                else if(propertyValue instanceof Integer || propertyValue instanceof  Long){
                    var encoding = RLE_ENCODED_INT_LAYER_NAMES.contains(columnName) ? ColumnEncoding.RLE : ColumnEncoding.VARINT;
                    var metadata = new ColumnMetadata(ColumnDataType.INT_64, encoding, new LinkedHashMap<>());
                    columnMetadata.put(columnName, metadata);
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

    private static byte[] convertIdColumn(List<Feature> features, ColumnEncoding encoding) throws IOException {
        //TODO: also sort if Rle encoding is used to ensure ascending order -> in transportation layer
        // this is a problem as 1,1,2,2,.... when sorted -> we need 1,2,1,2
        if(ColumnEncoding.DELTA_VARINT.equals(encoding)){
            features.sort(Comparator.comparing(p -> ((Long) p.id())));
        }

        var ids = features.stream().map(feature -> feature.id()).mapToLong(i -> i).toArray();

        if(ColumnEncoding.PLAIN.equals(encoding)){
            return EncodingUtils.encodeVarints(ids);
        }
        if(ColumnEncoding.DELTA_VARINT.equals(encoding)){
            return EncodingUtils.encodeDeltaVarints(ids);
        }
        if(ColumnEncoding.RLE.equals(encoding)){
            return EncodingUtils.encodeRle(ids, false);
        }

        throw new IllegalArgumentException("The specified encoding for the id column is not supported.");
    }

    private static GeometryColumData convertUnorderedGeometryColumn(List<Feature> features) throws IOException {
        var geometryTypes = new ArrayList<Integer>();
        var partOffsets = new ArrayList<Integer>();
        var ringOffsets = new ArrayList<Integer>();
        var geometryOffsets = new ArrayList<Integer>();
        var vertexBuffer = new byte[0];
        var numVertices = 0;
        for(var feature : features){
            var geometryType = feature.geometry().getGeometryType();

            /*
             * Depending on the geometry type the topology column has the following streams:
             * - Point: no stream
             * - LineString: Part offsets
             * - Polygon: Part offsets (Polygon), Ring offsets (LinearRing)
             * - MultiPoint: Geometry offsets -> array of offsets indicate where the vertices of each MultiPoint start
             * - MultiLineString: Geometry offsets, Part offsets (LineString)
             * - MultiPolygon -> Geometry offsets, Part offsets (Polygon), Ring offsets (LinearRing)
             * */
            if(geometryType.equals("Point")){
                geometryTypes.add(GeometryType.POINT.ordinal());
                var point = (Point)feature.geometry();
                var pointBuffer = EncodingUtils.encodeZigZagVarints(new int[]{(int)point.getX(), (int)point.getY()});
                vertexBuffer = ArrayUtils.addAll(vertexBuffer, pointBuffer);
                numVertices++;
            }
            else if(geometryType.equals("MultiPoint")) {
                throw new IllegalArgumentException("MultiPoint currently not supported.");
            }
            else if(geometryType.equals("LineString")){
                geometryTypes.add(GeometryType.LINESTRING.ordinal());
                var lineString = (LineString) feature.geometry();
                partOffsets.add(lineString.getCoordinates().length);
                var deltaVertices = deltaCodeLineString(lineString);
                var varintVertices = EncodingUtils.encodeZigZagVarints(deltaVertices);
                vertexBuffer = ArrayUtils.addAll(vertexBuffer, varintVertices);
                numVertices+= lineString.getCoordinates().length;
            }
            else if(geometryType.equals("MultiLineString")){
                geometryTypes.add( GeometryType.MULTILINESTRING.ordinal());
                var multiLineString = ((MultiLineString)feature.geometry());
                var numLineStrings = multiLineString.getNumGeometries();
                geometryOffsets.add(numLineStrings);
                for(var i = 0; i < numLineStrings; i++){
                    var lineString =  (LineString)multiLineString.getGeometryN(i);
                    partOffsets.add(lineString.getCoordinates().length);
                    var deltaVertices = deltaCodeLineString(lineString);
                    var varintVertices = EncodingUtils.encodeZigZagVarints(deltaVertices);
                    vertexBuffer = ArrayUtils.addAll(vertexBuffer, varintVertices);
                    numVertices += (deltaVertices.length / 2);
                }
            }
            else if(geometryType.equals("Polygon")){
                geometryTypes.add( GeometryType.POLYGON.ordinal());
                var polygon = (Polygon)feature.geometry();
                var deltaVertices = deltaCodePolygon(polygon, partOffsets, ringOffsets);
                var varintVertices = EncodingUtils.encodeZigZagVarints(deltaVertices);
                vertexBuffer = ArrayUtils.addAll(vertexBuffer, varintVertices);
                numVertices += (deltaVertices.length / 2);
            }
            else if(geometryType.equals("MultiPolygon")){
                geometryTypes.add( GeometryType.MULTIPOLYGON.ordinal());
                var multiPolygon = ((MultiPolygon) feature.geometry());
                var numPolygons = multiPolygon.getNumGeometries();
                geometryOffsets.add(numPolygons);
                for(var i = 0; i < numPolygons; i++){
                    var polygon = (Polygon)multiPolygon.getGeometryN(i);
                    var deltaVertices = deltaCodePolygon(polygon, partOffsets, ringOffsets);
                    var varintVertices = EncodingUtils.encodeZigZagVarints(deltaVertices);
                    vertexBuffer = ArrayUtils.addAll(vertexBuffer, varintVertices);
                    numVertices += (deltaVertices.length / 2);
                }
            }
            else{
                throw new IllegalArgumentException("Geometry type not supported.");
            }
        }

        var columnMetadata = new ColumnMetadata(ColumnDataType.GEOMETRY, ColumnEncoding.PLAIN, new LinkedHashMap<>());
        var geometryColumn = convertTopologyStreams(geometryTypes, geometryOffsets, partOffsets, ringOffsets, columnMetadata);
        geometryColumn = ArrayUtils.addAll(geometryColumn, vertexBuffer);
        columnMetadata.streams().put("vertex_buffer", new StreamMetadata(numVertices, vertexBuffer.length));

        return new GeometryColumData(columnMetadata, geometryColumn);
    }

    private static GeometryColumData convertIceCodedGeometryColumn(List<Feature> features) throws IOException {
        SmallHilbertCurve hilbertCurve =
                HilbertCurve.small().bits(14).dimensions(2);
        var vertexDictionary = createVertexDictionary(features, hilbertCurve);

        var geometryTypes = new ArrayList<Integer>();
        var partOffsets = new ArrayList<Integer>();
        var ringOffsets = new ArrayList<Integer>();
        var geometryOffsets = new ArrayList<Integer>();
        var vertexOffsets = new ArrayList<Integer>();
        for(var feature : features){
            var geometryType = feature.geometry().getGeometryType();
            if(geometryType.equals("LineString")){
                geometryTypes.add(GeometryType.LINESTRING.ordinal());
                var lineString = (LineString) feature.geometry();
                partOffsets.add(lineString.getCoordinates().length);
                var offsets = getVertexOffsets(lineString, vertexDictionary, hilbertCurve);
                vertexOffsets.addAll(offsets);
            }
            else if(geometryType.equals("MultiLineString")){
                geometryTypes.add( GeometryType.MULTILINESTRING.ordinal());
                var multiLineString = ((MultiLineString)feature.geometry());
                var numLineStrings = multiLineString.getNumGeometries();
                geometryOffsets.add(numLineStrings);
                for(var i = 0; i < numLineStrings; i++){
                    var lineString =  (LineString)multiLineString.getGeometryN(i);
                    partOffsets.add(lineString.getCoordinates().length);
                    var offsets = getVertexOffsets(lineString, vertexDictionary, hilbertCurve);
                    vertexOffsets.addAll(offsets);
                }
            }
            else{
                throw new IllegalArgumentException("Geometry type not supported.");
            }
        }

        var columnMetadata = new ColumnMetadata(ColumnDataType.GEOMETRY, ColumnEncoding.INDEXED_COORDINATE_ENCODING, new LinkedHashMap<>());
        var geometryColumn = convertTopologyStreams(geometryTypes, geometryOffsets, partOffsets, ringOffsets, columnMetadata);

        var longVertexOffsets = vertexOffsets.stream().mapToLong(i -> i).toArray();
        var vertexOffsetsStream = EncodingUtils.encodeDeltaVarints(longVertexOffsets);
        //var vertexOffsetsStreamDeltaRle = EncodingUtils.encodeDeltaRle(longVertexOffsets);
        //var vertexOffsetsStream = EncodingUtils.encodeDeltaFastPfor128(longVertexOffsets);
        /*System.out.println("vertexOffsets FastPfor Delta: " + vertexOffsetsStreamDeltaFastPfor.length / 1000d + " Delta Rle: "
                + vertexOffsetsStreamDeltaRle.length / 1000d + " Varint Delta: " + vertexOffsetsStream.length / 1000d);*/

        geometryColumn = ArrayUtils.addAll(geometryColumn, vertexOffsetsStream);
        var vertexBuffer = encodeVertexDictionary(vertexDictionary);
        geometryColumn = ArrayUtils.addAll(geometryColumn, vertexBuffer);
        columnMetadata.streams().put("vertex_offsets", new StreamMetadata(vertexOffsets.size(), vertexOffsetsStream.length));
        columnMetadata.streams().put("vertex_buffer", new StreamMetadata(vertexDictionary.size(), vertexBuffer.length));

        return new GeometryColumData(columnMetadata, geometryColumn);
    }

    private static TreeMap<Integer, Vertex> createVertexDictionary(List<Feature> features, SmallHilbertCurve hilbertCurve){
        var vertexDictionary = new TreeMap<Integer, Vertex>();
        for(var feature : features){
            var geometry = feature.geometry();
            var coordinates = geometry.getCoordinates();
            for(var coordinate : coordinates){
                var vertex = new Vertex((int)coordinate.x, (int)coordinate.y);
                var hilbertIndex = getHilbertIndex(hilbertCurve, vertex);
                vertexDictionary.put(hilbertIndex, vertex);
            }
        }

        return vertexDictionary;
    }

    private static byte [] convertTopologyStreams(List<Integer> geometryTypes, List<Integer> geometryOffsets, List<Integer> partOffsets,
                            List<Integer> ringOffsets, ColumnMetadata columnMetadata) throws IOException {
        var streams = columnMetadata.streams();

        var geometryTypeStream = EncodingUtils.encodeByteRle(
                ArrayUtils.toPrimitive(geometryTypes.stream().map(g -> g.byteValue()).toArray(Byte[]::new)));
        streams.put(GEOMETRY_TYPES_STREAM_NAME, new StreamMetadata(geometryTypes.size(), geometryTypeStream.length));
        var geometryColumn = geometryTypeStream;

        if(geometryOffsets.size() > 0){
            var longGeometryOffsets  = geometryOffsets.stream().mapToLong(i -> i).toArray();
            //var geometryOffsetsStreamRle = EncodingUtils.encodeDeltaFastPfor128(longGeometryOffsets);
            var geometryOffsetsStreamRle = EncodingUtils.encodeRle(longGeometryOffsets, false);
            /*System.out.println("geometry Offsets FastPfor Delta: " + geometryOffsetsStreamDeltaFastPfor.length / 1000d + " Rle: "
                    + geometryOffsetsStreamRle.length / 1000d);*/

            streams.put("geometry_offsets", new StreamMetadata(geometryOffsets.size(), geometryOffsetsStreamRle.length));
            geometryColumn = ArrayUtils.addAll(geometryColumn, geometryOffsetsStreamRle);
        }

        if(partOffsets.size() > 0){
            var longPartOffsets  = partOffsets.stream().mapToLong(i -> i).toArray();
            //var partOffsetsStreamRle = EncodingUtils.encodeDeltaFastPfor128(longPartOffsets);
            var partOffsetsStreamRle = EncodingUtils.encodeRle(longPartOffsets, false);
            /*System.out.println("partOffsets FastPfor Delta: " + partOffsetsStreamDeltaFastPfor.length / 1000d + " Rle: "
                    + partOffsetsStreamRle.length / 1000d);*/

            streams.put("part_offsets", new StreamMetadata(partOffsets.size(), partOffsetsStreamRle.length));
            geometryColumn = ArrayUtils.addAll(geometryColumn, partOffsetsStreamRle);
        }

        if(ringOffsets.size() > 0){
            var longRingOffsets = ringOffsets.stream().mapToLong(i -> i).toArray();
            //var ringOffsetsStreamRle = EncodingUtils.encodeDeltaFastPfor128(longRingOffsets);
            var ringOffsetsStreamRle = EncodingUtils.encodeRle(longRingOffsets, false);
            /*System.out.println("ringOffsets FastPfor Delta: " + ringOffsetsStreamDeltaFastPfor.length / 1000d + " Rle: "
                    + ringOffsetsStreamRle.length / 1000d);*/

            streams.put("ring_offsets", new StreamMetadata(ringOffsets.size(), ringOffsetsStreamRle.length));
            geometryColumn = ArrayUtils.addAll(geometryColumn, ringOffsetsStreamRle);
        }

        return geometryColumn;
    }

    private static byte[] encodeVertexDictionary(Map<Integer, Vertex> vertexDictionary){
        var previousX = 0;
        var previousY = 0;
        var vertices = new int[vertexDictionary.size() * 2];
        var i = 0;
        for(var vertex : vertexDictionary.values()){
            var x = vertex.x() - previousX;
            var y = vertex.y() - previousY;
            vertices[i++] = x;
            vertices[i++] = y;

            previousX = vertex.x();
            previousY = vertex.y();
        }
        //EncodingUtils.encodeDeltaFastPfor128()
        return EncodingUtils.encodeZigZagVarints(vertices);
        //return EncodingUtils.encodeZigZagFastPfor128(vertices);
    }

    private static List<Integer> getVertexOffsets(LineString lineString, TreeMap<Integer, Vertex> vertexDictionary, SmallHilbertCurve hilbertCurve){
        return Arrays.stream(lineString.getCoordinates()).map(coordinate -> {
            var vertex = new Vertex((int)coordinate.x, (int)coordinate.y);
            var hilbertIndex = getHilbertIndex(hilbertCurve, vertex);
            return Iterables.indexOf(vertexDictionary.entrySet(), v -> v.getKey().equals(hilbertIndex));
        }).collect(Collectors.toList());
    }

    private static int getHilbertIndex(SmallHilbertCurve curve, Vertex vertex){
        var shiftedX = NUM_COORDINATES_PER_QUADRANT + vertex.x();
        var shiftedY = NUM_COORDINATES_PER_QUADRANT + vertex.y();
        return (int) curve.index(shiftedX, shiftedY);
    }

    private static int[] deltaCodePolygon(Polygon polygon, List<Integer> partOffsets, List<Integer> ringOffsets){
        var vertexBuffer = new int[0];
        var numRings = polygon.getNumInteriorRing() + 1;
        partOffsets.add(numRings);

        var exteriorRing = polygon.getExteriorRing();
        var shell = new GeometryFactory().createLineString(Arrays.copyOf(exteriorRing.getCoordinates(),
                        exteriorRing.getCoordinates().length - 1));
        var shellVertices = deltaCodeLineString(shell);
        vertexBuffer = ArrayUtils.addAll(vertexBuffer, shellVertices);
        ringOffsets.add(shell.getNumPoints());

        for(var i = 0; i < polygon.getNumInteriorRing(); i++){
            var interiorRing = polygon.getInteriorRingN(i);
            var ring = new GeometryFactory().createLineString(Arrays.copyOf(interiorRing.getCoordinates(),
                            interiorRing.getCoordinates().length - 1));

            var ringVertices = deltaCodeLineString(ring);
            vertexBuffer = ArrayUtils.addAll(vertexBuffer, ringVertices);
            ringOffsets.add(ring.getNumPoints());
        }

        return vertexBuffer;
    }

    private static int[] deltaCodeLineString(LineString lineString){
        var vertices = lineString.getCoordinates();
        var previousVertexX = 0;
        var previousVertexY = 0;
        var deltaVertices = new int[vertices.length * 2];
        var i = 0;
        for (var vertex : vertices) {
            var deltaX = (int) vertex.x - previousVertexX;
            var deltaY = (int) vertex.y - previousVertexY;
            deltaVertices[i++] = deltaX;
            deltaVertices[i++] = deltaY;

            previousVertexX = (int)vertex.x;
            previousVertexY = (int)vertex.y;
        }

        return deltaVertices;
    }

    private static PropertyColumData convertPropertyColumns(List<Feature> features, LinkedHashMap<String, ColumnMetadata> columnMetadata) throws IOException {
        var booleanColumns = new HashMap<String, BooleanColumnData>();
        var longColumns = new HashMap<String, LongColumnData>();
        var stringDictionaryColumns = new HashMap<String, StringDictionaryColumnData>();
        var stringLocalizedDictionaryColumns = new HashMap<String, StringLocalizedDictionaryColumnData>();

        /*
        * -> List of streams per column
        *   -> boolean -> present, data (boolean) -> boolean columns
        *   -> int -> present, data (int) -> int columns
        *   -> string -> present, data (string), length, dictionary
        *   -> string localized ->
        *   -> boolean -> rle encode present, boolean rle encode data
        *       -> Map<columnName, booleanColumnData> -> ColumnMetadata, PresentStream, DataStream
        *   -> int
        *       -> Map<columnName, intColumnData>
        *   -> String Dictionary
        *       -> Map<columnName, stringDictionaryColumnData>
        *        -> stringDictionaryColumnData -> ColumnMetadata, PresentStream, LengthStream, DictionaryStream
        *   -> String Localized Dictionary
        *       -> Map<columName, stringLocalizedDictionaryColumnData>
                -> ColumnMetadata, Map<LocalizedStreamData>
        *       ->
        *
        * -> Encode
        *   -> String Dictionary -> order present, data, length, dictionary
        *       -> encode PresentStream, LengthStream
        *       -> convert DictionaryStream to byte [] and encode
        *       -> getDataStream from column and encode
        *   -> Localized String Dictionary
        *
        * */
        for(var metadataSet : columnMetadata.entrySet()){
            var columnName = metadataSet.getKey();
            var metadata = metadataSet.getValue();
            var columnDataType = metadata.columnDataType();

            switch (columnDataType){
                case BOOLEAN:
                    var booleanColumn = convertPropertyColumn(columnName, metadata, features,
                            (ColumnMetadata m, List<Boolean> p, List<Boolean> d) -> new BooleanColumnData(m, p, d));
                    booleanColumns.put(columnName, booleanColumn);
                    break;
                case INT_64:
                case UINT_64:
                    var longColumn = convertPropertyColumn(columnName, metadata, features,
                            (ColumnMetadata m, List<Boolean> p, List<Long> d) -> new LongColumnData(m, p, d));
                    longColumns.put(columnName, longColumn);
                    break;
                case STRING:
                    if(ColumnEncoding.LOCALIZED_DICTIONARY.equals(metadata.columnEncoding())) {
                        var stringLocalizedDictionaryColumn =
                                convertLocalizedStringDictionaryColumn(columnName, metadata, features);
                        stringLocalizedDictionaryColumns.put(columnName, stringLocalizedDictionaryColumn);
                    }
                    else{
                        var stringDictionaryColumn = convertStringDictionaryColumn(columnName, metadata, features);
                        stringDictionaryColumns.put(columnName, stringDictionaryColumn);
                    }
                    break;
                default:
                    throw new IllegalArgumentException("Column data type currently not supported.");
            }
        }

        var columnBuffer = new byte[0];
        if(booleanColumns.size() > 0){
            for(var column : booleanColumns.entrySet()){
                var booleanColumn = column.getValue();
                var presentStream = booleanColumn.presentStream();
                var dataStream = booleanColumn.dataStream();

                var encodedPresentStream = EncodingUtils.encodeBooleans(presentStream);
                var encodedData = EncodingUtils.encodeBooleans(dataStream);
                columnBuffer = ArrayUtils.addAll(columnBuffer, encodedPresentStream);
                columnBuffer = ArrayUtils.addAll(columnBuffer, encodedData);

                var metadata = booleanColumn.columnMetadata();
                metadata.streams().put(PRESENT_STREAM_NAME, new StreamMetadata(presentStream.size(), encodedPresentStream.length));
                metadata.streams().put(DATA_STREAM_NAME, new StreamMetadata(dataStream.size(), encodedData.length));
            }
        }

        if(longColumns.size() > 0){
            for(var column : longColumns.entrySet()){
                var longColumn = column.getValue();
                var presentStream = longColumn.presentStream();
                var dataStream = longColumn.dataStream();
                var metadata = longColumn.columnMetadata();

                var encodedPresentStream = EncodingUtils.encodeBooleans(presentStream);
                columnBuffer = ArrayUtils.addAll(columnBuffer, encodedPresentStream);

                var data = dataStream.stream().mapToLong(i -> i).toArray();
                //TODO: depending on the datatype is signed true or false
                var encodedData = ColumnEncoding.RLE.equals(metadata.columnEncoding()) ?
                        EncodingUtils.encodeRle(data, true) : EncodingUtils.encodeZigZagVarints(data);
                columnBuffer = ArrayUtils.addAll(columnBuffer, encodedData);

                metadata.streams().put(PRESENT_STREAM_NAME, new StreamMetadata(presentStream.size(), encodedPresentStream.length));
                metadata.streams().put(DATA_STREAM_NAME, new StreamMetadata(dataStream.size(), encodedData.length));
            }
        }

        if(stringDictionaryColumns.size() > 0){
            for(var column : stringDictionaryColumns.entrySet()){
                var stringDictionaryColumn = column.getValue();
                var presentStream = stringDictionaryColumn.presentStream();
                var dataStream = stringDictionaryColumn.dataStream();
                var lengthStream = stringDictionaryColumn.lengthStream();
                var dictionaryStream = stringDictionaryColumn.dictionaryStream();
                var metadata = stringDictionaryColumn.columnMetadata();

                var encodedPresentStream = EncodingUtils.encodeBooleans(presentStream);
                columnBuffer = ArrayUtils.addAll(columnBuffer, encodedPresentStream);

                var encodedDataStream = EncodingUtils.encodeRle(dataStream.stream().mapToLong(i -> i).toArray(), false);
                var encodedLengthStream = EncodingUtils.encodeRle(lengthStream.stream().mapToLong(i -> i).toArray(), false);
                var encodedDictionary = concatByteArrays(dictionaryStream.stream().
                        map(s -> s.getBytes(StandardCharsets.UTF_8)).collect(Collectors.toList()));

                columnBuffer = ArrayUtils.addAll(columnBuffer, encodedDataStream);
                columnBuffer = ArrayUtils.addAll(columnBuffer, encodedLengthStream);
                columnBuffer = ArrayUtils.addAll(columnBuffer, encodedDictionary);

                var streams = metadata.streams();
                streams.put(PRESENT_STREAM_NAME, new StreamMetadata(presentStream.size(), encodedPresentStream.length));
                streams.put(DATA_STREAM_NAME, new StreamMetadata(dataStream.size(), encodedDataStream.length));
                streams.put(LENGTH_STREAM_NAME, new StreamMetadata(dictionaryStream.size(), encodedLengthStream.length));
                streams.put(DICTIONARY_STREAM_NAME, new StreamMetadata(dictionaryStream.size(), encodedDictionary.length));
            }
        }

        /*if(stringLocalizedDictionaryColumns.size() > 0){
            for(var column : stringLocalizedDictionaryColumns.entrySet()){
                var columnData = column.getValue();
                columnData.columnMetadata().streams().clear();
                for(var stream : columnData.streamData().entrySet()){
                    var streamName = stream.getKey();
                    var streamData = stream.getValue();
                    var presentStream = streamData.presentStream();
                    var dataStream = streamData.dataStream();

                    var encodedPresentStream = EncodingUtils.encodeBooleans(presentStream);
                    columnBuffer = ArrayUtils.addAll(columnBuffer, encodedPresentStream);
                    var encodedDataStream = EncodingUtils.encodeRle(dataStream.stream().mapToLong(i -> i).toArray(), false);
                    columnBuffer = ArrayUtils.addAll(columnBuffer, encodedDataStream);

                    var streams = columnData.columnMetadata().streams();
                    streams.put(PRESENT_STREAM_NAME + "_" + streamName, new StreamMetadata(presentStream.size(), encodedPresentStream.length));
                    streams.put(streamName, new StreamMetadata(dataStream.size(), encodedDataStream.length));
                }

                var encodedLengthStream = EncodingUtils.encodeRle(columnData.lengthStream().stream().mapToLong(i -> i).toArray(), false);
                var encodedDictionary = concatByteArrays(columnData.dictionaryStream().stream().
                        map(s -> s.getBytes(StandardCharsets.UTF_8)).collect(Collectors.toList()));
                columnBuffer = ArrayUtils.addAll(columnBuffer, encodedLengthStream);
                columnBuffer = ArrayUtils.addAll(columnBuffer, encodedDictionary);

                var streams = columnData.columnMetadata().streams();
                var numValues = columnData.dictionaryStream().size();
                //TODO: quick and dirty -> leads to collisions when length or dictionary are feature properties
                streams.put(LENGTH_STREAM_NAME, new StreamMetadata(numValues, encodedLengthStream.length));
                streams.put(DICTIONARY_STREAM_NAME, new StreamMetadata(numValues, encodedDictionary.length));
            }
        }*/

        if(stringLocalizedDictionaryColumns.size() > 0){
            for(var column : stringLocalizedDictionaryColumns.entrySet()){
                var columnData = column.getValue();
                columnData.columnMetadata().streams().clear();

                /*var encodedLengthStream = EncodingUtils.encodeRle(columnData.lengthStream().stream().mapToLong(i -> i).toArray(), false);
                var encodedDictionary = concatByteArrays(columnData.dictionaryStream().stream().
                        map(s -> s.getBytes(StandardCharsets.UTF_8)).collect(Collectors.toList()));
                columnBuffer = ArrayUtils.addAll(columnBuffer, encodedLengthStream);
                columnBuffer = ArrayUtils.addAll(columnBuffer, encodedDictionary);

                var streams = columnData.columnMetadata().streams();
                var numValues = columnData.dictionaryStream().size();
                //TODO: quick and dirty -> leads to collisions when length or dictionary are feature properties
                streams.put(LENGTH_STREAM_NAME, new StreamMetadata(numValues, encodedLengthStream.length));
                streams.put(DICTIONARY_STREAM_NAME, new StreamMetadata(numValues, encodedDictionary.length));*/

                var streams = columnData.columnMetadata().streams();
                for(var stream : columnData.streamData().entrySet()){
                    var streamName = stream.getKey();
                    var streamData = stream.getValue();
                    var presentStream = streamData.presentStream();
                    var dataStream = streamData.dataStream();

                    var encodedPresentStream = EncodingUtils.encodeBooleans(presentStream);
                    columnBuffer = ArrayUtils.addAll(columnBuffer, encodedPresentStream);
                    var encodedDataStream = EncodingUtils.encodeRle(dataStream.stream().mapToLong(i -> i).toArray(), false);
                    columnBuffer = ArrayUtils.addAll(columnBuffer, encodedDataStream);

                    streams.put(PRESENT_STREAM_NAME + "_" + streamName, new StreamMetadata(presentStream.size(), encodedPresentStream.length));
                    streams.put(streamName, new StreamMetadata(dataStream.size(), encodedDataStream.length));
                }

                var encodedLengthStream = EncodingUtils.encodeRle(columnData.lengthStream().stream().mapToLong(i -> i).toArray(), false);
                var encodedDictionary = concatByteArrays(columnData.dictionaryStream().stream().
                        map(s -> s.getBytes(StandardCharsets.UTF_8)).collect(Collectors.toList()));
                columnBuffer = ArrayUtils.addAll(columnBuffer, encodedLengthStream);
                columnBuffer = ArrayUtils.addAll(columnBuffer, encodedDictionary);
                var numValues = columnData.dictionaryStream().size();
                //TODO: quick and dirty -> leads to collisions when length or dictionary are feature properties
                streams.put(LENGTH_STREAM_NAME, new StreamMetadata(numValues, encodedLengthStream.length));
                streams.put(DICTIONARY_STREAM_NAME, new StreamMetadata(numValues, encodedDictionary.length));
            }
        }

        var booleanColumnMetadata = booleanColumns.entrySet().stream().map(c -> new NamedColumnMetadata(c.getKey(),
                c.getValue().columnMetadata())).collect(Collectors.toList());
        var longColumnMetadata = longColumns.entrySet().stream().map(c -> new NamedColumnMetadata(c.getKey(),
                c.getValue().columnMetadata())).collect(Collectors.toList());
        var stringDictionaryColumnMetadata = stringDictionaryColumns.entrySet().stream().map(c -> new NamedColumnMetadata(c.getKey(),
                c.getValue().columnMetadata())).collect(Collectors.toList());
        var localizedStringDictionaryColumnMetadata = stringLocalizedDictionaryColumns.entrySet().stream().map(c -> new NamedColumnMetadata(c.getKey(),
                c.getValue().columnMetadata())).collect(Collectors.toList());
        var columnsMetadata = new PropertyColumnsMetadata(booleanColumnMetadata, longColumnMetadata, stringDictionaryColumnMetadata, localizedStringDictionaryColumnMetadata);

        return new PropertyColumData(columnsMetadata, columnBuffer);
    }

    private static StringLocalizedDictionaryColumnData convertLocalizedStringDictionaryColumn(String columnName, ColumnMetadata metadata, List<Feature> features){
        var streamData = new HashMap<String, LocalizedStringDictionaryStreamData>();
        var lengthStream = new ArrayList<Integer>();
        var dictionaryStream = new ArrayList<String>();
        for(var streamSet : metadata.streams().entrySet()){
            var streamName = streamSet.getKey();
            var presentStream = new ArrayList<Boolean>();
            var dataStream = new ArrayList<Integer>();

            for(var feature : features) {
                var properties = feature.properties();

                /* The default property has no suffix */
                if(!streamName.equals(columnName)){
                    var isPropertyPresent = Boolean.valueOf(false);
                    var propertyName = "";
                    for(var delimiter : LOCALIZE_DELIMITER){
                        propertyName = columnName + delimiter + streamName;
                        if(properties.containsKey(propertyName)){
                            isPropertyPresent = true;
                        }
                        else{
                            continue;
                        }

                        var value = properties.get(propertyName);
                        var stringValue = (String)value;
                        if(!dictionaryStream.contains((stringValue))){
                            var utf8EncodedData = stringValue.getBytes(StandardCharsets.UTF_8);
                            lengthStream.add(utf8EncodedData.length);
                            dictionaryStream.add(stringValue);

                            var index = dictionaryStream.size() - 1;
                            dataStream.add(index);
                        }
                        else{
                            var index = dictionaryStream.indexOf(stringValue);
                            dataStream.add(index);
                        }
                        break;
                    }
                    presentStream.add(isPropertyPresent);
                }
                else{
                    var isPropertyPresent = properties.containsKey(columnName);
                    presentStream.add(isPropertyPresent);

                    if(isPropertyPresent){
                        var stringValue = (String)properties.get(columnName);
                        if(!dictionaryStream.contains((stringValue))){
                            var utf8EncodedData = stringValue.getBytes(StandardCharsets.UTF_8);
                            lengthStream.add(utf8EncodedData.length);
                            dictionaryStream.add(stringValue);
                            var index = dictionaryStream.size() - 1;
                            dataStream.add(index);
                        }
                        else{
                            var index = dictionaryStream.indexOf(stringValue);
                            dataStream.add(index);
                        }
                    }
                }
            }

           streamData.put(streamName, new LocalizedStringDictionaryStreamData(presentStream, dataStream));
        }

        return new StringLocalizedDictionaryColumnData(metadata, lengthStream, dictionaryStream, streamData);
    }

    private static StringDictionaryColumnData convertStringDictionaryColumn(String columnName, ColumnMetadata metadata, List<Feature> features){
        var presentStream = new ArrayList<Boolean>();
        var dataStream = new ArrayList<Integer>();
        var lengthStream = new ArrayList<Integer>();
        var dictionaryStream = new ArrayList<String>();
        for(var feature : features){
            var properties = feature.properties();
            var propertyValue = properties.get(columnName);

            if(!properties.containsKey(columnName)){
                presentStream.add(false);
                continue;
            }

            presentStream.add(true);
            var stringValue = (String)propertyValue;
            if(!dictionaryStream.contains(stringValue)){
                dictionaryStream.add(stringValue);
                var utf8EncodedData = stringValue.getBytes(StandardCharsets.UTF_8);
                lengthStream.add(utf8EncodedData.length);
                var index = dictionaryStream.size() - 1;
                dataStream.add(index);
            }
            else{
                var index = dictionaryStream.indexOf(stringValue);
                dataStream.add(index);
            }
        }

        return new StringDictionaryColumnData(metadata, presentStream, dataStream, lengthStream, dictionaryStream);
    }

    private static <T, U> U convertPropertyColumn(String columnName, ColumnMetadata metadata, List<Feature> features,
                                                  TriFunction<ColumnMetadata, List<Boolean>, List<T>, U> columnDataFunc){
        var presentStream = new ArrayList<Boolean>();
        var dataStream = new ArrayList<T>();
        for(var feature : features){
            var properties = feature.properties();
            var propertyValue = properties.get(columnName);

            if(!properties.containsKey(columnName)){
                presentStream.add(false);
                continue;
            }

            presentStream.add(true);
            dataStream.add((T)propertyValue);
        }

        return columnDataFunc.apply(metadata, presentStream, dataStream);
    }

    private static byte[] concatByteArrays(List<byte[]> values){
        var buffer = new byte[0];
        for(var value : values){
            buffer = ArrayUtils.addAll(buffer, value);
        }
        return buffer;
    }
}
