package org.maplibre.mlt.converter;

import com.google.gson.Gson;
import jakarta.annotation.Nullable;
import java.io.ByteArrayOutputStream;
import java.io.DataOutputStream;
import java.io.IOException;
import java.net.URI;
import java.util.*;
import java.util.regex.Pattern;
import java.util.stream.Collectors;
import java.util.stream.IntStream;
import java.util.stream.Stream;
import org.apache.commons.lang3.StringUtils;
import org.apache.commons.lang3.tuple.Pair;
import org.jetbrains.annotations.NotNull;
import org.maplibre.mlt.converter.encodings.EncodingUtils;
import org.maplibre.mlt.converter.encodings.GeometryEncoder;
import org.maplibre.mlt.converter.encodings.MltTypeMap;
import org.maplibre.mlt.converter.encodings.PropertyEncoder;
import org.maplibre.mlt.converter.mvt.ColumnMapping;
import org.maplibre.mlt.converter.mvt.MapboxVectorTile;
import org.maplibre.mlt.data.Feature;
import org.maplibre.mlt.metadata.stream.PhysicalLevelTechnique;
import org.maplibre.mlt.metadata.tileset.MltMetadata;

public class MltConverter {
  public static MltMetadata.TileSetMetadata createTilesetMetadata(
      MapboxVectorTile tile,
      Map<Pattern, List<ColumnMapping>> columnMappings,
      boolean isIdPresent) {
    return createTilesetMetadata(tile, columnMappings, isIdPresent, false, false);
  }

  public static MltMetadata.TileSetMetadata createTilesetMetadata(
      MapboxVectorTile tile,
      Map<Pattern, List<ColumnMapping>> columnMappings,
      boolean isIdPresent,
      boolean enableCoerceOnMismatch,
      boolean enableElideOnMismatch) {

    // TODO: Allow determining whether ID is present automatically
    // TODO: Allow nullable ID columns

    var tileset = new MltMetadata.TileSetMetadata();

    for (var layer : tile.layers()) {
      final LinkedHashMap<String, MltMetadata.Column> columnSchemas = new LinkedHashMap<>();
      final LinkedHashMap<ColumnMapping, MltMetadata.ComplexField> complexPropertyColumnSchemas =
          new LinkedHashMap<>();

      var hasLongId = false;
      var featureIndex = 0;
      for (var feature : layer.features()) {
        final var currentFeatureIndex = featureIndex;
        feature.properties().entrySet().stream()
            .sorted(Map.Entry.comparingByKey())
            .forEach(
                property -> {
                  resolveColumnType(
                      property,
                      layer.name(),
                      currentFeatureIndex,
                      columnMappings,
                      columnSchemas,
                      complexPropertyColumnSchemas,
                      enableCoerceOnMismatch,
                      enableElideOnMismatch);
                });

        if (isIdPresent
            && feature.hasId()
            && (feature.id() > Integer.MAX_VALUE || feature.id() < Integer.MIN_VALUE)) {
          hasLongId = true;
        }
        featureIndex++;
      }

      for (var complexPropertyColumnScheme : complexPropertyColumnSchemas.entrySet()) {
        final var schema = complexPropertyColumnScheme.getValue();
        final var parentName = resolveComplexColumnMapping(schema);

        // Each complex column scheme needs to have a unique entry name in the column map, but there
        // is no specific column to which it maps.  For now, just ensure that the value is unique.
        final var column = createColumn(parentName, schema);
        IntStream.iterate(0, i -> i + 1)
            .mapToObj(i -> parentName + i)
            .filter(name -> !columnSchemas.containsKey(name))
            .findFirst()
            .ifPresent(name -> columnSchemas.put(name, column));
      }

      var featureTableSchema = new MltMetadata.FeatureTable(layer.name());

      // If present, `id` must be the first column
      if (columnSchemas.values().stream().anyMatch(MltTypeMap.Tag0x01::isID)) {
        throw new RuntimeException("Unexpected ID Column");
      }
      if (isIdPresent) {
        final var newColumn =
            new MltMetadata.Column(
                null, new MltMetadata.ScalarField(MltMetadata.LogicalScalarType.ID));
        newColumn.isNullable = layer.features().stream().anyMatch(feature -> !feature.hasId());
        newColumn.columnScope = MltMetadata.ColumnScope.FEATURE;
        newColumn.scalarType.hasLongId = hasLongId;
        featureTableSchema.columns.add(newColumn);
      }

      // The `geometry` column is mandatory and has to be the first column after `ID`
      featureTableSchema.columns.add(
          createComplexColumnScheme(null, false, MltMetadata.ComplexType.GEOMETRY));

      featureTableSchema.columns.addAll(columnSchemas.values());
      tileset.featureTables.add(featureTableSchema);
    }

    return tileset;
  }

