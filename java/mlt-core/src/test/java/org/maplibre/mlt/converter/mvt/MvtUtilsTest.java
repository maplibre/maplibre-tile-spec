package org.maplibre.mlt.converter.mvt;

import static org.junit.jupiter.api.Assertions.assertEquals;

import java.io.IOException;
import java.nio.file.Path;
import java.util.Objects;
import org.junit.jupiter.api.Test;
import org.maplibre.mlt.converter.ColumnMappingConfig;
import org.maplibre.mlt.converter.ConversionConfig;
import org.maplibre.mlt.converter.MltConverter;
import org.maplibre.mlt.data.Layer;
import org.maplibre.mlt.data.LayerSource;
import org.maplibre.mlt.decoder.MltDecoder;

class MvtUtilsTest {
  // In this tile, only one of the 13 `water` features has an ID, while all other features have one
  private static final Path MIXED_IDS_MVT =
      Path.of("..", "..", "test", "fixtures", "omt", "5_5_11_big_park.mvt");

  @Test
  void featuresWithoutIdHaveNoId() throws IOException {
    final var water = getLayer(MvtUtils.decodeMvt(MIXED_IDS_MVT), "water");
    assertEquals(13, water.features().size());
    assertEquals(1, countFeaturesWithId(water));

    final var park = getLayer(MvtUtils.decodeMvt(MIXED_IDS_MVT), "park");
    assertEquals(park.features().size(), countFeaturesWithId(park));
  }

  @Test
  void featuresWithoutIdHaveNoIdAfterRoundTrip() throws IOException {
    final var mvTile = MvtUtils.decodeMvt(MIXED_IDS_MVT);
    final var metadata =
        MltConverter.createTilesetMetadata(mvTile, new ColumnMappingConfig(), true);
    final var config = ConversionConfig.builder().includeIds(true).build();
    final var mlTile = MltDecoder.decodeMlTile(MltConverter.encode(mvTile, metadata, config, null));

    assertEquals(1, countFeaturesWithId(getLayer(mlTile, "water")));
  }

  private static Layer getLayer(LayerSource tile, String name) {
    return tile.getLayerStream(false).filter(l -> l.name().equals(name)).findFirst().orElseThrow();
  }

  private static long countFeaturesWithId(Layer layer) {
    return layer.features().stream().map(f -> f.idOrNull()).filter(Objects::nonNull).count();
  }
}
