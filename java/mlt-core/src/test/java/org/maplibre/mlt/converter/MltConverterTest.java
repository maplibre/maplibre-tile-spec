package org.maplibre.mlt.converter;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertThrows;

import java.util.List;
import java.util.Map;
import java.util.Optional;
import org.junit.jupiter.api.Test;
import org.locationtech.jts.geom.Geometry;
import org.locationtech.jts.geom.GeometryFactory;
import org.maplibre.mlt.data.Layer;
import org.maplibre.mlt.data.MVTFeature;
import org.maplibre.mlt.data.MapboxVectorTile;
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
            .flatMap(it -> it.columns.stream())
            .filter(it -> "key".equals(it.getName()))
            .findFirst()
            .get();
    assertEquals(MltMetadata.ScalarType.STRING, column.field.type.scalarType.physicalType);
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
            .flatMap(it -> it.columns.stream())
            .filter(it -> "key".equals(it.getName()))
            .findFirst()
            .get();
    assertEquals(MltMetadata.ScalarType.DOUBLE, column.field.type.scalarType.physicalType);
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

  private static final GeometryFactory factory = new GeometryFactory();
  private static final Geometry emptyGeometry = factory.createEmpty(2);
}
