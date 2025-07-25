package com.mlt.decoder;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

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
import java.util.Objects;
import java.util.Optional;
import java.util.stream.Collectors;
import java.util.stream.Stream;
import org.apache.commons.lang3.NotImplementedException;
import org.junit.jupiter.api.Disabled;
import org.junit.jupiter.api.DisplayName;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.params.ParameterizedTest;
import org.junit.jupiter.params.provider.MethodSource;

enum DecoderType {
  BOTH,
  SEQUENTIAL,
  VECTORIZED
}

enum EncodingType {
  BOTH,
  NONADVANCED,
  ADVANCED
}

record DecodingResult(int numErrors, int numErrorsAdvanced) {}

public class MltDecoderTest2 {

  /* Bing Maps tests --------------------------------------------------------- */

  // TODO: after https://github.com/maplibre/maplibre-tile-spec/issues/186 is fixed
  // remove this test and start testing all the Bing tiles with sorting enabled
  @Test
  @Disabled
  public void decodeBingTilesSortedFail() throws IOException {
    var tileId = "4-8-5";
    var result =
        testTile(tileId, TestSettings.BING_MVT_PATH, DecoderType.BOTH, EncodingType.ADVANCED, true);
    assertEquals(
        1148,
        result.numErrorsAdvanced(),
        "Error for " + tileId + "/advanced: " + result.numErrorsAdvanced());
  }

  private static Stream<String> bingProvider() {
    return Stream.of(
        "4-8-5", "4-9-5", "4-12-6", "4-13-6", "5-16-11", "5-17-11", "5-17-10", "6-32-22", "6-33-22",
        "6-32-23", "6-32-21", "7-65-42", "7-66-42", "7-66-43", "7-66-44");
  }

  @DisplayName("Decode Bing Tiles")
  @ParameterizedTest
  @MethodSource("bingProvider")
  @Disabled
  public void decodeBingTiles(String tileId) throws IOException {
    var result =
        testTile(tileId, TestSettings.BING_MVT_PATH, DecoderType.BOTH, EncodingType.BOTH, false);
    assertEquals(
        0, result.numErrors(), "Error for " + tileId + "/non-advanced: " + result.numErrors());
    assertEquals(
        0,
        result.numErrorsAdvanced(),
        "Error for " + tileId + "/advanced: " + result.numErrorsAdvanced());
  }

  // TODO:
  // once https://github.com/maplibre/maplibre-tile-spec/issues/182 is fixed
  // add the "5-16-9" tile to the bingProvider
  // and remove this test
  @Disabled
  @Test
  public void currentlyFailingBingDecoding1() {
    var exception =
        assertThrows(
            Exception.class,
            () ->
                testTile(
                    "5-16-9",
                    TestSettings.BING_MVT_PATH,
                    DecoderType.VECTORIZED,
                    EncodingType.ADVANCED,
                    false));
    assertEquals(
        "java.lang.IllegalArgumentException: VectorType not supported yet: CONST",
        exception.toString());
  }

  /* OpenMapTiles schema based vector tiles tests  --------------------------------------------------------- */

  // TODO: after https://github.com/maplibre/maplibre-tile-spec/issues/185 is fixed
  // remove this test and start testing all the OMT tiles with sorting enabled
  @Test
  @Disabled
  public void decodeOMTTilesSortedFail() {
    var exception =
        assertThrows(
            Exception.class,
            () ->
                testTile(
                    "4_8_10",
                    TestSettings.OMT_MVT_PATH,
                    DecoderType.BOTH,
                    EncodingType.ADVANCED,
                    true));
    assertEquals("java.lang.IndexOutOfBoundsException", exception.toString());
  }

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

  @DisplayName("Decode OMT Tiles (advanced encodings, non-sorted)")
  @ParameterizedTest()
  @MethodSource("omtProvider")
  @Disabled
  public void decodeOMTTiles(String tileId) throws IOException {
    var result =
        testTile(tileId, TestSettings.OMT_MVT_PATH, DecoderType.BOTH, EncodingType.ADVANCED, false);
    assertEquals(
        0,
        result.numErrorsAdvanced(),
        "Error for " + tileId + "/advanced: " + result.numErrorsAdvanced());
  }

  @DisplayName("Decode OMT Tiles (non-advanced encodings)")
  @ParameterizedTest()
  @MethodSource("omtProvider")
  @Disabled
  public void decodeOMTTiles2(String tileId) throws IOException {
    if (Objects.equals(tileId, "13_4265_5467")
        || Objects.equals(tileId, "14_8298_10748")
        || Objects.equals(tileId, "14_8299_10748")) {
      // TODO remove this special case for these 3 tiles once this bug is fixed:
      // https://github.com/maplibre/maplibre-tile-spec/issues/183
      var exception =
          assertThrows(
              Exception.class,
              () ->
                  testTile(
                      tileId,
                      TestSettings.OMT_MVT_PATH,
                      DecoderType.BOTH,
                      EncodingType.NONADVANCED,
                      false));
      assertTrue(exception.toString().contains("ArrayIndexOutOfBoundsException"));
    } else {
      var result =
          testTile(
              tileId, TestSettings.OMT_MVT_PATH, DecoderType.BOTH, EncodingType.NONADVANCED, false);
      assertEquals(
          0, result.numErrors(), "Error for " + tileId + "/advanced: " + result.numErrors());
    }
  }

  /* Test utility functions */

  private DecodingResult testTile(
      String tileId,
      String tileDirectory,
      DecoderType decoder,
      EncodingType encoding,
      boolean allowSorting)
      throws IOException {
    var mvtFilePath = Paths.get(tileDirectory, tileId + ".mvt");
    var mvTile = MvtUtils.decodeMvt(mvtFilePath);

    var columnMappings = Optional.<List<ColumnMapping>>empty();
    var tileMetadata = MltConverter.createTilesetMetadata(List.of(mvTile), columnMappings, true);

    var allowIdRegeneration = false;
    var optimization =
        new FeatureTableOptimizations(allowSorting, allowIdRegeneration, columnMappings);
    var optimizations =
        TestSettings.OPTIMIZED_MVT_LAYERS.stream()
            .collect(Collectors.toMap(l -> l, l -> optimization));
    var includeIds = true;
    var mlTile =
        MltConverter.convertMvt(
            mvTile, tileMetadata, new ConversionConfig(includeIds, false, optimizations), null);
    var mlTileAdvanced =
        MltConverter.convertMvt(
            mvTile, tileMetadata, new ConversionConfig(includeIds, true, optimizations), null);
    int numErrors = -1;
    int numErrorsAdvanced = -1;
    if (decoder == DecoderType.SEQUENTIAL || decoder == DecoderType.BOTH) {
      if (encoding == EncodingType.ADVANCED || encoding == EncodingType.BOTH) {
        var decodedAdvanced = MltDecoder.decodeMlTile(mlTileAdvanced, tileMetadata);
        numErrorsAdvanced +=
            TestUtils.compareTilesSequential(decodedAdvanced, mvTile, allowSorting);
      }
      if (encoding == EncodingType.NONADVANCED || encoding == EncodingType.BOTH) {
        var decoded = MltDecoder.decodeMlTile(mlTile, tileMetadata);
        numErrors += TestUtils.compareTilesSequential(decoded, mvTile, allowSorting);
      }
    }
    if (decoder == DecoderType.VECTORIZED || decoder == DecoderType.BOTH) {
      throw new NotImplementedException("Vectorized decoding is not available");
    }
    return new DecodingResult(numErrors, numErrorsAdvanced);
  }
}