  private static void resolveColumnType(
      Map.Entry<String, Object> property,
      String layerName,
      int featureIndex,
      Map<Pattern, List<ColumnMapping>> columnMappings,
      LinkedHashMap<String, MltMetadata.Column> columnSchemas,
      LinkedHashMap<ColumnMapping, MltMetadata.ComplexField> complexColumnSchemas,
      boolean enableCoerceOnMismatch,
      boolean enableElideOnMismatch) {
    final var mvtPropertyName = property.getKey();

    /* MVT can only contain scalar types */
    final var scalarType = getScalarType(property);

    // If this property already has a column...
    final var previousSchema = columnSchemas.get(mvtPropertyName);
    if (previousSchema != null) {
      // Make sure the types match.
      // If not, coercion or nullification must be enabled, and replace
      // the column with a string column, if it isn't already.
      if (previousSchema.scalarType != null) {
        if (previousSchema.scalarType.physicalType != null) {
          final var prevPhysicalType = previousSchema.scalarType.physicalType;
          if (prevPhysicalType != scalarType) {
            if (prevPhysicalType == MltMetadata.ScalarType.INT_32
                && scalarType == MltMetadata.ScalarType.INT_64) {
              // Allow implicit upgrade from INT_32 to INT_64
              previousSchema.scalarType.physicalType = MltMetadata.ScalarType.INT_64;
            } else if (prevPhysicalType == MltMetadata.ScalarType.INT_64
                && scalarType == MltMetadata.ScalarType.INT_32) {
              // no-op
              // keep INT_64
            } else if (prevPhysicalType == MltMetadata.ScalarType.FLOAT
                && scalarType == MltMetadata.ScalarType.DOUBLE) {
              // Allow implicit upgrade from FLOAT to DOUBLE
              previousSchema.scalarType.physicalType = MltMetadata.ScalarType.DOUBLE;
            } else if (prevPhysicalType == MltMetadata.ScalarType.DOUBLE
                && scalarType == MltMetadata.ScalarType.FLOAT) {
              // no-op
              // keep DOUBLE
            } else if (enableCoerceOnMismatch) {
              if (prevPhysicalType != MltMetadata.ScalarType.STRING) {
                previousSchema.scalarType.physicalType = MltMetadata.ScalarType.STRING;
              }
            } else if (!enableElideOnMismatch) {
              throw new RuntimeException(
                  String.format(
                      "Layer '%s' Feature index %d Property '%s' has different type: %s / %s",
                      layerName,
                      featureIndex,
                      property.getKey(),
                      scalarType.name(),
                      prevPhysicalType.name()));
            }
            return;
          }
        }
      }
    }

    final var columnMapping = ColumnMapping.findMapping(columnMappings, layerName, mvtPropertyName);
    if (columnMapping != null) {
      // A mapping exists for this property.
      // Create the parent type and add a child type entry.
      final var parentColumn =
          complexColumnSchemas.computeIfAbsent(columnMapping, k -> createComplexColumn());

      if (parentColumn.children.stream().noneMatch(c -> c.name.equals(mvtPropertyName))) {
        parentColumn.children.add(createScalarFieldScheme(mvtPropertyName, true, scalarType));
      }

      return;
    }

    // no matching column mappings, create a plain scalar column
    columnSchemas.put(mvtPropertyName, createScalarColumnScheme(mvtPropertyName, true, scalarType));
  }

