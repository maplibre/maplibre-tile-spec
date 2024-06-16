package com.mlt.decoder;

import static org.junit.jupiter.api.Assertions.assertEquals;

import com.mlt.TestSettings;
import com.mlt.converter.ConversionConfig;
import com.mlt.converter.FeatureTableOptimizations;
import com.mlt.converter.MltConverter;
import com.mlt.converter.mvt.ColumnMapping;
import com.mlt.converter.mvt.MapboxVectorTile;
import com.mlt.converter.mvt.MvtUtils;
import com.mlt.data.MapLibreTile;
import com.mlt.metadata.tileset.MltTilesetMetadata;
import com.mlt.vector.FeatureTable;
import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Paths;
import java.util.List;
import java.util.Map;
import java.util.Optional;
import java.util.stream.Collectors;
import org.junit.jupiter.api.Disabled;
import org.junit.jupiter.api.Test;

@FunctionalInterface
interface TriConsumer<A, B, C> {
  void apply(A a, B b, C c) throws IOException;
}

public class MltDecoderTest {

  /* Bing Maps tests --------------------------------------------------------- */

  @Test
  public void decodeMlTileVectorized_Z4BingMapsTile() throws IOException {
    var tileIds = List.of("4-8-5", "4-9-5", "4-12-6", "4-13-6");
    testBingTilesVectorized(tileIds);
  }

  @Test
  public void decodeMlTileVectorized_Z5BingMapsTile() throws IOException {
    // TODO: fix "5-16-9" and "5-15-10"
    var tileIds = List.of("5-16-11", "5-17-11", "5-17-10");
    testBingTilesVectorized(tileIds);
  }

  @Test
  public void decodeMlTileVectorized_Z6BingMapsTile() throws IOException {
    var tileIds = List.of("6-32-22", "6-33-22", "6-32-23", "6-32-21");
    testBingTilesVectorized(tileIds);
  }

  @Test
  public void decodeMlTileVectorized_Z7BingMapsTile() throws IOException {
    var tileIds = List.of("7-65-42", "7-66-42", "7-66-43", "7-66-44");
    testBingTilesVectorized(tileIds);
  }

  private void testBingTilesVectorized(List<String> tileIds) throws IOException {
    for (var tileId : tileIds) {
      testTileVectorized(tileId, TestSettings.BING_MVT_PATH);
    }
  }

  /* OpenMapTiles schema based vector tiles tests  --------------------------------------------------------- */

  /* decode tile in an in-memory format optimized for random access */

  @Test
  public void decodeMlTileVectorized_Z2() throws IOException {
    var tileId = String.format("%s_%s_%s", 2, 2, 2);
    testTileVectorized(tileId, TestSettings.OMT_MVT_PATH);
  }

  @Test
  public void decodeMlTileVectorized_Z3() throws IOException {
    var tileId = String.format("%s_%s_%s", 3, 4, 5);
    testTileVectorized(tileId, TestSettings.OMT_MVT_PATH);
  }

  @Test
  public void decodeMlTileVectorized_Z4() throws IOException {
    var tileId = String.format("%s_%s_%s", 4, 8, 10);
    testTileVectorized(tileId, TestSettings.OMT_MVT_PATH);

    var tileId2 = String.format("%s_%s_%s", 4, 3, 9);
    testTileVectorized(tileId2, TestSettings.OMT_MVT_PATH);
  }

  @Test
  public void decodeMlTileVectorized_Z5() throws IOException {
    var tileId = String.format("%s_%s_%s", 5, 16, 21);
    testTileVectorized(tileId, TestSettings.OMT_MVT_PATH);

    var tileId2 = String.format("%s_%s_%s", 5, 16, 20);
    testTileVectorized(tileId2, TestSettings.OMT_MVT_PATH);
  }

  @Test
  public void decodeMlTileVectorized_Z6() throws IOException {
    var tileId = String.format("%s_%s_%s", 6, 32, 41);
    testTileVectorized(tileId, TestSettings.OMT_MVT_PATH);

    var tileId2 = String.format("%s_%s_%s", 6, 33, 42);
    testTileVectorized(tileId2, TestSettings.OMT_MVT_PATH);
  }

  @Test
  public void decodeMlTileVectorized_Z7() throws IOException {
    var tileId = String.format("%s_%s_%s", 7, 66, 84);
    testTileVectorized(tileId, TestSettings.OMT_MVT_PATH);

    var tileId2 = String.format("%s_%s_%s", 7, 66, 85);
    testTileVectorized(tileId2, TestSettings.OMT_MVT_PATH);
  }

  @Test
  public void decodeMlTileVectorized_Z8() throws IOException {
    var tileId = String.format("%s_%s_%s", 8, 134, 171);
    testTileVectorized(tileId, TestSettings.OMT_MVT_PATH);

    var tileId2 = String.format("%s_%s_%s", 8, 132, 170);
    testTileVectorized(tileId2, TestSettings.OMT_MVT_PATH);
  }

