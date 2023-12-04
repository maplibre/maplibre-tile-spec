package com.covt.converter;

import com.covt.converter.geometry.GeometryType;
import com.covt.converter.geometry.Vertex;
import com.covt.converter.mvt.Feature;
import com.covt.converter.mvt.Layer;
import com.google.common.collect.Iterables;
import org.apache.commons.lang3.ArrayUtils;
import org.apache.commons.lang3.tuple.Pair;
import org.davidmoten.hilbert.HilbertCurve;
import org.locationtech.jts.geom.GeometryFactory;
import org.locationtech.jts.geom.LineString;
import org.locationtech.jts.geom.MultiLineString;
import org.locationtech.jts.geom.MultiPolygon;
import org.locationtech.jts.geom.Point;
import org.locationtech.jts.geom.Polygon;
import org.locationtech.jts.geom.LinearRing;

import java.io.ByteArrayOutputStream;
import java.io.IOException;
import java.nio.charset.StandardCharsets;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.Comparator;
import java.util.HashMap;
import java.util.HashSet;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;
import java.util.Objects;
import java.util.Set;
import java.util.TreeMap;
import java.util.function.Function;
import java.util.stream.Collectors;
import java.util.stream.IntStream;

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
record PrimitiveColumnData<T>(ColumnMetadata columnMetadata, List<Boolean> presentStream, List<T> dataStream){}

record StringDictionaryColumnData(ColumnMetadata columnMetadata, List<Boolean> presentStream, List<Integer> dataStream,
                                  List<Integer> lengthStream, List<String> dictionaryStream){}

record StringLocalizedDictionaryColumnData(ColumnMetadata columnMetadata, List<Integer> lengthStream,
                                           List<String> dictionaryStream, Map<String, LocalizedStringDictionaryStreamData> streamData){}

record LocalizedStringDictionaryStreamData(List<Boolean> presentStream, List<Integer> dataStream){}

