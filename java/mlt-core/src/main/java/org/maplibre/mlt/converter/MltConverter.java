package org.maplibre.mlt.converter;

import com.google.gson.Gson;
import jakarta.annotation.Nullable;
import java.io.ByteArrayOutputStream;
import java.io.DataOutputStream;
import java.io.IOException;
import java.io.OutputStream;
import java.net.URI;
import java.util.ArrayList;
import java.util.Collection;
import java.util.Comparator;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;
import java.util.Objects;
import java.util.SequencedCollection;
import java.util.TreeMap;
import java.util.function.Function;
import java.util.stream.Collectors;
import java.util.stream.IntStream;
import java.util.stream.Stream;
import org.apache.commons.lang3.NotImplementedException;
import org.apache.commons.lang3.StringUtils;
import org.apache.commons.lang3.tuple.Pair;
import org.jetbrains.annotations.NotNull;
import org.maplibre.mlt.converter.encodings.EncodingUtils;
import org.maplibre.mlt.converter.encodings.GeometryEncoder;
import org.maplibre.mlt.converter.encodings.MltTypeMap;
import org.maplibre.mlt.converter.encodings.PropertyEncoder;
import org.maplibre.mlt.data.Feature;
import org.maplibre.mlt.data.Layer;
import org.maplibre.mlt.data.LayerSource;
import org.maplibre.mlt.data.Property;
import org.maplibre.mlt.metadata.stream.PhysicalLevelTechnique;
import org.maplibre.mlt.metadata.tileset.MltMetadata;
import org.maplibre.mlt.util.ByteArrayUtil;

public class MltConverter {
  /// Create tileset metadata from source data
  /// Note that this method will read through all features of all layers to infer the column
  /// data types, nullability, etc., it's preferable to construct the metadata from a schema.
  /// @param layerSource The input tile to create metadata from
  /// @param columnMappingConfig Optional column mapping configuration
  /// @param includeIdIfPresent Whether to include an ID column
  public static MltMetadata.TileSetMetadata createTilesetMetadata(
      @NotNull LayerSource layerSource,
      @Nullable ColumnMappingConfig columnMappingConfig,
      boolean includeIdIfPresent) {
    // TODO: Allow determining whether ID is present automatically
    return createTilesetMetadata(
        layerSource,
        ConversionConfig.TypeMismatchPolicy.FAIL,
        columnMappingConfig,
        includeIdIfPresent);
  }

  /// Create tileset metadata from source data
  /// See {@link #createTilesetMetadata(LayerSource, ColumnMappingConfig, boolean)}
  /// @param layerSource The input tile to create metadata from
  /// @param columnMappingConfig Optional column mapping configuration to be applied to all layers
  /// @param includeIdIfPresent Whether to include an ID column
  /// @param enableCoerceOnMismatch Whether to coerce values to string on type mismatch
  /// @param enableElideOnMismatch Whether to elide values on type mismatch (for each property, the
  /// first type encountered is used)
  public static MltMetadata.TileSetMetadata createTilesetMetadata(
      @NotNull LayerSource layerSource,
      @Nullable ColumnMappingConfig columnMappingConfig,
      boolean includeIdIfPresent,
      boolean enableCoerceOnMismatch,
      boolean enableElideOnMismatch) {
    final var config =
        ConversionConfig.builder()
            .mismatchPolicy(enableCoerceOnMismatch, enableElideOnMismatch)
            .build();
    return createTilesetMetadata(layerSource, config, columnMappingConfig, includeIdIfPresent);
  }

  /// Create tileset metadata from source data
  /// See {@link #createTilesetMetadata(LayerSource, ColumnMappingConfig, boolean)}
  /// @param layerSource The input tile to create metadata from
  /// @param config Optional configuration
  /// @param columnMappingConfig Optional column mapping configuration to be applied to all layers
  /// @param includeIdIfPresent Whether to include an ID column
  public static MltMetadata.TileSetMetadata createTilesetMetadata(
      @NotNull LayerSource layerSource,
      @Nullable ConversionConfig config,
      @Nullable ColumnMappingConfig columnMappingConfig,
      boolean includeIdIfPresent) {
    return createTilesetMetadata(
        layerSource,
        (config != null) ? config.getTypeMismatchPolicy() : null,
        columnMappingConfig,
        includeIdIfPresent);
  }