  @Test
  public void decodeMlTileVectorized_Z9() throws IOException {
    var tileId = String.format("%s_%s_%s", 9, 265, 341);
    testTileVectorized(tileId, TestSettings.OMT_MVT_PATH);
  }

  @Test
  public void decodeMlTileVectorized_10() throws IOException {
    var tileId = String.format("%s_%s_%s", 10, 532, 682);
    testTileVectorized(tileId, TestSettings.OMT_MVT_PATH);
  }

  @Test
  public void decodeMlTileVectorized_11() throws IOException {
    var tileId = String.format("%s_%s_%s", 11, 1064, 1367);
    testTileVectorized(tileId, TestSettings.OMT_MVT_PATH);
  }

  @Test
  public void decodeMlTileVectorized_12() throws IOException {
    var tileId = String.format("%s_%s_%s", 12, 2132, 2734);
    testTileVectorized(tileId, TestSettings.OMT_MVT_PATH);
  }

  @Test
  public void decodeMlTileVectorized_13() throws IOException {
    var tileId = String.format("%s_%s_%s", 13, 4265, 5467);
    testTileVectorized(tileId, TestSettings.OMT_MVT_PATH);
  }

  @Test
  public void decodeMlTileVectorized_14() throws IOException {
    var tileId = String.format("%s_%s_%s", 14, 8298, 10748);
    testTileVectorized(tileId, TestSettings.OMT_MVT_PATH);

    var tileId2 = String.format("%s_%s_%s", 14, 8299, 10748);
    testTileVectorized(tileId2, TestSettings.OMT_MVT_PATH);
  }

  /** Decode tiles in an in-memory format optimized for sequential access */
  @Test
  @Disabled
  // org.opentest4j.AssertionFailedError: expected: <san pauro> but was: <null>
  public void decodeMlTile_Z2() throws IOException {
    var tileId = String.format("%s_%s_%s", 2, 2, 2);
    testTileSequential(tileId, TestSettings.OMT_MVT_PATH);
  }

  @Test
  public void decodeMlTile_Z4() throws IOException {
    var tileId = String.format("%s_%s_%s", 4, 8, 10);
    testTileSequential(tileId, TestSettings.OMT_MVT_PATH);

    var tileId2 = String.format("%s_%s_%s", 4, 3, 9);
    testTileSequential(tileId2, TestSettings.OMT_MVT_PATH);
  }

  @Test
  public void decodeMlTile_Z5() throws IOException {
    var tileId = String.format("%s_%s_%s", 5, 16, 21);
    testTileSequential(tileId, TestSettings.OMT_MVT_PATH);

    var tileId2 = String.format("%s_%s_%s", 5, 16, 20);
    testTileSequential(tileId2, TestSettings.OMT_MVT_PATH);
  }

  @Test
  // org.opentest4j.AssertionFailedError: expected: <1> but was: <2>
  public void decodeMlTile_Z6() throws IOException {
    var tileId = String.format("%s_%s_%s", 6, 32, 41);
    testTileSequential(tileId, TestSettings.OMT_MVT_PATH);
  }

  @Test
  public void decodeMlTile_Z10() throws IOException {
    var tileId = String.format("%s_%s_%s", 10, 532, 684);
    testTileSequential(tileId, TestSettings.OMT_MVT_PATH);
  }

  @Test
  public void decodeMlTile_Z13() throws IOException {
    var tileId = String.format("%s_%s_%s", 13, 4265, 5467);
    testTileSequential(tileId, TestSettings.OMT_MVT_PATH);
  }

  @Test
  @Disabled
  // java.lang.IllegalArgumentException: Invalid number of points in LineString (found 1 - must be 0
  // or >= 2)
  public void decodeMlTile_Z14() throws IOException {
    var tileId = String.format("%s_%s_%s", 14, 8298, 10748);
    testTileSequential(tileId, TestSettings.OMT_MVT_PATH);
  }

  private void testTileVectorized(String tileId, String tileDirectory) throws IOException {
    testTile(
        tileId,
        tileDirectory,
        (mlTile, tileMetadata, mvTile) -> {
          var decodedTile = MltDecoder.decodeMlTileVectorized(mlTile, tileMetadata);
          compareTilesVectorized(decodedTile, mvTile);
        });
  }

