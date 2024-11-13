package com.mlt.converter;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertTrue;

import com.mlt.converter.encodings.EncodingUtils;
import com.mlt.converter.mvt.ColumnMapping;
import com.mlt.converter.mvt.MvtUtils;
import com.mlt.decoder.MltDecoder;
import com.mlt.decoder.MltDecoderTest;
import com.mlt.metadata.tileset.MltTilesetMetadata;
import com.mlt.test.constants.TestConstants;
import java.io.File;
import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Paths;
import java.util.List;
import java.util.Map;
import java.util.Optional;
import java.util.stream.Collectors;
import java.util.stream.Stream;
import org.junit.jupiter.api.Disabled;
import org.junit.jupiter.api.Test;
import org.locationtech.jts.util.Assert;

public class MltConverterTest {
  ;

  @Test
  @Disabled
  // Fails currently with org.opentest4j.AssertionFailedError: expected: <STRING> but was: <name:
  // "class"
  public void createTileMetadata_Omt_ValidMetadata() throws IOException {
    var expectedPropertiesScheme =
        Map.of(
            "water", Map.of("class", MltTilesetMetadata.ScalarType.STRING),
            "waterway", Map.of("class", MltTilesetMetadata.ScalarType.STRING),
            "landuse", Map.of("class", MltTilesetMetadata.ScalarType.STRING),
            "transportation",
                Map.of(
                    "class",
                    MltTilesetMetadata.ScalarType.STRING,
                    "brunnel",
                    MltTilesetMetadata.ScalarType.STRING),
            "water_name",
                Map.of(
                    "class",
                    MltTilesetMetadata.ScalarType.STRING,
                    "intermittent",
                    MltTilesetMetadata.ScalarType.INT_64,
                    "name",
                    MltTilesetMetadata.ComplexType.STRUCT),
            "place",
                Map.of(
                    "class",
                    MltTilesetMetadata.ScalarType.STRING,
                    "iso:a2",
                    MltTilesetMetadata.ScalarType.STRING,
                    "name",
                    MltTilesetMetadata.ComplexType.STRUCT));
    var waternameNameProperties =
        List.of(
            "default", "ar", "az", "be", "bg", "br", "bs", "ca", "co", "cs", "cy", "da", "de", "el",
            "en", "eo", "es", "et", "eu", "fi", "fr", "fy", "ga", "he", "hi", "hr", "hu", "hy",
            "id", "int", "is", "it", "ja", "ka", "kk", "ko", "ku", "la", "latin", "lt", "lv", "mk",
            "ml", "mt", "nl", "no", "pl", "pt", "ro", "ru", "sk", "sl", "sr", "sr-Latn", "sv", "ta",
            "th", "tr", "uk", "zh");
    var placeNameProperties =
        Stream.concat(
                waternameNameProperties.stream(),
                List.of("am", "gd", "kn", "lb", "oc", "rm", "sq", "te", "nonlatin").stream())
            .collect(Collectors.toList());

    var tileId = String.format("%s_%s_%s", 5, 16, 21);
    var mvtFilePath = Paths.get(TestConstants.OMT_MVT_PATH, tileId + ".mvt");
    var mvTile = MvtUtils.decodeMvt(mvtFilePath);

    var mapping = new ColumnMapping("name", ":", true);
    var tileMetadata =
        MltConverter.createTilesetMetadata(mvTile, Optional.of(List.of(mapping)), true);

    assertEquals(mvTile.layers().size(), tileMetadata.getFeatureTablesCount());
    for (var i = 0; i < mvTile.layers().size(); i++) {
      var mvtLayer = mvTile.layers().get(i);
      var mltFeatureTableMetadata = tileMetadata.getFeatureTables(i);

      assertEquals(mvtLayer.name(), mltFeatureTableMetadata.getName());

      var idColumnMetadata =
          mltFeatureTableMetadata.getColumnsList().stream()
              .filter(f -> f.getName().equals("id"))
              .findFirst()
              .get();
      System.out.println(mvtLayer.name());
      var expectedIdDataType =
          mvtLayer.name().equals("place")
              ? MltTilesetMetadata.ScalarType.UINT_64
              : MltTilesetMetadata.ScalarType.UINT_32;
      assertEquals(expectedIdDataType, idColumnMetadata.getScalarType().getPhysicalType());

      var geometryColumnMetadata =
          mltFeatureTableMetadata.getColumnsList().stream()
              .filter(f -> f.getName().equals("geometry"))
              .findFirst()
              .get();
      assertEquals(
          MltTilesetMetadata.ComplexType.GEOMETRY,
          geometryColumnMetadata.getComplexType().getPhysicalType());

      var expectedPropertySchema = expectedPropertiesScheme.get(mltFeatureTableMetadata.getName());
      var mltProperties = mltFeatureTableMetadata.getColumnsList();
      for (var expectedProperty : expectedPropertySchema.entrySet()) {
        var mltProperty =
            mltProperties.stream()
                .filter(p -> expectedProperty.getKey().equals(p.getName()))
                .findFirst();
        assertTrue(mltProperty.isPresent());
        var actualDataType = mltProperty.get();
        assertEquals(expectedProperty.getValue(), actualDataType);

        if (actualDataType.equals(MltTilesetMetadata.ComplexType.STRUCT)) {
          var nestedFields = mltProperty.get().getComplexType().getChildrenList();
          var expectedPropertyNames =
              mltFeatureTableMetadata.getName().equals("place")
                  ? placeNameProperties
                  : waternameNameProperties;
          assertEquals(expectedPropertyNames.size(), nestedFields.size());
          for (var child : nestedFields) {
            /* In this test all nested name:* fields are of type string */
            assertEquals(
                MltTilesetMetadata.ScalarType.STRING, child.getScalarField().getPhysicalType());
            assertTrue(expectedPropertyNames.contains(child.getName()));
          }
        }
      }
    }
  }