  /// Resolve complex column mapping by determining common prefix and adjusting child names
  /// @return The longest common prefix which has been removed from the child field names (which may
  /// be blank)
  private static String resolveComplexColumnMapping(MltMetadata.ComplexField column) {
    final var prefix =
        StringUtils.getCommonPrefix(
            column.children.stream().map(c -> c.name).toArray(String[]::new));
    if (!prefix.isEmpty()) {
      for (var child : column.children) {
        final var name = child.name;
        assert (name.startsWith(prefix));
        child.name = name.substring(prefix.length());
      }
    }
    return prefix;
  }

  public static String createTilesetMetadataJSON(MltMetadata.TileSetMetadata pbMetadata) {
    var root = new TreeMap<String, Object>();
    final int version = 1;
    root.put("version", version);
    if (pbMetadata.name != null) {
      root.put("name", pbMetadata.name);
    }
    if (pbMetadata.description != null) {
      root.put("description", pbMetadata.description);
    }
    if (pbMetadata.attribution != null) {
      root.put("attribution", pbMetadata.attribution);
    }
    pbMetadata.minZoom.ifPresent(integer -> root.put("minZoom", integer));
    pbMetadata.maxZoom.ifPresent(integer -> root.put("maxZoom", integer));

    var bounds = new ArrayList<Map<String, Object>>();
    for (int i = 0; i < (pbMetadata.bounds.size() / 4); ++i) {
      var bound = new TreeMap<String, Object>();
      bound.put("left", pbMetadata.bounds.get(4 * i));
      bound.put("top", pbMetadata.bounds.get((4 * i) + 1));
      bound.put("right", pbMetadata.bounds.get((4 * i) + 2));
      bound.put("bottom", pbMetadata.bounds.get((4 * i) + 3));
      bounds.add(bound);
    }
    if (!bounds.isEmpty()) {
      root.put("bounds", bounds);
    }

    var centers = new ArrayList<Map<String, Object>>();
    for (int i = 0; i < (pbMetadata.center.size() / 2); ++i) {
      var center = new TreeMap<String, Object>();
      center.put("longitude", 2 * i);
      center.put("latitude", (2 * i) + 1);
      centers.add(center);
    }
    if (!centers.isEmpty()) {
      root.put("center", centers);
    }

    return new Gson().toJson(root);
  }

  /// Write the header block for a field or column.
  /// Takes the values individually because, despite having mostly
  /// the same information, the field and column are separate types.
  private static void writeColumnOrFieldType(
      DataOutputStream stream,
      String name,
      boolean isNullable,
      boolean hasLongIDs,
      @Nullable MltMetadata.ScalarType physicalScalarType,
      @Nullable MltMetadata.LogicalScalarType logicalScalarType,
      @Nullable MltMetadata.ComplexType physicalComplexType,
      @Nullable MltMetadata.LogicalComplexType logicalComplexType,
      @Nullable List<MltMetadata.Field> children)
      throws IOException {
    // We expect exactly one of these
    if (Stream.of(physicalScalarType, logicalScalarType, physicalComplexType, logicalComplexType)
            .filter(Objects::nonNull)
            .count()
        != 1) {
      throw new RuntimeException("Invalid Type");
    }

    final boolean hasChildren = (children != null && !children.isEmpty());
    final var typeCode =
        MltTypeMap.Tag0x01.encodeColumnType(
                physicalScalarType,
                logicalScalarType,
                physicalComplexType,
                logicalComplexType,
                isNullable,
                hasChildren,
                hasLongIDs)
            .orElseThrow(() -> new RuntimeException("Unsupported Type"));
    EncodingUtils.putVarInt(stream, typeCode);

    if (MltTypeMap.Tag0x01.columnTypeHasName(typeCode)) {
      EncodingUtils.putString(stream, name);
    }

    if (hasChildren) {
      EncodingUtils.putVarInt(stream, children.size());
      for (var child : children) {
        final boolean complex = child.complexType != null;
        final boolean logical =
            (complex && child.complexType.logicalType != null)
                || (!complex && child.scalarType.logicalType != null);

        writeColumnOrFieldType(
            stream,
            child.name,
            child.isNullable,
            /* hasLongIDs= */ false,
            (!complex && !logical) ? child.scalarType.physicalType : null,
            (!complex && logical) ? child.scalarType.logicalType : null,
            (complex && !logical) ? child.complexType.physicalType : null,
            (complex && logical) ? child.complexType.logicalType : null,
            complex ? child.complexType.children : null);
      }
    }
  }

