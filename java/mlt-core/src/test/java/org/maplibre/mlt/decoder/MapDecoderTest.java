package org.maplibre.mlt.decoder;

import static org.junit.jupiter.api.Assertions.assertEquals;

import java.io.IOException;
import java.math.BigDecimal;
import java.math.BigInteger;
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

/** Unit tests for MAP property decoder with nested list properties. */
class MapDecoderTest {

  private static final GeometryFactory gf = new GeometryFactory(new PrecisionModel(), 4326);
  private static final Point p0 = gf.createPoint(new Coordinate(13, 42));

  /**
   * Test that a nested list property containing mixed scalar types (int32, int64, uint32, uint64,
   * string, double) encodes and decodes correctly in a round-trip.
   */
  @Test
  void nestedNull() throws IOException {
    // empty is distinct from null
    final var features =
        List.of(
            (Feature)
                MLTFeature.builder()
                    .index(0)
                    .geometry(p0)
                    .rawProperties(Map.of("a", Map.of("b", Map.of())))
                    .build(),
            (Feature) MLTFeature.builder().index(1).geometry(p0).rawProperties(Map.of()).build());

    assertRoundTripEncoding(features);
  }

  @Test
  void nestedList() throws IOException {
    // can contain lists of mixed types, list-within-map and map-within-list
    final var features =
        List.of(
            (Feature)
                MLTFeature.builder()
                    .index(0)
                    .geometry(p0)
                    .rawProperties(
                        Map.of("a", Map.of("b", List.of(1, "2", Map.of("c", List.of(3.0, 4))))))
                    .build());

    assertRoundTripEncoding(features);
  }

  @Test
  void nestedListRootWithMixedTypes() throws IOException {
    final var features =
        List.of(
            (Feature)
                MLTFeature.builder()
                    .index(0)
                    .geometry(p0)
                    .rawProperties(
                        Map.of(
                            "a",
                            List.of(
                                true,
                                1, // int32
                                (long) Integer.MAX_VALUE + 2, // int64
                                U32.of(Integer.MAX_VALUE + 3L), // uint32
                                U64.of(
                                    BigInteger.valueOf(Long.MAX_VALUE)
                                        .add(BigInteger.valueOf(4L))), // uint64
                                "5",
                                6.0,
                                (double) Float.MAX_VALUE + 7)))
                    .build());

    assertRoundTripEncoding(features);
  }

  @Test
  void nestedBig() throws IOException {
    // BigInteger and BigDecimal support
    final var features =
        List.of(
            (Feature)
                MLTFeature.builder()
                    .index(0)
                    .geometry(p0)
                    .rawProperties(
                        Map.of(
                            "a",
                            List.of(
                                BigInteger.valueOf(1),
                                BigInteger.valueOf(-2),
                                BigInteger.valueOf(Integer.MAX_VALUE + 3L),
                                BigInteger.valueOf(Long.MAX_VALUE).add(BigInteger.valueOf(5L)),
                                BigDecimal.valueOf(6.0f),
                                BigDecimal.valueOf(7.0),
                                BigDecimal.valueOf(Math.nextUp(8.0)))))
                    .build());

    assertRoundTripEncoding(features);
  }

  @Test
  void nestedSpecials() throws IOException {
    // Make sure special float values are supported in nested properties
    final var features =
        List.of(
            (Feature)
                MLTFeature.builder()
                    .index(0)
                    .geometry(p0)
                    .rawProperties(
                        Map.of(
                            "a",
                            List.of(
                                Float.NaN,
                                Double.NaN,
                                Float.POSITIVE_INFINITY,
                                Float.NEGATIVE_INFINITY,
                                Double.POSITIVE_INFINITY,
                                Double.NEGATIVE_INFINITY)))
                    .build());

    assertRoundTripEncoding(features);
  }

  @Test
  void nestedMixedRoot() throws IOException {
    // Mixed scalar types in the same column as a nested value are supported
    final var features =
        List.of(
            (Feature)
                MLTFeature.builder().index(0).geometry(p0).rawProperties(Map.of("a", "b")).build(),
            (Feature)
                MLTFeature.builder()
                    .index(1)
                    .geometry(p0)
                    .rawProperties(Map.of("a", Map.of("b", "c")))
                    .build(),
            (Feature)
                MLTFeature.builder()
                    .index(2)
                    .geometry(p0)
                    .rawProperties(Map.of("a", Math.PI))
                    .build());

    assertRoundTripEncodingWithCoercion(features);
  }

  @Test
  void nestedShared() throws IOException {
    // Multiple nested columns share dictionaries
    final var features =
        List.of(
            (Feature)
                MLTFeature.builder()
                    .index(0)
                    .geometry(p0)
                    .rawProperties(Map.of("name:a", "b"))
                    .build(),
            (Feature)
                MLTFeature.builder()
                    .index(1)
                    .geometry(p0)
                    .rawProperties(Map.of("name:a", Map.of("b", "c")))
                    .build(),
            (Feature)
                MLTFeature.builder()
                    .index(2)
                    .geometry(p0)
                    .rawProperties(Map.of("name:b", Map.of("c", List.of("b"))))
                    .build(),
            (Feature)
                MLTFeature.builder()
                    .index(3)
                    .geometry(p0)
                    .rawProperties(Map.of("name:b", "b"))
                    .build(),
            (Feature)
                MLTFeature.builder()
                    .index(4)
                    .geometry(p0)
                    .rawProperties(Map.of("name:c", Map.of("b", Math.PI)))
                    .build(),
            (Feature)
                MLTFeature.builder()
                    .index(5)
                    .geometry(p0)
                    .rawProperties(Map.of("name:c", Math.PI))
                    .build());

    assertRoundTripEncodingWithCoercion(features);
  }

  private void assertRoundTripEncoding(List<Feature> features) throws IOException {
    assertRoundTripEncoding(features, ConversionConfig.TypeMismatchPolicy.FAIL);
  }

  private void assertRoundTripEncodingWithCoercion(List<Feature> features) throws IOException {
    assertRoundTripEncoding(features, ConversionConfig.TypeMismatchPolicy.COERCE);
  }

  private void assertRoundTripEncoding(
      List<Feature> features, ConversionConfig.TypeMismatchPolicy typeMismatchPolicy)
      throws IOException {
    final var layer = new Layer("layer1", features, 80);
    final var mltTile = new MapLibreTile(List.of(layer));
    final var mvtTile = new MapboxVectorTile(List.of(layer));

    final var config =
        new ConversionConfig.ConfigBuilder()
            .typeMismatchPolicy(typeMismatchPolicy)
            .includeIds(false)
            .build();
    final var metadata =
        MltConverter.createTilesetMetadata(
            mvtTile, typeMismatchPolicy, new ColumnMappingConfig(), false);
    final var mltData = MltConverter.encode(mvtTile, metadata, config, null);

    final var decodedTile = MltDecoder.decodeMlTile(mltData);

    assertEquals(
        Json.toGeoJson(mltTile, true),
        Json.toGeoJson(decodedTile, true),
        "Decoded JSON should match original JSON");
  }
}