  /* Amazon Here schema based vector tiles tests  --------------------------------------------------------- */

  @Test
  @Disabled
  public void convert_AmazonRandomZLevels_ValidMLtTile() throws IOException {
    var tiles =
        Stream.of(new File(TestConstants.AMZ_HERE_MVT_PATH).listFiles())
            .filter(file -> !file.isDirectory())
            .map(File::getAbsoluteFile)
            .collect(Collectors.toSet());
    for (var tile : tiles) {
      System.out.println(
          "-------------------------------------------------------------------------------");
      runOmtTest2(tile.getAbsolutePath());
    }
  }

  /* OpenMapTiles schema based vector tiles tests 2  --------------------------------------------------------- */

  @Test
  @Disabled
  // Fails currently with java.lang.IllegalArgumentException: Column mappings are required for
  // nested property columns.
  public void convert_OmtRandomZLevels_ValidMLtTile() throws IOException {
    var tiles =
        Stream.of(new File(TestConstants.OMT_MVT_PATH).listFiles())
            .filter(file -> !file.isDirectory())
            .map(File::getAbsoluteFile)
            .collect(Collectors.toSet());
    for (var tile : tiles) {
      System.out.println(
          "-------------------------------------------------------------------------------");
      runOmtTest2(tile.getAbsolutePath());
    }
  }

  private static void runOmtTest2(String tile) throws IOException {
    var mvtFilePath = Paths.get(tile);
    var mvTile = MvtUtils.decodeMvt(mvtFilePath);

    var columnMapping = new ColumnMapping("name", ":", true);
    var columnMappings = Optional.of(List.of(columnMapping));
    var tileMetadata =
        MltConverter.createTilesetMetadata(mvTile, Optional.of(List.of(columnMapping)), true);

    var allowIdRegeneration = true;
    var allowSorting = false;
    var optimization =
        new FeatureTableOptimizations(allowSorting, allowIdRegeneration, columnMappings);
    // TODO: fix -> either add columMappings per layer or global like when creating the scheme
    var optimizations = Map.of("place", optimization, "water_name", optimization);
    var conversionConfig = new ConversionConfig(true, true, optimizations);
    var mlTile = MltConverter.convertMvt(mvTile, conversionConfig, tileMetadata, false);

    var decodedMlTile = MltDecoder.decodeMlTile(mlTile, tileMetadata);
    MltDecoderTest.compareTilesSequential(decodedMlTile, mvTile);

    var mvtSize = Files.readAllBytes(mvtFilePath).length;
    System.out.printf(
        "MVT size: %s, MLT size: %s, reduction %s%% \n",
        mvtSize / 1024d, mlTile.length / 1024d, (1 - mlTile.length / (double) mvtSize) * 100);
  }

