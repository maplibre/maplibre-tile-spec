package org.maplibre.mlt.converter;

import static org.junit.jupiter.api.Assertions.assertArrayEquals;
import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertThrows;

import java.io.IOException;
import java.util.List;
import java.util.Map;
import java.util.Optional;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.params.ParameterizedTest;
import org.junit.jupiter.params.provider.ValueSource;
import org.locationtech.jts.geom.Coordinate;
import org.locationtech.jts.geom.Geometry;
import org.locationtech.jts.geom.GeometryFactory;
import org.locationtech.jts.geom.LinearRing;
import org.locationtech.jts.geom.Polygon;
import org.maplibre.mlt.data.Layer;
import org.maplibre.mlt.data.MLTFeature;
import org.maplibre.mlt.data.MVTFeature;
import org.maplibre.mlt.data.MapLibreTile;
import org.maplibre.mlt.data.MapboxVectorTile;
import org.maplibre.mlt.decoder.MltDecoder;
import org.maplibre.mlt.metadata.tileset.MltMetadata;

class MltConverterTest {
  @Test
  void coerceValuesToString() {
    var metadata =
        MltConverter.createTilesetMetadata(
            createTileWithMixedTypes(),
            new ColumnMappingConfig(),
            true,
            ConversionConfig.TypeMismatchPolicy.COERCE);
    var column =
        metadata.featureTables.stream()
            .flatMap(it -> it.columns().stream())
            .filter(it -> "key".equals(it.getName()))
            .findFirst()
            .get();
    assertEquals(MltMetadata.ScalarType.STRING, column.field().type().scalarType().physicalType());
  }

  @Test
  void elideValues() {
    var metadata =
        MltConverter.createTilesetMetadata(
            createTileWithMixedTypes(),
            new ColumnMappingConfig(),
            true,
            ConversionConfig.TypeMismatchPolicy.ELIDE);
    var column =
        metadata.featureTables.stream()
            .flatMap(it -> it.columns().stream())
            .filter(it -> "key".equals(it.getName()))
            .findFirst()
            .get();
    assertEquals(MltMetadata.ScalarType.DOUBLE, column.field().type().scalarType().physicalType());
  }

  @Test
  void failOnMismatchedValues() {
    assertThrows(
        RuntimeException.class,
        () -> {
          MltConverter.createTilesetMetadata(
              createTileWithMixedTypes(), new ColumnMappingConfig(), true);
        });
  }

  private static MapboxVectorTile createTileWithMixedTypes() {
    return new MapboxVectorTile(
        List.of(
            new Layer(
                "layer",
                List.of(
                    MVTFeature.builder()
                        .index(0)
                        .id(1)
                        .geometry(emptyGeometry)
                        .properties(Map.of("key", 1.2))
                        .build(),
                    MVTFeature.builder()
                        .index(1)
                        .id(2)
                        .geometry(emptyGeometry)
                        .properties(Map.of("key", "2"))
                        .build(),
                    MVTFeature.builder()
                        .index(2)
                        .id(3)
                        .geometry(emptyGeometry)
                        .properties(Map.of("key", 1))
                        .build()),
                4096)));
  }

  @Test
  void generateMetadataJSON() {
    final var metadata = new MltMetadata.TileSetMetadata();

    metadata.name = "test";
    metadata.description = "A test tileset";
    metadata.attribution = "MapLibre";
    metadata.bounds = List.of(-180.0, -85.0511287798066, 180.0, 85.0511287798066);
    metadata.center = List.of(0.0, 0.0);
    metadata.minZoom = Optional.of(0);
    metadata.maxZoom = Optional.of(14);
    metadata.featureTables = List.of();

    MltConverter.createTilesetMetadataJSON(metadata);
  }

  @Test
  void mergeMapColumnsIntoSingleSchema() {
    final var tile =
        new MapLibreTile(
            List.of(
                new Layer(
                    "layer",
                    List.of(
                        MLTFeature.builder()
                            .index(0)
                            .geometry(emptyGeometry)
                            .rawProperties(
                                Map.of("meta_a", Map.of("k1", 1), "meta_b", Map.of("k2", 2)))
                            .build()),
                    4096)));

    final var metadata = MltConverter.createTilesetMetadata(tile, new ColumnMappingConfig(), false);
    final var mapColumns =
        metadata.featureTables.stream()
            .flatMap(table -> table.columns().stream())
            .filter(column -> column.is(MltMetadata.ComplexType.MAP))
            .toList();

    assertEquals(1, mapColumns.size());
    final var mapColumn = mapColumns.getFirst();
    assertEquals("meta_", mapColumn.getName());

    final var childNames =
        mapColumn.getChildren().orElseThrow().stream().map(MltMetadata.Field::name).toList();
    assertEquals(List.of("a", "b"), childNames);
  }

  @ParameterizedTest
  @ValueSource(booleans = {false, true})
  void windsPolygonRingsAsMvt(boolean tessellate) throws IOException {
    // Exterior with a negative and hole with a positive area, both against MVT
    final var polygon =
        factory.createPolygon(
            factory.createLinearRing(coordinates(0, 0, 0, 10, 10, 10, 10, 0, 0, 0)),
            new LinearRing[] {factory.createLinearRing(coordinates(2, 2, 8, 2, 8, 8, 2, 8, 2, 2))});
    final var tile =
        new MapboxVectorTile(
            List.of(
                new Layer(
                    "layer",
                    List.of(MVTFeature.builder().index(0).geometry(polygon).build()),
                    4096)));
    final var metadata = MltConverter.createTilesetMetadata(tile, new ColumnMappingConfig(), false);
    final var config =
        ConversionConfig.builder()
            .includeIds(false)
            .preTessellatePolygons(tessellate)
            .outlineFeatureTableNames(tessellate ? List.of("ALL") : List.of())
            .build();

    final var decoded = MltDecoder.decodeMlTile(MltConverter.encode(tile, metadata, config, null));
    final var wound =
        (Polygon)
            decoded.getLayerStream().findFirst().orElseThrow().features().getFirst().getGeometry();

    // Each reversed ring keeps its first vertex
    assertArrayEquals(
        coordinates(0, 0, 10, 0, 10, 10, 0, 10, 0, 0), wound.getExteriorRing().getCoordinates());
    assertArrayEquals(
        coordinates(2, 2, 2, 8, 8, 8, 8, 2, 2, 2), wound.getInteriorRingN(0).getCoordinates());
  }

  private static Coordinate[] coordinates(int... xy) {
    final var coordinates = new Coordinate[xy.length / 2];
    for (var i = 0; i < coordinates.length; i++) {
      coordinates[i] = new Coordinate(xy[2 * i], xy[2 * i + 1]);
    }
    return coordinates;
  }

  private static final GeometryFactory factory = new GeometryFactory();
  private static final Geometry emptyGeometry = factory.createEmpty(2);
}
