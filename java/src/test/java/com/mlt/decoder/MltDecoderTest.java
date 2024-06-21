package com.mlt.decoder;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertThrows;

import com.mlt.TestSettings;
import com.mlt.TestUtils;
import com.mlt.converter.ConversionConfig;
import com.mlt.converter.FeatureTableOptimizations;
import com.mlt.converter.MltConverter;
import com.mlt.converter.mvt.ColumnMapping;
import com.mlt.converter.mvt.MvtUtils;
import java.io.IOException;
import java.nio.file.Paths;
import java.util.List;
import java.util.Optional;
import java.util.stream.Collectors;
import java.util.stream.Stream;
import org.junit.jupiter.api.DisplayName;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.params.ParameterizedTest;
import org.junit.jupiter.params.provider.MethodSource;

enum DecoderType {
  BOTH,
  SEQUENTIAL,
  VECTORIZED
}

public class MltDecoderTest {

  /* Bing Maps tests --------------------------------------------------------- */

  private static Stream<String> bingProvider() {
    return Stream.of(
        "4-8-5", "4-9-5", "4-12-6", "4-13-6", "5-16-11", "5-17-11", "5-17-10", "6-32-22", "6-33-22",
        "6-32-23", "6-32-21", "7-65-42", "7-66-42", "7-66-43", "7-66-44");
  }

  @DisplayName("Decode Bing Tiles (Vectorized)")
  @ParameterizedTest
  @MethodSource("bingProvider")
  public void decodeBingTilesVectorized(String tileId) throws IOException {
    int numErrors = testTile(tileId, TestSettings.BING_MVT_PATH, DecoderType.VECTORIZED, false);
    assertEquals(
        0,
        numErrors,
        "There should be no errors in the Bing tiles for "
            + tileId
            + " but there were "
            + numErrors
            + " errors");
  }

  // TODO Currently a lot of property errors are expected in the Bing tiles in SEQUENTIAL mode
  // TODO: start testing more tiles once the decoder is fixed
  public void decodeBingTilesSequential() throws IOException {
    String tileId = "7-66-43";
    int numErrors = testTile(tileId, TestSettings.BING_MVT_PATH, DecoderType.SEQUENTIAL, false);
    assertEquals(
        660,
        numErrors,
        "There should be no errors in the Bing tiles for "
            + tileId
            + " but there were "
            + numErrors
            + " errors");
  }

  @Test
  public void currentlyFailingBingDecoding1() throws IOException {
    var exception =
        assertThrows(
            Exception.class,
            () -> testTile("5-16-9", TestSettings.BING_MVT_PATH, DecoderType.VECTORIZED, false));
    assertEquals(
        "java.lang.IllegalArgumentException: VectorType not supported yet: CONST",
        exception.toString());
  }

  @Test
  // Currently fails, need to root cause property decoding difference
  public void currentlyFailingBingDecoding3() throws IOException {
    var exception =
        assertThrows(
            org.opentest4j.AssertionFailedError.class,
            () -> testTile("5-15-10", TestSettings.BING_MVT_PATH, DecoderType.VECTORIZED, false));
    assertEquals(
        "org.opentest4j.AssertionFailedError: expected: <(U.K.)> but was: <null>",
        exception.toString());
  }

  /* OpenMapTiles schema based vector tiles tests  --------------------------------------------------------- */

  private static Stream<String> omtProvider() {
    return Stream.of(
        "2_2_2",
        "3_4_5",
        "4_8_10",
        "4_3_9",
        "5_16_21",
        "5_16_20",
        "6_32_41",
        "6_33_42",
        "7_66_84",
        "7_66_85",
        "8_134_171",
        "8_132_170",
        "9_265_341",
        "10_532_682",
        "11_1064_1367",
        "12_2132_2734",
        "13_4265_5467",
        "14_8298_10748",
        "14_8299_10748");
  }

  @DisplayName("Decode OMT Tiles")
  @ParameterizedTest()
  @MethodSource("omtProvider")
  public void decodeOMTTiles(String tileId) throws IOException {
    int numErrors = testTile(tileId, TestSettings.OMT_MVT_PATH, DecoderType.BOTH, false);
    if (tileId == "2_2_2") {
      // We expect errors currently in OMT tiles where name:ja:rm is present
      assertEquals(4, numErrors, "There should be no errors in the OMT tiles: " + tileId);
    } else {
      assertEquals(0, numErrors, "There should be no errors in the OMT tiles: " + tileId);
    }
  }

  /* Test utility functions */

  private int testTile(String tileId, String tileDirectory, DecoderType type, boolean allowSorting)
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
    var includeIds = true;
    var useAdvancedEncodingSchemes = true;
    var mlTile =
        MltConverter.convertMvt(
            mvTile,
            new ConversionConfig(includeIds, useAdvancedEncodingSchemes, optimizations),
            tileMetadata);
    int numErrors = 0;
    if (type == DecoderType.SEQUENTIAL || type == DecoderType.BOTH) {
      var decoded = MltDecoder.decodeMlTile(mlTile, tileMetadata);
      numErrors += TestUtils.compareTilesSequential(decoded, mvTile);
    }
    if (type == DecoderType.VECTORIZED || type == DecoderType.BOTH) {
      var decoded = MltDecoder.decodeMlTileVectorized(mlTile, tileMetadata);
      numErrors += TestUtils.compareTilesVectorized(decoded, mvTile, allowSorting);
    }
    return numErrors;
  }
}
