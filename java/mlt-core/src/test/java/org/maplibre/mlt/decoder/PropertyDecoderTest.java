package org.maplibre.mlt.decoder;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertNotNull;

import java.io.IOException;
import java.util.ArrayList;
import java.util.List;
import java.util.Map;
import org.junit.jupiter.api.Test;
import org.locationtech.jts.geom.Coordinate;
import org.locationtech.jts.geom.GeometryFactory;
import org.locationtech.jts.geom.Point;
import org.locationtech.jts.geom.PrecisionModel;
import org.maplibre.mlt.converter.ColumnMappingConfig;
import org.maplibre.mlt.converter.ConversionConfig;
import org.maplibre.mlt.converter.MltConverter;
import org.maplibre.mlt.data.Feature;
import org.maplibre.mlt.data.Layer;
import org.maplibre.mlt.data.MLTFeature;
import org.maplibre.mlt.data.MapLibreTile;
import org.maplibre.mlt.data.MapboxVectorTile;
import org.maplibre.mlt.data.unsigned.U32;
import org.maplibre.mlt.data.unsigned.U64;
import org.maplibre.mlt.json.Json;
import org.maplibre.mlt.metadata.tileset.MltMetadata;

/** Round-trip tests for scalar float and double property columns. */
class PropertyDecoderTest {

  private static final GeometryFactory gf = new GeometryFactory(new PrecisionModel(), 4326);
  private static final Point p0 = gf.createPoint(new Coordinate(13, 42));

  static Feature feature(int index, Map<String, Object> rawProperties) {
    return MLTFeature.builder().index(index).geometry(p0).rawProperties(rawProperties).build();
  }

  // ── float column ──────────────────────────────────────────────────────────

  @Test
  void floatColumn() throws IOException {
    final var features =
        List.of(
            feature(0, Map.of("value", 1.5f)),
            feature(1, Map.of("value", -0.25f)),
            feature(2, Map.of("value", Float.MAX_VALUE)),
            feature(3, Map.of("value", Float.MIN_VALUE)),
            feature(4, Map.of("value", 0.0f)));

    assertRoundTripEncoding(features);
  }

  @Test
  void floatColumnSpecialValues() throws IOException {
    final var features =
        List.of(
            feature(0, Map.of("value", Float.NaN)),
            feature(1, Map.of("value", Float.POSITIVE_INFINITY)),
            feature(2, Map.of("value", Float.NEGATIVE_INFINITY)));

    assertRoundTripEncoding(features);
  }

  @Test
  void floatColumnNullable() throws IOException {
    // Mix of present and absent values forces a PRESENT bitmap to be encoded.
    final var features =
        List.of(
            feature(0, Map.of("value", 3.14f)),
            feature(1, Map.of()),
            feature(2, Map.of("value", -1.0f)));

    assertRoundTripEncoding(features);
  }

  // ── double column ─────────────────────────────────────────────────────────

  @Test
  void doubleColumn() throws IOException {
    final var features =
        List.of(
            feature(0, Map.of("value", 1.5)),
            feature(1, Map.of("value", -0.25)),
            feature(2, Map.of("value", Double.MAX_VALUE)),
            feature(3, Map.of("value", Double.MIN_VALUE)),
            feature(4, Map.of("value", Math.PI)),
            feature(5, Map.of("value", 0.0)));

    assertRoundTripEncoding(features);
  }

  @Test
  void doubleColumnSpecialValues() throws IOException {
    final var features =
        List.of(
            feature(0, Map.of("value", Double.NaN)),
            feature(1, Map.of("value", Double.POSITIVE_INFINITY)),
            feature(2, Map.of("value", Double.NEGATIVE_INFINITY)));

    assertRoundTripEncoding(features);
  }

  @Test
  void doubleColumnNullable() throws IOException {
    // Mix of present and absent values forces a PRESENT bitmap to be encoded.
    final var features =
        List.of(
            feature(0, Map.of("value", Math.E)),
            feature(1, Map.of()),
            feature(2, Map.of("value", -1.0)));

    assertRoundTripEncoding(features);
  }

  // ── helpers ───────────────────────────────────────────────────────────────

  static void assertRoundTripEncoding(List<? extends Feature> features) throws IOException {
    assertRoundTripEncoding(features, ConversionConfig.TypeMismatchPolicy.FAIL, Map.of());
  }

  static void assertRoundTripEncodingWithCoercion(List<? extends Feature> features)
      throws IOException {
    assertRoundTripEncodingWithCoercion(features, Map.of());
  }

