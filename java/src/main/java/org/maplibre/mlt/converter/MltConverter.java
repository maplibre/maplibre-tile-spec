package org.maplibre.mlt.converter;

import com.google.gson.Gson;
import java.io.ByteArrayOutputStream;
import java.io.DataOutputStream;
import java.io.IOException;
import java.io.InputStream;
import java.net.URI;
import java.util.*;
import java.util.stream.Collectors;
import javax.annotation.Nullable;
import org.apache.commons.lang3.tuple.Pair;
import org.apache.commons.lang3.tuple.Triple;
import org.maplibre.mlt.converter.encodings.EncodingUtils;
import org.maplibre.mlt.converter.encodings.GeometryEncoder;
import org.maplibre.mlt.converter.encodings.PropertyEncoder;
import org.maplibre.mlt.converter.mvt.ColumnMapping;
import org.maplibre.mlt.converter.mvt.MapboxVectorTile;
import org.maplibre.mlt.data.Feature;
import org.maplibre.mlt.decoder.DecodingUtils;
import org.maplibre.mlt.metadata.stream.PhysicalLevelTechnique;
import org.maplibre.mlt.metadata.tileset.MltTilesetMetadata;

public class MltConverter {
  private static final byte VERSION = 1;
  private static final String ID_COLUMN_NAME = "id";
  private static final String GEOMETRY_COLUMN_NAME = "geometry";

  /*
   * The tileset metadata are serialized to a separate protobuf file.
   * This metadata file holds the scheme for the complete tileset, so it only has to be requested once from a map client for
   * the full session because it contains the scheme information for all tiles.
   * This is a POC approach for generating the MLT metadata from a MVT tile.
   * Possible approaches in production:
   * - Generate the metadata scheme from the MVT TileJson file -> but not all types of MLT (like float, double, ...)
   *   are available in the TileJSON file
   * - Use the TileJSON as the base to get all property names and request as many tiles as needed to get the concrete data types
   * - Define a separate scheme file where all information are contained
   * To bring the flattened MVT properties into a nested structure it has to have the following structure:
   * propertyPrefix|Delimiter|propertySuffix -> Example: name, name:us, name:en
   * */
  public static Collection<MltTilesetMetadata.TileSetMetadata> createTilesetMetadata(
      MapboxVectorTile tile,
      @SuppressWarnings("OptionalUsedAsFieldOrParameterType")
          Optional<List<ColumnMapping>> ignoredColumnMappings,
      boolean isIdPresent) {
    return createTilesetMetadata(tile, isIdPresent);
  }