  /// Produce the binary tile header containing the tile metadata
  /// <p>Note: Uses the protobuf format as input to avoid repeating the logic there, could be
  /// refactored to eliminate it</p>
  public static byte[] createEmbeddedMetadata(MltMetadata.FeatureTable table, int extent)
      throws IOException {
    try (var byteStream = new ByteArrayOutputStream()) {
      try (var dataStream = new DataOutputStream(byteStream)) {
        EncodingUtils.putString(dataStream, table.name);
        EncodingUtils.putVarInt(dataStream, extent);
        EncodingUtils.putVarInt(dataStream, table.columns.size());
        for (var column : table.columns) {
          if (column.columnScope != MltMetadata.ColumnScope.FEATURE) {
            throw new RuntimeException("Vertex scoped properties are not yet supported");
          }
          writeColumnOrFieldType(
              dataStream,
              column.name,
              column.isNullable,
              column.scalarType != null && column.scalarType.hasLongId,
              column.scalarType != null && column.scalarType.physicalType != null
                  ? column.scalarType.physicalType
                  : null,
              column.scalarType != null && column.scalarType.logicalType != null
                  ? column.scalarType.logicalType
                  : null,
              column.complexType != null && column.complexType.physicalType != null
                  ? column.complexType.physicalType
                  : null,
              column.complexType != null && column.complexType.logicalType != null
                  ? column.complexType.logicalType
                  : null,
              column.complexType != null ? column.complexType.children : null);
        }
      }
      return byteStream.toByteArray();
    }
  }

  /*
   * Converts a MVT file to an MLT file.
   *
   * @param mvt Tile to convert
   * @param config Settings for the conversion
   * @param tilesetMetadata Metadata of the tile such as the scheme
   * @return Converted MapLibreTile
   * @throws IOException
   */
  public static byte[] convertMvt(
      MapboxVectorTile mvt,
      MltMetadata.TileSetMetadata tilesetMetadata,
      ConversionConfig config,
      @Nullable URI tessellateSource)
      throws IOException {
    return convertMvt(
        mvt, tilesetMetadata, config, tessellateSource, new MLTStreamObserverDefault());
  }