record PropertyColumnsMetadata(List<NamedColumnMetadata> booleanMetadata, List<NamedColumnMetadata> longMetadata,
                               List<NamedColumnMetadata> floatMetadata,
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

    public enum GeometryEncoding{
        PLAIN,
        PLAIN_MORTON,
        /* use a dictionary for the coordinates*/
        ICE,
        /* use a vertex dictionary and transform the 2D coordinates to morton codes */
        ICE_MORTON
    }

    private static final String ID_COLUMN_NAME = "id";
    private static final String GEOMETRY_COLUMN_NAME = "geometry";
    private static final List<String> LOCALIZED_COLUM_NAME_PREFIXES = Arrays.asList("name");
    private static final Set<String>  LOCALIZE_DELIMITER = new HashSet<>(List.of(":", "_"));
    private static String PRESENT_STREAM_NAME = "present";
    private static String DATA_STREAM_NAME = "data";
    private static String LENGTH_STREAM_NAME = "length";
    public static final String DICTIONARY_STREAM_NAME = "dictionary";
    private static String GEOMETRY_TYPES_STREAM_NAME = "geometry_types";

    //TODO: use a extent per layer instead of tile
    public static byte[] convertMvtTile(List<Layer> layers, int tileExtent, GeometryEncoding geometryEncoding,
                                        boolean allowFastPforForTopologyStreams,
                                        boolean allowFastPforForVertexBuffer,
                                        boolean allowLocalizedStringDictionary,
                                        boolean includeIds
                                        ) throws IOException {
        try(ByteArrayOutputStream stream = new ByteArrayOutputStream()){
            var header = convertHeader(layers);
            stream.write(header);

            for(var layer : layers){
                var layerName = layer.name();
                var features = layer.features();
                var propertyColumnMetadata = getPropertyColumnMetadata(features, allowLocalizedStringDictionary);

                ColumnMetadata idMetadata = null;
                byte[] idColumn = null;
                if(includeIds == true) {
                    var allowReordering = false;
                    var idColumnData = convertIdColumn(features, allowReordering);
                    idColumn = idColumnData.getRight();
                    //TODO: id and geometry has to be the first columns in the metadata
                    idMetadata = new ColumnMetadata(ColumnDataType.UINT_64, ColumnEncoding.PLAIN,
                            new LinkedHashMap<>(Map.of(
                                    DATA_STREAM_NAME,
                                    new StreamMetadata(features.size(), idColumn.length, idColumnData.getLeft())
                            )));
                }

                //TODO: if features are not sorted based on id sort the geometry
                //for example part_offsets when ICE is used or point geometries without ICE
                var allowIceEncodig = geometryEncoding == GeometryEncoding.ICE ||
                        geometryEncoding == GeometryEncoding.ICE_MORTON;
                GeometryColumData geometryColumnData;
                var unorderedGeometryColumnData = convertUnorderedGeometryColumn(features,
                        allowFastPforForTopologyStreams, allowFastPforForVertexBuffer);
                if(!allowIceEncodig){
                    geometryColumnData = unorderedGeometryColumnData;
                }
                else{
                    var iceCodedGeometryColumnData = convertIceCodedGeometryColumn(features, tileExtent,
                            geometryEncoding, allowFastPforForTopologyStreams, allowFastPforForVertexBuffer);
                    geometryColumnData = iceCodedGeometryColumnData.geometryColumn().length < unorderedGeometryColumnData.geometryColumn().length?
                            iceCodedGeometryColumnData : unorderedGeometryColumnData;
                }

                //System.out.println(layerName + " geometry size: " + geometryColumnData.geometryColumn().length / 1024d);
                var geometryColumn = geometryColumnData.geometryColumn();
                var geometryMetadata = geometryColumnData.columnMetadata();

                var propertyColumnData = convertPropertyColumns(features, propertyColumnMetadata);
                var propertyMetadata = propertyColumnData.metadata();
                var propertyColumns = propertyColumnData.propertyColumns();
                //System.out.println(layerName + " property size: " + propertyColumnData.propertyColumns().length / 1024d);

                var layerMetadata = convertLayerMetadata(layerName, idMetadata, geometryMetadata, propertyMetadata, tileExtent);

                stream.write(layerMetadata);
                if(includeIds == true){
                    stream.write(idColumn);
                }
                stream.write(geometryColumn);
                stream.write(propertyColumns);
            }

            return stream.toByteArray();
        }
    }

    private static byte[] convertLayerMetadata(String layerName, ColumnMetadata idMetadata, ColumnMetadata geometryMetadata,
                                           PropertyColumnsMetadata propertyColumnData, int layerExtent) throws IOException {
        /*
        * -> LayerHeader -> LayerName, NumFeatures, NumColumns, ColumnMetadata[]
        * -> ColumnMetadata -> ColumnName, DataType, ColumnEncoding, numStreams, GeometryMetadata | PrimitiveTypeMetadata | StringDictionaryMetadata | LocalizedStringDictionaryMetadata
        * -> StreamMetadata -> StreamName, NumValues, ByteLength, StreamEncoding
        * -> IdMetadata -> PrimitiveTypeMetadata -> StreamMetadata[1]
        * -> GeometryMetadata -> StreamMetadata[max 6],
        * -> PrimitiveTypeMetadata -> StreamMetadata[2]
        * -> StringDictionaryMetadata -> StreamMetadata[4]
        * -> LocalizedStringDictionaryMetadata -> StreamMetadata[n]
        * */
        var numFeatures = geometryMetadata.streams().get(GEOMETRY_TYPES_STREAM_NAME).numValues();
        var numColumns = propertyColumnData.booleanMetadata().size() + propertyColumnData.longMetadata().size() +
                propertyColumnData.floatMetadata().size() +
                propertyColumnData.stringDictionaryMetadata().size() + propertyColumnData.localizedStringDictionaryMetadata().size() +
                (idMetadata != null ? 2 : 1) ;
        /* layerName, layerExtent, numFeatures, numColumns */
        var metadata = ArrayUtils.addAll(EncodingUtils.encodeString(layerName),
                EncodingUtils.encodeVarints(new long[]{layerExtent, numFeatures, numColumns}, false, false));

        if(idMetadata != null){
            metadata = ArrayUtils.addAll(metadata, EncodingUtils.encodeString(ID_COLUMN_NAME));
            metadata  = ArrayUtils.addAll(metadata, (byte)ColumnDataType.UINT_64.ordinal());
            metadata = ArrayUtils.addAll(metadata, (byte) idMetadata.columnEncoding().ordinal());
            metadata = ArrayUtils.addAll(metadata, EncodingUtils.encodeVarints(new long[]{1}, false, false));
            metadata = addStreamMetadata(metadata, DATA_STREAM_NAME,  idMetadata.streams().get(DATA_STREAM_NAME));
        }

        var geometryStreams = geometryMetadata.streams();
        metadata = ArrayUtils.addAll(metadata, EncodingUtils.encodeString(GEOMETRY_COLUMN_NAME));
        metadata  = ArrayUtils.addAll(metadata, (byte) ColumnDataType.GEOMETRY.ordinal());
        metadata = ArrayUtils.addAll(metadata, (byte) geometryMetadata.columnEncoding().ordinal());
        metadata = ArrayUtils.addAll(metadata, EncodingUtils.encodeVarints(new long[]{geometryStreams.size()}, false, false));
        for(var geometryStream : geometryStreams.entrySet()){
            metadata = addStreamMetadata(metadata, geometryStream.getKey(), geometryStream.getValue());
        }

        metadata = addNamedColumnMetadata(metadata, propertyColumnData.booleanMetadata());
        metadata = addNamedColumnMetadata(metadata, propertyColumnData.longMetadata());
        metadata = addNamedColumnMetadata(metadata, propertyColumnData.floatMetadata());
        metadata = addNamedColumnMetadata(metadata, propertyColumnData.stringDictionaryMetadata());
        return addNamedColumnMetadata(metadata, propertyColumnData.localizedStringDictionaryMetadata());
    }

    private static byte[] addNamedColumnMetadata(byte[] metadata, List<NamedColumnMetadata> namedMetadata) throws IOException {
        for(var column : namedMetadata){
            var columnMetadata = column.columnMetadata();
            metadata = ArrayUtils.addAll(metadata, EncodingUtils.encodeString(column.columnName()));
            metadata = ArrayUtils.addAll(metadata, (byte)columnMetadata.columnDataType().ordinal());
            metadata = ArrayUtils.addAll(metadata, (byte)columnMetadata.columnEncoding().ordinal());
            metadata = ArrayUtils.addAll(metadata, EncodingUtils.encodeVarints(new long[]{columnMetadata.streams().size()}, false, false));

            for(var stream : columnMetadata.streams().entrySet()){
                metadata = addStreamMetadata(metadata, stream.getKey(), stream.getValue());
            }
        }

        return metadata;
    }

    private static byte[] addStreamMetadata(byte[] metadata, String streamName, StreamMetadata streamMetadata) throws IOException {
        metadata = ArrayUtils.addAll(metadata, EncodingUtils.encodeString(streamName));
        metadata = ArrayUtils.addAll(metadata, EncodingUtils.encodeVarints(new long[]{streamMetadata.numValues()}, false, false));
        metadata = ArrayUtils.addAll(metadata, EncodingUtils.encodeVarints(new long[]{streamMetadata.byteLength()}, false, false));
        return ArrayUtils.addAll(metadata, EncodingUtils.encodeVarints(new long[]{streamMetadata.streamEncoding().ordinal()}, false, false));
    }

    private static byte[] convertHeader(List<Layer> layers) {
        /*
         * -> Version: UInt32 (Varint)
         * -> Num Layers: UInt32 (Varint)
         * */
        var version = 1;
        var numLayers = layers.size();

        return EncodingUtils.encodeVarints(new long[]{version, numLayers}, false, false);
    }

    private static LinkedHashMap<String, ColumnMetadata> getPropertyColumnMetadata(List<Feature> features, boolean allowLocalizedStringDictionary){
        /*
         * Layer header:
         * -> Metadata -> Name (String) | NumFeatures (UInt32 | Varint) | NumColumns (UInt32 | Varint) | Struct Column Metadata []
         * -> Column Metadata Struct -> Column name (String), data type (Byte), encoding (Byte), Struct Stream Metadata []
         * -> Stream Metadata Struct -> name (String), numValues (UInt32 | Varint), byteLength (UInt32 | Varint), encoding (UInt32, Varint)
         * -> Streams per column type
         *   -> id -> data stream -> no present stream because when id colum is present the values are of the column are not optional
         *   -> geometry -> up to 6 streams -> geometryTypes, geometryOffsets, partOffsets, ringOffsets, vertexOffsets, vertexBuffer
         *   -> properties
         *       -> boolean -> present and data stream
         *       -> int -> present and data stream
         *       -> string
         *           -> plain -> present, length, data
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
                    if(LOCALIZED_COLUM_NAME_PREFIXES.stream().anyMatch(prefix -> columnName.contains(prefix)) && allowLocalizedStringDictionary){
                        /* a feature with localized properties has to use : or _ as separator*/
                        var columnNameComponents = Arrays.stream(columnName.split(":|_"))
                                .filter(e -> !e.isEmpty()).toArray(String[]::new);
                        var baseColumnName = columnNameComponents[0];
                        var streamName = columnNameComponents.length > 1 ? columnNameComponents[1] : columnName;
                        var localizedColumn = columnMetadata.get(baseColumnName);
                        /* Used as a placeholder since can only be filled after the compression of the columns */
                        var emptyStreamData = new StreamMetadata(0,0, StreamEncoding.PLAIN);
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
                    var metadata = new ColumnMetadata(ColumnDataType.BOOLEAN, ColumnEncoding.PLAIN, new LinkedHashMap<>());
                    columnMetadata.put(columnName, metadata);
                }
                //TODO: also handle unsigned int and long to avoid zigZag coding
                else if(propertyValue instanceof Integer || propertyValue instanceof  Long){
                    var metadata = new ColumnMetadata(ColumnDataType.INT_64, ColumnEncoding.PLAIN, new LinkedHashMap<>());
                    columnMetadata.put(columnName, metadata);
                }
                else if(propertyValue instanceof Float){
                    var metadata = new ColumnMetadata(ColumnDataType.FLOAT, ColumnEncoding.PLAIN, new LinkedHashMap<>());
                    columnMetadata.put(columnName, metadata);
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

    private static Pair<StreamEncoding, byte[]> convertIdColumn(List<Feature> features, boolean allowReordering) throws IOException {
        var ids = features.stream().map(feature -> feature.id()).mapToLong(i -> i).toArray();

        var varintEncodedIds = EncodingUtils.encodeVarints(ids, false, false);
        //TODO: also sort if Rle encoding is used to ensure ascending order -> in transportation layer
        // this is a problem as 1,1,2,2,.... when sorted -> we need 1,2,1,2
        var rleEncodedIds = EncodingUtils.encodeRle(ids, false);;
        if(allowReordering){
            features.sort(Comparator.comparing(p -> p.id()));
        }
        var deltaVarintEncodedIds = EncodingUtils.encodeVarints(ids, true, true);

        if(rleEncodedIds.length < varintEncodedIds.length && rleEncodedIds.length < deltaVarintEncodedIds.length){
            return Pair.of(StreamEncoding.RLE, rleEncodedIds);
        }
        else if(deltaVarintEncodedIds.length < varintEncodedIds.length){
            return Pair.of(StreamEncoding.VARINT_DELTA_ZIG_ZAG, rleEncodedIds);
        }

        return Pair.of(StreamEncoding.VARINT, varintEncodedIds);
    }

    private static GeometryColumData convertUnorderedGeometryColumn(List<Feature> features,
                                                                    boolean allowFastPforForTopologyStreams,
                                                                    boolean allowFastPforForVertexBuffer) throws IOException {
        var geometryTypes = new ArrayList<Integer>();
        var partOffsets = new ArrayList<Integer>();
        var ringOffsets = new ArrayList<Integer>();
        var geometryOffsets = new ArrayList<Integer>();
        var vertexBuffer = new ArrayList<Integer>();
        //TODO: if not sorted after id sort the geometries -> points based on HilbertCurve
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
                vertexBuffer.add((int)point.getX());
                vertexBuffer.add((int)point.getY());
            }
            else if(geometryType.equals("MultiPoint")) {
                throw new IllegalArgumentException("MultiPoint currently not supported.");
            }
            else if(geometryType.equals("LineString")){
                geometryTypes.add(GeometryType.LINESTRING.ordinal());
                var lineString = (LineString) feature.geometry();
                partOffsets.add(lineString.getCoordinates().length);
                vertexBuffer.addAll(flatLineString(lineString));
            }
            else if(geometryType.equals("MultiLineString")){
                geometryTypes.add( GeometryType.MULTILINESTRING.ordinal());
                var multiLineString = ((MultiLineString)feature.geometry());
                var numLineStrings = multiLineString.getNumGeometries();
                geometryOffsets.add(numLineStrings);
                for(var i = 0; i < numLineStrings; i++){
                    var lineString =  (LineString)multiLineString.getGeometryN(i);
                    partOffsets.add(lineString.getCoordinates().length);
                    var vertices = flatLineString(lineString);
                    vertexBuffer.addAll(vertices);
                }
            }
            else if(geometryType.equals("Polygon")){
                geometryTypes.add( GeometryType.POLYGON.ordinal());
                var polygon = (Polygon)feature.geometry();
                var vertices = flatPolygon(polygon, partOffsets, ringOffsets);
                vertexBuffer.addAll(vertices);
            }
            else if(geometryType.equals("MultiPolygon")){
                geometryTypes.add( GeometryType.MULTIPOLYGON.ordinal());
                var multiPolygon = ((MultiPolygon) feature.geometry());
                var numPolygons = multiPolygon.getNumGeometries();
                geometryOffsets.add(numPolygons);
                for(var i = 0; i < numPolygons; i++){
                    var polygon = (Polygon)multiPolygon.getGeometryN(i);
                    var vertices = flatPolygon(polygon, partOffsets, ringOffsets);
                    vertexBuffer.addAll(vertices);
                }
            }
            else{
                throw new IllegalArgumentException("Geometry type not supported.");
            }
        }

        var numVertices = vertexBuffer.size();
        var columnMetadata = new ColumnMetadata(ColumnDataType.GEOMETRY, ColumnEncoding.PLAIN, new LinkedHashMap<>());
        var geometryColumn = convertTopologyStreams(geometryTypes, geometryOffsets, partOffsets, ringOffsets,
                columnMetadata, allowFastPforForTopologyStreams);

        var zigZagDeltaCodedVertexBuffer = EncodingUtils.encodeZigZagDeltaCoordinates(vertexBuffer);
        var varintZigZagDeltaVertexBuffer = EncodingUtils.encodeVarints(Arrays.stream(zigZagDeltaCodedVertexBuffer).mapToLong(v -> v).toArray(),
                false, false);
        if(!allowFastPforForVertexBuffer){
            columnMetadata.streams().put("vertex_buffer", new StreamMetadata(numVertices, varintZigZagDeltaVertexBuffer.length,
                    StreamEncoding.VARINT_DELTA_ZIG_ZAG));
            geometryColumn = ArrayUtils.addAll(geometryColumn, varintZigZagDeltaVertexBuffer);
            return new GeometryColumData(columnMetadata, geometryColumn);
        }

        var fastPforZigZagDeltaVertexBuffer = EncodingUtils.encodeFastPfor128(zigZagDeltaCodedVertexBuffer, false, false);
        if(fastPforZigZagDeltaVertexBuffer.length <= varintZigZagDeltaVertexBuffer.length){
            columnMetadata.streams().put("vertex_buffer", new StreamMetadata(numVertices, fastPforZigZagDeltaVertexBuffer.length,
                    StreamEncoding.FAST_PFOR_DELTA_ZIG_ZAG));
            geometryColumn = ArrayUtils.addAll(geometryColumn, fastPforZigZagDeltaVertexBuffer);
            return new GeometryColumData(columnMetadata, geometryColumn);
        }
        else{
            columnMetadata.streams().put("vertex_buffer", new StreamMetadata(numVertices, varintZigZagDeltaVertexBuffer.length,
                    StreamEncoding.VARINT_DELTA_ZIG_ZAG));
            geometryColumn = ArrayUtils.addAll(geometryColumn, varintZigZagDeltaVertexBuffer);
            return new GeometryColumData(columnMetadata, geometryColumn);
        }
    }

    private static GeometryColumData convertIceCodedGeometryColumn(List<Feature> features, int tileExtent, GeometryEncoding geometryEncoding,
                                                                   boolean allowFastPforForTopologyStreams,
                                                                   boolean allowFastPfor) throws IOException {
        if(tileExtent != 2<<11 && tileExtent != 2<<12){
            throw new RuntimeException("The specified tile extent is not (yet) supported.");
        }
        var numBits = tileExtent == 2<<11 ? 13 : 14;
        var hilbertCurve = HilbertCurve.small().bits(numBits).dimensions(2);
        Function<Vertex, Integer> sfcIdGenerator = geometryEncoding == GeometryEncoding.ICE_MORTON ?
                    vertex -> GeometryUtils.encodeMorton(vertex.x(), vertex.y(), numBits):
                    vertex -> GeometryUtils.encodeHilbertIndex(hilbertCurve, vertex);
        var vertexDictionary = createVertexDictionary(features, sfcIdGenerator);

        var geometryTypes = new ArrayList<Integer>();
        var partOffsets = new ArrayList<Integer>();
        var ringOffsets = new ArrayList<Integer>();
        var geometryOffsets = new ArrayList<Integer>();
        var vertexOffsets = new ArrayList<Integer>();
        for(var feature : features){
            var geometryType = feature.geometry().getGeometryType();
            //TODO: verify if this is working
            if(geometryType.equals("Point")){
                geometryTypes.add(GeometryType.POINT.ordinal());
                var point = (Point) feature.geometry();
                var vertex = new Vertex((int)point.getX(), (int)point.getY());
                var sfcId = sfcIdGenerator.apply(vertex);
                var offset = Iterables.indexOf(vertexDictionary.entrySet(), v -> v.getKey().equals(sfcId));
                vertexOffsets.add(offset);
            }
            else if(geometryType.equals("LineString")){
                geometryTypes.add(GeometryType.LINESTRING.ordinal());
                var lineString = (LineString) feature.geometry();
                partOffsets.add(lineString.getCoordinates().length);
                var offsets = getVertexOffsets(lineString, vertexDictionary, sfcIdGenerator);
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
                    var offsets = getVertexOffsets(lineString, vertexDictionary, sfcIdGenerator);
                    vertexOffsets.addAll(offsets);
                }
            }
            //TODO: verify polygon and multipolygon
            else if(geometryType.equals("Polygon")){
                geometryTypes.add( GeometryType.POLYGON.ordinal());
                var polygon = ((Polygon)feature.geometry());
                var numRings = polygon.getNumInteriorRing() + 1;
                partOffsets.add(numRings);
                for(var i = 0; i < numRings; i++){
                    LinearRing linearRing = i == 0 ? polygon.getExteriorRing() : polygon.getInteriorRingN(i-1);
                    ringOffsets.add(linearRing.getCoordinates().length);
                    var offsets = getVertexOffsets(linearRing, vertexDictionary, sfcIdGenerator);
                    vertexOffsets.addAll(offsets);
                }

            }
            else if(geometryType.equals("MultiPolygon")){
                geometryTypes.add( GeometryType.MULTIPOLYGON.ordinal());
                var multiPolygon = ((MultiPolygon)feature.geometry());
                var numPolygons = multiPolygon.getNumGeometries();
                geometryOffsets.add(numPolygons);
                for(var i = 0; i < numPolygons; i++){
                    var polygon = (Polygon)multiPolygon.getGeometryN(i);
                    var numRings = polygon.getNumInteriorRing() + 1;
                    partOffsets.add(numRings);
                    for(var j = 0; j < numRings; j++){
                        LinearRing linearRing = j == 0 ? polygon.getExteriorRing() : polygon.getInteriorRingN(j-1);
                        ringOffsets.add(linearRing.getCoordinates().length);
                        var offsets = getVertexOffsets(linearRing, vertexDictionary, sfcIdGenerator);
                        vertexOffsets.addAll(offsets);
                    }
                }
            }
            else{
                throw new IllegalArgumentException(String.format("Geometry type %s not supported.", geometryType));
            }
        }

        var allowMortonEncoding = geometryEncoding == GeometryEncoding.ICE_MORTON;
        var vertexData = encodeVertexBuffer(vertexDictionary, vertexOffsets, allowFastPfor,
                allowMortonEncoding, sfcIdGenerator);
        var columnMetadata = vertexData.getLeft();
        var geometryColumn = convertTopologyStreams(geometryTypes, geometryOffsets, partOffsets, ringOffsets,
                columnMetadata, allowFastPforForTopologyStreams);
        geometryColumn = ArrayUtils.addAll(geometryColumn, vertexData.getRight());
        return new GeometryColumData(columnMetadata, geometryColumn);
    }

    //TODO: refactor -> remove redundant code
    private static Pair<ColumnMetadata, byte[]> encodeVertexBuffer(TreeMap<Integer, Vertex> vertexDictionary, List<Integer> vertexOffsets,
                                                                   boolean allowFastPforDelta, boolean allowMortonEncoding, Function<Vertex, Integer> sfcIdGenerator){
        var varintDeltaOffsets = EncodingUtils.encodeVarints(vertexOffsets.stream().mapToLong(i -> i).toArray(), true, true);
        var varintDeltaVertexBuffer = encodeVertexDictionary(vertexDictionary, false);
        if(!allowFastPforDelta && !allowMortonEncoding){
            var columnMetadata = new ColumnMetadata(ColumnDataType.GEOMETRY, ColumnEncoding.ICE, new LinkedHashMap<>());
            columnMetadata.streams().put("vertex_offsets", new StreamMetadata(vertexOffsets.size(), varintDeltaOffsets.length,
                    StreamEncoding.VARINT_DELTA_ZIG_ZAG));
            columnMetadata.streams().put("vertex_buffer", new StreamMetadata(vertexDictionary.size(), varintDeltaVertexBuffer.length,
                    StreamEncoding.VARINT_DELTA_ZIG_ZAG));
            return Pair.of(columnMetadata, ArrayUtils.addAll(varintDeltaOffsets, varintDeltaVertexBuffer));
        }

        var fastPforOffsets = EncodingUtils.encodeFastPfor128(vertexOffsets.stream().mapToInt(i -> i).toArray(), true, true);
        var fastPforVertexBuffer = encodeVertexDictionary(vertexDictionary, true);
        if(allowFastPforDelta  && !allowMortonEncoding){
            var columnMetadata = new ColumnMetadata(ColumnDataType.GEOMETRY, ColumnEncoding.ICE, new LinkedHashMap<>());
            columnMetadata.streams().put("vertex_offsets", new StreamMetadata(vertexOffsets.size(), fastPforOffsets.length,
                    StreamEncoding.FAST_PFOR_DELTA_ZIG_ZAG));
            columnMetadata.streams().put("vertex_buffer", new StreamMetadata(vertexDictionary.size(), fastPforVertexBuffer.length,
                    StreamEncoding.FAST_PFOR_DELTA_ZIG_ZAG));
            return Pair.of(columnMetadata, ArrayUtils.addAll(fastPforOffsets, fastPforVertexBuffer));
        }

        var varintDeltaMortonVertexBuffer = encodeVertexDictionaryVarintWithMortonId(vertexDictionary, sfcIdGenerator);
        if(!allowFastPforDelta && allowMortonEncoding){
            var columnMetadata = new ColumnMetadata(ColumnDataType.GEOMETRY, ColumnEncoding.ICE_MORTON_CODE, new LinkedHashMap<>());
            columnMetadata.streams().put("vertex_offsets", new StreamMetadata(vertexOffsets.size(), varintDeltaOffsets.length,
                    StreamEncoding.VARINT_DELTA_ZIG_ZAG));
            columnMetadata.streams().put("vertex_buffer", new StreamMetadata(vertexDictionary.size(), varintDeltaMortonVertexBuffer.length,
                    StreamEncoding.VARINT_DELTA_ZIG_ZAG));
            return Pair.of(columnMetadata, ArrayUtils.addAll(varintDeltaOffsets, varintDeltaMortonVertexBuffer));
        }

        var fastPforDeltaMortonVertexBuffer = encodeVertexDictionaryFastPforWithMortonId(vertexDictionary, sfcIdGenerator);

        StreamEncoding vertexOffsetsEncoding;
        byte[] encodedVertexOffsets;
        if(varintDeltaOffsets.length < fastPforOffsets.length){
            vertexOffsetsEncoding = StreamEncoding.VARINT_DELTA_ZIG_ZAG;
            encodedVertexOffsets = varintDeltaOffsets;
        }
        else{
            vertexOffsetsEncoding = StreamEncoding.FAST_PFOR_DELTA_ZIG_ZAG;
            encodedVertexOffsets = fastPforOffsets;
        }

        var varintDeltaGeometryColumSize = varintDeltaVertexBuffer.length;
        var fastPforDeltaGeometryColumnSize = fastPforVertexBuffer.length;
        var varintDeltaMortonGeometryColumnSize = varintDeltaMortonVertexBuffer.length;
        var fastPforDeltaMortonGeometryColumSize = fastPforDeltaMortonVertexBuffer.length;
        if(varintDeltaGeometryColumSize < fastPforDeltaMortonGeometryColumSize && varintDeltaGeometryColumSize < fastPforDeltaGeometryColumnSize &&
                varintDeltaGeometryColumSize < varintDeltaMortonGeometryColumnSize){
            var columnMetadata = new ColumnMetadata(ColumnDataType.GEOMETRY, ColumnEncoding.ICE, new LinkedHashMap<>());
            columnMetadata.streams().put("vertex_offsets", new StreamMetadata(vertexOffsets.size(),  encodedVertexOffsets.length,
                    vertexOffsetsEncoding));
            columnMetadata.streams().put("vertex_buffer", new StreamMetadata(vertexDictionary.size(), varintDeltaVertexBuffer.length,
                    StreamEncoding.VARINT_DELTA_ZIG_ZAG));
            return Pair.of(columnMetadata, ArrayUtils.addAll(encodedVertexOffsets, varintDeltaVertexBuffer));
        }

        if(fastPforDeltaGeometryColumnSize < varintDeltaGeometryColumSize && fastPforDeltaGeometryColumnSize < varintDeltaMortonGeometryColumnSize &&
            fastPforDeltaGeometryColumnSize < fastPforDeltaMortonGeometryColumSize){
            var columnMetadata = new ColumnMetadata(ColumnDataType.GEOMETRY, ColumnEncoding.ICE, new LinkedHashMap<>());
            columnMetadata.streams().put("vertex_offsets", new StreamMetadata(vertexOffsets.size(), encodedVertexOffsets.length,
                    vertexOffsetsEncoding));
            columnMetadata.streams().put("vertex_buffer", new StreamMetadata(vertexDictionary.size(), fastPforVertexBuffer.length,
                    StreamEncoding.FAST_PFOR_DELTA_ZIG_ZAG));
            return Pair.of(columnMetadata, ArrayUtils.addAll(encodedVertexOffsets, fastPforVertexBuffer));
        }

        if(varintDeltaMortonGeometryColumnSize < varintDeltaGeometryColumSize && varintDeltaMortonGeometryColumnSize < fastPforDeltaGeometryColumnSize
            && varintDeltaMortonGeometryColumnSize < fastPforDeltaMortonGeometryColumSize){
            var columnMetadata = new ColumnMetadata(ColumnDataType.GEOMETRY, ColumnEncoding.ICE_MORTON_CODE, new LinkedHashMap<>());
            columnMetadata.streams().put("vertex_offsets", new StreamMetadata(vertexOffsets.size(),  encodedVertexOffsets.length,
                    vertexOffsetsEncoding));
            columnMetadata.streams().put("vertex_buffer", new StreamMetadata(vertexDictionary.size(), varintDeltaMortonVertexBuffer.length,
                    StreamEncoding.VARINT_DELTA_ZIG_ZAG));
            return Pair.of(columnMetadata, ArrayUtils.addAll(encodedVertexOffsets, varintDeltaMortonVertexBuffer));
        }

        var columnMetadata = new ColumnMetadata(ColumnDataType.GEOMETRY, ColumnEncoding.ICE_MORTON_CODE, new LinkedHashMap<>());
        columnMetadata.streams().put("vertex_offsets", new StreamMetadata(vertexOffsets.size(),  encodedVertexOffsets.length,
                vertexOffsetsEncoding));
        columnMetadata.streams().put("vertex_buffer", new StreamMetadata(vertexDictionary.size(), fastPforDeltaMortonVertexBuffer.length,
                StreamEncoding.FAST_PFOR_DELTA_ZIG_ZAG));
        return Pair.of(columnMetadata, ArrayUtils.addAll(encodedVertexOffsets, fastPforDeltaMortonVertexBuffer));
    }

    private static TreeMap<Integer, Vertex> createVertexDictionary(List<Feature> features, Function<Vertex, Integer> sfcIdGenerator){
        var vertexDictionary = new TreeMap<Integer, Vertex>();
        for(var feature : features){
            var geometry = feature.geometry();
            var coordinates = geometry.getCoordinates();
            for(var coordinate : coordinates){
                var vertex = new Vertex((int)coordinate.x, (int)coordinate.y);
                var sfcId = sfcIdGenerator.apply(vertex);
                vertexDictionary.put(sfcId, vertex);
            }
        }
        return vertexDictionary;
    }

    private static byte [] convertTopologyStreams(List<Integer> geometryTypes, List<Integer> geometryOffsets, List<Integer> partOffsets,
                            List<Integer> ringOffsets, ColumnMetadata columnMetadata, boolean allowFastPforDelta) throws IOException {
        var streams = columnMetadata.streams();

        var geometryTypeStream = EncodingUtils.encodeByteRle(
                ArrayUtils.toPrimitive(geometryTypes.stream().map(g -> g.byteValue()).toArray(Byte[]::new)));
        streams.put(GEOMETRY_TYPES_STREAM_NAME, new StreamMetadata(geometryTypes.size(), geometryTypeStream.length, StreamEncoding.BYTE_RLE));
        var geometryColumn = geometryTypeStream;

        if(geometryOffsets.size() > 0){
            geometryColumn = addOffsets(geometryOffsets, allowFastPforDelta,
                    "geometry_offsets", streams, geometryColumn);
        }

        if(partOffsets.size() > 0){
            geometryColumn = addOffsets(partOffsets, allowFastPforDelta,
                    "part_offsets", streams, geometryColumn);
        }

        if(ringOffsets.size() > 0){
            geometryColumn = addOffsets(ringOffsets, allowFastPforDelta,
                    "ring_offsets", streams, geometryColumn);
        }

        return geometryColumn;
    }

    private static byte[] addOffsets(List<Integer> offsets, Boolean useFastPforDelta, String streamName,
                              LinkedHashMap<String, StreamMetadata> streams, byte[] geometryColumn) throws IOException {
        var rleOffsets= EncodingUtils.encodeRle(offsets.stream().mapToLong(i -> i).toArray(), false);
        if(!useFastPforDelta){
            streams.put(streamName, new StreamMetadata(offsets.size(), rleOffsets.length,
                    StreamEncoding.RLE));
            return ArrayUtils.addAll(geometryColumn, rleOffsets);
        }

        var fastPforDeltaOffsets = EncodingUtils.encodeFastPfor128(
                offsets.stream().mapToInt(i -> i).toArray(), true, true);

        if(fastPforDeltaOffsets.length <= rleOffsets.length){
            streams.put(streamName, new StreamMetadata(offsets.size(), fastPforDeltaOffsets.length,
                    StreamEncoding.FAST_PFOR_DELTA_ZIG_ZAG));
            return ArrayUtils.addAll(geometryColumn, fastPforDeltaOffsets);
        }
        else{
            streams.put(streamName, new StreamMetadata(offsets.size(), rleOffsets.length,
                    StreamEncoding.RLE));
            return ArrayUtils.addAll(geometryColumn, rleOffsets);
        }
    }

    private static byte[] encodeVertexDictionary(Map<Integer, Vertex> vertexDictionary, boolean useFastPfor){
        var vertices = vertexDictionary.values().stream().flatMapToInt(v -> IntStream.of(v.x(), v.y())).boxed().
                collect(Collectors.toList());
        var zigZagDeltaVertices = EncodingUtils.encodeZigZagDeltaCoordinates(vertices);

        var varintEncodedVertexBuffer = EncodingUtils.encodeVarints(Arrays.stream(zigZagDeltaVertices).mapToLong(i -> i).toArray(),
                false, false);
        if(!useFastPfor){
            return varintEncodedVertexBuffer;
        }
        var fastPforEncodedVertexBuffer =  EncodingUtils.encodeFastPfor128(zigZagDeltaVertices, false, false);
        //TODO: add to the stream metadata which encoding is used
        return fastPforEncodedVertexBuffer.length < varintEncodedVertexBuffer.length?
                fastPforEncodedVertexBuffer:
                varintEncodedVertexBuffer;
    }

    private static byte[] encodeVertexDictionaryVarintWithMortonId(Map<Integer, Vertex> vertexDictionary, Function<Vertex, Integer> sfcIdGenerator){
        var mortonCodes = vertexDictionary.entrySet().stream().map(v -> sfcIdGenerator.apply(v.getValue())).mapToLong(i -> i).toArray();
        return EncodingUtils.encodeVarints(mortonCodes, false, true);
    }

    private static byte[] encodeVertexDictionaryFastPforWithMortonId(Map<Integer, Vertex> vertexDictionary, Function<Vertex, Integer> sfcIdGenerator){
        var mortonCodes = vertexDictionary.entrySet().stream().map(v -> sfcIdGenerator.apply(v.getValue())).mapToLong(i -> i).toArray();
        return EncodingUtils.encodeFastPfor128(Arrays.stream(mortonCodes).mapToInt(i -> (int)i).toArray(),
                false, true);
    }

    private static List<Integer> getVertexOffsets(LineString lineString, TreeMap<Integer, Vertex> vertexDictionary, Function<Vertex, Integer> sfcIdGenerator){
        return Arrays.stream(lineString.getCoordinates()).map(coordinate -> {
            var vertex = new Vertex((int)coordinate.x, (int)coordinate.y);
            var sfcIndex = sfcIdGenerator.apply(vertex);
            return Iterables.indexOf(vertexDictionary.entrySet(), v -> v.getKey().equals(sfcIndex));
        }).collect(Collectors.toList());
    }

    private static List<Integer> flatPolygon(Polygon polygon, List<Integer> partOffsets, List<Integer> ringOffsets) {
        var vertexBuffer = new ArrayList<Integer>();
        var numRings = polygon.getNumInteriorRing() + 1;
        partOffsets.add(numRings);

        var exteriorRing = polygon.getExteriorRing();
        var shell = new GeometryFactory().createLineString(Arrays.copyOf(exteriorRing.getCoordinates(),
                exteriorRing.getCoordinates().length - 1));
        var shellVertices = flatLineString(shell);
        vertexBuffer.addAll(shellVertices);
        ringOffsets.add(shell.getNumPoints());

        for (var i = 0; i < polygon.getNumInteriorRing(); i++) {
            var interiorRing = polygon.getInteriorRingN(i);
            var ring = new GeometryFactory().createLineString(Arrays.copyOf(interiorRing.getCoordinates(),
                    interiorRing.getCoordinates().length - 1));

            var ringVertices = flatLineString(ring);
            vertexBuffer.addAll(ringVertices);
            ringOffsets.add(ring.getNumPoints());
        }

        return vertexBuffer;
    }

    private static List<Integer> flatLineString(LineString lineString){
        return Arrays.stream(lineString.getCoordinates()).flatMapToInt(v -> IntStream.of((int)v.x, (int)v.y)).boxed().
                collect(Collectors.toList());
    }

    private static PropertyColumData convertPropertyColumns(List<Feature> features, LinkedHashMap<String, ColumnMetadata> columnMetadata) throws IOException {
        var booleanColumns = new HashMap<String, PrimitiveColumnData<Boolean>>();
        var longColumns = new HashMap<String, PrimitiveColumnData<Long>>();
        var floatColumns = new HashMap<String, PrimitiveColumnData<Float>>();
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
                            (ColumnMetadata m, List<Boolean> p, List<Boolean> d) -> new PrimitiveColumnData(m, p, d));
                    booleanColumns.put(columnName, booleanColumn);
                    break;
                case INT_64:
                case UINT_64:
                    var longColumn = convertPropertyColumn(columnName, metadata, features,
                            (ColumnMetadata m, List<Boolean> p, List<Long> d) -> new PrimitiveColumnData(m, p, d));
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
                case FLOAT:
                    var floatColumn = convertPropertyColumn(columnName, metadata, features,
                            (ColumnMetadata m, List<Boolean> p, List<Float> d) -> new PrimitiveColumnData(m, p, d));
                    floatColumns.put(columnName, floatColumn);
                    break;
                default:
                    throw new IllegalArgumentException("Column data type currently not supported.");
            }
        }

        var columnBuffer = new byte[0];
        if(booleanColumns.size() > 0){
            for(var column : booleanColumns.entrySet()){
                var booleanColumn = column.getValue();
                var dataStream = booleanColumn.dataStream();
                var encodedData = EncodingUtils.encodeBooleans(dataStream);
                columnBuffer = ArrayUtils.addAll(columnBuffer, encodedData);
                var metadata = booleanColumn.columnMetadata();
                metadata.streams().put(DATA_STREAM_NAME, new StreamMetadata(dataStream.size(), encodedData.length, StreamEncoding.BOOLEAN_RLE));
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
                //TODO: add supporrt for UInt64 -> depending on the datatype is signed true or false
                var varintValues = EncodingUtils.encodeVarints(data, true, false);
                var deltaVarintValues = EncodingUtils.encodeVarints(data, true, true);
                var rleValues = EncodingUtils.encodeRle(data, true);

                metadata.streams().put(PRESENT_STREAM_NAME, new StreamMetadata(presentStream.size(), encodedPresentStream.length,
                        StreamEncoding.BOOLEAN_RLE));
                if(rleValues.length < varintValues.length && rleValues.length < deltaVarintValues.length){
                    columnBuffer = ArrayUtils.addAll(columnBuffer, rleValues);
                    metadata.streams().put(DATA_STREAM_NAME, new StreamMetadata(dataStream.size(), rleValues.length,
                            StreamEncoding.RLE));
                }
                else if(deltaVarintValues.length < rleValues.length && deltaVarintValues.length < varintValues.length){
                    columnBuffer = ArrayUtils.addAll(columnBuffer, deltaVarintValues);
                    metadata.streams().put(DATA_STREAM_NAME, new StreamMetadata(dataStream.size(), deltaVarintValues.length,
                            StreamEncoding.VARINT_DELTA_ZIG_ZAG));
                }
                else{
                    columnBuffer = ArrayUtils.addAll(columnBuffer, varintValues);
                    metadata.streams().put(DATA_STREAM_NAME, new StreamMetadata(dataStream.size(), varintValues.length,
                            StreamEncoding.VARINT_ZIG_ZAG));
                }
            }
        }

        if(floatColumns.size() > 0){
            for(var column : floatColumns.entrySet()){
                var floatColumn = column.getValue();
                var presentStream = floatColumn.presentStream();
                var dataStream = floatColumn.dataStream();
                var metadata = floatColumn.columnMetadata();

                var encodedPresentStream = EncodingUtils.encodeBooleans(presentStream);
                columnBuffer = ArrayUtils.addAll(columnBuffer, encodedPresentStream);

                var data = new float[dataStream.size()];
                for(var i = 0; i < dataStream.size(); i++){
                    data[i] = dataStream.get(i);
                }
                var encodedData = EncodingUtils.encodeFloatsLE(data);
                columnBuffer = ArrayUtils.addAll(columnBuffer, encodedData);

                //TODO: add also encoding for floats like XOR-based compression
                metadata.streams().put(PRESENT_STREAM_NAME, new StreamMetadata(presentStream.size(), encodedPresentStream.length, StreamEncoding.PLAIN));
                metadata.streams().put(DATA_STREAM_NAME, new StreamMetadata(dataStream.size(), encodedData.length, StreamEncoding.PLAIN));
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

                //TODO: test different encodings like delta encoding
                var encodedDataStream = EncodingUtils.encodeRle(dataStream.stream().mapToLong(i -> i).toArray(), false);
                var encodedLengthStream = EncodingUtils.encodeRle(lengthStream.stream().mapToLong(i -> i).toArray(), false);
                var encodedDictionary = CollectionUtils.concatByteArrays(dictionaryStream.stream().
                        map(s -> s.getBytes(StandardCharsets.UTF_8)).collect(Collectors.toList()));

                columnBuffer = ArrayUtils.addAll(columnBuffer, encodedDataStream);
                columnBuffer = ArrayUtils.addAll(columnBuffer, encodedLengthStream);
                columnBuffer = ArrayUtils.addAll(columnBuffer, encodedDictionary);

                var streams = metadata.streams();
                streams.put(PRESENT_STREAM_NAME, new StreamMetadata(presentStream.size(), encodedPresentStream.length, StreamEncoding.BOOLEAN_RLE));
                streams.put(DATA_STREAM_NAME, new StreamMetadata(dataStream.size(), encodedDataStream.length, StreamEncoding.RLE));
                streams.put(LENGTH_STREAM_NAME, new StreamMetadata(dictionaryStream.size(), encodedLengthStream.length, StreamEncoding.RLE));
                streams.put(DICTIONARY_STREAM_NAME, new StreamMetadata(dictionaryStream.size(), encodedDictionary.length, StreamEncoding.PLAIN));
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

                    streams.put(PRESENT_STREAM_NAME + "_" + streamName, new StreamMetadata(presentStream.size(), encodedPresentStream.length, StreamEncoding.BOOLEAN_RLE));
                    streams.put(streamName, new StreamMetadata(dataStream.size(), encodedDataStream.length, StreamEncoding.RLE));
                }

                var encodedLengthStream = EncodingUtils.encodeRle(columnData.lengthStream().stream().mapToLong(i -> i).toArray(), false);
                var encodedDictionary = CollectionUtils.concatByteArrays(columnData.dictionaryStream().stream().
                        map(s -> s.getBytes(StandardCharsets.UTF_8)).collect(Collectors.toList()));
                columnBuffer = ArrayUtils.addAll(columnBuffer, encodedLengthStream);
                columnBuffer = ArrayUtils.addAll(columnBuffer, encodedDictionary);
                var numValues = columnData.dictionaryStream().size();
                //TODO: quick and dirty -> leads to collisions when length or dictionary are feature properties
                streams.put(LENGTH_STREAM_NAME, new StreamMetadata(numValues, encodedLengthStream.length, StreamEncoding.RLE));
                streams.put(DICTIONARY_STREAM_NAME, new StreamMetadata(numValues, encodedDictionary.length, StreamEncoding.PLAIN));
            }
        }

        var booleanColumnMetadata = booleanColumns.entrySet().stream().map(c -> new NamedColumnMetadata(c.getKey(),
                c.getValue().columnMetadata())).collect(Collectors.toList());
        var longColumnMetadata = longColumns.entrySet().stream().map(c -> new NamedColumnMetadata(c.getKey(),
                c.getValue().columnMetadata())).collect(Collectors.toList());
        var floatColumnMetadata = floatColumns.entrySet().stream().map(c -> new NamedColumnMetadata(c.getKey(),
                c.getValue().columnMetadata())).collect(Collectors.toList());
        var stringDictionaryColumnMetadata = stringDictionaryColumns.entrySet().stream().map(c -> new NamedColumnMetadata(c.getKey(),
                c.getValue().columnMetadata())).collect(Collectors.toList());
        var localizedStringDictionaryColumnMetadata = stringLocalizedDictionaryColumns.entrySet().stream().map(c -> new NamedColumnMetadata(c.getKey(),
                c.getValue().columnMetadata())).collect(Collectors.toList());
        var columnsMetadata = new PropertyColumnsMetadata(booleanColumnMetadata, longColumnMetadata, floatColumnMetadata, stringDictionaryColumnMetadata, localizedStringDictionaryColumnMetadata);

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

}
