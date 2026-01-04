package org.maplibre.mlt.converter;

import org.junit.jupiter.api.Test;
import org.locationtech.jts.geom.GeometryFactory;
import org.maplibre.mlt.converter.mvt.MapboxVectorTile;
import org.maplibre.mlt.data.Feature;
import org.maplibre.mlt.data.Layer;
import org.maplibre.mlt.metadata.tileset.MltMetadata;

import java.util.List;
import java.util.Map;

import static org.junit.jupiter.api.Assertions.assertEquals;

class MltConverterTest {
  @Test
  void coerceValuesToString() {
    GeometryFactory factory = new GeometryFactory();
    var layer =
        new Layer(
            "layer",
            List.of(
                new Feature(1, factory.createEmpty(2), Map.of("key", 1.2)),
                new Feature(2, factory.createEmpty(2), Map.of("key", "2")),
                new Feature(3, factory.createEmpty(2), Map.of("key", 1))),
            4096);
    var tile = new MapboxVectorTile(List.of(layer));
    var metadata = MltConverter.createTilesetMetadata(tile, Map.of(), true, true, false);
    var column =
        metadata.featureTables.stream()
            .flatMap(it -> it.columns.stream())
            .filter(it -> "key".equals(it.name))
            .findFirst()
            .get();
    assertEquals(MltMetadata.ScalarType.STRING, column.scalarType.physicalType);
  }
}
