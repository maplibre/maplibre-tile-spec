package com.mlt;

import com.mlt.converter.ConversionConfig;
import com.mlt.converter.FeatureTableOptimizations;
import com.mlt.converter.MltConverter;
import com.mlt.converter.encodings.EncodingUtils;
import com.mlt.converter.mvt.ColumnMapping;
import com.mlt.converter.mvt.MapboxVectorTile;
import com.mlt.converter.mvt.MvtUtils;
import com.mlt.metadata.tileset.MltTilesetMetadata;
import java.io.ByteArrayInputStream;
import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Paths;
import java.util.List;
import java.util.Map;
import java.util.Optional;
import java.util.stream.Collectors;
import org.apache.commons.lang3.tuple.Pair;

public class BenchmarkUtils {

  public static void encodeTile(
      int z,
      int x,
      int y,
      Map<Integer, byte[]> encodedMvtTiles,
      Map<Integer, ByteArrayInputStream> encodedMvtTiles2,
      Map<Integer, byte[]> compressedMvtTiles,
      Map<Integer, byte[]> encodedMltTiles,
      Map<Integer, MltTilesetMetadata.TileSetMetadata> tileMetadata,
      String path,
      String separator)
      throws IOException {
    var encodedMvtTile = getMvtFile(z, x, y, path, separator);
    encodedMvtTiles.put(z, encodedMvtTile.getLeft());
    encodedMvtTiles2.put(z, new ByteArrayInputStream(encodedMvtTile.getLeft()));
    compressedMvtTiles.put(z, EncodingUtils.gzip(encodedMvtTile.getLeft()));

    var columnMapping = new ColumnMapping("name", ":", true);
    var columnMappings = Optional.of(List.of(columnMapping));
    var metadata =
        MltConverter.createTilesetMetadata(encodedMvtTile.getRight(), columnMappings, true);
    tileMetadata.put(z, metadata);

    var allowIdRegeneration = true;
    var allowSorting = true;
    var optimization =
        new FeatureTableOptimizations(allowSorting, allowIdRegeneration, columnMappings);
    var optimizations =
        TestSettings.OPTIMIZED_MVT_LAYERS.stream()
            .collect(Collectors.toMap(l -> l, l -> optimization));
    var encodedMltTile =
        MltConverter.convertMvt(
            encodedMvtTile.getRight(), new ConversionConfig(true, true, optimizations), metadata);
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