  /* OpenMapTiles schema based vector tiles tests --------------------------------------------------------- */

  @Test
  @Disabled
  // Fails currently with java.nio.file.NoSuchFileException: ../../test/fixtures/omt/mvt/5_a.pbf
  public void shared_delta_dictionary() throws IOException {
    var tileId = String.format("%s_%s_%s", 4, 8, 10);
    // var mvtFilePath = Paths.get(TestConstants.OMT_MVT_PATH, tileId + ".mvt" );
    var mvtFilePath = Paths.get(TestConstants.OMT_MVT_PATH, "5_a" + ".pbf");
    // var mvTile = Files.readAllBytes(mvtFilePath);
    var decodedMvTile = MvtUtils.decodeMvt(mvtFilePath);
    // var tileId2 = String.format("%s_%s_%s", 5, 16, 20);
    // var mvtFilePath2 = Paths.get(TestConstants.OMT_MVT_PATH, tileId2 + ".mvt" );
    var mvtFilePath2 = Paths.get(TestConstants.OMT_MVT_PATH, "11_b" + ".pbf");
    // var mvTile2 = Files.readAllBytes(mvtFilePath2);
    var decodedMvTile2 = MvtUtils.decodeMvt(mvtFilePath2);
    System.out.println("test");
  }

  @Test
  public void convert_OmtTileZ2_ValidMLtTile() throws IOException {
    // TODO: change vector tiles decoder -> polygons are not valid
    var tileId = String.format("%s_%s_%s", 2, 2, 2);
    runOmtTest(tileId);
  }

  @Test
  public void convert_OmtTilesZ3_ValidMltTile() throws IOException {
    runOmtTests(3, 4, 4, 5, 5);
  }

  @Test
  public void convert_OmtTileZ4_ValidMltTile() throws IOException {
    var tileId = String.format("%s_%s_%s", 4, 8, 10);
    runOmtTest(tileId);

    var tileId2 = String.format("%s_%s_%s", 4, 3, 9);
    runOmtTest(tileId2);
  }

  @Test
  public void convert_OmtTileZ5_ValidMltTile() throws IOException {
    runOmtTests(5, 16, 17, 20, 21);
  }

  @Test
  public void convert_OmtTileZ6_ValidMltTile() throws IOException {
    runOmtTests(6, 32, 34, 41, 42);
  }

  @Test
  public void convert_OmtTilesZ7_ValidMltTile() throws IOException {
    runOmtTests(7, 66, 68, 83, 85);
  }

  @Test
  public void convert_OmtTilesZ8_ValidMltTile() throws IOException {
    runOmtTests(8, 132, 135, 170, 171);
  }

  @Test
  public void convert_OmtTilesZ9_ValidMltTile() throws IOException {
    runOmtTests(9, 264, 266, 340, 342);
  }

  @Test
  public void convert_OmtTilesZ10_ValidMltTile() throws IOException {
    runOmtTests(10, 530, 533, 682, 684);
  }

  @Test
  public void convert_OmtTilesZ11_ValidMltTile() throws IOException {
    runOmtTests(11, 1062, 1065, 1366, 1368);
  }

  @Test
  public void convert_OmtTilesZ12_ValidMltTile() throws IOException {
    runOmtTests(12, 2130, 2134, 2733, 2734);
  }

  @Test
  public void convert_OmtTilesZ13_ValidMltTile() throws IOException {
    runOmtTests(13, 4264, 4267, 5467, 5468);
  }

  @Test
  public void convert_OmtTileZ14_ValidMltTile() throws IOException {
    runOmtTests(14, 8296, 8300, 10748, 10749);
  }