  /*
   * Converts a MVT file to an MLT file.
   *
   * @param mvt The decoded MVT tile to convert
   * @param config Settings for the conversion
   * @param tilesetMetadata Metadata of the tile
   * @param tessellateSource Optional URI of a tessellation service to use if polygon pre-tessellation is enabled
   * @param streamRecorder Recorder for observing streams during conversion
   * @return Converted MapLibreTile
   * @throws IOException
   */
  public static byte[] convertMvt(
      MapboxVectorTile mvt,
      MltMetadata.TileSetMetadata tilesetMetadata,
      ConversionConfig config,
      @Nullable URI tessellateSource,
      @NotNull MLTStreamObserver streamRecorder)
      throws IOException {

    // Convert the list of metadatas (one per layer) into a lookup by the first and only layer name
    // We assume that the names are unique.
    final var metaMap =
        tilesetMetadata.featureTables.stream()
            .collect(
                Collectors.toMap(
                    t -> t.name,
                    table -> table,
                    (existing, replacement) -> {
                      throw new RuntimeException("duplicate key");
                    }));

    var physicalLevelTechnique =
        config.getUseFastPFOR() ? PhysicalLevelTechnique.FAST_PFOR : PhysicalLevelTechnique.VARINT;

    var mapLibreTileBuffer = new byte[0];
    for (var mvtLayer : mvt.layers()) {
      final var featureTableName = mvtLayer.name();
      streamRecorder.setLayerName(featureTableName);

      if (config.getLayerFilterPattern() != null) {
        final var matcher = config.getLayerFilterPattern().matcher(featureTableName);
        final var isMatch = matcher.matches() ^ config.getLayerFilterInvert();
        if (!isMatch) {
          continue;
        }
      }

      final var layerMetadata = metaMap.get(featureTableName);
      if (layerMetadata == null) {
        throw new RuntimeException("Missing Metadata");
      }

      final var mvtFeatures = mvtLayer.features();
      if (mvtFeatures.isEmpty()) {
        continue;
      }

      final var featureTableOptimizations =
          config.getOptimizations() == null
              ? null
              : config.getOptimizations().get(featureTableName);

      final var createPolygonOutline =
          config.getOutlineFeatureTableNames().contains(featureTableName)
              || config.getOutlineFeatureTableNames().contains("ALL");
      final var result =
          sortFeaturesAndEncodeGeometryColumn(
              config,
              featureTableOptimizations,
              mvtFeatures,
              mvtFeatures,
              physicalLevelTechnique,
              createPolygonOutline,
              tessellateSource,
              streamRecorder);
      final var sortedFeatures = result.getLeft();
      final var encodedGeometryColumn = result.getRight();
      final var encodedGeometryFieldMetadata =
          EncodingUtils.encodeVarint(encodedGeometryColumn.numStreams(), false);

      var encodedPropertyColumns =
          encodePropertyColumns(
              config, layerMetadata, sortedFeatures, featureTableOptimizations, streamRecorder);

      var featureTableBodyBuffer = new byte[0];
      if (config.getIncludeIds()) {
        final var idMetadata =
            layerMetadata.columns.stream()
                .filter(MltTypeMap.Tag0x01::isID)
                .findFirst()
                .orElseThrow();

        // Write ID as a 32- or 64-bit scalar depending on the flag stored in the column metadata.
        // The decoding assumes unsigned (no zigzag)
        final var rawType =
            idMetadata.scalarType.hasLongId
                ? MltMetadata.ScalarType.UINT_64
                : MltMetadata.ScalarType.UINT_32;
        final var name = "id"; // Name is used only for stream capture
        final var scalarColumnMetadata =
            new MltMetadata.Column(name, new MltMetadata.ScalarField(rawType));
        scalarColumnMetadata.isNullable = idMetadata.isNullable;
        scalarColumnMetadata.columnScope = MltMetadata.ColumnScope.FEATURE;
        featureTableBodyBuffer =
            PropertyEncoder.encodeScalarPropertyColumn(
                scalarColumnMetadata,
                true,
                sortedFeatures,
                physicalLevelTechnique,
                config.getUseFSST(),
                config.getCoercePropertyValues(),
                config.getIntegerEncodingOption(),
                streamRecorder);
      }

      featureTableBodyBuffer =
          CollectionUtils.concatByteArrays(
              featureTableBodyBuffer,
              encodedGeometryFieldMetadata,
              encodedGeometryColumn.encodedValues(),
              encodedPropertyColumns);

      final var metadataBuffer = createEmbeddedMetadata(layerMetadata, mvtLayer.tileExtent());

      final var tag = 1;
      final var tagBuffer = EncodingUtils.encodeVarint(tag, false);
      final var tagLength =
          tagBuffer.length + metadataBuffer.length + featureTableBodyBuffer.length;

      mapLibreTileBuffer =
          CollectionUtils.concatByteArrays(
              mapLibreTileBuffer,
              EncodingUtils.encodeVarint(tagLength, false),
              tagBuffer,
              metadataBuffer,
              featureTableBodyBuffer);
    }

    return mapLibreTileBuffer;
  }