  public static Collection<MltTilesetMetadata.TileSetMetadata> createTilesetMetadata(
      MapboxVectorTile tile, boolean isIdPresent) {

    final var featureTables = new ArrayList<MltTilesetMetadata.TileSetMetadata>();
    for (var layer : tile.layers()) {
      final LinkedHashMap<String, MltTilesetMetadata.Column> columnSchema = new LinkedHashMap<>();

      /* Create the scheme for the property columns by iterating over all features in the tile and collecting
       * information about the feature properties*/
      var isLongId = false;
      for (var feature : layer.features()) {
        // Accumulate the complete set of property names.
        // MVT can only contain scalar types.
        // The type is taken from the first value encountered, later types are ignored.
        // All columns are treated as optional.
        feature.properties().entrySet().stream()
            .sorted(Map.Entry.comparingByKey())
            .forEach(
                entry -> {
                  final var type = getScalarType(entry);
                  /*final var column = */ columnSchema.computeIfAbsent(
                      entry.getKey(), key -> createScalarColumnScheme(entry.getKey(), true, type));
                  // TODO: warn if multiple types map to the same column? fail?
                });

        if (isIdPresent
            && !isLongId
            && (feature.id() > Integer.MAX_VALUE || feature.id() < Integer.MIN_VALUE)) {
          isLongId = true;
        }
      }

      // If the `id` column needs to be created, or needs to be a different type, (re-)create it.
      // If none of the `id` values we encountered need 64 bits use a 32-bit column, as it can
      // currently be more efficiently encoded based on FastPFOR (64 bit variant not yet
      // implemented) instead of Varint and decoded faster in JS.
      if ((isIdPresent && !columnSchema.containsKey(ID_COLUMN_NAME))
          || (isLongId
              && columnSchema.get(ID_COLUMN_NAME).getScalarType().getPhysicalType()
                  != MltTilesetMetadata.ScalarType.INT_64)) {
        columnSchema.put(
            ID_COLUMN_NAME,
            createScalarColumnScheme(
                ID_COLUMN_NAME,
                false,
                isLongId
                    ? MltTilesetMetadata.ScalarType.UINT_64
                    : MltTilesetMetadata.ScalarType.UINT_32));
      }

      var tilesetBuilder = MltTilesetMetadata.TileSetMetadata.newBuilder();
      tilesetBuilder.setVersion(VERSION);

      var featureTableSchemaBuilder = MltTilesetMetadata.FeatureTableSchema.newBuilder();
      featureTableSchemaBuilder.setName(layer.name());

      // If present, `id` must be the first column
      var idColumn = columnSchema.get(ID_COLUMN_NAME);
      if (idColumn != null) {
        featureTableSchemaBuilder.addColumns(idColumn);
        columnSchema.remove(ID_COLUMN_NAME);
      }

      // The `geometry` column is mandatory and has to be the first column (after `id`)
      featureTableSchemaBuilder.addColumns(
          createComplexColumnScheme(
              GEOMETRY_COLUMN_NAME, false, MltTilesetMetadata.ComplexType.GEOMETRY));

      columnSchema.forEach((k, v) -> featureTableSchemaBuilder.addColumns(v));
      tilesetBuilder.addFeatureTables(featureTableSchemaBuilder.build());

      featureTables.add(tilesetBuilder.build());
    }

    return featureTables;
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
      boolean vertexScope,
      @Nullable MltTilesetMetadata.ScalarType physicalScalarType,
      @Nullable MltTilesetMetadata.LogicalScalarType logicalScalarType,
      @Nullable MltTilesetMetadata.ComplexType physicalComplexType,
      @Nullable MltTilesetMetadata.LogicalComplexType logicalComplexType,
      @Nullable List<MltTilesetMetadata.Field> children)
      throws IOException {
    final boolean isComplex = physicalComplexType != null || logicalComplexType != null;
    final boolean isLogical = logicalScalarType != null || logicalComplexType != null;
    final boolean hasChildren = (children != null && !children.isEmpty());

    final var options =
        (isNullable ? ColumnOptions.NULLABLE : 0)
            | (isComplex ? ColumnOptions.COMPLEX_TYPE : 0)
            | (isLogical ? ColumnOptions.LOGICAL_TYPE : 0)
            | (hasChildren ? ColumnOptions.HAS_CHILDREN : 0)
            | (vertexScope ? ColumnOptions.VERTEX_SCOPE : 0);
    EncodingUtils.putVarInt(stream, options);

    EncodingUtils.putString(stream, name);

    if (physicalScalarType != null) {
      EncodingUtils.putVarInt(stream, physicalScalarType.getNumber());
    } else if (physicalComplexType != null) {
      EncodingUtils.putVarInt(stream, physicalComplexType.getNumber());
    } else if (logicalScalarType != null) {
      EncodingUtils.putVarInt(stream, logicalScalarType.getNumber());
    } else if (logicalComplexType != null) {
      EncodingUtils.putVarInt(stream, logicalComplexType.getNumber());
    } else {
      throw new IllegalArgumentException("invalid type specification");
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
            /* vertexScope= */ false,
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
  public static byte[] createEmbeddedMetadata(MltTilesetMetadata.TileSetMetadata pbMetadata)
      throws IOException {
    try (var byteStream = new ByteArrayOutputStream()) {
      try (var dataStream = new DataOutputStream(byteStream)) {
        if (pbMetadata.getFeatureTablesCount() != 1) {
          throw new RuntimeException("not supported");
        }
        final var table = pbMetadata.getFeatureTables(0);

        EncodingUtils.putString(dataStream, table.getName());
        EncodingUtils.putVarInt(dataStream, table.getColumnsCount());
        for (var column : table.getColumnsList()) {
          writeColumnOrFieldType(
              dataStream,
              column.getName(),
              column.getNullable(),
              (column.getColumnScope() == MltTilesetMetadata.ColumnScope.VERTEX),
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

  static class FieldOptions {
    public static final int NULLABLE = 1;
    public static final int COMPLEX_TYPE = (1 << 1);
    public static final int LOGICAL_TYPE = (1 << 2);
    public static final int HAS_CHILDREN = (1 << 3);
  }

  static class ColumnOptions extends FieldOptions {
    public static final int VERTEX_SCOPE = (1 << 4);
  }

  private static void decodeField(InputStream stream, MltTilesetMetadata.Field.Builder field)
      throws IOException {
    field.setName(DecodingUtils.decodeString(stream));

    final var options = DecodingUtils.decodeVarint(stream);
    final boolean logical = ((options & FieldOptions.LOGICAL_TYPE) != 0);
    field.setNullable((options & FieldOptions.NULLABLE) != 0);

    final var type = DecodingUtils.decodeVarint(stream);
    if ((options & FieldOptions.LOGICAL_TYPE) != 0) {
      final var complexField = field.getComplexFieldBuilder();
      if (logical) {
        complexField.setLogicalTypeValue(type);
      } else {
        complexField.setPhysicalTypeValue(type);
      }
      if ((options & FieldOptions.HAS_CHILDREN) != 0) {
        final var childCount = DecodingUtils.decodeVarint(stream);
        for (int i = 0; i < childCount; ++i) {
          decodeField(stream, complexField.addChildrenBuilder());
        }
      }
    } else {
      final var scalarType = field.getScalarFieldBuilder();
      if (logical) {
        scalarType.setLogicalTypeValue(type);
      } else {
        scalarType.setPhysicalTypeValue(type);
      }
    }
  }

  private static void decodeColumn(
      InputStream stream, int options, MltTilesetMetadata.Column.Builder column)
      throws IOException {
    final boolean logical = ((options & ColumnOptions.LOGICAL_TYPE) != 0);
    final var type = DecodingUtils.decodeVarint(stream);
    if ((options & ColumnOptions.COMPLEX_TYPE) != 0) {
      final var complexType = column.getComplexTypeBuilder();
      if (logical) {
        complexType.setLogicalTypeValue(type);
      } else {
        complexType.setPhysicalTypeValue(type);
      }
      if ((options & ColumnOptions.HAS_CHILDREN) != 0) {
        final var childCount = DecodingUtils.decodeVarint(stream);
        for (int i = 0; i < childCount; ++i) {
          decodeField(stream, complexType.addChildrenBuilder());
        }
      }
    } else {
      final var scalarType = column.getScalarTypeBuilder();
      if (logical) {
        scalarType.setLogicalTypeValue(type);
      } else {
        scalarType.setPhysicalTypeValue(type);
      }
    }
  }

  public static MltTilesetMetadata.TileSetMetadata parseEmbeddedMetadata(InputStream stream) {
    try {
      final var result = MltTilesetMetadata.TileSetMetadata.newBuilder();
      while (stream.available() > 0) {
        final var table = result.addFeatureTablesBuilder();
        table.setName(DecodingUtils.decodeString(stream));
        final var columnCount = DecodingUtils.decodeVarint(stream);
        for (int i = 0; i < columnCount; ++i) {
          final var columnOptions = DecodingUtils.decodeVarint(stream);
          final boolean vertexScope = (columnOptions & ColumnOptions.VERTEX_SCOPE) != 0;
          final var column = table.addColumnsBuilder();
          column.setName(DecodingUtils.decodeString(stream));
          column.setNullable((columnOptions & ColumnOptions.NULLABLE) != 0);
          column.setColumnScope(
              vertexScope
                  ? MltTilesetMetadata.ColumnScope.VERTEX
                  : MltTilesetMetadata.ColumnScope.FEATURE);
          decodeColumn(stream, columnOptions, column);
        }
      }
      return result.build();
    } catch (IOException ignore) {
      return null;
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
      Collection<MltTilesetMetadata.TileSetMetadata> tilesetMetadata,
      ConversionConfig config,
      @Nullable URI tessellateSource)
      throws IOException {
    return convertMvt(mvt, tilesetMetadata, config, tessellateSource, null);
  }

  public static byte[] convertMvt(
      MapboxVectorTile mvt,
      Collection<MltTilesetMetadata.TileSetMetadata> tilesetMetadatas,
      ConversionConfig config,
      @Nullable URI tessellateSource,
      @Nullable HashMap<String, Triple<byte[], byte[], String>> rawStreamData)
      throws IOException {

    // Convert the list of metadatas (one per layer) into a lookup by the first and only layer name
    // We assume that the names are unique.
    final var metaMap =
        tilesetMetadatas.stream()
            .collect(
                Collectors.toMap(
                    meta -> meta.getFeatureTables(0).getName(),
                    meta -> meta,
                    (existing, replacement) -> {
                      throw new RuntimeException("duplicate key");
                    }));

    var physicalLevelTechnique =
        config.getUseAdvancedEncodingSchemes()
            ? PhysicalLevelTechnique.FAST_PFOR
            : PhysicalLevelTechnique.VARINT;

    var mapLibreTileBuffer = new byte[0];
    for (var mvtLayer : mvt.layers()) {
      final var tilesetMetadata = metaMap.get(mvtLayer.name());
      if (tilesetMetadata == null) {
        throw new RuntimeException("Missing Metadata");
      }

      var tag = 1;
      if (mvtLayer.tileExtent() != 4096) {
        tag = 2;
      }

      var featureTableBodyBuffer = new byte[0];
      final var featureTableName = mvtLayer.name();
      final var mvtFeatures = mvtLayer.features();
      final var featureTableMetadata = tilesetMetadata.getFeatureTables(0);

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
              config,
              featureTableMetadata,
              sortedFeatures,
              featureTableOptimizations,
              rawStreamData);

      if (config.getIncludeIds()) {
        final var idMetadata =
            featureTableMetadata.getColumnsList().stream()
                .filter(f -> f.getName().equals(ID_COLUMN_NAME))
                .findFirst()
                .orElseThrow();

        featureTableBodyBuffer =
            PropertyEncoder.encodeScalarPropertyColumn(
                idMetadata,
                sortedFeatures,
                physicalLevelTechnique,
                config.getUseAdvancedEncodingSchemes(),
                rawStreamData);
      }

      featureTableBodyBuffer =
          CollectionUtils.concatByteArrays(
              featureTableBodyBuffer,
              encodedGeometryFieldMetadata,
              encodedGeometryColumn.encodedValues(),
              encodedPropertyColumns);

      final var encodedFeatureTableInfo =
          (tag == 1) ? new byte[0] : EncodingUtils.encodeVarint(mvtLayer.tileExtent(), false);

      final var metadataBuffer = createEmbeddedMetadata(tilesetMetadata);
      final var tagLength =
          metadataBuffer.length + encodedFeatureTableInfo.length + featureTableBodyBuffer.length;

      mapLibreTileBuffer =
          CollectionUtils.concatByteArrays(
              mapLibreTileBuffer,
              EncodingUtils.encodeVarint(tag, false),
              EncodingUtils.encodeVarint(tagLength, false),
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
    var propertyColumns = filterPropertyColumns(featureTableMetadata);
    return PropertyEncoder.encodePropertyColumns(
        propertyColumns,
        sortedFeatures,
        config.getUseAdvancedEncodingSchemes(),
        featureTableOptimizations != null
            ? featureTableOptimizations.columnMappings()
            : Optional.empty(),
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
          @Nullable HashMap<String, Triple<byte[], byte[], String>> rawStreamData) {
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
            f -> !f.getName().equals(ID_COLUMN_NAME) && !f.getName().equals(GEOMETRY_COLUMN_NAME))
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
      String columnName, boolean nullable, MltTilesetMetadata.ScalarType type) {
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
        .setNullable(true)
        .setColumnScope(MltTilesetMetadata.ColumnScope.FEATURE)
        .setComplexType(complexColumn)
        .build();
  }
}