  private static void runOmtTest(String tileId) throws IOException {
    var mvtFilePath = Paths.get(TestConstants.OMT_MVT_PATH, tileId + ".mvt");
    var mvTile = MvtUtils.decodeMvt(mvtFilePath);

    var columnMapping = new ColumnMapping("name", ":", true);
    var columnMappings = Optional.of(List.of(columnMapping));
    var tileMetadata =
        MltConverter.createTilesetMetadata(mvTile, Optional.of(List.of(columnMapping)), true);

    var allowIdRegeneration = true;
    var allowSorting = false;
    var optimization =
        new FeatureTableOptimizations(allowSorting, allowIdRegeneration, columnMappings);
    // TODO: fix -> either add columMappings per layer or global like when creating the scheme
    var optimizations = Map.of("place", optimization, "water_name", optimization);
    var conversionConfig = new ConversionConfig(true, true, optimizations);
    var mlTile = MltConverter.convertMvt(mvTile, conversionConfig, tileMetadata, false);

    // var decodedMlTile = MltDecoder.decodeMlTile(mlTile, tileMetadata);
    // MltDecoderTest.compareTiles(decodedMlTile, mvTile);

    var mvtSize = Files.readAllBytes(mvtFilePath).length;
    System.out.printf(
        "MVT size: %s, MLT size: %s, reduction %s%% \n",
        mvtSize / 1024d, mlTile.length / 1024d, (1 - mlTile.length / (double) mvtSize) * 100);
  }

  private static void runOmtTests(int zoom, int minX, int maxX, int minY, int maxY)
      throws IOException {
    var ratios = 0d;
    var counter = 0;
    for (var x = minX; x <= maxX; x++) {
      for (var y = minY; y <= maxY; y++) {
        var tileId = String.format("%s_%s_%s", zoom, x, y);
        var mvtFilePath = Paths.get(TestConstants.OMT_MVT_PATH, tileId + ".mvt");
        var mvTile = Files.readAllBytes(mvtFilePath);
        var decodedMvTile = MvtUtils.decodeMvt(mvtFilePath);

        try {
          System.out.printf(
              "z:%s, x:%s, y:%s -------------------------------------------- \n", zoom, x, y);
          var columnMapping = new ColumnMapping("name", ":", true);
          var columnMappings = Optional.of(List.of(columnMapping));
          var tileMetadata =
              MltConverter.createTilesetMetadata(
                  decodedMvTile, Optional.of(List.of(columnMapping)), true);

          var allowIdRegeneration = true;
          var allowSorting = false;
          var optimization =
              new FeatureTableOptimizations(allowSorting, allowIdRegeneration, columnMappings);
          // TODO: fix -> either add columMappings per layer or global like when creating the scheme
          var optimizations =
              Map.of(
                  "place",
                  optimization,
                  "water_name",
                  optimization,
                  "transportation",
                  optimization,
                  "transportation_name",
                  optimization,
                  "park",
                  optimization,
                  "mountain_peak",
                  optimization,
                  "poi",
                  optimization,
                  "waterway",
                  optimization,
                  "aerodrome_label",
                  optimization);
          var conversionConfig = new ConversionConfig(true, true, optimizations);
          var mlTile = MltConverter.convertMvt(decodedMvTile, conversionConfig, tileMetadata, false);

          // var decodedMlTile = MltDecoder.decodeMlTile(mlTile, tileMetadata);
          // MltDecoderTest.compareTiles(decodedMlTile, decodedMvTile);

          ratios += printStats(mvTile, mlTile);
          counter++;
        } catch (Exception e) {
          System.out.println(e);
        }
      }
    }

    System.out.println("Total ratio: " + (ratios / counter));
  }

  private static double printStats(byte[] mvTile, byte[] mlTile) throws IOException {
    var mvtGzipBuffer = EncodingUtils.gzip(mvTile);
    var mltGzipBuffer = EncodingUtils.gzip(mlTile);

    System.out.printf("MVT size: %s, Gzip MVT size: %s%n", mvTile.length, mvtGzipBuffer.length);
    System.out.printf("MLT size: %s, Gzip MLT size: %s%n", mlTile.length, mltGzipBuffer.length);
    System.out.printf(
        "Ratio uncompressed: %s, Ratio compressed: %s%n",
        ((double) mvTile.length) / mlTile.length, ((double) mvTile.length) / mlTile.length);

    var compressionRatio = (1 - (1 / (((double) mvTile.length) / mlTile.length))) * 100;
    var compressionRatioCompressed =
        (1 - (1 / (((double) mvtGzipBuffer.length) / mltGzipBuffer.length))) * 100;
    System.out.printf(
        "Reduction uncompressed: %s%%, Reduction compressed: %s%% %n",
        compressionRatio, compressionRatioCompressed);
    return compressionRatio;
  }

