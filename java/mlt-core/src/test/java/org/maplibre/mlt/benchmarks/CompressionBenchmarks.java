package org.maplibre.mlt.benchmarks;

import static org.maplibre.mlt.TestSettings.ID_REASSIGNABLE_MVT_LAYERS;

import java.io.File;
import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Paths;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.List;
import java.util.stream.Collectors;
import org.apache.commons.lang3.tuple.Pair;
import org.apache.commons.lang3.tuple.Triple;
import org.junit.jupiter.api.Disabled;
import org.junit.jupiter.api.Test;
import org.maplibre.mlt.TestSettings;
import org.maplibre.mlt.converter.ConversionConfig;
import org.maplibre.mlt.converter.FeatureTableOptimizations;
import org.maplibre.mlt.converter.MltConverter;
import org.maplibre.mlt.converter.mvt.ColumnMapping;
import org.maplibre.mlt.converter.mvt.MvtUtils;
import org.maplibre.mlt.decoder.MltDecoder;

/*
 * Add the tiles which should be benchmarked to the respective directories.
 * */
public class CompressionBenchmarks {
  private static final String BING_MAPS_PATH = "../test/benchmarks/bing";
  private static final String OMT_PATH = "../test/benchmarks/omt";
  public static final String PLACEHOLDER_FILE = ".gitkeep";

  @Test
  @Disabled
  public void bingMapsCompressionBenchmarks_Unsorted() throws IOException {
    var results = runBenchmarks(BING_MAPS_PATH, false, List.of());
    if (results == null) {
      return;
    }

    System.out.printf("Bing Maps unsorted tile size reduction: %s\n", results.getMiddle());
    System.out.printf("Bing Maps  unsorted compression ratio: %s%% \n", results.getRight());
    System.out.printf("Bing Maps unsorted max tile size reduction: %s\n", results.getLeft());
  }

  @Test
  @Disabled
  public void omtCompressionBenchmarks_Unsorted() throws IOException {
    var results = runBenchmarks(OMT_PATH, false, List.of());
    if (results == null) {
      return;
    }

    System.out.printf("Omt unsorted max tile size reduction: %s\n", results.getLeft());
    System.out.printf("Omt unsorted compression ratio: %s%% \n", results.getRight());
    System.out.printf("Omt unsorted tile size reduction: %s\n", results.getMiddle());
  }

  @Test
  @Disabled
  public void omtCompressionBenchmarks_Sorted() throws IOException {
    var results = runBenchmarks(OMT_PATH, true, List.of());
    if (results == null) {
      return;
    }

    System.out.printf("Omt sorted tile size reduction: %s\n", results.getMiddle());
    System.out.printf("Omt sorted compression ratio: %s%% \n", results.getRight());
    System.out.printf("Omt sorted max tile size reduction: %s\n", results.getLeft());
  }

  @Test
  public void omtCompressionBenchmarks_OptimizedIds() throws IOException {
    var results = runBenchmarks(OMT_PATH, true, ID_REASSIGNABLE_MVT_LAYERS);
    if (results == null) {
      return;
    }

    System.out.printf("Omt optimized ids tile size reduction: %s\n", results.getMiddle());
    System.out.printf("Omt optimized ids compression ratio: %s%% \n", results.getRight());
    System.out.printf("Omt optimized ids max tile size reduction: %s\n", results.getLeft());
  }

  private static Triple<Double, Double, Double> runBenchmarks(
      String path, boolean allowSorting, List<String> reassignableLayers) throws IOException {
    File bingDirectory = new File(path);
    File[] files = bingDirectory.listFiles();

    var tileSizes = new ArrayList<Pair<Integer, Integer>>();
    var tiles =
        Arrays.stream(files)
            .filter(file -> file.isFile() && !file.getName().equals(PLACEHOLDER_FILE))
            .toList();
    if (tiles.isEmpty()) {
      System.out.printf("No tiles found in directory %s\n", path);
      return null;
    }

    for (File tile : tiles) {
      var tilePath = tile.getAbsolutePath();
      var sizes = getBenchmarksAndVerifyTiles(tilePath, allowSorting, reassignableLayers);
      tileSizes.add(sizes);
    }

    var totalMltSizes = 0d;
    var totalMvtSizes = 0d;
    var maxReduction = 0d;
    for (var sizes : tileSizes) {
      totalMltSizes += sizes.getLeft();
      totalMvtSizes += sizes.getRight();

      var reduction = (1 - (double) sizes.getLeft() / sizes.getRight()) * 100;
      if (reduction > maxReduction) {
        maxReduction = reduction;
      }
    }

    var averageMltTileSize = totalMltSizes / tileSizes.size();
    var averageMvtTileSize = totalMvtSizes / tileSizes.size();

    var averageReduction = (1 - averageMltTileSize / averageMvtTileSize) * 100;
    var averageRatio = averageMvtTileSize / averageMltTileSize;
    return Triple.of(maxReduction, averageReduction, averageRatio);
  }

  private static Pair<Integer, Integer> getBenchmarksAndVerifyTiles(
      String tilePath, boolean allowSorting, List<String> reassignableLayers) throws IOException {
    var mvtFilePath = Paths.get(tilePath);
    var mvTile = MvtUtils.decodeMvt(mvtFilePath);

    var columnMapping = new ColumnMapping("name", ":", true);
    var columnMappings = List.of(columnMapping);
    final var isIdPresent = true;
    var tileMetadata = MltConverter.createTilesetMetadata(mvTile, columnMappings, isIdPresent);

    var optimization = new FeatureTableOptimizations(allowSorting, false, columnMappings);
    var optimizations =
        TestSettings.OPTIMIZED_MVT_LAYERS.stream()
            .collect(Collectors.toMap(l -> l, l -> optimization));
    for (var reassignableLayer : reassignableLayers) {
      optimizations.put(
          reassignableLayer, new FeatureTableOptimizations(allowSorting, true, columnMappings));
    }

    var mlTile =
        MltConverter.convertMvt(
            mvTile, tileMetadata, new ConversionConfig(true, true, optimizations), null);

    if (reassignableLayers.isEmpty()) {
      /* Only test when the ids are not reassigned since it is verified based on the other tests */
      var decodedMlt = MltDecoder.decodeMlTile(mlTile);
      System.out.println("Vectorized Decoding not implemented");
    }

    var mvtSize = Files.readAllBytes(mvtFilePath).length;
    return Pair.of(mlTile.length, mvtSize);
  }
}
