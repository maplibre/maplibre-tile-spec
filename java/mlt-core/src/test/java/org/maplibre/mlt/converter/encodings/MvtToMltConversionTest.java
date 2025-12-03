package org.maplibre.mlt.converter.encodings;

import java.io.IOException;
import java.nio.file.Path;
import java.nio.file.Paths;
import java.util.Map;
import org.junit.jupiter.api.Test;
import org.locationtech.jts.util.Assert;
import org.maplibre.mlt.converter.ConversionConfig;
import org.maplibre.mlt.converter.MltConverter;
import org.maplibre.mlt.converter.mvt.MvtUtils;

public class MvtToMltConversionTest {

  @Test
  public void convertMvtToMlt_SimpleExample() throws IOException {
    // INPUT: MVT file path
    Path mvtPath = Paths.get("..", "..", "test", "fixtures", "simple", "point-boolean.mvt");
    System.out.println("Input MVT file: " + mvtPath.toAbsolutePath());

    // Step 1: Decode MVT
    var mvtTile = MvtUtils.decodeMvt(mvtPath);
    System.out.println("MVT decoded: " + mvtTile.layers().size() + " layers");

    // Step 2: Create metadata
    var metadata = MltConverter.createTilesetMetadata(mvtTile, Map.of(), true, false, false);

    // Step 3: Create conversion config
    var config = new ConversionConfig(true, false, false, Map.of());

    // Step 4: Convert MVT to MLT
    byte[] mltTile = MltConverter.convertMvt(mvtTile, metadata, config, null);

    // OUTPUT: MLT byte array
    Assert.isTrue(mltTile.length > 0, "MLT tile should not be empty");
    System.out.println("Output MLT size: " + mltTile.length + " bytes");
  }
}