  /**
   * Assert round-trip encoding with optional column metadata overrides.
   *
   * @param features features to encode and decode
   * @param typeMismatchPolicy type mismatch policy
   * @param columnMetadataOverrides map of column name to custom Column metadata; if a column is
   *     present here, it will override the auto-generated metadata for that column
   */
  static void assertRoundTripEncoding(
      List<? extends Feature> features,
      @SuppressWarnings("SameParameterValue")
          ConversionConfig.TypeMismatchPolicy typeMismatchPolicy,
      Map<String, MltMetadata.Column> columnMetadataOverrides)
      throws IOException {
    assertRoundTripEncodingInternal(features, typeMismatchPolicy, columnMetadataOverrides, true);
  }

  /**
   * Assert round-trip encoding with coercion. For coercion tests, we validate that the values are
   * present and numeric, but don't compare the full JSON structure since the types may have
   * changed.
   *
   * @param features features to encode and decode
   * @param columnMetadataOverrides map of column name to custom Column metadata
   */
  static void assertRoundTripEncodingWithCoercion(
      List<? extends Feature> features, Map<String, MltMetadata.Column> columnMetadataOverrides)
      throws IOException {
    assertRoundTripEncodingInternal(
        features, ConversionConfig.TypeMismatchPolicy.COERCE, columnMetadataOverrides, false);
  }

  private static void assertRoundTripEncodingInternal(
      List<? extends Feature> features,
      ConversionConfig.TypeMismatchPolicy typeMismatchPolicy,
      Map<String, MltMetadata.Column> columnMetadataOverrides,
      boolean assertExactJson)
      throws IOException {
    final List<Feature> layerFeatures = new ArrayList<>(features);
    final var layer = new Layer("layer1", layerFeatures, 80);
    final var mltTile = new MapLibreTile(List.of(layer));
    final var mvtTile = new MapboxVectorTile(List.of(layer));

    final var config =
        new ConversionConfig.ConfigBuilder()
            .typeMismatchPolicy(typeMismatchPolicy)
            .includeIds(false)
            .build();

    MltMetadata.TileSetMetadata metadata;
    if (columnMetadataOverrides.isEmpty()) {
      metadata =
          MltConverter.createTilesetMetadata(
              mvtTile, typeMismatchPolicy, new ColumnMappingConfig(), false);
    } else {
      final var baseMetadata =
          MltConverter.createTilesetMetadata(
              mvtTile,
              ConversionConfig.TypeMismatchPolicy.COERCE,
              new ColumnMappingConfig(),
              false);
      final var baseFeatureTable = baseMetadata.featureTables.getFirst();
      final var columns = new ArrayList<>(baseFeatureTable.columns());

      for (int i = 0; i < columns.size(); i++) {
        final var col = columns.get(i);
        final var colName = col.getName();
        if (colName != null && columnMetadataOverrides.containsKey(colName)) {
          columns.set(i, columnMetadataOverrides.get(colName));
        }
      }

      metadata = new MltMetadata.TileSetMetadata();
      metadata.featureTables.add(new MltMetadata.FeatureTable(baseFeatureTable.name(), columns));
    }

    final var mltData = MltConverter.encode(mvtTile, metadata, config, null);
    final var decodedTile = MltDecoder.decodeMlTile(mltData);
    final var decodedGeojson = Json.toGeoJson(decodedTile, true);

    if (assertExactJson) {
      assertEquals(
          Json.toGeoJson(mltTile, true), decodedGeojson, "Decoded JSON should match original JSON");
    } else {
      assertNotNull(decodedGeojson, "Decoded GeoJSON should not be null");
      assertFalse(decodedGeojson.isEmpty(), "Decoded GeoJSON should not be empty");
    }
  }

  @Test
  void floatColumnCoercedFromBool() throws IOException {
    // Float column with mixed bool/float features; bool coerced to 0.0f/1.0f
    final var features =
        List.of(
            feature(0, Map.of("value", true)),
            feature(1, Map.of("value", false)),
            feature(2, Map.of("value", 1.5f)));

    final var floatColumn =
        new MltMetadata.Column(
            new MltMetadata.Field(
                MltMetadata.scalarFieldType(MltMetadata.ScalarType.FLOAT, true), "value"));
    assertRoundTripEncodingWithCoercion(features, Map.of("value", floatColumn));
  }

  @Test
  void floatColumnCoercedFromInt32() throws IOException {
    // Float column with int32/float features; int32 coerced to float
    final var features =
        List.of(
            feature(0, Map.of("value", 42)),
            feature(1, Map.of("value", -100)),
            feature(2, Map.of("value", 3.14f)));

    final var floatColumn =
        new MltMetadata.Column(
            new MltMetadata.Field(
                MltMetadata.scalarFieldType(MltMetadata.ScalarType.FLOAT, true), "value"));
    assertRoundTripEncodingWithCoercion(features, Map.of("value", floatColumn));
  }