  private static byte[] encodePropertyColumns(
      ConversionConfig config,
      MltMetadata.FeatureTable featureTableMetadata,
      List<Feature> sortedFeatures,
      FeatureTableOptimizations featureTableOptimizations,
      @NotNull MLTStreamObserver streamRecorder)
      throws IOException {
    final var propertyColumns = filterPropertyColumns(featureTableMetadata);
    final List<ColumnMapping> columnMappings =
        (featureTableOptimizations != null)
            ? featureTableOptimizations.columnMappings()
            : List.of();
    return PropertyEncoder.encodePropertyColumns(
        propertyColumns,
        sortedFeatures,
        config.getUseFastPFOR(),
        config.getUseFSST(),
        config.getCoercePropertyValues(),
        columnMappings,
        config.getIntegerEncodingOption(),
        streamRecorder);
  }

  private static Pair<List<Feature>, GeometryEncoder.EncodedGeometryColumn>
      sortFeaturesAndEncodeGeometryColumn(
          ConversionConfig config,
          FeatureTableOptimizations featureTableOptimizations,
          List<Feature> sortedFeatures,
          List<Feature> mvtFeatures,
          PhysicalLevelTechnique physicalLevelTechnique,
          boolean encodePolygonOutlines,
          @Nullable URI tessellateSource,
          @NotNull MLTStreamObserver streamRecorder)
          throws IOException {
    /*
     * Following simple strategy is currently used for ordering the features when sorting is enabled:
     * - if id column is present and ids should not be reassigned -> sort id column
     * - if id column is presented and ids can be reassigned  -> sort geometry column (VertexOffsets)
     *   and regenerate ids
     * - if id column is not presented -> sort geometry column
     * In general finding an optimal column arrangement is NP-hard, but by implementing a more sophisticated strategy
     * based on the latest academic results in the future, the compression ratio can be further improved
     * */

    if (mvtFeatures.isEmpty()) {
      throw new IllegalArgumentException("No features to encode");
    }

    var isColumnSortable =
        config.getIncludeIds()
            && featureTableOptimizations != null
            && featureTableOptimizations.allowSorting();
    if (isColumnSortable && !featureTableOptimizations.allowIdRegeneration()) {
      sortedFeatures = sortFeaturesById(mvtFeatures);
    }

    var ids = sortedFeatures.stream().map(Feature::idOrNull).collect(Collectors.toList());
    var geometries = sortedFeatures.stream().map(Feature::geometry).collect(Collectors.toList());

    if (geometries.isEmpty()) {
      throw new IllegalArgumentException("No geometries to encode");
    }

    /* Only sort geometries if ids can be reassigned since sorting the id column turned out
     * to be more efficient in the tests */
    var sortSettings =
        new GeometryEncoder.SortSettings(
            isColumnSortable && featureTableOptimizations.allowIdRegeneration(), ids);
    /* Morton Vertex Dictionary encoding is currently not supported in pre-tessellation */
    var useMortonEncoding = false;
    var encodedGeometryColumn =
        config.getPreTessellatePolygons()
            ? GeometryEncoder.encodePretessellatedGeometryColumn(
                geometries,
                physicalLevelTechnique,
                sortSettings,
                useMortonEncoding,
                encodePolygonOutlines,
                tessellateSource,
                streamRecorder)
            : GeometryEncoder.encodeGeometryColumn(
                geometries,
                physicalLevelTechnique,
                sortSettings,
                config.getUseMortonEncoding(),
                streamRecorder);

    if (encodedGeometryColumn.geometryColumnSorted()) {
      sortedFeatures =
          ids.stream()
              .map(
                  id ->
                      mvtFeatures.stream()
                          .filter(fe -> Objects.equals(fe.idOrNull(), id))
                          .findFirst()
                          .orElseThrow())
              .collect(Collectors.toList());
    }

    if (config.getIncludeIds()
        && featureTableOptimizations != null
        && featureTableOptimizations.allowIdRegeneration()) {
      sortedFeatures = generateSequenceIds(sortedFeatures);
    }

    return Pair.of(sortedFeatures, encodedGeometryColumn);
  }

