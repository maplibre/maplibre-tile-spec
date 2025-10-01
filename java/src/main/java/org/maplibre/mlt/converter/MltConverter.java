package org.maplibre.mlt.converter;

import com.google.gson.Gson;
import jakarta.annotation.Nullable;
import java.io.ByteArrayOutputStream;
import java.io.DataOutputStream;
import java.io.IOException;
import java.net.URI;
import java.util.*;
import java.util.stream.Collectors;
import java.util.stream.Stream;
import org.apache.commons.lang3.tuple.Pair;
import org.apache.commons.lang3.tuple.Triple;
import org.maplibre.mlt.converter.encodings.EncodingUtils;
import org.maplibre.mlt.converter.encodings.GeometryEncoder;
import org.maplibre.mlt.converter.encodings.MltTypeMap;
import org.maplibre.mlt.converter.encodings.PropertyEncoder;
import org.maplibre.mlt.converter.mvt.ColumnMapping;
import org.maplibre.mlt.converter.mvt.MapboxVectorTile;
import org.maplibre.mlt.data.Feature;
import org.maplibre.mlt.metadata.stream.PhysicalLevelTechnique;
import org.maplibre.mlt.metadata.tileset.MltTilesetMetadata;

public class MltConverter {
  public static MltTilesetMetadata.TileSetMetadata createTilesetMetadata(
      MapboxVectorTile tile, Collection<ColumnMapping> columnMappings, boolean isIdPresent) {
    return createTilesetMetadata(tile, columnMappings, isIdPresent, false, false);
  }

  public static MltTilesetMetadata.TileSetMetadata createTilesetMetadata(
      MapboxVectorTile tile,
      Collection<ColumnMapping> columnMappings,
      boolean isIdPresent,
      boolean enableCoerceOnMismatch,
      boolean enableElideOnMismatch) {

    // TODO: Allow determining whether ID is present automatically
    // TODO: Allow nullable ID columns

    var tilesetBuilder = MltTilesetMetadata.TileSetMetadata.newBuilder();

    for (var layer : tile.layers()) {
      final LinkedHashMap<String, MltTilesetMetadata.Column> columnSchemas = new LinkedHashMap<>();
      final LinkedHashMap<String, MltTilesetMetadata.ComplexColumn.Builder>
          complexPropertyColumnSchemas = new LinkedHashMap<>();

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

        if (isIdPresent && (feature.id() > Integer.MAX_VALUE || feature.id() < Integer.MIN_VALUE)) {
          hasLongId = true;
        }
        featureIndex++;
      }

      for (var complexPropertyColumnScheme : complexPropertyColumnSchemas.entrySet()) {
        columnSchemas.put(
            complexPropertyColumnScheme.getKey(),
            createColumn(
                complexPropertyColumnScheme.getKey(),
                complexPropertyColumnScheme.getValue().build()));
      }

      var featureTableSchemaBuilder =
          MltTilesetMetadata.FeatureTableSchema.newBuilder().setName(layer.name());

      // If present, `id` must be the first column
      if (columnSchemas.containsKey(PropertyEncoder.ID_COLUMN_NAME)) {
        throw new RuntimeException("Unexpected ID Column");
      }
      if (isIdPresent) {
        featureTableSchemaBuilder.addColumns(
            MltTilesetMetadata.Column.newBuilder()
                .setName(PropertyEncoder.ID_COLUMN_NAME)
                .setNullable(false)
                .setColumnScope(MltTilesetMetadata.ColumnScope.FEATURE)
                .setScalarType(
                    MltTilesetMetadata.ScalarColumn.newBuilder()
                        .setLogicalType(MltTilesetMetadata.LogicalScalarType.ID)
                        .setLongID(hasLongId)
                        .build())
                .build());
      }

      // The `geometry` column is mandatory and has to be the first column after `ID`
      featureTableSchemaBuilder.addColumns(
          createComplexColumnScheme(
              PropertyEncoder.GEOMETRY_COLUMN_NAME,
              false,
              MltTilesetMetadata.ComplexType.GEOMETRY));

      columnSchemas.values().forEach(featureTableSchemaBuilder::addColumns);
      tilesetBuilder.addFeatureTables(featureTableSchemaBuilder.build());
    }

