package org.maplibre.mlt;

import java.io.ByteArrayInputStream;
import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Paths;
import java.util.List;
import java.util.Map;
import java.util.regex.Pattern;
import java.util.stream.Collectors;
import org.apache.commons.lang3.tuple.Pair;
import org.maplibre.mlt.converter.ColumnMapping;
import org.maplibre.mlt.converter.ColumnMappingConfig;
import org.maplibre.mlt.converter.ConversionConfig;
import org.maplibre.mlt.converter.FeatureTableOptimizations;
import org.maplibre.mlt.converter.MltConverter;
import org.maplibre.mlt.converter.encodings.EncodingUtils;
import org.maplibre.mlt.converter.mvt.MvtUtils;
import org.maplibre.mlt.data.MapboxVectorTile;

public class BenchmarkUtils {
  private BenchmarkUtils() {}

  public static void encodeTile(
      int z,
      int x,
      int y,
      Map<Integer, byte[]> encodedMvtTiles,
      Map<Integer, ByteArrayInputStream> encodedMvtTiles2,
      Map<Integer, byte[]> compressedMvtTiles,
      Map<Integer, byte[]> encodedMltTiles,
      String path,
      String separator)
      throws IOException {
    final var encodedMvtTile = getMvtFile(z, x, y, path, separator);
    encodedMvtTiles.put(z, encodedMvtTile.getLeft());
    encodedMvtTiles2.put(z, new ByteArrayInputStream(encodedMvtTile.getLeft()));
    compressedMvtTiles.put(z, EncodingUtils.gzip(encodedMvtTile.getLeft()));

    final var columnMapping = new ColumnMapping("name", ":", true);
    final var columnMappings = List.of(columnMapping);
    final var columnMappingMap = ColumnMappingConfig.of(Pattern.compile(".*"), columnMappings);
    final var isIdPresent = true;
    final var metadata =
        MltConverter.createTilesetMetadata(
            encodedMvtTile.getRight(), columnMappingMap, isIdPresent);

    final var allowIdRegeneration = true;
    final var allowSorting = true;
    final var optimization =
        new FeatureTableOptimizations(allowSorting, allowIdRegeneration, columnMappings);
    final var optimizations =
        TestSettings.OPTIMIZED_MVT_LAYERS.stream()
            .collect(Collectors.toMap(l -> l, l -> optimization));
    final var config =
        ConversionConfig.builder()
            .includeIds(true)
            .useFastPFOR(true)
            .useFSST(true)
            .optimizations(optimizations)
            .build();
    final var encodedMltTile =
        MltConverter.encode(encodedMvtTile.getRight(), metadata, config, null);
    encodedMltTiles.put(z, encodedMltTile);
  }

  private static Pair<byte[], MapboxVectorTile> getMvtFile(
      int z, int x, int y, String path, String separator) throws IOException {
    var tileId = String.format("%s%s%s%s%s", z, separator, x, separator, y);
    var mvtFilePath = Paths.get(path, tileId + ".mvt");
    var encodedTile = Files.readAllBytes(mvtFilePath);
    var decodedTile = MvtUtils.decodeMvt(mvtFilePath);
    return Pair.of(encodedTile, decodedTile);
  }
}
