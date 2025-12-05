package org.maplibre.mlt.converter.encodings;

import java.io.IOException;
import java.nio.file.Path;
import java.nio.file.Paths;
import java.util.Map;
import org.junit.jupiter.api.Test;
import org.locationtech.jts.util.Assert;
import org.maplibre.mlt.converter.ConversionConfig;
import org.maplibre.mlt.converter.ConversionConfig.IntegerEncodingOption;
import org.maplibre.mlt.converter.MltConverter;
import org.maplibre.mlt.converter.mvt.MvtUtils;
import org.maplibre.mlt.decoder.MltDecoder;
import org.maplibre.mlt.data.MapLibreTile;

public class MvtToMltConversionTest {

  @Test
  public void convertMvtToMlt_SimpleExample() throws IOException {
    // INPUT: MVT file path - using OMT (OpenMapTiles) tile with multiple layers
    Path mvtPath = Paths.get("..", "..", "test", "fixtures", "omt", "7_66_84.mvt");
    System.out.println("Input MVT file: " + mvtPath.toAbsolutePath());

    // Step 1: Decode MVT
    var mvtTile = MvtUtils.decodeMvt(mvtPath);
    System.out.println("MVT decoded: " + mvtTile.layers().size() + " layers");
    int totalFeatures = 0;
    for (var layer : mvtTile.layers()) {
      int featureCount = layer.features().size();
      totalFeatures += featureCount;
      System.out.println("  Layer '" + layer.name() + "': " + featureCount + " features");
    }
    System.out.println("Total features: " + totalFeatures);

    // Step 2: Create metadata
    var metadata = MltConverter.createTilesetMetadata(mvtTile, Map.of(), true, false, false);

    // Step 3: Create conversion config using builder
    var config =
        ConversionConfig.builder()
            .includeIds(true)
            .useFastPFOR(false)
            .useFSST(false)
            .optimizations(Map.of())
            .integerEncoding(IntegerEncodingOption.RLE)
            .build();

    // Step 4: Convert MVT to MLT
    byte[] mltTile = MltConverter.convertMvt(mvtTile, metadata, config, null);

    // OUTPUT: MLT byte array
    Assert.isTrue(mltTile.length > 0, "MLT tile should not be empty");
    System.out.println("Output MLT size: " + mltTile.length + " bytes");

    // Step 5: Decode MLT back to inspect contents
    MapLibreTile decodedMlt = MltDecoder.decodeMlTile(mltTile);
    System.out.println("\nDecoded MLT contents:");
    System.out.println("Layers: " + decodedMlt.layers().size());

    int decodedTotalFeatures = 0;
    for (var layer : decodedMlt.layers()) {
      int featureCount = layer.features().size();
      decodedTotalFeatures += featureCount;
      System.out.println("\nLayer: " + layer.name());
      System.out.println("  Extent: " + layer.tileExtent());
      System.out.println("  Features: " + featureCount);

      // Show first feature as example
      if (featureCount > 0) {
        var feature = layer.features().get(0);
        System.out.println("  Example feature:");
        System.out.println("    ID: " + feature.id());
        System.out.println("    Geometry: " + feature.geometry().getGeometryType());
        System.out.println("    Properties count: " + feature.properties().size());
        if (!feature.properties().isEmpty()) {
          var firstProp = feature.properties().entrySet().iterator().next();
          System.out.println("    Sample property: " + firstProp.getKey() + " = " + firstProp.getValue());
        }
      }
    }
    System.out.println("\nTotal decoded features: " + decodedTotalFeatures);
    long mvtFileSize = mvtPath.toFile().length();
    System.out.println("MVT file size: " + mvtFileSize + " bytes");
    System.out.println("MLT size: " + mltTile.length + " bytes");
    if (mvtFileSize > 0) {
      double ratio = (double) mltTile.length / mvtFileSize;
      System.out.println("Size ratio (MLT/MVT): " + String.format("%.2f", ratio));
    }
  }
}
