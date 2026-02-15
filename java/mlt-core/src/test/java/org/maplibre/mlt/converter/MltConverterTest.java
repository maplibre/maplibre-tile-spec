package org.maplibre.mlt.converter;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertThrows;

import java.util.List;
import java.util.Map;
import java.util.Optional;
import org.junit.jupiter.api.Test;
import org.locationtech.jts.geom.GeometryFactory;
import org.maplibre.mlt.converter.mvt.MapboxVectorTile;
import org.maplibre.mlt.data.Feature;
import org.maplibre.mlt.data.Layer;
import org.maplibre.mlt.metadata.tileset.MltMetadata;

class MltConverterTest {
  @Test
  void coerceValuesToString() {
    var metadata =
        MltConverter.createTilesetMetadata(createTileWithMixedTypes(), Map.of(), true, true, false);
    var column =
        metadata.featureTables.stream()
            .flatMap(it -> it.columns.stream())
            .filter(it -> "key".equals(it.name))
            .findFirst()
            .get();
    assertEquals(MltMetadata.ScalarType.STRING, column.scalarType.physicalType);
  }

  @Test
  void elideValues() {
    var metadata =
        MltConverter.createTilesetMetadata(createTileWithMixedTypes(), Map.of(), true, false, true);
    var column =
        metadata.featureTables.stream()
            .flatMap(it -> it.columns.stream())
            .filter(it -> "key".equals(it.name))
            .findFirst()
            .get();
    assertEquals(MltMetadata.ScalarType.DOUBLE, column.scalarType.physicalType);
  }

  @Test
  void failOnMismatchedValues() {
    assertThrows(
        RuntimeException.class,
        () -> {
          MltConverter.createTilesetMetadata(createTileWithMixedTypes(), Map.of(), true);
        });
  }

  private static MapboxVectorTile createTileWithMixedTypes() {
    final var factory = new GeometryFactory();
    return new MapboxVectorTile(
        List.of(
            new Layer(
                "layer",
                List.of(
                    new Feature(1, factory.createEmpty(2), Map.of("key", 1.2)),
                    new Feature(2, factory.createEmpty(2), Map.of("key", "2")),
                    new Feature(3, factory.createEmpty(2), Map.of("key", 1))),
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
}
