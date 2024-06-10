package com.mlt.decoder;

import com.mlt.converter.ConversionConfig;
import com.mlt.converter.FeatureTableOptimizations;
import com.mlt.converter.MltConverter;
import com.mlt.converter.mvt.ColumnMapping;
import com.mlt.converter.mvt.MapboxVectorTile;
import com.mlt.converter.mvt.MvtUtils;
import com.mlt.test.constants.TestConstants;
import org.junit.jupiter.api.Test;

import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Paths;
import java.util.List;
import java.util.Map;
import java.util.Optional;

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

    private void benchmarkDecoding(String tileId) throws IOException {
        var mvtFilePath = Paths.get(TestConstants.OMT_MVT_PATH, tileId + ".mvt" );

        var mvt = Files.readAllBytes(mvtFilePath);
        var mvtTimeElapsed = 0l;
        for(int i = 0; i <= 200; i++) {
            long start = System.currentTimeMillis();
            //var mvTile = MvtUtils.decodeMvtFast(mvt);
            var mvTile = MvtUtils.decodeMvt2Fast(mvt);
            long finish = System.currentTimeMillis();

            if(i > 100){
                mvtTimeElapsed += (finish - start);
            }
        }
        System.out.println("MVT decoding time: " + (mvtTimeElapsed / 100.0));

        MapboxVectorTile mvTile = MvtUtils.decodeMvt(mvtFilePath);

        var columnMapping = new ColumnMapping("name", ":", true);
        var columnMappings = Optional.of(List.of(columnMapping));
        var tileMetadata = MltConverter.createTilesetMetadata(mvTile, columnMappings, true);

        var allowIdRegeneration = true;
        var allowSorting = false;
        //var allowIdRegeneration = true;
        //var allowSorting = true;
        var optimization = new FeatureTableOptimizations(allowSorting, allowIdRegeneration, columnMappings);
        //TODO: fix -> either add columMappings per layer or global like when creating the scheme
        var optimizations = Map.of("place", optimization, "water_name", optimization, "transportation",
                optimization, "transportation_name", optimization, "park", optimization, "mountain_peak",
                optimization, "poi", optimization, "waterway", optimization, "aerodrome_label", optimization);
        var mlTile = MltConverter.convertMvt(mvTile, new ConversionConfig(true, true, optimizations),
                tileMetadata);

        var mltTimeElapsed = 0l;
        for(int i = 0; i <= 200; i++) {
            long start = System.currentTimeMillis();
            var decodedTile = MltDecoder.decodeMlTileVectorized(mlTile, tileMetadata);
            long finish = System.currentTimeMillis();

            if(i > 100){
                mltTimeElapsed += (finish - start);
            }
        }
        System.out.println("MLT decoding time: " + (mltTimeElapsed / 100.0));
    }
}