  /* Bing Maps Tests --------------------------------------------------------- */

  @Test
  @Disabled
  public void convert_BingMaps_Z4Tile() throws IOException {
    var fileNames = List.of("4-8-5", "4-9-5", "4-12-6", "4-13-6");
    runBingTests(fileNames);
  }

  @Test
  @Disabled
  // Fails currently with java.lang.IllegalArgumentException: Column mappings are required for
  // nested property columns.
  public void convert_BingMaps_Z5Tiles() throws IOException {
    var fileNames = List.of("5-16-11", "5-16-9", "5-17-11", "5-17-10", "5-15-10");
    runBingTests(fileNames);
  }

  @Test
  @Disabled
  public void convert_BingMaps_Z6Tiles() throws IOException {
    var fileNames = List.of("6-32-22", "6-33-22", "6-32-23", "6-32-21");
    runBingTests(fileNames);
  }

  @Test
  @Disabled
  public void convert_BingMaps_Z7Tiles() throws IOException {
    var fileNames = List.of("7-65-42", "7-66-42", "7-66-43", "7-66-44", "7-69-44");
    runBingTests(fileNames);
  }

  @Test
  public void convertMapTileAndTriangulatePolygons() throws IOException { // TODO: ADD test to validate indexBuffer
    var mvtFilePath = Paths.get(TestConstants.BING_MVT_PATH, "4-8-5" + ".mvt");
    var decodedMvTile = MvtUtils.decodeMvt(mvtFilePath);
    var columnMapping = new ColumnMapping("name", ":", true);
    var columnMappings = Optional.of(List.of(columnMapping));
    var optimization =
            new FeatureTableOptimizations(false, false, columnMappings);
    var optimizations = Map.of("place", optimization, "water_name", optimization);
    var conversionConfig = new ConversionConfig(true, true, optimizations);

    var tileMetadata =
            MltConverter.createTilesetMetadata(
                    decodedMvTile, Optional.of(List.of(columnMapping)), true);

    // converted to MLT
    var mlTile = MltConverter.convertMvt(decodedMvTile, conversionConfig, tileMetadata, false);

    var decodedMlTile = MltDecoder.decodeMlTile(mlTile, tileMetadata);

    Assert.isTrue(decodedMlTile.layers().size() > 0);
  }

  private void runBingTests(List<String> fileNames) throws IOException {
    var compressionRatios = 0d;
    for (var fileName : fileNames) {
      compressionRatios += runBingTest(fileName);
    }

    System.out.printf(
        "Total Compression Ratio Without Gzip: %s", compressionRatios / fileNames.size());
  }

  private double runBingTest(String tileId) throws IOException {
    System.out.println(tileId + " ------------------------------------------");
    var mvtFilePath = Paths.get(TestConstants.BING_MVT_PATH, tileId + ".mvt");
    var mvTile = Files.readAllBytes(mvtFilePath);
    var decodedMvTile = MvtUtils.decodeMvt(Paths.get(TestConstants.BING_MVT_PATH, tileId + ".mvt"));

    var columnMapping = new ColumnMapping("name", ":", true);
    var columnMappings = Optional.of(List.of(columnMapping));
    var tileMetadata =
        MltConverter.createTilesetMetadata(
            decodedMvTile, Optional.of(List.of(columnMapping)), true);

    var allowIdRegeneration = true;
    var allowSorting = false;
    var optimization =
        new FeatureTableOptimizations(allowSorting, allowIdRegeneration, columnMappings);
    // TODO: fix -> either add columMappings per layer or global like when creating the scheme
    var optimizations = Map.of("place", optimization, "water_name", optimization);
    var conversionConfig = new ConversionConfig(true, true, optimizations);
    var mlTile = MltConverter.convertMvt(decodedMvTile, conversionConfig, tileMetadata, false);

    var decodedMlTile = MltDecoder.decodeMlTile(mlTile, tileMetadata);
    MltDecoderTest.compareTilesSequential(decodedMlTile, decodedMvTile);

    var compressionRatio = printStats(mvTile, mlTile);

    return compressionRatio;
  }
}
