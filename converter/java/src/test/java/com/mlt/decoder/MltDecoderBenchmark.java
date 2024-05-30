package com.mlt.decoder;

import com.mlt.converter.ConversionConfig;
import com.mlt.converter.FeatureTableOptimizations;
import com.mlt.converter.MltConverter;
import com.mlt.converter.mvt.ColumnMapping;
import com.mlt.converter.mvt.MapboxVectorTile;
import com.mlt.converter.mvt.MvtUtils;
import org.junit.jupiter.api.Test;

import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Paths;
import java.util.List;
import java.util.Map;
import java.util.Optional;

public class MltDecoderBenchmark {
    private static final String OMT_MVT_PATH = Paths.get("..","..","test","fixtures","omt","mvt").toString();

    @Test
    public void decodeMlTileVectorized_Z3() throws IOException {
        var tileId = String.format("%s_%s_%s", 3, 4, 5);
        testTile(tileId);
    }

    @Test
    public void decodeMlTileVectorized_Z4() throws IOException {
        var tileId = String.format("%s_%s_%s", 4, 8, 10);
        testTile(tileId);

        var tileId2 = String.format("%s_%s_%s", 4, 3, 9);
        testTile(tileId2);
    }

    @Test
    public void decodeMlTileVectorized_Z5() throws IOException {
        var tileId = String.format("%s_%s_%s", 5, 16, 21);
        testTile(tileId);

        var tileId2 = String.format("%s_%s_%s", 5, 16, 20);
        testTile(tileId2);
    }

    @Test
    public void decodeMlTileVectorized_Z6() throws IOException {
        var tileId = String.format("%s_%s_%s", 6, 32, 41);
        testTile(tileId);
    }

    private void testTile(String tileId) throws IOException {
        var mvtFilePath = Paths.get(OMT_MVT_PATH, tileId + ".mvt" );

        var mvt = Files.readAllBytes(mvtFilePath);
        var mvtTimeElapsed = 0l;
        //var mvTile2 = MvtUtils.decodeMvt2Fast(mvt);
        for(int i = 0; i <= 200; i++) {
            long start = System.currentTimeMillis();
            var mvTile = MvtUtils.decodeMvtFast(mvt);
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
        var optimizations = Map.of("place", optimization, "water_name", optimization, "transportation", optimization,
                "transportation_name", optimization, "park", optimization, "mountain_peak", optimization);
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

        System.out.println("Ratio: " + Files.readAllBytes(mvtFilePath).length / (double)mlTile.length );
        System.out.println("Reduction: " + ((1 - (double)mlTile.length / Files.readAllBytes(mvtFilePath).length)*100));
    }
}
