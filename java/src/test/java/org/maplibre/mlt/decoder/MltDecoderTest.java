package org.maplibre.mlt.decoder;

import java.io.IOException;
import java.net.URI;
import java.net.URISyntaxException;
import java.nio.file.Paths;
import java.util.List;
import java.util.Optional;
import java.util.stream.Collectors;
import java.util.stream.Stream;
import org.apache.commons.lang3.tuple.Triple;
import org.junit.jupiter.api.Disabled;
import org.junit.jupiter.api.DisplayName;
import org.junit.jupiter.params.ParameterizedTest;
import org.junit.jupiter.params.provider.MethodSource;
import org.maplibre.mlt.TestSettings;
import org.maplibre.mlt.TestUtils;
import org.maplibre.mlt.converter.ConversionConfig;
import org.maplibre.mlt.converter.FeatureTableOptimizations;
import org.maplibre.mlt.converter.MltConverter;
import org.maplibre.mlt.converter.mvt.ColumnMapping;
import org.maplibre.mlt.converter.mvt.MapboxVectorTile;
import org.maplibre.mlt.converter.mvt.MvtUtils;
import org.maplibre.mlt.metadata.tileset.MltTilesetMetadata;

@FunctionalInterface
interface TriConsumer<A, B, C> {
  void apply(A a, B b, C c) throws IOException;
}

public class MltDecoderTest {

  private static Stream<Triple<Integer, Integer, Integer>> bingMapsTileIdProvider() {
    return Stream.of(
        Triple.of(4, 8, 5), Triple.of(5, 16, 11), Triple.of(6, 32, 22), Triple.of(7, 65, 42));
  }

  private static Stream<Triple<Integer, Integer, Integer>> omtTileIdProvider() {
    return Stream.of(
        Triple.of(0, 0, 0),
        Triple.of(1, 1, 1),
        Triple.of(2, 2, 2),
        Triple.of(3, 4, 5),
        Triple.of(4, 8, 10),
        Triple.of(5, 16, 21),
        Triple.of(6, 32, 41),
        Triple.of(7, 66, 84),
        Triple.of(8, 134, 171),
        Triple.of(9, 265, 341),
        Triple.of(10, 532, 682),
        Triple.of(11, 1064, 1367),
        Triple.of(12, 2132, 2734),
        Triple.of(13, 4265, 5467),
        Triple.of(14, 8298, 10748));
  }

  /* Decode tiles in an in-memory format optimized for sequential access */

  @DisplayName("Decode scalar unsorted OpenMapTiles schema based vector tiles")
  @ParameterizedTest
  @MethodSource("omtTileIdProvider")
  @Disabled
  public void decodeMlTile_UnsortedOMT(Triple<Integer, Integer, Integer> tileId)
      throws IOException, URISyntaxException {
    // TODO: fix -> 2_2_2
    if (tileId.getLeft() == 2) {
      return;
    }

    var id = String.format("%s_%s_%s", tileId.getLeft(), tileId.getMiddle(), tileId.getRight());
    testTileSequential(id, TestSettings.OMT_MVT_PATH);
  }

  private void testTileSequential(String tileId, String tileDirectory)
      throws IOException, URISyntaxException {
    testTile(
        tileId,
        tileDirectory,
        (mlTile, tileMetadata, mvTile) -> {
          var decodedTile = MltDecoder.decodeMlTile(mlTile);
          TestUtils.compareTilesSequential(decodedTile, mvTile);
        },
        TestUtils.Optimization.NONE,
        List.of(),
        true);
  }

  private void testTile(
      String tileId,
      String tileDirectory,
      TriConsumer<byte[], MltTilesetMetadata.TileSetMetadata, MapboxVectorTile> decodeAndCompare,
      TestUtils.Optimization optimization,
      List<String> reassignableLayers,
      boolean advancedEncodings)
      throws IOException, URISyntaxException {
    var mvtFilePath = Paths.get(tileDirectory, tileId + ".mvt");
    var mvTile = MvtUtils.decodeMvt(mvtFilePath);

    var columnMapping = new ColumnMapping("name", ":", true);
    var columnMappings = Optional.of(List.of(columnMapping));
    var tileMetadata = MltConverter.createTilesetMetadata(mvTile, columnMappings, true);

    var allowSorting = optimization == TestUtils.Optimization.SORTED;
    var featureTableOptimization =
        new FeatureTableOptimizations(allowSorting, false, columnMappings);
    var optimizations =
        TestSettings.OPTIMIZED_MVT_LAYERS.stream()
            .collect(Collectors.toMap(l -> l, l -> featureTableOptimization));

    /* Only regenerate the ids for specific layers when the column is not sorted for comparison reasons */
    if (optimization == TestUtils.Optimization.IDS_REASSIGNED) {
      for (var reassignableLayer : reassignableLayers) {
        optimizations.put(
            reassignableLayer, new FeatureTableOptimizations(false, true, columnMappings));
      }
    }

    var config = new ConversionConfig(true, advancedEncodings, optimizations);

    var mlTile = MltConverter.convertMvt(mvTile, tileMetadata, config, null);
    // decodeAndCompare.apply(mlTile, tileMetadata, mvTile);

    mlTile =
        MltConverter.convertMvt(mvTile, tileMetadata, config, new URI("http://localhost:3000"));
    // decodeAndCompare.apply(mlTile, tileMetadata, mvTile);
  }
}