  private static void compareTilesVectorized(
      FeatureTable[] featureTables, MapboxVectorTile mvTile) {
    var mvtLayers = mvTile.layers();
    for (var i = 0; i < mvtLayers.size(); i++) {
      var featureTable = featureTables[i];
      var mvtLayer = mvtLayers.get(i);
      var mvtFeatures = mvtLayer.features();
      var featureIterator = featureTable.iterator();

      for (var j = 0; j < mvtFeatures.size(); j++) {
        var mvtFeature = mvtFeatures.get(j);
        var mltFeature = featureIterator.next();

        assertEquals(mvtFeature.id(), mltFeature.id());

        var mvtGeometry = mvtFeature.geometry();
        var mltGeometry = mltFeature.geometry();
        assertEquals(mvtGeometry, mltGeometry);

        var mltProperties = mltFeature.properties();
        for (var property : mltProperties.entrySet()) {
          var mltPropertyKey = property.getKey();
          var mltPropertyValue = property.getValue();
          if (mltPropertyValue instanceof Map<?, ?>) {
            /* Handle shared dictionary case -> currently only String is supported
             * as nested property in the converter, so only handle this case */
            var mvtProperties = mvtFeature.properties();
            var nestedStringValues = (Map<String, String>) mltPropertyValue;
            var mvtStringProperties =
                mvtProperties.entrySet().stream()
                    .filter(
                        p -> p.getKey().contains(mltPropertyKey) && p.getValue() instanceof String)
                    .toList();
            // TODO: verify why mlt seems to have a property more than mvt on the
            // name:* column in some tiles
            for (var mvtProperty : mvtStringProperties) {
              var mvtPropertyKey = mvtProperty.getKey();
              var mvtPropertyValue = mvtProperty.getValue();
              var mltValue = nestedStringValues.get(mvtPropertyKey);

              if (mvtPropertyKey.equals("name:ja:rm")) {
                // TODO: fix -> currently the converter can't handle a triple nested property name
                System.out.println(
                    "Skip verification for the name:ja:rm property name since it is currently"
                        + "not supported in the converter.");
                continue;
              }

              assertEquals(mvtPropertyValue, mltValue);
            }
          } else {
            assertEquals(mvtFeature.properties().get(mltPropertyKey), mltPropertyValue);
          }
        }
      }
    }
  }

  private void testTileSequential(String tileId, String tileDirectory) throws IOException {
    testTile(
        tileId,
        tileDirectory,
        (mlTile, tileMetadata, mvTile) -> {
          var decodedTile = MltDecoder.decodeMlTile(mlTile, tileMetadata);
          compareTilesSequential(decodedTile, mvTile);
        });
  }

  public static void compareTilesSequential(MapLibreTile mlTile, MapboxVectorTile mvTile) {
    var mltLayers = mlTile.layers();
    var mvtLayers = mvTile.layers();

    for (var i = 0; i < mvtLayers.size(); i++) {
      var mltLayer = mltLayers.get(i);
      var mvtLayer = mvtLayers.get(i);
      var mltFeatures = mltLayer.features();
      var mvtFeatures = mvtLayer.features();
      for (var j = 0; j < mvtFeatures.size(); j++) {
        var mvtFeature = mvtFeatures.get(j);
        var mltFeature =
            mltFeatures.stream().filter(f -> f.id() == mvtFeature.id()).findFirst().get();

        assertEquals(mvtFeature.id(), mltFeature.id());

        var mltGeometry = mltFeature.geometry();
        var mvtGeometry = mvtFeature.geometry();
        assertEquals(mvtGeometry, mltGeometry);

        var mltProperties = mltFeature.properties();
        var mvtProperties = mvtFeature.properties();
        for (var mvtProperty : mvtProperties.entrySet()) {
          /*if(mvtProperty.getKey().contains("name:ja:rm")){
              System.out.println(mvtProperty.getKey() + " " + mvtProperty.getValue() + " " + mltProperties.get(mvtProperty.getKey()) + " " + j + " " + i);
              continue;
          }*/

          var mltProperty = mltProperties.get(mvtProperty.getKey());
          assertEquals(mvtProperty.getValue(), mltProperty);
        }
      }
    }
  }

  private void testTile(
      String tileId,
      String tileDirectory,
      TriConsumer<byte[], MltTilesetMetadata.TileSetMetadata, MapboxVectorTile> decodeAndCompare)
      throws IOException {
    var mvtFilePath = Paths.get(tileDirectory, tileId + ".mvt");
    var mvTile = MvtUtils.decodeMvt(mvtFilePath);

    var columnMapping = new ColumnMapping("name", ":", true);
    var columnMappings = Optional.of(List.of(columnMapping));
    var tileMetadata = MltConverter.createTilesetMetadata(mvTile, columnMappings, true);

    var allowIdRegeneration = true;
    var allowSorting = false;
    // var allowIdRegeneration = true;
    // var allowSorting = true;
    var optimization =
        new FeatureTableOptimizations(allowSorting, allowIdRegeneration, columnMappings);
    // TODO: fix -> either add columMappings per layer or global like when creating the scheme
    var optimizations =
        TestSettings.OPTIMIZED_MVT_LAYERS.stream()
            .collect(Collectors.toMap(l -> l, l -> optimization));
    var mlTile =
        MltConverter.convertMvt(
            mvTile, new ConversionConfig(true, true, optimizations), tileMetadata);

    decodeAndCompare.apply(mlTile, tileMetadata, mvTile);

    System.out.println("Ratio: " + Files.readAllBytes(mvtFilePath).length / (double) mlTile.length);
    System.out.println(
        "Reduction: "
            + ((1 - (double) mlTile.length / Files.readAllBytes(mvtFilePath).length) * 100));
  }
}
