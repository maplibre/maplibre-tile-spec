package org.maplibre.mlt.converter.mvt;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;

import java.io.IOException;
import java.nio.file.Path;
import java.util.Objects;
import org.junit.jupiter.api.Test;
import org.maplibre.mlt.compare.CompareHelper;
import org.maplibre.mlt.converter.ColumnMappingConfig;
import org.maplibre.mlt.converter.ConversionConfig;
import org.maplibre.mlt.converter.MltConverter;
import org.maplibre.mlt.data.Layer;
import org.maplibre.mlt.data.LayerSource;
import org.maplibre.mlt.data.MapLibreTile;
import org.maplibre.mlt.data.MapboxVectorTile;
import org.maplibre.mlt.decoder.MltDecoder;

class MvtUtilsTest {
  // In this tile, only one of the 13 `water` features has an ID, while all other features have one
  private static final Path MIXED_IDS_MVT =
      Path.of("..", "..", "test", "fixtures", "omt", "5_5_11_big_park.mvt");
  // No feature in this tile has an ID
  private static final Path NO_IDS_MVT =
      Path.of("..", "..", "test", "fixtures", "amazon", "9_259_176.mvt");

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
    final var mlTile = roundTrip(MvtUtils.decodeMvt(MIXED_IDS_MVT));
    assertEquals(1, countFeaturesWithId(getLayer(mlTile, "water")));
  }

  @Test
  void tileWithoutIdsRoundTrips() throws IOException {
    final var mvTile = MvtUtils.decodeMvt(NO_IDS_MVT);
    assertEquals(0, mvTile.getLayerStream().mapToLong(MvtUtilsTest::countFeaturesWithId).sum());

    final var diff =
        CompareHelper.compareTiles(roundTrip(mvTile), mvTile, CompareHelper.CompareMode.All);
    assertFalse(diff.isPresent(), () -> diff.get().toString());
  }

  private static MapLibreTile roundTrip(MapboxVectorTile mvTile) throws IOException {
    final var metadata =
        MltConverter.createTilesetMetadata(mvTile, new ColumnMappingConfig(), true);
    final var config = ConversionConfig.builder().includeIds(true).build();
    return MltDecoder.decodeMlTile(MltConverter.encode(mvTile, metadata, config, null));
  }

  private static Layer getLayer(LayerSource tile, String name) {
    return tile.getLayerStream(false).filter(l -> l.name().equals(name)).findFirst().orElseThrow();
  }

  private static long countFeaturesWithId(Layer layer) {
    return layer.features().stream().map(f -> f.idOrNull()).filter(Objects::nonNull).count();
  }
}