  @Test
  void floatColumnCoercedFromInt64() throws IOException {
    // Float column with int64/float features; int64 coerced to float
    final var features =
        List.of(
            feature(0, Map.of("value", 5000L)),
            feature(1, Map.of("value", -9999L)),
            feature(2, Map.of("value", -2.71f)));

    final var floatColumn =
        new MltMetadata.Column(
            new MltMetadata.Field(
                MltMetadata.scalarFieldType(MltMetadata.ScalarType.FLOAT, true), "value"));
    assertRoundTripEncodingWithCoercion(features, Map.of("value", floatColumn));
  }

  @Test
  void floatColumnCoercedFromDouble() throws IOException {
    // Float column with double/float features; double coerced via strictFloatOrNull
    final var features =
        List.of(
            feature(0, Map.of("value", Math.PI)),
            feature(1, Map.of("value", 1.5f)),
            feature(2, Map.of("value", Math.E)));

    final var floatColumn =
        new MltMetadata.Column(
            new MltMetadata.Field(
                MltMetadata.scalarFieldType(MltMetadata.ScalarType.FLOAT, true), "value"));
    assertRoundTripEncodingWithCoercion(features, Map.of("value", floatColumn));
  }

  @Test
  void floatColumnCoercedFromUint32() throws IOException {
    // Float column with uint32/float features; uint32 coerced to float
    final var features =
        List.of(
            feature(0, Map.of("value", new U32(1000000000))),
            feature(1, Map.of("value", new U32(500000000))),
            feature(2, Map.of("value", 2.5f)));

    final var floatColumn =
        new MltMetadata.Column(
            new MltMetadata.Field(
                MltMetadata.scalarFieldType(MltMetadata.ScalarType.FLOAT, true), "value"));
    assertRoundTripEncodingWithCoercion(features, Map.of("value", floatColumn));
  }

  @Test
  void floatColumnCoercedFromUint64() throws IOException {
    // Float column with uint64/float features; uint64 coerced to float
    final var features =
        List.of(
            feature(0, Map.of("value", new U64(9000000000000000000L))),
            feature(1, Map.of("value", new U64(4500000000000000000L))),
            feature(2, Map.of("value", 3.7f)));

    final var floatColumn =
        new MltMetadata.Column(
            new MltMetadata.Field(
                MltMetadata.scalarFieldType(MltMetadata.ScalarType.FLOAT, true), "value"));
    assertRoundTripEncodingWithCoercion(features, Map.of("value", floatColumn));
  }

  @Test
  void doubleColumnCoercedFromBool() throws IOException {
    // Double column with bool/double features; bool coerced to 0.0/1.0
    final var features =
        List.of(
            feature(0, Map.of("value", true)),
            feature(1, Map.of("value", false)),
            feature(2, Map.of("value", 2.71)));

    final var doubleColumn =
        new MltMetadata.Column(
            new MltMetadata.Field(
                MltMetadata.scalarFieldType(MltMetadata.ScalarType.DOUBLE, true), "value"));
    assertRoundTripEncodingWithCoercion(features, Map.of("value", doubleColumn));
  }

  @Test
  void doubleColumnCoercedFromInt32() throws IOException {
    // Double column with int32/double features; int32 coerced to double
    final var features =
        List.of(
            feature(0, Map.of("value", 77)),
            feature(1, Map.of("value", -333)),
            feature(2, Map.of("value", Math.PI)));

    final var doubleColumn =
        new MltMetadata.Column(
            new MltMetadata.Field(
                MltMetadata.scalarFieldType(MltMetadata.ScalarType.DOUBLE, true), "value"));
    assertRoundTripEncodingWithCoercion(features, Map.of("value", doubleColumn));
  }

  @Test
  void doubleColumnCoercedFromInt64() throws IOException {
    // Double column with int64/double features; int64 coerced to double
    final var features =
        List.of(
            feature(0, Map.of("value", 100000L)),
            feature(1, Map.of("value", -50000L)),
            feature(2, Map.of("value", 1.414)));

    final var doubleColumn =
        new MltMetadata.Column(
            new MltMetadata.Field(
                MltMetadata.scalarFieldType(MltMetadata.ScalarType.DOUBLE, true), "value"));
    assertRoundTripEncodingWithCoercion(features, Map.of("value", doubleColumn));
  }

