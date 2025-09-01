package org.maplibre.mlt.converter;

import java.io.IOException;
import java.util.*;
import java.util.stream.Collectors;
import java.util.stream.IntStream;
import org.apache.commons.lang3.tuple.Pair;
import org.maplibre.mlt.converter.encodings.EncodingUtils;
import org.maplibre.mlt.converter.encodings.GeometryEncoder;
import org.maplibre.mlt.converter.encodings.PropertyEncoder;
import org.maplibre.mlt.converter.mvt.ColumnMapping;
import org.maplibre.mlt.converter.mvt.MapboxVectorTile;
import org.maplibre.mlt.data.Feature;
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
  public static MltTilesetMetadata.TileSetMetadata createTilesetMetadata(
      Iterable<MapboxVectorTile> mvTiles,
      Optional<List<ColumnMapping>> columnMappings,
      boolean isIdPresent) {
    var tilesetBuilder = MltTilesetMetadata.TileSetMetadata.newBuilder();
    tilesetBuilder.setVersion(VERSION);

    var featureTableSchemes =
        new LinkedHashMap<String, LinkedHashMap<String, MltTilesetMetadata.Column>>();
    var complexPropertyColumnSchemesContainer =
        new LinkedHashMap<
            String, LinkedHashMap<String, MltTilesetMetadata.ComplexColumn.Builder>>();
    for (var tile : mvTiles) {
      for (var layer : tile.layers()) {
        final LinkedHashMap<String, MltTilesetMetadata.Column> featureTableScheme;

        if (!featureTableSchemes.containsKey(layer.name())) {
          featureTableScheme = new LinkedHashMap<>();
          featureTableSchemes.put(layer.name(), featureTableScheme);
        } else {
          featureTableScheme = featureTableSchemes.get(layer.name());
        }

        var complexPropertyColumnSchemes =
            complexPropertyColumnSchemesContainer.computeIfAbsent(
                layer.name(), k -> new LinkedHashMap<>());

        /* Create the scheme for the property columns by iterating over all features in the tile and collecting
         * information about the feature properties*/
        var isLongId = false;
        for (var feature : layer.features()) {
          /* sort so that the name of the parent column comes first before the nested fields as it has to be the shortest */
          // TODO: refactor
          var properties =
              feature.properties().entrySet().stream()
                  .sorted((a, b) -> a.getKey().compareTo(b.getKey()))
                  .toList();
          for (var property : properties) {
            var mvtPropertyName = property.getKey();

            if (featureTableScheme.containsKey(mvtPropertyName)) {
              continue;
            }

            /* MVT can only contain scalar types */
            var scalarType = getScalarType(property);

            if (columnMappings.isPresent()) {
              // TODO: refactor quick and dirty solution -> simplify that complex logic
              if (columnMappings.get().stream()
                      .anyMatch(m -> mvtPropertyName.equals(m.mvtPropertyPrefix()))
                  && !complexPropertyColumnSchemes.containsKey(mvtPropertyName)) {
                /* case where the top-level field is present like name (name:de, name:us, ...) and has a value.
                 * In this case the field is mapped to the name default. */
                var childField = createScalarFieldScheme("default", true, scalarType);
                var fieldMetadataBuilder = createComplexColumnBuilder(childField);
                complexPropertyColumnSchemes.put(mvtPropertyName, fieldMetadataBuilder);
                continue;
              } else if (columnMappings.get().stream()
                      .anyMatch(m -> mvtPropertyName.equals(m.mvtPropertyPrefix()))
                  && complexPropertyColumnSchemes.containsKey(mvtPropertyName)
                  && complexPropertyColumnSchemes.get(mvtPropertyName).getChildrenList().stream()
                      .noneMatch(c -> c.getName().equals("default"))) {
                /* Case where the top-level field such as name is not present in the first feature */
                var childField = createScalarFieldScheme("default", true, scalarType);
                var columnMapping =
                    columnMappings.get().stream()
                        .filter(m -> mvtPropertyName.equals(m.mvtPropertyPrefix()))
                        .findFirst()
                        .get();
                complexPropertyColumnSchemes
                    .get(columnMapping.mvtPropertyPrefix())
                    .addChildren(childField);
                continue;
              } else if (columnMappings.get().stream()
                  .anyMatch(
                      m ->
                          mvtPropertyName.contains(
                              m.mvtPropertyPrefix() + Settings.MLT_CHILD_FIELD_SEPARATOR))) {
                var columnMapping =
                    columnMappings.get().stream()
                        .filter(
                            m ->
                                mvtPropertyName.contains(
                                    m.mvtPropertyPrefix() + Settings.MLT_CHILD_FIELD_SEPARATOR))
                        .findFirst()
                        .get();
                var columnName = columnMapping.mvtPropertyPrefix();

                var fieldNames = mvtPropertyName.split(Settings.MLT_CHILD_FIELD_SEPARATOR);
                /* There are cases with double nested property names like name_ja_kana */
                var fieldName =
                    Arrays.stream(fieldNames)
                        .skip(1)
                        .collect(Collectors.joining(Settings.MLT_CHILD_FIELD_SEPARATOR));
                var children = createScalarFieldScheme(fieldName, true, scalarType);
                if (complexPropertyColumnSchemes.containsKey(columnName)) {
                  /* add the nested properties to the parent like the name:* properties to the name parent struct */
                  if (complexPropertyColumnSchemes.get(columnName).getChildrenList().stream()
                      .noneMatch(c -> c.getName().equals(fieldName))) {
                    complexPropertyColumnSchemes.get(columnName).addChildren(children);
                  }
                } else {
                  /* Case where there is no explicit property available which serves as the name
                   * for the top-level field. For example there is no name property only name:* */
                  var complexColumnBuilder = createComplexColumnBuilder(children);
                  complexPropertyColumnSchemes.put(columnName, complexColumnBuilder);
                }
                continue;
              }
            }

            var columnScheme = createScalarColumnScheme(mvtPropertyName, true, scalarType);
            featureTableScheme.put(mvtPropertyName, columnScheme);
          }

          if (isIdPresent
              && (feature.id() > Integer.MAX_VALUE || feature.id() < Integer.MIN_VALUE)) {
            isLongId = true;
          }
        }

        if (isIdPresent && (!featureTableScheme.containsKey(ID_COLUMN_NAME))
            || (isLongId
                && featureTableScheme.get(ID_COLUMN_NAME).getScalarType().getPhysicalType()
                    != MltTilesetMetadata.ScalarType.INT_64)) {
          /* Narrow down unsigned long to unsigned int if possible as it can be currently more efficiently encoded
           * based on FastPFOR (64 bit variant not yet implemented) instead of Varint and faster
           * decoded in the Js decoder.
           * */
          var idDataType =
              isLongId
                  ? MltTilesetMetadata.ScalarType.UINT_64
                  : MltTilesetMetadata.ScalarType.UINT_32;
          var idMetadata = createScalarColumnScheme(ID_COLUMN_NAME, false, idDataType);
          featureTableScheme.put(ID_COLUMN_NAME, idMetadata);
        }
      }
    }

    for (var complexPropertyColumnSchemeLayer : complexPropertyColumnSchemesContainer.entrySet()) {
      for (var complexPropertyColumnScheme :
          complexPropertyColumnSchemeLayer.getValue().entrySet()) {
        featureTableSchemes
            .get(complexPropertyColumnSchemeLayer.getKey())
            .put(
                complexPropertyColumnScheme.getKey(),
                createColumn(
                    complexPropertyColumnScheme.getKey(),
                    complexPropertyColumnScheme.getValue().build()));
      }
    }

    for (var featureTableScheme : featureTableSchemes.entrySet()) {
      var featureTableSchemaBuilder = MltTilesetMetadata.FeatureTableSchema.newBuilder();
      featureTableSchemaBuilder.setName(featureTableScheme.getKey());

      var columnSchema = featureTableScheme.getValue();
      var idColumn = columnSchema.get(ID_COLUMN_NAME);
      /* If present the Id column has to be the first column in a FeatureTable */
      if (idColumn != null) {
        featureTableSchemaBuilder.addColumns(idColumn);
        featureTableScheme.getValue().remove(ID_COLUMN_NAME);
      }

      /* The geometry column is mandatory and has to be the second column in a FeatureTable */
      var geometryMetadata =
          createComplexColumnScheme(
              GEOMETRY_COLUMN_NAME, false, MltTilesetMetadata.ComplexType.GEOMETRY);
      featureTableSchemaBuilder.addColumns(geometryMetadata);

      columnSchema.forEach((k, v) -> featureTableSchemaBuilder.addColumns(v));
      tilesetBuilder.addFeatureTables(featureTableSchemaBuilder.build());
    }

    return tilesetBuilder.build();
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
      ConversionConfig config,
      MltTilesetMetadata.TileSetMetadata tilesetMetadata)
      throws IOException {
    var physicalLevelTechnique =
        config.useAdvancedEncodingSchemes()
            ? PhysicalLevelTechnique.FAST_PFOR
            : PhysicalLevelTechnique.VARINT;

    var mapLibreTileBuffer = new byte[0];
    for (var mvtLayer : mvt.layers()) {
      var featureTableBodyBuffer = new byte[0];

      var featureTableName = mvtLayer.name();
      var mvtFeatures = mvtLayer.features();

      /* Layout FeatureTableMetadata header (all u32 types are varint encoded):
       *  version: u8 | featureTableId: u32 | layerExtent: u32 | maxLayerExtent: u32 | numFeatures: u32
       *  */
      var featureTables = tilesetMetadata.getFeatureTablesList();
      var featureTableId =
          IntStream.range(0, featureTables.size())
              .filter(i -> featureTables.get(i).getName().equals(featureTableName))
              .findFirst()
              .getAsInt();
      var featureTableMetadata = featureTables.get(featureTableId);

      var featureTableOptimizations =
          config.optimizations() == null ? null : config.optimizations().get(featureTableName);

      var createPolygonOutline =
          config instanceof RenderingOptimizedConversionConfig
              && ((RenderingOptimizedConversionConfig) config)
                  .getOutlineFeatureTableNames()
                  .contains(featureTableName);
      var result =
          sortFeaturesAndEncodeGeometryColumn(
              config,
              featureTableOptimizations,
              mvtFeatures,
              mvtFeatures,
              physicalLevelTechnique,
              createPolygonOutline);
      var sortedFeatures = result.getLeft();
      var encodedGeometryColumn = result.getRight();
      var encodedGeometryFieldMetadata =
          EncodingUtils.encodeVarints(
              new long[] {encodedGeometryColumn.numStreams()}, false, false);

      var encodedPropertyColumns =
          encodePropertyColumns(
              config, featureTableMetadata, sortedFeatures, featureTableOptimizations);

      if (config.includeIds()) {
        var idMetadata =
            featureTableMetadata.getColumnsList().stream()
                .filter(f -> f.getName().equals(ID_COLUMN_NAME))
                .findFirst()
                .get();

        featureTableBodyBuffer =
            PropertyEncoder.encodeScalarPropertyColumn(
                idMetadata,
                sortedFeatures,
                physicalLevelTechnique,
                config.useAdvancedEncodingSchemes());
      }

      featureTableBodyBuffer =
          CollectionUtils.concatByteArrays(
              featureTableBodyBuffer,
              encodedGeometryFieldMetadata,
              encodedGeometryColumn.encodedValues(),
              encodedPropertyColumns);

      var featureTableBodySize = featureTableBodyBuffer.length;
      var encodedFeatureTableInfo =
          EncodingUtils.encodeVarints(
              new long[] {
                featureTableId,
                featureTableBodySize,
                mvtLayer.tileExtent(),
                EncodingUtils.encodeZigZag(encodedGeometryColumn.maxVertexValue()),
                sortedFeatures.size()
              },
              false,
              false);

      mapLibreTileBuffer =
          CollectionUtils.concatByteArrays(
              mapLibreTileBuffer,
              new byte[] {VERSION},
              encodedFeatureTableInfo,
              featureTableBodyBuffer);
    }

    return mapLibreTileBuffer;
  }

  private static byte[] encodePropertyColumns(
      ConversionConfig config,
      MltTilesetMetadata.FeatureTableSchema featureTableMetadata,
      List<Feature> sortedFeatures,
      FeatureTableOptimizations featureTableOptimizations)
      throws IOException {
    var propertyColumns = filterPropertyColumns(featureTableMetadata);
    return PropertyEncoder.encodePropertyColumns(
        propertyColumns,
        sortedFeatures,
        config.useAdvancedEncodingSchemes(),
        featureTableOptimizations != null
            ? featureTableOptimizations.columnMappings()
            : Optional.empty());
  }

  private static Pair<List<Feature>, GeometryEncoder.EncodedGeometryColumn>
      sortFeaturesAndEncodeGeometryColumn(
          ConversionConfig config,
          FeatureTableOptimizations featureTableOptimizations,
          List<Feature> sortedFeatures,
          List<Feature> mvtFeatures,
          PhysicalLevelTechnique physicalLevelTechnique,
          boolean encodePolygonOutlines) {
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
        config.includeIds()
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
        config instanceof RenderingOptimizedConversionConfig
            ? GeometryEncoder.encodePretessellatedGeometryColumn(
                geometries,
                physicalLevelTechnique,
                sortSettings,
                useMortonEncoding,
                encodePolygonOutlines)
            : GeometryEncoder.encodeGeometryColumn(
                geometries, physicalLevelTechnique, sortSettings, config.useMortonEncoding());

    if (encodedGeometryColumn.geometryColumnSorted()) {
      sortedFeatures =
          ids.stream()
              .map(id -> mvtFeatures.stream().filter(fe -> fe.id() == id).findFirst().get())
              .collect(Collectors.toList());
    }

    if (config.includeIds()
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
      String fieldName, boolean nullable, MltTilesetMetadata.ScalarType type) {
    var scalarField = MltTilesetMetadata.ScalarField.newBuilder().setPhysicalType(type);
    return MltTilesetMetadata.Field.newBuilder()
        .setName(fieldName)
        .setNullable(nullable)
        .setScalarField(scalarField)
        .build();
  }

  private static MltTilesetMetadata.Column createComplexColumnScheme(
      String columnName, boolean nullable, MltTilesetMetadata.ComplexType type) {
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