  /// Create tileset metadata from source data
  /// See {@link #createTilesetMetadata(LayerSource, ColumnMappingConfig, boolean)}
  /// @param layerSource The input tile to create metadata from
  /// @param config Optional configuration
  /// @param columnMappingConfig Optional column mapping configuration to be applied to all layers
  /// @param includeIdIfPresent Whether to include an ID column
  public static MltMetadata.TileSetMetadata createTilesetMetadata(
      @NotNull LayerSource layerSource,
      @NotNull ConversionConfig.TypeMismatchPolicy typeMismatchPolicy,
      @Nullable ColumnMappingConfig columnMappingConfig,
      boolean includeIdIfPresent) {
    final var tileset = new MltMetadata.TileSetMetadata();
    tileset.featureTables =
        layerSource
            .getLayerStream()
            .map(
                layer ->
                    createTilesetMetadata(
                        layer, typeMismatchPolicy, columnMappingConfig, includeIdIfPresent))
            .toList();
    return tileset;
  }

  private static MltMetadata.FeatureTable createTilesetMetadata(
      @NotNull Layer layer,
      @NotNull ConversionConfig.TypeMismatchPolicy typeMismatchPolicy,
      @Nullable ColumnMappingConfig columnMappingConfig,
      boolean includeIdIfPresent) {
    final LinkedHashMap<String, MltMetadata.Column> columnSchemas = new LinkedHashMap<>();
    final LinkedHashMap<ColumnMapping, MltMetadata.ComplexField> complexPropertyColumnSchemas =
        new LinkedHashMap<>();

    var hasId = false;
    var hasLongId = false;
    var hasNullId = false;
    var featureIndex = 0;
    for (var feature : layer.features()) {
      final var currentFeatureIndex = featureIndex;

      feature
          .getPropertyStream()
          .forEach(
              property -> {
                resolveColumnType(
                    property,
                    layer.name(),
                    currentFeatureIndex,
                    columnMappingConfig,
                    columnSchemas,
                    complexPropertyColumnSchemas,
                    typeMismatchPolicy);
              });

      if (includeIdIfPresent) {
        if (feature.hasId()) {
          hasId = true;
          if ((!hasLongId && feature.getId() > Integer.MAX_VALUE)
              || feature.getId() < Integer.MIN_VALUE) {
            hasLongId = true;
          }
        } else {
          hasNullId = true;
        }
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

    final var estimatedColumns = 2 + columnSchemas.size() + complexPropertyColumnSchemas.size();
    final var featureTableSchema = new MltMetadata.FeatureTable(layer.name(), estimatedColumns);

    // If present, `id` must be the first column
    if (columnSchemas.values().stream().anyMatch(MltTypeMap.Tag0x01::isID)) {
      throw new RuntimeException("Unexpected ID Column");
    }
    if (hasId) {
      featureTableSchema.columns.add(
          MltMetadata.columnBuilder().id(hasLongId).nullable(hasNullId).build());
    }

    // The `geometry` column is mandatory and has to be the first column after `ID`
    featureTableSchema.columns.add(
        createComplexColumnScheme(null, false, MltMetadata.ComplexType.GEOMETRY));

    // Add the remaining items in name order for consistent output.
    // Put complex columns after scalar columns to match old behavior.
    columnSchemas.values().stream()
        .sorted(
            Comparator.comparing((MltMetadata.Column c) -> (c.complexType != null) ? 1 : 0)
                .thenComparing(c -> c.name))
        .forEach(featureTableSchema.columns::add);

    return featureTableSchema;
  }

  private static void resolveColumnType(
      @NotNull Property property,
      @NotNull String layerName,
      int featureIndex,
      @Nullable ColumnMappingConfig columnMappingConfig,
      @NotNull LinkedHashMap<String, MltMetadata.Column> columnSchemas,
      @NotNull LinkedHashMap<ColumnMapping, MltMetadata.ComplexField> complexColumnSchemas,
      @NotNull ConversionConfig.TypeMismatchPolicy typeMismatchPolicy) {
    final var sourcePropertyName = property.getName();

    if (property.isNestedProperty()) {
      throw new NotImplementedException("Nested property types are not yet supported");
    }
    final var scalarType = property.getType();

    // If this property already has a column...
    final var previousSchema = columnSchemas.get(sourcePropertyName);
    if (previousSchema != null) {
      // Make sure the types match.
      // If not, coercion or nullification must be enabled, and replace
      // the column with a string column, if it isn't already.
      if (previousSchema.scalarType != null) {
        if (previousSchema.scalarType.physicalType != null) {
          final var prevPhysicalType = previousSchema.scalarType.physicalType;
          if (prevPhysicalType != scalarType) {
            final var newSchema = checkUpgrade(previousSchema, scalarType);
            if (newSchema != null) {
              if (newSchema != previousSchema) {
                columnSchemas.put(sourcePropertyName, newSchema);
              }
            } else if (typeMismatchPolicy == ConversionConfig.TypeMismatchPolicy.COERCE) {
              if (prevPhysicalType != MltMetadata.ScalarType.STRING) {
                columnSchemas.put(
                    sourcePropertyName,
                    MltMetadata.columnBuilder()
                        .name(previousSchema.name)
                        .scalar(MltMetadata.ScalarType.STRING)
                        .nullable(previousSchema.isNullable)
                        .scope(MltMetadata.ColumnScope.FEATURE)
                        .build());
              }
            } else if (typeMismatchPolicy != ConversionConfig.TypeMismatchPolicy.ELIDE) {
              throw new RuntimeException(
                  String.format(
                      "Layer '%s' Feature index %d Property '%s' has different type: %s / %s",
                      layerName,
                      featureIndex,
                      property.getName(),
                      scalarType.name(),
                      prevPhysicalType.name()));
            }
            return;
          }
        }
      }
    }

    if (columnMappingConfig != null) {
      final var columnMapping =
          ColumnMapping.findMapping(columnMappingConfig, layerName, sourcePropertyName);
      if (columnMapping != null) {
        // A mapping exists for this property.
        // Create the parent type and add a child type entry.
        final var parentColumn =
            complexColumnSchemas.computeIfAbsent(columnMapping, k -> createComplexColumn());

        if (parentColumn.children.stream().noneMatch(c -> c.name.equals(sourcePropertyName))) {
          parentColumn.children.add(createScalarFieldScheme(sourcePropertyName, true, scalarType));
        }

        return;
      }
    }

    // no matching column mappings, create a plain scalar column
    columnSchemas.put(
        sourcePropertyName, createScalarColumnScheme(sourcePropertyName, true, scalarType));
  }

  private static MltMetadata.Column checkUpgrade(
      MltMetadata.Column previousSchema, MltMetadata.ScalarType scalarType) {
    final var prevPhysicalType = previousSchema.scalarType.physicalType;

    if (prevPhysicalType == MltMetadata.ScalarType.INT_32
        && scalarType == MltMetadata.ScalarType.INT_64) {
      // Allow implicit upgrade from INT_32 to INT_64
      return MltMetadata.columnBuilder()
          .name(previousSchema.name)
          .scalar(MltMetadata.ScalarType.INT_64)
          .nullable(previousSchema.isNullable)
          .scope(MltMetadata.ColumnScope.FEATURE)
          .build();
    } else if (prevPhysicalType == MltMetadata.ScalarType.INT_64
        && scalarType == MltMetadata.ScalarType.INT_32) {
      // no-op
      // keep INT_64
      return previousSchema;
    } else if (prevPhysicalType == MltMetadata.ScalarType.FLOAT
        && scalarType == MltMetadata.ScalarType.DOUBLE) {
      // Allow implicit upgrade from FLOAT to DOUBLE
      return MltMetadata.columnBuilder()
          .name(previousSchema.name)
          .scalar(MltMetadata.ScalarType.DOUBLE)
          .nullable(previousSchema.isNullable)
          .scope(MltMetadata.ColumnScope.FEATURE)
          .build();
    } else if (prevPhysicalType == MltMetadata.ScalarType.DOUBLE
        && scalarType == MltMetadata.ScalarType.FLOAT) {
      // no-op
      // keep DOUBLE
      return previousSchema;
    }
    return null;
  }

  /// Resolve complex column mapping by determining common prefix and adjusting child names
  /// @return The longest common prefix which has been removed from the child field names (which may
  /// be blank)
  private static String resolveComplexColumnMapping(MltMetadata.ComplexField column) {
    final var prefix =
        StringUtils.getCommonPrefix(
            column.children.stream().map(c -> c.name).toArray(String[]::new));
    if (!prefix.isEmpty()) {
      column.children =
          column.children.stream()
              .map(
                  child -> {
                    final var name = child.name;
                    if (!name.startsWith(prefix)) {
                      throw new RuntimeException(
                          "Unexpected column mapping: prefix is not present");
                    }
                    return child.asFieldBuilder().name(name.substring(prefix.length())).build();
                  })
              .sorted(Comparator.comparing(f -> f.name))
              .toList();
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

    final var bounds = new ArrayList<Map<String, Object>>();
    if (pbMetadata.bounds.size() % 4 != 0) {
      throw new IllegalArgumentException("Invalid bounds length");
    }
    for (var iterator = pbMetadata.bounds.iterator(); iterator.hasNext(); ) {
      final var bound = new TreeMap<String, Object>();
      bound.put("left", iterator.next());
      bound.put("top", iterator.next());
      bound.put("right", iterator.next());
      bound.put("bottom", iterator.next());
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
   * Converts a collection of layers into an MLT tile
   *
   * @param sourceLayers The input layers
   * @param tilesetMetadata Metadata of the tile
   * @param config Settings for the conversion
   * @param tessellateSource Optional URI of a tessellation service to use if polygon pre-tessellation is enabled
   * @return Converted MapLibreTile as a byte array
   * @throws IOException
   */
  public static byte[] encode(
      LayerSource sourceLayers,
      MltMetadata.TileSetMetadata tilesetMetadata,
      ConversionConfig config,
      @Nullable URI tessellateSource)
      throws IOException {
    return encode(
            sourceLayers, tilesetMetadata, config, tessellateSource, ByteArrayOutputStream::new)
        .toByteArray();
  }

  /*
   * Converts a collection of layers into an MLT tile
   *
   * @param sourceLayers The input layers
   * @param tilesetMetadata Metadata of the tile
   * @param config Settings for the conversion
   * @param tessellateSource Optional URI of a tessellation service to use if polygon pre-tessellation is enabled
   * @param outputStreamSupplier A function producing the output stream to which the converted tile will be written.
   * @return The output stream to which the data was written
   * @throws IOException
   */
  public static <T extends OutputStream> T encode(
      @NotNull LayerSource sourceLayers,
      @NotNull MltMetadata.TileSetMetadata tilesetMetadata,
      @NotNull ConversionConfig config,
      @Nullable URI tessellateSource,
      @NotNull Function<Integer, T> outputStreamSupplier)
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

    final var physicalLevelTechnique =
        config.getUseFastPFOR() ? PhysicalLevelTechnique.FAST_PFOR : PhysicalLevelTechnique.VARINT;

    final var tileBuffers = new ArrayList<byte[]>((int) sourceLayers.getLayerCount() * 10);
    for (var sourceLayer : sourceLayers.getLayers()) {
      final var featureTableName = sourceLayer.name();

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

      final var sourceFeatures = sourceLayer.features();
      if (sourceFeatures.isEmpty()) {
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
              sourceFeatures,
              sourceFeatures,
              physicalLevelTechnique,
              createPolygonOutline,
              tessellateSource);
      final var sortedFeatures = result.getLeft();
      final var encodedGeometryColumn = result.getRight();
      final var encodedGeometryFieldMetadata =
          EncodingUtils.encodeVarint(encodedGeometryColumn.numStreams(), false);

      final var encodedPropertyColumns =
          encodePropertyColumns(config, layerMetadata, sortedFeatures, featureTableOptimizations);

      final var featureTableBodyBuffer = new ArrayList<byte[]>(20);
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
        final var scalarColumnMetadata =
            MltMetadata.columnBuilder()
                .scalar(rawType)
                .nullable(idMetadata.isNullable)
                .scope(MltMetadata.ColumnScope.FEATURE)
                .build();
        featureTableBodyBuffer.addAll(
            PropertyEncoder.encodeScalarPropertyColumn(
                scalarColumnMetadata,
                true,
                sortedFeatures,
                physicalLevelTechnique,
                config.getUseFSST(),
                config.getTypeMismatchPolicy() == ConversionConfig.TypeMismatchPolicy.COERCE,
                config.getIntegerEncodingOption()));
      }

      featureTableBodyBuffer.add(encodedGeometryFieldMetadata);
      featureTableBodyBuffer.addAll(encodedGeometryColumn.encodedValues());
      featureTableBodyBuffer.addAll(encodedPropertyColumns);

      final var metadataBuffer = createEmbeddedMetadata(layerMetadata, sourceLayer.tileExtent());

      final var tag = 1;
      final var tagBuffer = EncodingUtils.encodeVarint(tag, false);
      final var tagLength =
          tagBuffer.length
              + metadataBuffer.length
              + ByteArrayUtil.totalLength(featureTableBodyBuffer);

      tileBuffers.add(EncodingUtils.encodeVarint(tagLength, false));
      tileBuffers.add(tagBuffer);
      tileBuffers.add(metadataBuffer);
      tileBuffers.addAll(featureTableBodyBuffer);
    }

    final var targetStream = outputStreamSupplier.apply(ByteArrayUtil.totalLength(tileBuffers));
    Objects.requireNonNull(targetStream, "Output stream supplier returned null");
    return ByteArrayUtil.concat(targetStream, tileBuffers);
  }

  private static ArrayList<byte[]> encodePropertyColumns(
      ConversionConfig config,
      MltMetadata.FeatureTable featureTableMetadata,
      SequencedCollection<Feature> sortedFeatures,
      FeatureTableOptimizations featureTableOptimizations)
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
        config.getTypeMismatchPolicy() == ConversionConfig.TypeMismatchPolicy.COERCE,
        columnMappings,
        config.getIntegerEncodingOption());
  }

  private static Pair<SequencedCollection<Feature>, GeometryEncoder.EncodedGeometryColumn>
      sortFeaturesAndEncodeGeometryColumn(
          ConversionConfig config,
          FeatureTableOptimizations featureTableOptimizations,
          SequencedCollection<Feature> sortedFeatures,
          SequencedCollection<Feature> sourceFeatures,
          PhysicalLevelTechnique physicalLevelTechnique,
          boolean encodePolygonOutlines,
          @Nullable URI tessellateSource)
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

    if (sourceFeatures.isEmpty()) {
      throw new IllegalArgumentException("No features to encode");
    }

    var isColumnSortable =
        config.getIncludeIds()
            && featureTableOptimizations != null
            && featureTableOptimizations.allowSorting();
    if (isColumnSortable && !featureTableOptimizations.allowIdRegeneration()) {
      sortedFeatures = sortFeaturesById(sourceFeatures).toList();
    }

    var ids = sortedFeatures.stream().map(Feature::idOrNull).collect(Collectors.toList());
    var geometries = sortedFeatures.stream().map(Feature::getGeometry).collect(Collectors.toList());

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
    var geometryEncodingOption = config.getGeometryEncodingOption();
    var encodedGeometryColumn =
        config.getPreTessellatePolygons()
            ? GeometryEncoder.encodePretessellatedGeometryColumn(
                geometries,
                physicalLevelTechnique,
                sortSettings,
                useMortonEncoding,
                encodePolygonOutlines,
                tessellateSource,
                geometryEncodingOption)
            : GeometryEncoder.encodeGeometryColumn(
                geometries,
                physicalLevelTechnique,
                sortSettings,
                config.getUseMortonEncoding(),
                geometryEncodingOption);

    if (encodedGeometryColumn.geometryColumnSorted()) {
      sortedFeatures =
          ids.stream()
              .map(
                  id ->
                      sourceFeatures.stream()
                          .filter(fe -> Objects.equals(fe.idOrNull(), id))
                          .findFirst()
                          .orElseThrow())
              .collect(Collectors.toList());
    }

    if (config.getIncludeIds()
        && featureTableOptimizations != null
        && featureTableOptimizations.allowIdRegeneration()) {
      sortedFeatures = generateSequenceIds(sortedFeatures).toList();
    }

    return Pair.of(sortedFeatures, encodedGeometryColumn);
  }

