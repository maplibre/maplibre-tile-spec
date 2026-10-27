package org.maplibre.mlt.decoder;

import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.io.IOException;
import java.nio.file.Path;
import java.util.List;
import org.junit.jupiter.api.Test;
import org.maplibre.mlt.compare.CompareHelper;
import org.maplibre.mlt.converter.ColumnMappingConfig;
import org.maplibre.mlt.converter.ConversionConfig;
import org.maplibre.mlt.converter.MltConverter;
import org.maplibre.mlt.converter.mvt.MvtUtils;
import org.maplibre.mlt.data.MapboxVectorTile;

class GeometryDecoderTest {
  private static final Path POLYGON_MVT =
      Path.of("..", "..", "test", "fixtures", "simple", "polygon-boolean.mvt");

  @Test
  void decodesTessellatedPolygonsWithOutlines() throws IOException {
    final var mvTile = MvtUtils.decodeMvt(POLYGON_MVT);
    final var decoded = MltDecoder.decodeMlTile(encodeTessellated(mvTile, List.of("ALL")));
    final var diff = CompareHelper.compareTiles(decoded, mvTile, CompareHelper.CompareMode.All);
    assertFalse(diff.isPresent(), () -> diff.get().toString());
  }

  @Test
  void rejectsTessellatedPolygonsWithoutOutlines() throws IOException {
    final var mlTile = encodeTessellated(MvtUtils.decodeMvt(POLYGON_MVT), List.of());
    final var exception =
        assertThrows(IllegalArgumentException.class, () -> MltDecoder.decodeMlTile(mlTile));
    assertTrue(exception.getMessage().contains("outlines"));
  }

  private static byte[] encodeTessellated(MapboxVectorTile mvTile, List<String> outlines)
      throws IOException {
    final var metadata =
        MltConverter.createTilesetMetadata(mvTile, new ColumnMappingConfig(), true);
    final var config =
        ConversionConfig.builder()
            .preTessellatePolygons(true)
            .outlineFeatureTableNames(outlines)
            .build();
    return MltConverter.encode(mvTile, metadata, config, null);
  }
}