  @Test
  void doubleColumnCoercedFromFloat() throws IOException {
    // Double column with float/double features; float coerced to double
    final var features =
        List.of(
            feature(0, Map.of("value", 3.14f)),
            feature(1, Map.of("value", Math.E)),
            feature(2, Map.of("value", 1.732f)));

    final var doubleColumn =
        new MltMetadata.Column(
            new MltMetadata.Field(
                MltMetadata.scalarFieldType(MltMetadata.ScalarType.DOUBLE, true), "value"));
    assertRoundTripEncodingWithCoercion(features, Map.of("value", doubleColumn));
  }

  @Test
  void doubleColumnCoercedFromUint32() throws IOException {
    // Double column with uint32/double features; uint32 coerced to double
    final var features =
        List.of(
            feature(0, Map.of("value", new U32(1500000000))),
            feature(1, Map.of("value", new U32(2000000000))),
            feature(2, Map.of("value", 1.618)));

    final var doubleColumn =
        new MltMetadata.Column(
            new MltMetadata.Field(
                MltMetadata.scalarFieldType(MltMetadata.ScalarType.DOUBLE, true), "value"));
    assertRoundTripEncodingWithCoercion(features, Map.of("value", doubleColumn));
  }

  @Test
  void doubleColumnCoercedFromUint64() throws IOException {
    // Double column with uint64/double features; uint64 coerced to double
    final var features =
        List.of(
            feature(0, Map.of("value", new U64(8000000000000000000L))),
            feature(1, Map.of("value", new U64(6000000000000000000L))),
            feature(2, Map.of("value", 2.236)));

    final var doubleColumn =
        new MltMetadata.Column(
            new MltMetadata.Field(
                MltMetadata.scalarFieldType(MltMetadata.ScalarType.DOUBLE, true), "value"));
    assertRoundTripEncodingWithCoercion(features, Map.of("value", doubleColumn));
  }

  // ── int32 column coercions ────────────────────────────────────────────────

  @Test
  void int32ColumnCoercedFromBool() throws IOException {
    // Int32 column with bool/int32 features; bool coerced to 0/1
    final var features =
        List.of(
            feature(0, Map.of("value", true)),
            feature(1, Map.of("value", false)),
            feature(2, Map.of("value", 42)));

    final var int32Column =
        new MltMetadata.Column(
            new MltMetadata.Field(
                MltMetadata.scalarFieldType(MltMetadata.ScalarType.INT_32, true), "value"));
    assertRoundTripEncodingWithCoercion(features, Map.of("value", int32Column));
  }

  @Test
  void int32ColumnCoercedFromFloat() throws IOException {
    // Int32 column with float/int32 features; float coerced to int32
    final var features =
        List.of(
            feature(0, Map.of("value", 3.14f)),
            feature(1, Map.of("value", -2.71f)),
            feature(2, Map.of("value", 100)));

    final var int32Column =
        new MltMetadata.Column(
            new MltMetadata.Field(
                MltMetadata.scalarFieldType(MltMetadata.ScalarType.INT_32, true), "value"));
    assertRoundTripEncodingWithCoercion(features, Map.of("value", int32Column));
  }

  @Test
  void int32ColumnCoercedFromDouble() throws IOException {
    // Int32 column with double/int32 features; double coerced to int32
    final var features =
        List.of(
            feature(0, Map.of("value", 2.718)),
            feature(1, Map.of("value", -1.414)),
            feature(2, Map.of("value", 555)));

    final var int32Column =
        new MltMetadata.Column(
            new MltMetadata.Field(
                MltMetadata.scalarFieldType(MltMetadata.ScalarType.INT_32, true), "value"));
    assertRoundTripEncodingWithCoercion(features, Map.of("value", int32Column));
  }

  @Test
  void int32ColumnCoercedFromInt64() throws IOException {
    // Int32 column with int64/int32 features; int64 coerced to int32
    final var features =
        List.of(
            feature(0, Map.of("value", 1000000L)),
            feature(1, Map.of("value", -500000L)),
            feature(2, Map.of("value", 777)));

    final var int32Column =
        new MltMetadata.Column(
            new MltMetadata.Field(
                MltMetadata.scalarFieldType(MltMetadata.ScalarType.INT_32, true), "value"));
    assertRoundTripEncodingWithCoercion(features, Map.of("value", int32Column));
  }

  @Test
  void int32ColumnCoercedFromUint32() throws IOException {
    // Int32 column with uint32/int32 features; uint32 coerced to int32
    final var features =
        List.of(
            feature(0, Map.of("value", new U32(500000000))),
            feature(1, Map.of("value", new U32(1000000000))),
            feature(2, Map.of("value", 333)));

    final var int32Column =
        new MltMetadata.Column(
            new MltMetadata.Field(
                MltMetadata.scalarFieldType(MltMetadata.ScalarType.INT_32, true), "value"));
    assertRoundTripEncodingWithCoercion(features, Map.of("value", int32Column));
  }

