package com.mlt.decoder;

import com.mlt.TestSettings;
import com.mlt.TestUtils;
import com.mlt.converter.ConversionConfig;
import com.mlt.converter.FeatureTableOptimizations;
import com.mlt.converter.MltConverter;
import com.mlt.converter.mvt.ColumnMapping;
import com.mlt.converter.mvt.MapboxVectorTile;
import com.mlt.converter.mvt.MvtUtils;
import com.mlt.metadata.tileset.MltTilesetMetadata;
import java.io.IOException;
import java.nio.file.Paths;
import java.util.List;
import java.util.Optional;
import java.util.stream.Collectors;
import java.util.stream.Stream;
import org.apache.commons.lang3.tuple.Triple;
import org.junit.jupiter.api.DisplayName;
import org.junit.jupiter.params.ParameterizedTest;
import org.junit.jupiter.params.provider.MethodSource;

@FunctionalInterface
interface TriConsumer<A, B, C> {
  void apply(A a, B b, C c) throws IOException;
}

public class MltDecoderTest {

  private static Stream<Triple<Integer, Integer, Integer>> bingMapsTileIdProvider() {
    return Stream.of(
        Triple.of(4, 8, 5), Triple.of(5, 16, 11), Triple.of(6, 32, 22), Triple.of(7, 65, 42));
  }

  @DisplayName("Decode unsorted Bing Maps based vector tiles")
  @ParameterizedTest
  @MethodSource("bingMapsTileIdProvider")
  public void decodeMlTileVectorized_UnsortedBingMaps(Triple<Integer, Integer, Integer> tileId)
      throws IOException {
    // TODO: fix -> 5-15-10, 5-17-10, 5-17-11, 7-65-42, 9-259-176
    var id = String.format("%s-%s-%s", tileId.getLeft(), tileId.getMiddle(), tileId.getRight());
    testTileVectorized(id, TestSettings.BING_MVT_PATH, false);
  }

  private static Stream<Triple<Integer, Integer, Integer>> omtTileIdProvider() {
    return Stream.of(
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

  /* Decode tiles in an in-memory format optimized for random access */

  @DisplayName("Decode unsorted OpenMapTiles schema based vector tiles")
  @ParameterizedTest
  @MethodSource("omtTileIdProvider")
  public void decodeMlTileVectorized_UnsortedOMT(Triple<Integer, Integer, Integer> tileId)
      throws IOException {
    var id = String.format("%s_%s_%s", tileId.getLeft(), tileId.getMiddle(), tileId.getRight());
    testTileVectorized(id, TestSettings.OMT_MVT_PATH, false);
  }

  @DisplayName("Decode sorted OpenMapTiles schema based vector tiles")
  @ParameterizedTest
  @MethodSource("omtTileIdProvider")
  public void decodeMlTileVectorized_SortedOMT(Triple<Integer, Integer, Integer> tileId)
      throws IOException {
    // TODO: fix 10_531_683
    var id = String.format("%s_%s_%s", tileId.getLeft(), tileId.getMiddle(), tileId.getRight());
    testTileVectorized(id, TestSettings.OMT_MVT_PATH, true);
  }

  /* Decode tiles in an in-memory format optimized for sequential access */

  @DisplayName("Decode scalar unsorted OpenMapTiles schema based vector tiles")
  @ParameterizedTest
  @MethodSource("omtTileIdProvider")
  public void decodeMlTile_UnsortedOMT(Triple<Integer, Integer, Integer> tileId)
      throws IOException {
    // TODO: fix -> 2_2_2
    if (tileId.getLeft() == 2) {
      return;
    }

    var id = String.format("%s_%s_%s", tileId.getLeft(), tileId.getMiddle(), tileId.getRight());
    ;
    testTileSequential(id, TestSettings.OMT_MVT_PATH);
  }

  private void testTileVectorized(String tileId, String tileDirectory, boolean allowSorting)
      throws IOException {
    testTile(
        tileId,
        tileDirectory,
        (mlTile, tileMetadata, mvTile) -> {
          var decodedTile = MltDecoder.decodeMlTileVectorized(mlTile, tileMetadata);
          TestUtils.compareTilesVectorized(decodedTile, mvTile, allowSorting);
        },
        allowSorting);
  }

  private void testTileSequential(String tileId, String tileDirectory) throws IOException {
    testTile(
        tileId,
        tileDirectory,
        (mlTile, tileMetadata, mvTile) -> {
          var decodedTile = MltDecoder.decodeMlTile(mlTile, tileMetadata);
          TestUtils.compareTilesSequential(decodedTile, mvTile);
        },
        false);
  }

  private void testTile(
      String tileId,
      String tileDirectory,
      TriConsumer<byte[], MltTilesetMetadata.TileSetMetadata, MapboxVectorTile> decodeAndCompare,
      boolean allowSorting)
      throws IOException {
    var mvtFilePath = Paths.get(tileDirectory, tileId + ".mvt");
    var mvTile = MvtUtils.decodeMvt(mvtFilePath);

    var columnMapping = new ColumnMapping("name", ":", true);
    var columnMappings = Optional.of(List.of(columnMapping));
    var tileMetadata = MltConverter.createTilesetMetadata(mvTile, columnMappings, true);

    var allowIdRegeneration = false;
    var optimization =
        new FeatureTableOptimizations(allowSorting, allowIdRegeneration, columnMappings);
    var optimizations =
        TestSettings.OPTIMIZED_MVT_LAYERS.stream()
            .collect(Collectors.toMap(l -> l, l -> optimization));

    var mlTile =
        MltConverter.convertMvt(
            mvTile, new ConversionConfig(true, true, optimizations), tileMetadata);

    decodeAndCompare.apply(mlTile, tileMetadata, mvTile);
  }
}
