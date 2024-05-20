package com.covt.converter;

import com.covt.converter.geometry.GeometryType;
import com.covt.converter.geometry.Vertex;
import com.covt.converter.mvt.Feature;
import com.covt.converter.mvt.Layer;
import com.covt.converter.tilejson.TileJson;
import com.covt.converter.tilejson.VectorLayer;
import com.fasterxml.jackson.databind.ObjectMapper;
import com.google.common.collect.Iterables;
import org.apache.commons.lang3.ArrayUtils;
import org.apache.commons.lang3.tuple.ImmutablePair;
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
import java.util.*;
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

    private static final byte FILE_VERSION = 1;
    private static final List<String> LOCALIZED_COLUM_NAME_PREFIXES = Arrays.asList("name");
    private static final Set<String>  LOCALIZE_DELIMITER = new HashSet<>(List.of(":", "_"));

    //TODO: use a extent per layer instead of tile
    public static byte[] convertMvtTile(List<Layer> layers, int tileExtent, GeometryEncoding geometryEncoding,
                                        boolean allowFastPforForTopologyStreams,
                                        boolean allowFastPforForVertexBuffer,
                                        boolean allowLocalizedStringDictionary,
                                        boolean includeIds,
                                        boolean optimizeMetadata
    ) throws IOException {
        var vectorLayers = new ArrayList<VectorLayer>();
        try(ByteArrayOutputStream stream = new ByteArrayOutputStream()){
            var totalLayerMetadataSize = 0;
            var layerId = 0;
            for(var layer : layers){
                var features = layer.features();
                var propertyColumnMetadata = getPropertyColumnMetadata(features, allowLocalizedStringDictionary);

                ColumnMetadata idMetadata = null;
                byte[] idColumn = null;
                if(includeIds == true) {
                    var allowReordering = false;
                    /*var idColumnData = convertIdColumn(features, allowReordering);
                    idColumn = idColumnData.getRight();
                    //TODO: id and geometry has to be the first columns in the metadata
                    idMetadata = new ColumnMetadata(ColumnDataType.UINT_64, ColumnType.PLAIN,
                            new LinkedHashMap<>(Map.of(
                                    DATA_STREAM_NAME,
                                    new StreamMetadata(features.size(), idColumn.length, idColumnData.getLeft(), StreamType.DATA)
                            )));*/
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

                var geometryColumn = geometryColumnData.geometryColumn();
                var geometryMetadata = geometryColumnData.columnMetadata();

                var propertyColumnData = convertPropertyColumns(features, propertyColumnMetadata);
                var propertyMetadata = propertyColumnData.metadata();
                var propertyColumns = propertyColumnData.propertyColumns();
                //System.out.println(layerName + " property size: " + propertyColumnData.propertyColumns().length / 1000d);

                byte[] layerMetadata = null;
                if(optimizeMetadata){
                    layerMetadata = convertOptimizedLayerMetadata(layerId++, idMetadata, geometryMetadata, propertyMetadata, tileExtent);

                    var vectorLayer = new VectorLayer();
                    vectorLayer.id = layer.name();
                    vectorLayer.fields = new LinkedHashMap<>();
                    for(var property : propertyMetadata.floatMetadata()){
                        var columnName = property.columnName();
                        var columnMetadata = property.columnMetadata().columnDataType();
                        //TODO: add real data type
                        vectorLayer.fields.put(columnName, "String");
                    }
                    vectorLayers.add(vectorLayer);
                }
                else{
                    throw new RuntimeException("Currently only the optimized metadata mode is supported.");
                }


                //System.out.println(layerName + " metadata size: " + layerMetadata.length / 1000d);
                totalLayerMetadataSize += layerMetadata.length;

                stream.write(layerMetadata);
                if(includeIds == true){
                    stream.write(idColumn);
                }
                stream.write(geometryColumn);
                stream.write(propertyColumns);
            }

            //System.out.println("------------------------------------------------------------");
            //System.out.println("Total Layer Metadata Size: " + totalLayerMetadataSize);
            //System.out.printf("Contribution of metadata to the total file size: %f%%%n", (1d / (stream.toByteArray().length / (double)totalLayerMetadataSize ))  * 100);
            //System.out.println("------------------------------------------------------------");


            var tileJson = new TileJson();
            tileJson.vectorLayers = vectorLayers;
            ObjectMapper mapper = new ObjectMapper();
            var tileJsonStr = mapper.writerWithDefaultPrettyPrinter().writeValueAsString(tileJson);

            return stream.toByteArray();
        }
    }

    public static Pair<String, byte[]> convertMvtTile2(List<Layer> layers, int tileExtent, GeometryEncoding geometryEncoding,
                                        boolean allowFastPforForTopologyStreams,
                                        boolean allowFastPforForVertexBuffer,
                                        boolean allowLocalizedStringDictionary,
                                        boolean includeIds,
                                        boolean optimizeMetadata
    ) throws IOException {
        var vectorLayers = new ArrayList<VectorLayer>();
        try(ByteArrayOutputStream stream = new ByteArrayOutputStream()){
            var totalLayerMetadataSize = 0;
            var layerId = 0;
            for(var layer : layers){
                var features = layer.features();
                var propertyColumnMetadata = getPropertyColumnMetadata(features, allowLocalizedStringDictionary);

                ColumnMetadata idMetadata = null;
                byte[] idColumn = null;
                if(includeIds == true) {
                    var allowReordering = false;
                    throw new RuntimeException("Id columns currently not supported.");
                    /*var idColumnData = convertIdColumn(features, allowReordering);
                    idColumn = idColumnData.getRight();
                    //TODO: id and geometry has to be the first columns in the metadata
                    idMetadata = new ColumnMetadata(ColumnDataType.UINT_64, ColumnType.PLAIN,
                            new LinkedHashMap<>(Map.of(
                                    DATA_STREAM_NAME,
                                    new StreamMetadata(features.size(), idColumn.length, idColumnData.getLeft(), StreamType.DATA)
                            )));*/
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

                var geometryColumn = geometryColumnData.geometryColumn();
                var geometryMetadata = geometryColumnData.columnMetadata();

                var propertyColumnData = convertPropertyColumns(features, propertyColumnMetadata);
                var propertyMetadata = propertyColumnData.metadata();
                var propertyColumns = propertyColumnData.propertyColumns();
                //System.out.println(layerName + " property size: " + propertyColumnData.propertyColumns().length / 1000d);

                byte[] layerMetadata = null;
                if(optimizeMetadata){
                    layerMetadata = convertOptimizedLayerMetadata(layerId++, idMetadata, geometryMetadata, propertyMetadata, tileExtent);

                    var vectorLayer = new VectorLayer();
                    vectorLayer.id = layer.name();
                    vectorLayer.fields = new LinkedHashMap<>();
                    var metadata = new ArrayList<>(propertyMetadata.booleanMetadata());
                    metadata.addAll(propertyMetadata.longMetadata());
                    metadata.addAll(propertyMetadata.floatMetadata());
                    metadata.addAll(propertyMetadata.stringDictionaryMetadata());
                    metadata.addAll(propertyMetadata.localizedStringDictionaryMetadata());
                    for(var property : metadata){
                        var columnName = property.columnName();
                        var columnMetadata = property.columnMetadata().columnDataType();
                        //TODO: add real data type
                        vectorLayer.fields.put(columnName, "String");
                    }
                    vectorLayers.add(vectorLayer);
                }
                else{
                    layerMetadata = convertLayerMetadata(layer.name(), idMetadata, geometryMetadata, propertyMetadata, tileExtent);
                }


                //System.out.println(layerName + " metadata size: " + layerMetadata.length / 1000d);
                totalLayerMetadataSize += layerMetadata.length;

                stream.write(layerMetadata);
                if(includeIds == true){
                    stream.write(idColumn);
                }
                stream.write(geometryColumn);
                stream.write(propertyColumns);
            }

            //System.out.println("------------------------------------------------------------");
            //System.out.println("Total Layer Metadata Size: " + totalLayerMetadataSize);
            //System.out.printf("Contribution of metadata to the total file size: %f%%%n", (1d / (stream.toByteArray().length / (double)totalLayerMetadataSize ))  * 100);
            //System.out.println("------------------------------------------------------------");


            var tileJson = new TileJson();
            tileJson.vectorLayers = vectorLayers;
            ObjectMapper mapper = new ObjectMapper();
            var tileJsonStr = mapper.writerWithDefaultPrettyPrinter().writeValueAsString(tileJson);

            return new ImmutablePair<>(tileJsonStr, stream.toByteArray());
        }


    }

    private static byte[] convertOptimizedLayerMetadata(int layerId, ColumnMetadata idMetadata, ColumnMetadata geometryMetadata,
                                                        PropertyColumnsMetadata propertyColumnData, int layerExtent) {
        /*
        * -> LayerHeader -> version (7 bits), optimizeMetadata (1 bit), name (String | u32), layerExtent (u32),
        *                   numFeatures (u32), numColumns (u32), ColumnMetadata[]
        * -> ColumnMetadata -> columnName(String | u32), required(1 bit), dataType (4 bit), columnType (3 bit),
        *                      GeometryMetadata | PrimitiveTypeMetadata | StringDictionaryMetadata | LocalizedStringDictionaryMetadata
        * -> StreamMetadata -> streamType (4 bits), streamEncoding(4 bits), streamName (optional), numValues (u32), byteLength (u32)
        * -> IdMetadata -> PrimitiveTypeMetadata -> StreamMetadata[1]
        * -> GeometryMetadata -> StreamMetadata[max 6 vor geometries plus z- and m-values]
        * -> PrimitiveTypeMetadata -> StreamMetadata[2]
        * -> StringDictionaryMetadata -> StreamMetadata[4] -> order: present, data, length, dictionary
        * -> LocalizedStringDictionaryMetadata -> StreamMetadata[n]
        * */

        /* id colum is optional but geometry column is required */
        var numColumnsBeforePropertyColumns = idMetadata != null ? 2 : 1;

        var numFeatures = geometryMetadata.streams().get(StreamType.GEOMETRY_TYPES).numValues();
        var numColumns = propertyColumnData.booleanMetadata().size() + propertyColumnData.longMetadata().size() +
                propertyColumnData.floatMetadata().size() +
                propertyColumnData.stringDictionaryMetadata().size() + propertyColumnData.localizedStringDictionaryMetadata().size() +
                numColumnsBeforePropertyColumns;

        /* version and optimizeMetadata flag */
        var encodedVersion = (byte) (FILE_VERSION << 1 | 1);
        var metadata = ArrayUtils.addAll(new byte[]{encodedVersion},
                EncodingUtils.encodeVarints(new long[]{layerId, layerExtent, numFeatures, numColumns}, false, false));

        if(idMetadata != null){
            /*//metadata = ArrayUtils.addAll(metadata, EncodingUtils.encodeString(ID_COLUMN_NAME));
            //metadata  = ArrayUtils.addAll(metadata, (byte)ColumnDataType.UINT_64.ordinal());
            metadata = ArrayUtils.addAll(metadata, (byte) idMetadata.columnType().ordinal());
            metadata = ArrayUtils.addAll(metadata, EncodingUtils.encodeVarints(new long[]{1}, false, false));
            //metadata = addStreamMetadata(metadata, DATA_STREAM_NAME,  idMetadata.streams().get(DATA_STREAM_NAME));*/
            throw new RuntimeException("Id column currently no supported.");
        }

        var geometryStreams = geometryMetadata.streams();
        var geometryColumnId = 1;
        metadata = addOptimizedColumnHeader(metadata, geometryMetadata, geometryColumnId);
        for(var geometryStream : geometryStreams.entrySet()){
            //TODO: add support for z-values and m-values
            metadata = addOptimizedStreamMetadata(metadata, geometryStream.getValue(), geometryStream.getKey());
        }

        /* column id 0 and 1 is always reserved for id and geometry independent of the presence of the id column */
        var nextColumnId = 2;
        metadata = addOptimizedNamedColumnMetadata(metadata, propertyColumnData.booleanMetadata(), nextColumnId);
        nextColumnId += propertyColumnData.booleanMetadata().size();
        metadata = addOptimizedNamedColumnMetadata(metadata, propertyColumnData.longMetadata(), nextColumnId);
        nextColumnId += propertyColumnData.longMetadata().size();
        metadata = addOptimizedNamedColumnMetadata(metadata, propertyColumnData.floatMetadata(), nextColumnId);
        nextColumnId += propertyColumnData.floatMetadata().size();
        metadata = addOptimizedNamedColumnMetadata(metadata, propertyColumnData.stringDictionaryMetadata(), nextColumnId);
        //nextColumnId += propertyColumnData.stringDictionaryMetadata().size();
        //return addOptimizedNamedColumnMetadata(metadata, propertyColumnData.localizedStringDictionaryMetadata(), nextColumnId);

        if(propertyColumnData.localizedStringDictionaryMetadata().size() > 0){
            throw new RuntimeException("Localized Dictionary currently not supported.");
        }

        return metadata;
    }

    private static byte[] convertLayerMetadata(String layerName, ColumnMetadata idMetadata, ColumnMetadata geometryMetadata,
                                                        PropertyColumnsMetadata propertyColumnData, int layerExtent) throws IOException {
        /*
         * -> LayerHeader -> version (7 bits), optimizeMetadata (1 bit), name (String | u32), layerExtent (u32),
         *                   numFeatures (u32), numColumns (u32), ColumnMetadata[]
         * -> ColumnMetadata -> columnName(String | u32), required(1 bit), dataType (4 bit), columnType (3 bit),
         *                      GeometryMetadata | PrimitiveTypeMetadata | StringDictionaryMetadata | LocalizedStringDictionaryMetadata
         * -> StreamMetadata -> streamType (4 bits), streamEncoding(4 bits), streamName (optional), numValues (u32), byteLength (u32)
         * -> IdMetadata -> PrimitiveTypeMetadata -> StreamMetadata[1]
         * -> GeometryMetadata -> StreamMetadata[max 6 vor geometries plus z- and m-values]
         * -> PrimitiveTypeMetadata -> StreamMetadata[2]
         * -> StringDictionaryMetadata -> StreamMetadata[4] -> order: present, data, length, dictionary
         * -> LocalizedStringDictionaryMetadata -> StreamMetadata[n]
         * */

        /* id colum is optional but geometry column is required */
        var numColumnsBeforePropertyColumns = idMetadata != null ? 2 : 1;

        var numFeatures = geometryMetadata.streams().get(StreamType.GEOMETRY_TYPES).numValues();
        var numColumns = propertyColumnData.booleanMetadata().size() + propertyColumnData.longMetadata().size() +
                propertyColumnData.floatMetadata().size() +
                propertyColumnData.stringDictionaryMetadata().size() + propertyColumnData.localizedStringDictionaryMetadata().size() +
                numColumnsBeforePropertyColumns;

        /* version and optimizeMetadata flag */
        var encodedVersion = (byte) (FILE_VERSION << 1 | 0);
        var metadata = ArrayUtils.addAll(new byte[]{encodedVersion}, EncodingUtils.encodeString(layerName));
        metadata = ArrayUtils.addAll(metadata, EncodingUtils.encodeVarints(new long[]{layerExtent, numFeatures, numColumns},
                false, false));

        if(idMetadata != null){
            /*//metadata = ArrayUtils.addAll(metadata, EncodingUtils.encodeString(ID_COLUMN_NAME));
            //metadata  = ArrayUtils.addAll(metadata, (byte)ColumnDataType.UINT_64.ordinal());
            metadata = ArrayUtils.addAll(metadata, (byte) idMetadata.columnType().ordinal());
            metadata = ArrayUtils.addAll(metadata, EncodingUtils.encodeVarints(new long[]{1}, false, false));
            //metadata = addStreamMetadata(metadata, DATA_STREAM_NAME,  idMetadata.streams().get(DATA_STREAM_NAME));*/
            throw new RuntimeException("Id column currently no supported.");
        }

        var geometryStreams = geometryMetadata.streams();
        var geometryColumnId = 1;
        metadata = addOptimizedColumnHeader(metadata, geometryMetadata, geometryColumnId);
        for(var geometryStream : geometryStreams.entrySet()){
            //TODO: add support for z-values and m-values
            metadata = addOptimizedStreamMetadata(metadata, geometryStream.getValue(), geometryStream.getKey());
        }

        metadata = addNamedColumnMetadata(metadata, propertyColumnData.booleanMetadata());
        metadata = addNamedColumnMetadata(metadata, propertyColumnData.longMetadata());
        metadata = addNamedColumnMetadata(metadata, propertyColumnData.floatMetadata());
        metadata = addNamedColumnMetadata(metadata, propertyColumnData.stringDictionaryMetadata());
        //nextColumnId += propertyColumnData.stringDictionaryMetadata().size();
        //return addOptimizedNamedColumnMetadata(metadata, propertyColumnData.localizedStringDictionaryMetadata(), nextColumnId);

        if(propertyColumnData.localizedStringDictionaryMetadata().size() > 0){
            throw new RuntimeException("Localized Dictionary currently not supported.");
        }

        return metadata;
    }

    private static byte[] addOptimizedNamedColumnMetadata(byte[] metadata, List<NamedColumnMetadata> namedMetadata, int nextId){
        for(var column : namedMetadata){
            var columnMetadata = column.columnMetadata();
            metadata = addOptimizedColumnHeader(metadata, columnMetadata, nextId++);

            for(var stream : columnMetadata.streams().entrySet()){
                var streamType = stream.getKey();
                var streamMetadata = stream.getValue();
                if(streamType == StreamType.PRESENT){
                    continue;
                }

                metadata = addOptimizedStreamMetadata(metadata, streamMetadata, streamType);
            }
        }

        return metadata;
    }

    private static byte[] addOptimizedColumnHeader(byte[] metadata, ColumnMetadata columnMetadata, int columnId) {
        metadata = ArrayUtils.addAll(metadata, EncodingUtils.encodeVarints(new long[]{columnId}, false, false));
        var columnDesc = columnMetadata.columnDataType().ordinal() << 3 | columnMetadata.columnType().ordinal();
        /* required (0 == false), dataType and columnType */
        return ArrayUtils.addAll(metadata, (byte)columnDesc);
    }

    private static byte[] addNamedColumnMetadata(byte[] metadata, List<NamedColumnMetadata> namedMetadata) throws IOException {
        for(var column : namedMetadata){
            var columnMetadata = column.columnMetadata();
            metadata = addColumnHeader(metadata, columnMetadata, column.columnName());

            for(var stream : columnMetadata.streams().entrySet()){
                var streamType = stream.getKey();
                var streamMetadata = stream.getValue();
                if(streamType == StreamType.PRESENT){
                    continue;
                }

                metadata = addOptimizedStreamMetadata(metadata, streamMetadata, streamType);
            }
        }

        return metadata;
    }

    private static byte[] addColumnHeader(byte[] metadata, ColumnMetadata columnMetadata, String columnName) throws IOException {
        metadata = ArrayUtils.addAll(metadata, EncodingUtils.encodeString(columnName));
        var columnDesc = columnMetadata.columnDataType().ordinal() << 3 | columnMetadata.columnType().ordinal();
        /* required (0 == false), dataType and columnType */
        return ArrayUtils.addAll(metadata, (byte)columnDesc);
    }

    private static byte[] addOptimizedStreamMetadata(byte[] metadata, StreamMetadata streamMetadata, StreamType streamType) {
        var streamTypeAndEncoding = streamType.ordinal() << 4 | streamMetadata.streamEncoding().ordinal();
        metadata = ArrayUtils.addAll(metadata, new byte[]{(byte)streamTypeAndEncoding});
        return ArrayUtils.addAll(metadata, EncodingUtils.encodeVarints(new long[]{streamMetadata.numValues(),
                streamMetadata.byteLength()}, false, false));
    }

    private static LinkedHashMap<String, ColumnMetadata> getPropertyColumnMetadata(List<Feature> features, boolean allowLocalizedStringDictionary){
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
                        throw new RuntimeException("implement");
                        /* Used as a placeholder since can only be filled after the compression of the columns */
                        /*var emptyStreamData = new StreamMetadata(StreamEncoding.PLAIN, 0,0);
                        if(localizedColumn == null){
                            var metadata = new ColumnMetadata(ColumnDataType.STRING,
                                    ColumnType.LOCALIZED_DICTIONARY, new TreeMap<StreamType, StreamMetadata>(Map.of(streamName, emptyStreamData)));
                            columnMetadata.put(baseColumnName, metadata);
                        }
                        else{
                            if(!localizedColumn.streams().containsKey(streamName)){
                                localizedColumn.streams().put(streamName, emptyStreamData);
                            }
                        }*/
                    }
                    else{
                        var metadata = new ColumnMetadata(ColumnDataType.STRING, ColumnType.DICTIONARY, new TreeMap<>());
                        columnMetadata.put(columnName, metadata);
                    }
                }
                else if(propertyValue instanceof Boolean){
                    var metadata = new ColumnMetadata(ColumnDataType.BOOLEAN, ColumnType.PLAIN, new TreeMap<>());
                    columnMetadata.put(columnName, metadata);
                }
                //TODO: also handle unsigned int and long to avoid zigZag coding
                else if(propertyValue instanceof Integer || propertyValue instanceof  Long){
                    var metadata = new ColumnMetadata(ColumnDataType.INT_64, ColumnType.PLAIN, new TreeMap<>());
                    columnMetadata.put(columnName, metadata);
                }
                else if(propertyValue instanceof Float){
                    var metadata = new ColumnMetadata(ColumnDataType.FLOAT, ColumnType.PLAIN, new TreeMap<>());
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
        var columnMetadata = new ColumnMetadata(ColumnDataType.GEOMETRY, ColumnType.PLAIN, new TreeMap<>());
        var geometryColumn = convertTopologyStreams(geometryTypes, geometryOffsets, partOffsets, ringOffsets,
                columnMetadata, allowFastPforForTopologyStreams);

        var zigZagDeltaCodedVertexBuffer = EncodingUtils.encodeZigZagDeltaCoordinates(vertexBuffer);
        var varintZigZagDeltaVertexBuffer = EncodingUtils.encodeVarints(Arrays.stream(zigZagDeltaCodedVertexBuffer).mapToLong(v -> v).toArray(),
                false, false);
        if(!allowFastPforForVertexBuffer){
            columnMetadata.streams().put(StreamType.VERTEX_BUFFER, new StreamMetadata(StreamEncoding.VARINT_DELTA_ZIG_ZAG,
                    numVertices, varintZigZagDeltaVertexBuffer.length));
            geometryColumn = ArrayUtils.addAll(geometryColumn, varintZigZagDeltaVertexBuffer);
            return new GeometryColumData(columnMetadata, geometryColumn);
        }

        var fastPforZigZagDeltaVertexBuffer = EncodingUtils.encodeFastPfor128(zigZagDeltaCodedVertexBuffer, false, false);
        if(fastPforZigZagDeltaVertexBuffer.length <= varintZigZagDeltaVertexBuffer.length){
            columnMetadata.streams().put(StreamType.VERTEX_BUFFER, new StreamMetadata(StreamEncoding.FAST_PFOR_DELTA_ZIG_ZAG,
                    numVertices, fastPforZigZagDeltaVertexBuffer.length));
            geometryColumn = ArrayUtils.addAll(geometryColumn, fastPforZigZagDeltaVertexBuffer);
            return new GeometryColumData(columnMetadata, geometryColumn);
        }
        else{
            columnMetadata.streams().put(StreamType.VERTEX_BUFFER, new StreamMetadata(StreamEncoding.VARINT_DELTA_ZIG_ZAG,
                    numVertices, varintZigZagDeltaVertexBuffer.length));
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
                    //TODO: refactor -> store length without closing points also when ICE encoding is not used
                    ringOffsets.add(linearRing.getCoordinates().length - 1);
                    var ring = new GeometryFactory().createLineString(Arrays.copyOf(linearRing.getCoordinates(),
                            linearRing.getCoordinates().length - 1));
                    var offsets = getVertexOffsets(ring, vertexDictionary, sfcIdGenerator);
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
                        ringOffsets.add(linearRing.getCoordinates().length -1);
                        var ring = new GeometryFactory().createLineString(Arrays.copyOf(linearRing.getCoordinates(),
                                linearRing.getCoordinates().length - 1));
                        var offsets = getVertexOffsets(ring, vertexDictionary, sfcIdGenerator);
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
            var columnMetadata = new ColumnMetadata(ColumnDataType.GEOMETRY, ColumnType.ICE, new TreeMap<>());
            columnMetadata.streams().put(StreamType.VERTEX_OFFSETS, new StreamMetadata(StreamEncoding.VARINT_DELTA_ZIG_ZAG,
                    vertexOffsets.size(), varintDeltaOffsets.length));
            columnMetadata.streams().put(StreamType.VERTEX_BUFFER, new StreamMetadata(StreamEncoding.VARINT_DELTA_ZIG_ZAG,
                    vertexDictionary.size(), varintDeltaVertexBuffer.length));
            return Pair.of(columnMetadata, ArrayUtils.addAll(varintDeltaOffsets, varintDeltaVertexBuffer));
        }

        var fastPforOffsets = EncodingUtils.encodeFastPfor128(vertexOffsets.stream().mapToInt(i -> i).toArray(), true, true);
        var fastPforVertexBuffer = encodeVertexDictionary(vertexDictionary, true);
        if(allowFastPforDelta  && !allowMortonEncoding){
            var columnMetadata = new ColumnMetadata(ColumnDataType.GEOMETRY, ColumnType.ICE, new TreeMap<>());
            columnMetadata.streams().put(StreamType.VERTEX_OFFSETS, new StreamMetadata(StreamEncoding.FAST_PFOR_DELTA_ZIG_ZAG, vertexOffsets.size(), fastPforOffsets.length));
            columnMetadata.streams().put(StreamType.VERTEX_BUFFER, new StreamMetadata(StreamEncoding.FAST_PFOR_DELTA_ZIG_ZAG, vertexDictionary.size(), fastPforVertexBuffer.length));
            return Pair.of(columnMetadata, ArrayUtils.addAll(fastPforOffsets, fastPforVertexBuffer));
        }

        var varintDeltaMortonVertexBuffer = encodeVertexDictionaryVarintWithMortonId(vertexDictionary, sfcIdGenerator);
        if(!allowFastPforDelta && allowMortonEncoding){
            var columnMetadata = new ColumnMetadata(ColumnDataType.GEOMETRY, ColumnType.ICE_MORTON_CODE, new TreeMap<>());
            columnMetadata.streams().put(StreamType.VERTEX_OFFSETS, new StreamMetadata(StreamEncoding.VARINT_DELTA_ZIG_ZAG,
                    vertexOffsets.size(), varintDeltaOffsets.length));
            columnMetadata.streams().put(StreamType.VERTEX_BUFFER, new StreamMetadata(StreamEncoding.VARINT_DELTA_ZIG_ZAG,
                    vertexDictionary.size(), varintDeltaMortonVertexBuffer.length));
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
            var columnMetadata = new ColumnMetadata(ColumnDataType.GEOMETRY, ColumnType.ICE, new TreeMap<>());
            columnMetadata.streams().put(StreamType.VERTEX_OFFSETS, new StreamMetadata(vertexOffsetsEncoding,
                    vertexOffsets.size(),  encodedVertexOffsets.length));
            columnMetadata.streams().put(StreamType.VERTEX_BUFFER, new StreamMetadata(StreamEncoding.VARINT_DELTA_ZIG_ZAG,
                    vertexDictionary.size(), varintDeltaVertexBuffer.length));
            return Pair.of(columnMetadata, ArrayUtils.addAll(encodedVertexOffsets, varintDeltaVertexBuffer));
        }

        if(fastPforDeltaGeometryColumnSize < varintDeltaGeometryColumSize && fastPforDeltaGeometryColumnSize < varintDeltaMortonGeometryColumnSize &&
            fastPforDeltaGeometryColumnSize < fastPforDeltaMortonGeometryColumSize){
            var columnMetadata = new ColumnMetadata(ColumnDataType.GEOMETRY, ColumnType.ICE, new TreeMap<>());
            columnMetadata.streams().put(StreamType.VERTEX_OFFSETS, new StreamMetadata(vertexOffsetsEncoding,
                    vertexOffsets.size(), encodedVertexOffsets.length));
            columnMetadata.streams().put(StreamType.VERTEX_BUFFER, new StreamMetadata(StreamEncoding.FAST_PFOR_DELTA_ZIG_ZAG,
                    vertexDictionary.size(), fastPforVertexBuffer.length));
            return Pair.of(columnMetadata, ArrayUtils.addAll(encodedVertexOffsets, fastPforVertexBuffer));
        }

        if(varintDeltaMortonGeometryColumnSize < varintDeltaGeometryColumSize && varintDeltaMortonGeometryColumnSize < fastPforDeltaGeometryColumnSize
            && varintDeltaMortonGeometryColumnSize < fastPforDeltaMortonGeometryColumSize){
            var columnMetadata = new ColumnMetadata(ColumnDataType.GEOMETRY, ColumnType.ICE_MORTON_CODE, new TreeMap<>());
            columnMetadata.streams().put(StreamType.VERTEX_OFFSETS, new StreamMetadata(vertexOffsetsEncoding,
                    vertexOffsets.size(),  encodedVertexOffsets.length));
            columnMetadata.streams().put(StreamType.VERTEX_BUFFER, new StreamMetadata(StreamEncoding.VARINT_DELTA_ZIG_ZAG,
                    vertexDictionary.size(), varintDeltaMortonVertexBuffer.length));
            return Pair.of(columnMetadata, ArrayUtils.addAll(encodedVertexOffsets, varintDeltaMortonVertexBuffer));
        }

        var columnMetadata = new ColumnMetadata(ColumnDataType.GEOMETRY, ColumnType.ICE_MORTON_CODE, new TreeMap<>());
        columnMetadata.streams().put(StreamType.VERTEX_OFFSETS, new StreamMetadata(vertexOffsetsEncoding,
                vertexOffsets.size(),  encodedVertexOffsets.length));
        columnMetadata.streams().put(StreamType.VERTEX_BUFFER, new StreamMetadata(StreamEncoding.FAST_PFOR_DELTA_ZIG_ZAG,
                vertexDictionary.size(), fastPforDeltaMortonVertexBuffer.length));
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
        streams.put(StreamType.GEOMETRY_TYPES, new StreamMetadata(StreamEncoding.BYTE_RLE, geometryTypes.size(), geometryTypeStream.length));
        var geometryColumn = geometryTypeStream;

        if(geometryOffsets.size() > 0){
            geometryColumn = addOffsets(geometryOffsets, allowFastPforDelta,
                    streams, geometryColumn, StreamType.GEOMETRY_OFFSETS);
        }

        if(partOffsets.size() > 0){
            geometryColumn = addOffsets(partOffsets, allowFastPforDelta,
                    streams, geometryColumn, StreamType.PART_OFFSETS);
        }

        if(ringOffsets.size() > 0){
            geometryColumn = addOffsets(ringOffsets, allowFastPforDelta,
                    streams, geometryColumn, StreamType.RING_OFFSETS);
        }

        return geometryColumn;
    }

    private static byte[] addOffsets(List<Integer> offsets, Boolean useFastPforDelta,
                                     TreeMap<StreamType, StreamMetadata> streams, byte[] geometryColumn,
                                     StreamType streamType) throws IOException {
        //TODO: change again
        var rleOffsets= EncodingUtils.encodeRle(offsets.stream().mapToLong(i -> i).toArray(), false);
        //var rleOffsets= EncodingUtils.encodeVarints(offsets.stream().mapToLong(i -> i).toArray(), true, true);
        if(!useFastPforDelta){
            streams.put(streamType, new StreamMetadata(StreamEncoding.RLE, offsets.size(), rleOffsets.length));
            return ArrayUtils.addAll(geometryColumn, rleOffsets);
        }

        var fastPforDeltaOffsets = EncodingUtils.encodeFastPfor128(
                offsets.stream().mapToInt(i -> i).toArray(), true, true);


        if(rleOffsets.length > 50){
            System.out.printf("Fast Pfor Offsests: %s, RLE Offsets: %s\n", fastPforDeltaOffsets.length, rleOffsets.length);
        }

        if(fastPforDeltaOffsets.length <= rleOffsets.length){
            streams.put(streamType, new StreamMetadata(StreamEncoding.FAST_PFOR_DELTA_ZIG_ZAG, offsets.size(),
                    fastPforDeltaOffsets.length));
            return ArrayUtils.addAll(geometryColumn, fastPforDeltaOffsets);
        }
        else{
            streams.put(streamType, new StreamMetadata(StreamEncoding.RLE, offsets.size(), rleOffsets.length));
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
                    if(ColumnType.LOCALIZED_DICTIONARY.equals(metadata.columnType())) {
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
                var values = new ArrayList<Boolean>();
                var i = 0;
                for(var present : booleanColumn.presentStream()){
                    var value = present ? dataStream.get(i++) : false;
                    values.add(value);
                }
                var encodedData = EncodingUtils.encodeBooleans(values);
                columnBuffer = ArrayUtils.addAll(columnBuffer, encodedData);
                var metadata = booleanColumn.columnMetadata();
                metadata.streams().put(StreamType.DATA, new StreamMetadata(StreamEncoding.BOOLEAN_RLE, dataStream.size(), encodedData.length));
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

                metadata.streams().put(StreamType.PRESENT, new StreamMetadata(StreamEncoding.BOOLEAN_RLE,
                        presentStream.size(), encodedPresentStream.length));
                if(rleValues.length < varintValues.length && rleValues.length < deltaVarintValues.length){
                    columnBuffer = ArrayUtils.addAll(columnBuffer, rleValues);
                    metadata.streams().put(StreamType.DATA, new StreamMetadata(StreamEncoding.RLE,
                            dataStream.size(), rleValues.length));
                }
                else if(deltaVarintValues.length < rleValues.length && deltaVarintValues.length < varintValues.length){
                    columnBuffer = ArrayUtils.addAll(columnBuffer, deltaVarintValues);
                    metadata.streams().put(StreamType.DATA, new StreamMetadata(StreamEncoding.VARINT_DELTA_ZIG_ZAG,
                            dataStream.size(), deltaVarintValues.length));
                }
                else{
                    columnBuffer = ArrayUtils.addAll(columnBuffer, varintValues);
                    metadata.streams().put(StreamType.DATA, new StreamMetadata(StreamEncoding.VARINT_ZIG_ZAG,
                            dataStream.size(), varintValues.length));
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
                metadata.streams().put(StreamType.PRESENT, new StreamMetadata(StreamEncoding.PLAIN, presentStream.size(), encodedPresentStream.length));
                metadata.streams().put(StreamType.DATA, new StreamMetadata(StreamEncoding.PLAIN, dataStream.size(), encodedData.length));
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
                streams.put(StreamType.PRESENT, new StreamMetadata(StreamEncoding.BOOLEAN_RLE,
                        presentStream.size(), encodedPresentStream.length));
                streams.put(StreamType.DATA, new StreamMetadata(StreamEncoding.RLE, dataStream.size(), encodedDataStream.length));
                streams.put(StreamType.LENGTH, new StreamMetadata(StreamEncoding.RLE,
                        dictionaryStream.size(), encodedLengthStream.length));
                streams.put(StreamType.DICTIONARY, new StreamMetadata(StreamEncoding.PLAIN,
                        dictionaryStream.size(), encodedDictionary.length));
            }
        }

        if(stringLocalizedDictionaryColumns.size() > 0){
            throw new RuntimeException("Localized Dictionary encoding currently not supported.");
            /*for(var column : stringLocalizedDictionaryColumns.entrySet()){
                var columnData = column.getValue();
                columnData.columnMetadata().streams().clear();

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

                    //TODO: add the metadata ids for the name streams
                    streams.put(PRESENT_STREAM_ID + "_" + streamName, new StreamMetadata(presentStream.size(), encodedPresentStream.length,
                            StreamEncoding.BOOLEAN_RLE, StreamType.PRESENT));
                    streams.put(streamName, new StreamMetadata(dataStream.size(), encodedDataStream.length,
                            StreamEncoding.RLE, StreamType.DATA));
                }

                var encodedLengthStream = EncodingUtils.encodeRle(columnData.lengthStream().stream().mapToLong(i -> i).toArray(), false);
                var encodedDictionary = CollectionUtils.concatByteArrays(columnData.dictionaryStream().stream().
                        map(s -> s.getBytes(StandardCharsets.UTF_8)).collect(Collectors.toList()));
                columnBuffer = ArrayUtils.addAll(columnBuffer, encodedLengthStream);
                columnBuffer = ArrayUtils.addAll(columnBuffer, encodedDictionary);
                var numValues = columnData.dictionaryStream().size();
                //TODO: quick and dirty -> leads to collisions when length or dictionary are feature properties
                streams.put(LENGTH_STREAM_NAME, new StreamMetadata(numValues, encodedLengthStream.length, StreamEncoding.RLE,
                        StreamType.LENGTH));
                streams.put(DICTIONARY_STREAM_NAME, new StreamMetadata(numValues, encodedDictionary.length, StreamEncoding.PLAIN,
                        StreamType.DICTIONARY));
            }*/
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

            throw new RuntimeException("implement");
           //streamData.put(streamName, new LocalizedStringDictionaryStreamData(presentStream, dataStream));
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
