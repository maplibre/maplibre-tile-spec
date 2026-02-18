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
import org.maplibre.mlt.converter.ConversionConfig;
import org.maplibre.mlt.converter.FeatureTableOptimizations;
import org.maplibre.mlt.converter.MltConverter;
import org.maplibre.mlt.converter.encodings.EncodingUtils;
import org.maplibre.mlt.converter.mvt.ColumnMapping;
import org.maplibre.mlt.converter.mvt.MapboxVectorTile;
import org.maplibre.mlt.converter.mvt.MvtUtils;

public class BenchmarkUtils {

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
    var encodedMvtTile = getMvtFile(z, x, y, path, separator);
    encodedMvtTiles.put(z, encodedMvtTile.getLeft());
    encodedMvtTiles2.put(z, new ByteArrayInputStream(encodedMvtTile.getLeft()));
    compressedMvtTiles.put(z, EncodingUtils.gzip(encodedMvtTile.getLeft()));

    var columnMapping = new ColumnMapping("name", ":", true);
    var columnMappings = List.of(columnMapping);
    var columnMappingMap = Map.of(Pattern.compile(".*"), columnMappings);
    final var isIdPresent = true;
    var metadata =
        MltConverter.createTilesetMetadata(
            encodedMvtTile.getRight(), columnMappingMap, isIdPresent);

    var allowIdRegeneration = true;
    var allowSorting = true;
    var optimization =
        new FeatureTableOptimizations(allowSorting, allowIdRegeneration, columnMappings);
    var optimizations =
        TestSettings.OPTIMIZED_MVT_LAYERS.stream()
            .collect(Collectors.toMap(l -> l, l -> optimization));
    var config = new ConversionConfig(true, true, true, optimizations);
    var encodedMltTile = MltConverter.convertMvt(encodedMvtTile.getRight(), metadata, config, null);
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