    return tilesetBuilder.build();
  }

  private static void resolveColumnType(
      Map.Entry<String, Object> property,
      String layerName,
      int featureIndex,
      Collection<ColumnMapping> columnMappings,
      LinkedHashMap<String, MltTilesetMetadata.Column> columnSchemas,
      LinkedHashMap<String, MltTilesetMetadata.ComplexColumn.Builder> complexPropertyColumnSchemas,
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
      if (previousSchema.hasScalarType()) {
        final var previousScalarType = previousSchema.getScalarType();
        if (previousScalarType.hasPhysicalType()) {
          final var previousPhysicalType = previousScalarType.getPhysicalType();
          if (previousPhysicalType != scalarType) {
            if (enableCoerceOnMismatch) {
              if (previousPhysicalType != MltTilesetMetadata.ScalarType.STRING) {
                columnSchemas.put(
                    mvtPropertyName,
                    createScalarColumnScheme(
                        mvtPropertyName, true, MltTilesetMetadata.ScalarType.STRING));
              }
            } else if (!enableElideOnMismatch) {
              throw new RuntimeException(
                  "Layer '"
                      + layerName
                      + "' Feature index "
                      + featureIndex
                      + " Property '"
                      + property.getKey()
                      + "' has different type: "
                      + scalarType.name()
                      + " vs. "
                      + previousPhysicalType.name());
            }
          }
        }
      }
      return;
    }

    if (!columnMappings.isEmpty()) {
      // TODO: refactor quick and dirty solution -> simplify that complex logic
      if (columnMappings.stream().anyMatch(m -> mvtPropertyName.equals(m.mvtPropertyPrefix()))
          && !complexPropertyColumnSchemas.containsKey(mvtPropertyName)) {
        /* case where the top-level field is present like name (name:de, name:us, ...) and has a value.
         * In this case the field is mapped to the name default. */
        final var childField = createScalarFieldScheme("default", true, scalarType);
        final var fieldMetadataBuilder = createComplexColumnBuilder(childField);
        complexPropertyColumnSchemas.put(mvtPropertyName, fieldMetadataBuilder);
        return;
      } else if (columnMappings.stream()
              .anyMatch(m -> mvtPropertyName.equals(m.mvtPropertyPrefix()))
          && complexPropertyColumnSchemas.containsKey(mvtPropertyName)
          && complexPropertyColumnSchemas.get(mvtPropertyName).getChildrenList().stream()
              .noneMatch(c -> c.getName().equals("default"))) {
        /* Case where the top-level field such as name is not present in the first feature */
        final var childField = createScalarFieldScheme("default", true, scalarType);
        final var columnMapping =
            columnMappings.stream()
                .filter(m -> mvtPropertyName.equals(m.mvtPropertyPrefix()))
                .findFirst()
                .orElseThrow();
        complexPropertyColumnSchemas.get(columnMapping.mvtPropertyPrefix()).addChildren(childField);
        return;
      } else if (columnMappings.stream()
          .anyMatch(
              m ->
                  mvtPropertyName.contains(
                      m.mvtPropertyPrefix() + Settings.MLT_CHILD_FIELD_SEPARATOR))) {
        final var columnMapping =
            columnMappings.stream()
                .filter(
                    m ->
                        mvtPropertyName.contains(
                            m.mvtPropertyPrefix() + Settings.MLT_CHILD_FIELD_SEPARATOR))
                .findFirst()
                .orElseThrow();
        final var columnName = columnMapping.mvtPropertyPrefix();

        final var fieldNames = mvtPropertyName.split(Settings.MLT_CHILD_FIELD_SEPARATOR);
        /* There are cases with double nested property names like name_ja_kana */
        final var fieldName =
            Arrays.stream(fieldNames)
                .skip(1)
                .collect(Collectors.joining(Settings.MLT_CHILD_FIELD_SEPARATOR));
        final var children = createScalarFieldScheme(fieldName, true, scalarType);
        if (complexPropertyColumnSchemas.containsKey(columnName)) {
          /* add the nested properties to the parent like the name:* properties to the name parent struct */
          if (complexPropertyColumnSchemas.get(columnName).getChildrenList().stream()
              .noneMatch(c -> c.getName().equals(fieldName))) {
            complexPropertyColumnSchemas.get(columnName).addChildren(children);
          }
        } else {
          /* Case where there is no explicit property available which serves as the name
           * for the top-level field. For example there is no name property only name:* */
          final var complexColumnBuilder = createComplexColumnBuilder(children);
          complexPropertyColumnSchemas.put(columnName, complexColumnBuilder);
        }
        return;
      }
    }

    var columnScheme = createScalarColumnScheme(mvtPropertyName, true, scalarType);
    columnSchemas.put(mvtPropertyName, columnScheme);
  }

  public static String createTilesetMetadataJSON(MltTilesetMetadata.TileSetMetadata pbMetadata) {
    var root = new TreeMap<String, Object>();
    final int version = 1;
    root.put("version", version);
    if (pbMetadata.hasName()) {
      root.put("name", pbMetadata.getName());
    }
    if (pbMetadata.hasDescription()) {
      root.put("description", pbMetadata.getDescription());
    }
    if (pbMetadata.hasAttribution()) {
      root.put("attribution", pbMetadata.getAttribution());
    }
    if (pbMetadata.hasMinZoom()) {
      root.put("minZoom", pbMetadata.getMinZoom());
    }
    if (pbMetadata.hasMaxZoom()) {
      root.put("maxZoom", pbMetadata.getMaxZoom());
    }

    var bounds = new ArrayList<Map<String, Object>>();
    for (int i = 0; i < (pbMetadata.getBoundsCount() / 4); ++i) {
      var bound = new TreeMap<String, Object>();
      bound.put("left", pbMetadata.getBounds(4 * i));
      bound.put("top", pbMetadata.getBounds((4 * i) + 1));
      bound.put("right", pbMetadata.getBounds((4 * i) + 2));
      bound.put("bottom", pbMetadata.getBounds((4 * i) + 3));
      bounds.add(bound);
    }
    if (!bounds.isEmpty()) {
      root.put("bounds", bounds);
    }

    var centers = new ArrayList<Map<String, Object>>();
    for (int i = 0; i < (pbMetadata.getCenterCount() / 2); ++i) {
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
      @Nullable MltTilesetMetadata.ScalarType physicalScalarType,
      @Nullable MltTilesetMetadata.LogicalScalarType logicalScalarType,
      @Nullable MltTilesetMetadata.ComplexType physicalComplexType,
      @Nullable MltTilesetMetadata.LogicalComplexType logicalComplexType,
      @Nullable List<MltTilesetMetadata.Field> children)
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
        final boolean complex = child.hasComplexField();
        final boolean logical =
            (complex && child.getComplexField().hasLogicalType())
                || (!complex && child.getScalarField().hasLogicalType());

        writeColumnOrFieldType(
            stream,
            child.getName(),
            child.getNullable(),
            /* hasLongIDs= */ false,
            (!complex && !logical) ? child.getScalarField().getPhysicalType() : null,
            (!complex && logical) ? child.getScalarField().getLogicalType() : null,
            (complex && !logical) ? child.getComplexField().getPhysicalType() : null,
            (complex && logical) ? child.getComplexField().getLogicalType() : null,
            complex ? child.getComplexField().getChildrenList() : null);
      }
    }
  }

  /// Produce the binary tile header containing the tile metadata
  /// <p>Note: Uses the protobuf format as input to avoid repeating the logic there, could be
  /// refactored to eliminate it</p>
  public static byte[] createEmbeddedMetadata(MltTilesetMetadata.FeatureTableSchema table)
      throws IOException {
    try (var byteStream = new ByteArrayOutputStream()) {
      try (var dataStream = new DataOutputStream(byteStream)) {
        EncodingUtils.putString(dataStream, table.getName());
        EncodingUtils.putVarInt(dataStream, table.getColumnsCount());
        for (var column : table.getColumnsList()) {
          if (column.getColumnScope() != MltTilesetMetadata.ColumnScope.FEATURE) {
            throw new RuntimeException("Vertex scoped properties are not yet supported");
          }
          // Note: this is ugly because `Column` is nearly, but not quite, identical to `Field` but
          // does not inherit from it
          writeColumnOrFieldType(
              dataStream,
              column.getName(),
              column.getNullable(),
              column.hasScalarType() && column.getScalarType().getLongID(),
              column.hasScalarType() && column.getScalarType().hasPhysicalType()
                  ? column.getScalarType().getPhysicalType()
                  : null,
              column.hasScalarType() && column.getScalarType().hasLogicalType()
                  ? column.getScalarType().getLogicalType()
                  : null,
              column.hasComplexType() && column.getComplexType().hasPhysicalType()
                  ? column.getComplexType().getPhysicalType()
                  : null,
              column.hasComplexType() && column.getComplexType().hasLogicalType()
                  ? column.getComplexType().getLogicalType()
                  : null,
              column.hasComplexType() ? column.getComplexType().getChildrenList() : null);
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
      MltTilesetMetadata.TileSetMetadata tilesetMetadata,
      ConversionConfig config,
      @Nullable URI tessellateSource)
      throws IOException {
    return convertMvt(mvt, tilesetMetadata, config, tessellateSource, null);
  }

  public static byte[] convertMvt(
      MapboxVectorTile mvt,
      MltTilesetMetadata.TileSetMetadata tilesetMetadata,
      ConversionConfig config,
      @Nullable URI tessellateSource,
      @Nullable HashMap<String, Triple<byte[], byte[], String>> rawStreamData)
      throws IOException {

    // Convert the list of metadatas (one per layer) into a lookup by the first and only layer name
    // We assume that the names are unique.
    final var metaMap =
        tilesetMetadata.getFeatureTablesList().stream()
            .collect(
                Collectors.toMap(
                    MltTilesetMetadata.FeatureTableSchema::getName,
                    table -> table,
                    (existing, replacement) -> {
                      throw new RuntimeException("duplicate key");
                    }));

    var physicalLevelTechnique =
        config.getUseAdvancedEncodingSchemes()
            ? PhysicalLevelTechnique.FAST_PFOR
            : PhysicalLevelTechnique.VARINT;

    var mapLibreTileBuffer = new byte[0];
    for (var mvtLayer : mvt.layers()) {
      final var layerMetadata = metaMap.get(mvtLayer.name());
      if (layerMetadata == null) {
        throw new RuntimeException("Missing Metadata");
      }

      final var featureTableName = mvtLayer.name();
      final var mvtFeatures = mvtLayer.features();
      final var featureTableOptimizations =
          config.getOptimizations() == null
              ? null
              : config.getOptimizations().get(featureTableName);

      final var createPolygonOutline =
          config.getOutlineFeatureTableNames().contains(featureTableName)
              || config.getOutlineFeatureTableNames().contains("*");
      final var result =
          sortFeaturesAndEncodeGeometryColumn(
              config,
              featureTableOptimizations,
              mvtFeatures,
              mvtFeatures,
              physicalLevelTechnique,
              createPolygonOutline,
              tessellateSource,
              rawStreamData);
      final var sortedFeatures = result.getLeft();
      final var encodedGeometryColumn = result.getRight();
      final var encodedGeometryFieldMetadata =
          EncodingUtils.encodeVarint(encodedGeometryColumn.numStreams(), false);

      var encodedPropertyColumns =
          encodePropertyColumns(
              config, layerMetadata, sortedFeatures, featureTableOptimizations, rawStreamData);

      var featureTableBodyBuffer = new byte[0];
      if (config.getIncludeIds()) {
        final var idMetadata =
            layerMetadata.getColumnsList().stream()
                .filter(f -> f.getName().equals(PropertyEncoder.ID_COLUMN_NAME))
                .findFirst()
                .orElseThrow();

        // Write ID as a 32- or 64-bit scalar depending on the flag stored in the column metadata.
        // The decoding assumes unsigned (no zigzag)
        final var rawType =
            idMetadata.getScalarType().getLongID()
                ? MltTilesetMetadata.ScalarType.UINT_64
                : MltTilesetMetadata.ScalarType.UINT_32;
        final var scalarColumnMetadata =
            MltTilesetMetadata.Column.newBuilder()
                .setName(idMetadata.getName())
                .setNullable(idMetadata.getNullable())
                .setColumnScope(MltTilesetMetadata.ColumnScope.FEATURE)
                .setScalarType(
                    MltTilesetMetadata.ScalarColumn.newBuilder().setPhysicalType(rawType))
                .build();
        featureTableBodyBuffer =
            PropertyEncoder.encodeScalarPropertyColumn(
                scalarColumnMetadata,
                sortedFeatures,
                physicalLevelTechnique,
                config.getUseAdvancedEncodingSchemes(),
                config.getCoercePropertyValues(),
                rawStreamData);
      }

      featureTableBodyBuffer =
          CollectionUtils.concatByteArrays(
              featureTableBodyBuffer,
              encodedGeometryFieldMetadata,
              encodedGeometryColumn.encodedValues(),
              encodedPropertyColumns);

      final var encodedFeatureTableInfo = EncodingUtils.encodeVarint(mvtLayer.tileExtent(), false);
      final var metadataBuffer = createEmbeddedMetadata(layerMetadata);

      final var tag = 1;
      final var tagBuffer = EncodingUtils.encodeVarint(tag, false);
      final var tagLength =
          tagBuffer.length
              + metadataBuffer.length
              + encodedFeatureTableInfo.length
              + featureTableBodyBuffer.length;

      mapLibreTileBuffer =
          CollectionUtils.concatByteArrays(
              mapLibreTileBuffer,
              EncodingUtils.encodeVarint(tagLength, false),
              tagBuffer,
              metadataBuffer,
              encodedFeatureTableInfo,
              featureTableBodyBuffer);
    }

    return mapLibreTileBuffer;
  }

  private static byte[] encodePropertyColumns(
      ConversionConfig config,
      MltTilesetMetadata.FeatureTableSchema featureTableMetadata,
      List<Feature> sortedFeatures,
      FeatureTableOptimizations featureTableOptimizations,
      @Nullable HashMap<String, Triple<byte[], byte[], String>> rawStreamData)
      throws IOException {
    final var propertyColumns = filterPropertyColumns(featureTableMetadata);
    final List<ColumnMapping> columnMappings =
        (featureTableOptimizations != null)
            ? featureTableOptimizations.columnMappings()
            : List.of();
    return PropertyEncoder.encodePropertyColumns(
        propertyColumns,
        sortedFeatures,
        config.getUseAdvancedEncodingSchemes(),
        config.getCoercePropertyValues(),
        columnMappings,
        rawStreamData);
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
          @Nullable HashMap<String, Triple<byte[], byte[], String>> rawStreamData)
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

    var isColumnSortable =
        config.getIncludeIds()
            && featureTableOptimizations != null
            && featureTableOptimizations.allowSorting();
    if (isColumnSortable && !featureTableOptimizations.allowIdRegeneration()) {
      sortedFeatures = sortFeaturesById(mvtFeatures);
    }

    var ids = sortedFeatures.stream().map(Feature::id).collect(Collectors.toList());
    var geometries = sortedFeatures.stream().map(Feature::geometry).collect(Collectors.toList());

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
                rawStreamData)
            : GeometryEncoder.encodeGeometryColumn(
                geometries,
                physicalLevelTechnique,
                sortSettings,
                config.getUseMortonEncoding(),
                rawStreamData);

    if (encodedGeometryColumn.geometryColumnSorted()) {
      sortedFeatures =
          ids.stream()
              .map(id -> mvtFeatures.stream().filter(fe -> fe.id() == id).findFirst().orElseThrow())
              .collect(Collectors.toList());
    }

    if (config.getIncludeIds()
        && featureTableOptimizations != null
        && featureTableOptimizations.allowIdRegeneration()) {
      sortedFeatures = generateSequenceIds(sortedFeatures);
    }

    return Pair.of(sortedFeatures, encodedGeometryColumn);
  }

  private static List<MltTilesetMetadata.Column> filterPropertyColumns(
      MltTilesetMetadata.FeatureTableSchema featureTableMetadata) {
    return featureTableMetadata.getColumnsList().stream()
        .filter(
            f ->
                !f.getName().equals(PropertyEncoder.ID_COLUMN_NAME)
                    && !f.getName().equals(PropertyEncoder.GEOMETRY_COLUMN_NAME))
        .collect(Collectors.toList());
  }

  private static List<Feature> sortFeaturesById(List<Feature> features) {
    return features.stream()
        .sorted(Comparator.comparingLong(Feature::id))
        .collect(Collectors.toList());
  }

  private static List<Feature> generateSequenceIds(List<Feature> features) {
    var sortedFeatures = new ArrayList<Feature>();
    var idCounter = 0;
    for (var feature : features) {
      sortedFeatures.add(new Feature(idCounter++, feature.geometry(), feature.properties()));
    }
    return sortedFeatures;
  }

  private static MltTilesetMetadata.ScalarType getScalarType(Map.Entry<String, Object> property) {
    var propertyValue = property.getValue();
    if (propertyValue instanceof Boolean) {
      return MltTilesetMetadata.ScalarType.BOOLEAN;
    }
    // TODO: also handle unsigned int to avoid zigZag coding
    // TODO: quick and dirty fix for wrong data types -> make proper solution
    else if (propertyValue instanceof Integer || propertyValue instanceof Long) {
      return MltTilesetMetadata.ScalarType.INT_32;
    }
    // TODO: also handle unsigned long to avoid zigZag coding
    /*else if (propertyValue instanceof Long) {
      return MltTilesetMetadata.ScalarType.INT_64;
    }*/ else if (propertyValue instanceof Float) {
      return MltTilesetMetadata.ScalarType.FLOAT;
    } else if (propertyValue instanceof Double) {
      return MltTilesetMetadata.ScalarType.DOUBLE;
    } else if (propertyValue instanceof String) {
      return MltTilesetMetadata.ScalarType.STRING;
    }

    throw new IllegalArgumentException("Specified data type currently not supported.");
  }

  private static MltTilesetMetadata.Column createScalarColumnScheme(
      String columnName,
      @SuppressWarnings("SameParameterValue") boolean nullable,
      MltTilesetMetadata.ScalarType type) {
    var scalarColumn = MltTilesetMetadata.ScalarColumn.newBuilder().setPhysicalType(type);
    return MltTilesetMetadata.Column.newBuilder()
        .setName(columnName)
        .setNullable(nullable)
        .setColumnScope(MltTilesetMetadata.ColumnScope.FEATURE)
        .setScalarType(scalarColumn)
        .build();
  }

  private static MltTilesetMetadata.Field createScalarFieldScheme(
      String fieldName,
      @SuppressWarnings("SameParameterValue") boolean nullable,
      MltTilesetMetadata.ScalarType type) {
    var scalarField = MltTilesetMetadata.ScalarField.newBuilder().setPhysicalType(type);
    return MltTilesetMetadata.Field.newBuilder()
        .setName(fieldName)
        .setNullable(nullable)
        .setScalarField(scalarField)
        .build();
  }

  private static MltTilesetMetadata.Column createComplexColumnScheme(
      @SuppressWarnings("SameParameterValue") String columnName,
      @SuppressWarnings("SameParameterValue") boolean nullable,
      @SuppressWarnings("SameParameterValue") MltTilesetMetadata.ComplexType type) {
    var complexColumn = MltTilesetMetadata.ComplexColumn.newBuilder().setPhysicalType(type).build();
    return MltTilesetMetadata.Column.newBuilder()
        .setName(columnName)
        .setNullable(nullable)
        .setColumnScope(MltTilesetMetadata.ColumnScope.FEATURE)
        .setComplexType(complexColumn)
        .build();
  }

  private static MltTilesetMetadata.ComplexColumn.Builder createComplexColumnBuilder(
      MltTilesetMetadata.Field children) {
    return MltTilesetMetadata.ComplexColumn.newBuilder()
        .setPhysicalType(MltTilesetMetadata.ComplexType.STRUCT)
        .addChildren(children);
  }

  private static MltTilesetMetadata.Column createColumn(
      String columnName, MltTilesetMetadata.ComplexColumn complexColumn) {
    return MltTilesetMetadata.Column.newBuilder()
        .setName(columnName)
        .setNullable(false) // See `PropertyDecoder.decodePropertyColumn()`
        .setColumnScope(MltTilesetMetadata.ColumnScope.FEATURE)
        .setComplexType(complexColumn)
        .build();
  }
}