  private static List<MltMetadata.Column> filterPropertyColumns(
      MltMetadata.FeatureTable featureTableMetadata) {
    return featureTableMetadata.columns.stream()
        .filter(f -> !MltTypeMap.Tag0x01.isID(f) && !MltTypeMap.Tag0x01.isGeometry(f))
        .collect(Collectors.toList());
  }

  private static Stream<Feature> sortFeaturesById(Collection<Feature> features) {
    return features.stream()
        .sorted(Comparator.comparing(Feature::hasId).thenComparingLong(Feature::getId));
  }

  private static Stream<Feature> generateSequenceIds(Collection<Feature> features) {
    final var idCounter = new long[] {0};
    return features.stream().map(feature -> feature.asBuilder().id(idCounter[0]++).build());
  }

  private static MltMetadata.Column createScalarColumnScheme(
      String columnName,
      @SuppressWarnings("SameParameterValue") boolean nullable,
      MltMetadata.ScalarType type) {
    return MltMetadata.columnBuilder()
        .name(columnName)
        .scalar(type)
        .nullable(nullable)
        .scope(MltMetadata.ColumnScope.FEATURE)
        .build();
  }

  private static MltMetadata.Column createScalarFieldScheme(
      String fieldName,
      @SuppressWarnings("SameParameterValue") boolean nullable,
      MltMetadata.ScalarType type) {
    return MltMetadata.columnBuilder()
        .name(fieldName)
        .scalar(type)
        .nullable(nullable)
        .scope(MltMetadata.ColumnScope.FEATURE)
        .build();
  }

  private static MltMetadata.Column createComplexColumnScheme(
      @SuppressWarnings("SameParameterValue") @Nullable String columnName,
      @SuppressWarnings("SameParameterValue") boolean nullable,
      @SuppressWarnings("SameParameterValue") MltMetadata.ComplexType type) {
    return MltMetadata.columnBuilder()
        .name(columnName)
        .complex(type)
        .nullable(nullable)
        .scope(MltMetadata.ColumnScope.FEATURE)
        .build();
  }

  private static MltMetadata.ComplexField createComplexColumn() {
    return new MltMetadata.ComplexField(MltMetadata.ComplexType.STRUCT);
  }

  private static MltMetadata.Column createColumn(
      String columnName, MltMetadata.ComplexField complexField) {
    final var isNullable = false; // See `PropertyDecoder.decodePropertyColumn()`
    return MltMetadata.columnBuilder()
        .name(columnName)
        .complex(complexField)
        .nullable(isNullable)
        .scope(MltMetadata.ColumnScope.FEATURE)
        .build();
  }
}
