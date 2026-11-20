package org.maplibre.mlt.decoder;

import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.List;
import java.util.regex.Pattern;
import java.util.stream.Collectors;
import java.util.stream.Stream;
import org.junit.jupiter.api.Assertions;
import org.junit.jupiter.api.DisplayName;
import org.junit.jupiter.api.Named;
import org.junit.jupiter.params.ParameterizedTest;
import org.junit.jupiter.params.provider.MethodSource;
import org.maplibre.mlt.TestSettings;
import org.maplibre.mlt.compare.CompareHelper;
import org.maplibre.mlt.converter.ColumnMapping;
import org.maplibre.mlt.converter.ColumnMappingConfig;
import org.maplibre.mlt.converter.ConversionConfig;
import org.maplibre.mlt.converter.FeatureTableOptimizations;
import org.maplibre.mlt.converter.MltConverter;
import org.maplibre.mlt.converter.mvt.MvtUtils;
import org.maplibre.mlt.metadata.tileset.MltMetadata;

public class MltDecoderTest {

  /// Every MVT fixture, so that new fixtures are covered without having to list them here
  private static Stream<Named<Path>> mvtFixtureProvider() throws IOException {
    final var root = Path.of(TestSettings.FIXTURES_PATH);
    try (var paths = Files.walk(root)) {
      return paths
          .filter(path -> path.toString().endsWith(".mvt"))
          .sorted()
          .map(path -> Named.of(root.relativize(path).toString(), path))
          .toList()
          .stream();
    }
  }

  @DisplayName("Round-trip MVT fixtures with plain encodings")
  @ParameterizedTest
  @MethodSource("mvtFixtureProvider")
  public void decodeMlTile_Plain(Path mvtFilePath) throws IOException {
    testTile(mvtFilePath, false, false, false);
  }

  @DisplayName("Round-trip MVT fixtures with FastPFOR, FSST and tessellation")
  @ParameterizedTest
  @MethodSource("mvtFixtureProvider")
  public void decodeMlTile_Advanced(Path mvtFilePath) throws IOException {
    testTile(mvtFilePath, true, true, true);
  }

  private void testTile(Path mvtFilePath, boolean useFastPFOR, boolean useFSST, boolean tessellate)
      throws IOException {
    final var mvTile = MvtUtils.decodeMvt(mvtFilePath);

    final var columnMapping = new ColumnMapping("name", ":", true);
    final var columnMappings =
        ColumnMappingConfig.of(Pattern.compile(".*"), List.of(columnMapping));
    final var isIdPresent = true;
    // Some fixtures mix value types within a property, e.g. `all_route_types` in amazon_here
    final var typeMismatchPolicy = ConversionConfig.TypeMismatchPolicy.COERCE;
    final var tileMetadata =
        MltConverter.createTilesetMetadata(mvTile, columnMappings, isIdPresent, typeMismatchPolicy);

    // The column mappings above apply to every layer, so the optimizations must cover every layer
    final var featureTableOptimization =
        new FeatureTableOptimizations(false, false, List.of(columnMapping));
    final var optimizations =
        tileMetadata.featureTables.stream()
            .collect(
                Collectors.toMap(MltMetadata.FeatureTable::name, t -> featureTableOptimization));

    final var config =
        ConversionConfig.builder()
            .includeIds(true)
            .useFastPFOR(useFastPFOR)
            .useFSST(useFSST)
            .typeMismatchPolicy(typeMismatchPolicy)
            .optimizations(optimizations)
            .preTessellatePolygons(tessellate)
            // Polygons can only be decoded from tessellated layers that keep their outlines
            .outlineFeatureTableNames(tessellate ? List.of("ALL") : List.of())
            .build();

    final var mlTile = MltConverter.encode(mvTile, tileMetadata, config, null);
    Assertions.assertNotNull(mlTile);

    final var decodedTile = MltDecoder.decodeMlTile(mlTile);
    Assertions.assertNotNull(decodedTile);

    final var compareResult =
        CompareHelper.compareTiles(decodedTile, mvTile, CompareHelper.CompareMode.All);
    Assertions.assertFalse(compareResult.isPresent(), () -> compareResult.get().toString());
  }
}
