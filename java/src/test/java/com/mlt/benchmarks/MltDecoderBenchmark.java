package com.mlt.benchmarks;

import com.mlt.TestSettings;
import com.mlt.converter.ConversionConfig;
import com.mlt.converter.FeatureTableOptimizations;
import com.mlt.converter.MltConverter;
import com.mlt.converter.mvt.ColumnMapping;
import com.mlt.converter.mvt.MapboxVectorTile;
import com.mlt.converter.mvt.MvtUtils;
import java.io.ByteArrayInputStream;
import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Paths;
import java.util.List;
import java.util.Map;
import java.util.Optional;
import org.junit.jupiter.api.Test;

/**
 * Quick and dirty benchmarks for the decoding of OpenMapTiles schema based tiles into the MVT and
 * MLT in-memory representations. Can be used for simple profiling. For more proper benchmarks based
 * on JMH see {@link com.mlt.OmtDecoderBenchmark}
 */
public class MltDecoderBenchmark {

  @Test
  public void decodeMlTileVectorized_Z2() throws IOException {
    var tileId = String.format("%s_%s_%s", 2, 2, 2);
    benchmarkDecoding(tileId);
  }

  @Test
  public void decodeMlTileVectorized_Z3() throws IOException {
    var tileId = String.format("%s_%s_%s", 3, 4, 5);
    benchmarkDecoding(tileId);
  }

  @Test
  public void decodeMlTileVectorized_Z4() throws IOException {
    var tileId = String.format("%s_%s_%s", 4, 8, 10);
    benchmarkDecoding(tileId);

    var tileId2 = String.format("%s_%s_%s", 4, 3, 9);
    benchmarkDecoding(tileId2);
  }

  @Test
  public void decodeMlTileVectorized_Z5() throws IOException {
    var tileId = String.format("%s_%s_%s", 5, 16, 21);
    benchmarkDecoding(tileId);

    var tileId2 = String.format("%s_%s_%s", 5, 16, 20);
    benchmarkDecoding(tileId2);
  }

  @Test
  public void decodeMlTileVectorized_Z6() throws IOException {
    var tileId = String.format("%s_%s_%s", 6, 33, 42);
    benchmarkDecoding(tileId);
  }

  @Test
  public void decodeMlTileVectorized_Z7() throws IOException {
    var tileId = String.format("%s_%s_%s", 7, 66, 85);
    benchmarkDecoding(tileId);
  }

  @Test
  public void decodeMlTileVectorized_Z8() throws IOException {
    var tileId = String.format("%s_%s_%s", 8, 132, 170);
    benchmarkDecoding(tileId);
  }

  @Test
  public void decodeMlTileVectorized_Z9() throws IOException {
    var tileId = String.format("%s_%s_%s", 9, 265, 341);
    benchmarkDecoding(tileId);
  }

  @Test
  public void decodeMlTileVectorized_Z10() throws IOException {
    var tileId = String.format("%s_%s_%s", 10, 532, 682);
    benchmarkDecoding(tileId);
  }

  @Test
  public void decodeMlTileVectorized_Z11() throws IOException {
    var tileId = String.format("%s_%s_%s", 11, 1064, 1367);
    benchmarkDecoding(tileId);
  }

  @Test
  public void decodeMlTileVectorized_Z12() throws IOException {
    var tileId = String.format("%s_%s_%s", 12, 2132, 2734);
    benchmarkDecoding(tileId);
  }

  @Test
  public void decodeMlTileVectorized_Z13() throws IOException {
    var tileId = String.format("%s_%s_%s", 13, 4265, 5467);
    benchmarkDecoding(tileId);
  }

  @Test
  public void decodeMlTileVectorized_Z14() throws IOException {
    var tileId = String.format("%s_%s_%s", 14, 8298, 10748);
    benchmarkDecoding(tileId);

    var tileId2 = String.format("%s_%s_%s", 14, 8299, 10748);
    benchmarkDecoding(tileId2);
  }

  @Test
  public void benchmarkSuite() throws IOException {
    System.out.println("Zoom 2 ---------------------------------------");
    decodeMlTileVectorized_Z2();
    System.out.println("Zoom 3 ---------------------------------------");
    decodeMlTileVectorized_Z3();
    System.out.println("Zoom 4 ---------------------------------------");
    decodeMlTileVectorized_Z4();
    System.out.println("Zoom 5 ---------------------------------------");
    decodeMlTileVectorized_Z5();
    System.out.println("Zoom 6 ---------------------------------------");
    decodeMlTileVectorized_Z6();
    System.out.println("Zoom 7 ---------------------------------------");
    decodeMlTileVectorized_Z7();
    System.out.println("Zoom 8 ---------------------------------------");
    decodeMlTileVectorized_Z8();
    System.out.println("Zoom 9 ---------------------------------------");
    decodeMlTileVectorized_Z9();
    System.out.println("Zoom 10 ---------------------------------------");
    decodeMlTileVectorized_Z10();
    System.out.println("Zoom 11 ---------------------------------------");
    decodeMlTileVectorized_Z11();
    System.out.println("Zoom 12 ---------------------------------------");
    decodeMlTileVectorized_Z12();
    System.out.println("Zoom 13 ---------------------------------------");
    decodeMlTileVectorized_Z13();
    System.out.println("Zoom 14 ---------------------------------------");
    decodeMlTileVectorized_Z14();
  }

  private void benchmarkDecoding(String tileId) throws IOException {
    var mvtFilePath = Paths.get(TestSettings.OMT_MVT_PATH, tileId + ".mvt");

    var mvt = Files.readAllBytes(mvtFilePath);
    var mvtTimeElapsed = 0L;
    var is = new ByteArrayInputStream(mvt);
    for (int i = 0; i <= 200; i++) {
      long start = System.currentTimeMillis();
      var mvTile = MvtUtils.decodeMvtFast(mvt);
      long finish = System.currentTimeMillis();
      is.reset();

      if (i > 100) {
        mvtTimeElapsed += (finish - start);
      }
    }
    System.out.println("MVT decoding time: " + (mvtTimeElapsed / 100.0));

    MapboxVectorTile mvTile = MvtUtils.decodeMvt(mvtFilePath);

    var columnMapping = new ColumnMapping("name", ":", true);
    var columnMappings = Optional.of(List.of(columnMapping));
    var tileMetadata = MltConverter.createTilesetMetadata(List.of(mvTile), columnMappings, true);

    var allowIdRegeneration = true;
    var allowSorting = false;
    // var allowIdRegeneration = true;
    // var allowSorting = true;
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
    var mlTile =
        MltConverter.convertMvt(
            mvTile, new ConversionConfig(true, true, optimizations), tileMetadata);

    var mltTimeElapsed = 0L;
    for (int i = 0; i <= 200; i++) {
      long start = System.currentTimeMillis();
      // var decodedTile = MltDecoder.decodeMlTileVectorized(mlTile, tileMetadata);
      long finish = System.currentTimeMillis();

      if (i > 100) {
        mltTimeElapsed += (finish - start);
      }
    }
    System.out.println("MLT decoding time: " + (mltTimeElapsed / 100.0));
  }
}