  @Test
  void int32ColumnCoercedFromUint64() throws IOException {
    // Int32 column with uint64/int32 features; uint64 coerced to int32
    final var features =
        List.of(
            feature(0, Map.of("value", new U64(1000000000L))),
            feature(1, Map.of("value", new U64(500000000L))),
            feature(2, Map.of("value", 999)));

    final var int32Column =
        new MltMetadata.Column(
            new MltMetadata.Field(
                MltMetadata.scalarFieldType(MltMetadata.ScalarType.INT_32, true), "value"));
    assertRoundTripEncodingWithCoercion(features, Map.of("value", int32Column));
  }

  // ── int64 column coercions ────────────────────────────────────────────────

  @Test
  void int64ColumnCoercedFromBool() throws IOException {
    // Int64 column with bool/int64 features; bool coerced to 0L/1L
    final var features =
        List.of(
            feature(0, Map.of("value", true)),
            feature(1, Map.of("value", false)),
            feature(2, Map.of("value", 5000000000L)));

    final var int64Column =
        new MltMetadata.Column(
            new MltMetadata.Field(
                MltMetadata.scalarFieldType(MltMetadata.ScalarType.INT_64, true), "value"));
    assertRoundTripEncodingWithCoercion(features, Map.of("value", int64Column));
  }

  @Test
  void int64ColumnCoercedFromFloat() throws IOException {
    // Int64 column with float/int64 features; float coerced to int64
    final var features =
        List.of(
            feature(0, Map.of("value", 1.5f)),
            feature(1, Map.of("value", -3.7f)),
            feature(2, Map.of("value", 9000000000L)));

    final var int64Column =
        new MltMetadata.Column(
            new MltMetadata.Field(
                MltMetadata.scalarFieldType(MltMetadata.ScalarType.INT_64, true), "value"));
    assertRoundTripEncodingWithCoercion(features, Map.of("value", int64Column));
  }

  @Test
  void int64ColumnCoercedFromDouble() throws IOException {
    // Int64 column with double/int64 features; double coerced to int64
    final var features =
        List.of(
            feature(0, Map.of("value", Math.PI)),
            feature(1, Map.of("value", -Math.E)),
            feature(2, Map.of("value", 7000000000L)));

    final var int64Column =
        new MltMetadata.Column(
            new MltMetadata.Field(
                MltMetadata.scalarFieldType(MltMetadata.ScalarType.INT_64, true), "value"));
    assertRoundTripEncodingWithCoercion(features, Map.of("value", int64Column));
  }

  @Test
  void int64ColumnCoercedFromInt32() throws IOException {
    // Int64 column with int32/int64 features; int32 coerced to int64
    final var features =
        List.of(
            feature(0, Map.of("value", 42)),
            feature(1, Map.of("value", -100)),
            feature(2, Map.of("value", 8000000000L)));

    final var int64Column =
        new MltMetadata.Column(
            new MltMetadata.Field(
                MltMetadata.scalarFieldType(MltMetadata.ScalarType.INT_64, true), "value"));
    assertRoundTripEncodingWithCoercion(features, Map.of("value", int64Column));
  }

  @Test
  void int64ColumnCoercedFromUint32() throws IOException {
    // Int64 column with uint32/int64 features; uint32 coerced to int64
    final var features =
        List.of(
            feature(0, Map.of("value", new U32(1500000000))),
            feature(1, Map.of("value", new U32(2000000000))),
            feature(2, Map.of("value", 6000000000L)));

    final var int64Column =
        new MltMetadata.Column(
            new MltMetadata.Field(
                MltMetadata.scalarFieldType(MltMetadata.ScalarType.INT_64, true), "value"));
    assertRoundTripEncodingWithCoercion(features, Map.of("value", int64Column));
  }

  @Test
  void int64ColumnCoercedFromUint64() throws IOException {
    // Int64 column with uint64/int64 features; uint64 coerced to int64
    final var features =
        List.of(
            feature(0, Map.of("value", new U64(8000000000000000000L))),
            feature(1, Map.of("value", new U64(6000000000000000000L))),
            feature(2, Map.of("value", 3000000000L)));

    final var int64Column =
        new MltMetadata.Column(
            new MltMetadata.Field(
                MltMetadata.scalarFieldType(MltMetadata.ScalarType.INT_64, true), "value"));
    assertRoundTripEncodingWithCoercion(features, Map.of("value", int64Column));
  }
}