  private static List<MltMetadata.Column> filterPropertyColumns(
      MltMetadata.FeatureTable featureTableMetadata) {
    return featureTableMetadata.columns.stream()
        .filter(f -> !MltTypeMap.Tag0x01.isID(f) && !MltTypeMap.Tag0x01.isGeometry(f))
        .collect(Collectors.toList());
  }

  private static List<Feature> sortFeaturesById(List<Feature> features) {
    return features.stream()
        .sorted(Comparator.comparing(Feature::hasId).thenComparingLong(Feature::id))
        .collect(Collectors.toList());
  }

  private static List<Feature> generateSequenceIds(List<Feature> features) {
    var sortedFeatures = new ArrayList<Feature>();
    long idCounter = 0;
    for (var feature : features) {
      sortedFeatures.add(new Feature(idCounter++, feature.geometry(), feature.properties()));
    }
    return sortedFeatures;
  }

  private static MltMetadata.ScalarType getScalarType(Map.Entry<String, Object> property) {
    var propertyValue = property.getValue();
    if (propertyValue instanceof Boolean) {
      return MltMetadata.ScalarType.BOOLEAN;
    }
    // TODO: also handle unsigned int to avoid zigZag coding
    // TODO: quick and dirty fix for wrong data types -> make proper solution
    else if (propertyValue instanceof Integer) {
      return MltMetadata.ScalarType.INT_32;
    } else if (propertyValue instanceof Long) {
      return ((long) propertyValue > Integer.MAX_VALUE || (long) propertyValue < Integer.MIN_VALUE)
          ? MltMetadata.ScalarType.INT_64
          : MltMetadata.ScalarType.INT_32;
    } else if (propertyValue instanceof Float) {
      return MltMetadata.ScalarType.FLOAT;
    } else if (propertyValue instanceof Double) {
      return MltMetadata.ScalarType.DOUBLE;
    } else if (propertyValue instanceof String) {
      return MltMetadata.ScalarType.STRING;
    }

    throw new IllegalArgumentException("Specified data type currently not supported.");
  }

  private static MltMetadata.Column createScalarColumnScheme(
      String columnName,
      @SuppressWarnings("SameParameterValue") boolean nullable,
      MltMetadata.ScalarType type) {
    final var column = new MltMetadata.Column(columnName, new MltMetadata.ScalarField(type));
    column.isNullable = nullable;
    column.columnScope = MltMetadata.ColumnScope.FEATURE;
    return column;
  }

  private static MltMetadata.Column createScalarFieldScheme(
      String fieldName,
      @SuppressWarnings("SameParameterValue") boolean nullable,
      MltMetadata.ScalarType type) {
    final var column = new MltMetadata.Column(fieldName, new MltMetadata.ScalarField(type));
    column.isNullable = nullable;
    return column;
  }

  private static MltMetadata.Column createComplexColumnScheme(
      @SuppressWarnings("SameParameterValue") @Nullable String columnName,
      @SuppressWarnings("SameParameterValue") boolean nullable,
      @SuppressWarnings("SameParameterValue") MltMetadata.ComplexType type) {
    final var column = new MltMetadata.Column(columnName, new MltMetadata.ComplexField(type));
    column.isNullable = nullable;
    column.columnScope = MltMetadata.ColumnScope.FEATURE;
    return column;
  }

  private static MltMetadata.ComplexField createComplexColumn() {
    return new MltMetadata.ComplexField(MltMetadata.ComplexType.STRUCT);
  }

  private static MltMetadata.Column createColumn(
      String columnName, MltMetadata.ComplexField complexField) {
    final var column = new MltMetadata.Column(columnName, complexField);
    column.isNullable = false; // See `PropertyDecoder.decodePropertyColumn()`
    column.columnScope = MltMetadata.ColumnScope.FEATURE;
    return column;
  }
}
