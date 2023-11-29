package comt.covt.converter;

import com.covt.converter.CovtConverter;
import com.covt.converter.EncodingUtils;
import com.covt.converter.mvt.MapboxVectorTile;
import com.covt.converter.mvt.MvtUtils;
import org.junit.jupiter.api.Test;

import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Paths;
import java.util.List;

import static org.junit.jupiter.api.Assertions.assertTrue;

public class CovtConverterTest {
    private static final String BING_MVT_PATH = ".\\data\\bing\\mvt";
    private static final String BING_COVT_PATH = ".\\data\\bing\\covt";
    private static final String OMT_MVT_PATH = ".\\data\\omt\\mvt";
    private static final String OMT_COVT_PATH = ".\\data\\omt\\covt";
    private static final String MAPBOX_MVT_PATH = ".\\data\\mapbox\\mvt";
    private static final String MAPBOX_COVT_PATH = ".\\data\\mapbox\\covt";
    private static final String AMAZON_MVT_PATH = ".\\data\\amazon\\mvt";
    private static final String AMAZON_COVT_PATH = ".\\data\\amazon\\covt";
    private static final String AMAZON_HERE_MVT_PATH = ".\\data\\amazon_here\\mvt";
    private static final String AMAZON_HERE_COVT_PATH = ".\\data\\amazon_here\\covt";
    private static final boolean SAVE_GENERATED_TILES = true;

    /* Amazon Here Tile Tests ---------------------------------- */

    @Test
    public void convert_AmazonHereTiles() throws IOException {
        var fileNames = List.of("4_8_5", "4_9_4", "5_16_10", "6_33_22", "8_132_85");

        for(var fileName : fileNames){
            System.out.println(fileName + " ------------------------------------------------------------");
            runAmazonHereTests(fileName);
        }
    }

    private void runAmazonHereTests(String tileId) throws IOException {
        var mvtTile = MvtUtils.decodeMvt2(Paths.get(AMAZON_HERE_MVT_PATH, tileId + ".pbf"));
        var mvtLayers = mvtTile.layers();

        var covTile = CovtConverter.convertMvtTile(mvtLayers, mvtTile.tileExtent(),
                CovtConverter.GeometryEncoding.ICE, CovtConverter.VertexBufferCompression.FAST_PFOR_DELTA,
                CovtConverter.TopologyStreamsCompression.FAST_PFOR_DELTA, true);

        assertTrue(covTile.length > 0);

        printStats(mvtTile, covTile);

        if(SAVE_GENERATED_TILES == true){
            Files.write(Paths.get(AMAZON_HERE_COVT_PATH, tileId + ".covt" ), covTile);
        }
    }

    /* Amazon Tile Tests ---------------------------------- */

    @Test
    public void convert_AmazonTiles() throws IOException {
        var fileNames = List.of("5_5_11", "5_6_21", "5_8_12", "5_16_11",
                "6_33_21", "6_33_22", "7_68_44", "8_136_89", "9_259_176", "10_518_352", "11_1037_704");

        for(var fileName : fileNames){
            System.out.println(fileName + " ------------------------------------------------------------");
            runAmazonTests(fileName);
        }
    }

    private void runAmazonTests(String tileId) throws IOException {
        var mvtTile = MvtUtils.decodeMvt2(Paths.get(AMAZON_MVT_PATH, tileId + ".pbf"));
        var mvtLayers = mvtTile.layers();

        var covTile = CovtConverter.convertMvtTile(mvtLayers, mvtTile.tileExtent(),
                CovtConverter.GeometryEncoding.ICE, CovtConverter.VertexBufferCompression.FAST_PFOR_DELTA,
                CovtConverter.TopologyStreamsCompression.FAST_PFOR_DELTA, true);

        assertTrue(covTile.length > 0);

        printStats(mvtTile, covTile);
        if(SAVE_GENERATED_TILES == true){
            Files.write(Paths.get(AMAZON_COVT_PATH, tileId + ".covt" ), covTile);
        }
    }

    /* Mapbox Tile Tests ---------------------------------- */

    @Test
    public void convert_MapboxTiles() throws IOException {
        var fileNames = List.of("5_17_11", "6_33_21", "6_33_22", "7_67_44", "8_135_89", "9_273_178");

        for(var fileName : fileNames){
            runMapboxTests(fileName);
        }
    }

    private void runMapboxTests(String tileId) throws IOException {
        var mvtTile = MvtUtils.decodeMvt2(Paths.get(MAPBOX_MVT_PATH, tileId + ".pbf"));
        var mvtLayers = mvtTile.layers();

        var covTile = CovtConverter.convertMvtTile(mvtLayers, mvtTile.tileExtent(),
                CovtConverter.GeometryEncoding.ICE, CovtConverter.VertexBufferCompression.FAST_PFOR_DELTA,
                CovtConverter.TopologyStreamsCompression.FAST_PFOR_DELTA, true);

        assertTrue(covTile.length > 0);

        printStats(mvtTile, covTile);
        if(SAVE_GENERATED_TILES == true){
            Files.write(Paths.get(MAPBOX_COVT_PATH, tileId + ".covt" ), covTile);
        }
    }

    /* Bing Maps Tests --------------------------------------------------------- */

    @Test
    public void convert_BingMaps_Z4Tile() throws IOException {
        var fileNames = List.of("4-8-5", "4-12-6", "4-13-6", "4-9-5");

        for(var fileName : fileNames){
            runBingTests(fileName);
        }
    }

    @Test
    public void convert_BingMaps_Z5Tiles() throws IOException {
        var fileNames = List.of("5-16-11", "5-17-11", "5-17-10", "5-25-13", "5-26-13", "5-8-12", "5-15-10");

        for(var fileName : fileNames){
           runBingTests(fileName);
        }
    }

    @Test
    public void convert_BingMaps_Z6Tiles() throws IOException {
        var fileNames = List.of("6-32-22", "6-33-22", "6-32-23", "6-32-21");

        for(var fileName : fileNames){
            runBingTests(fileName);
        }
    }

    @Test
    public void convert_BingMaps_Z7Tiles() throws IOException {
        var fileNames = List.of("7-65-43", "7-64-44", "7-66-43", "7-66-44", "7-65-42", "7-66-42", "7-67-43");

        for(var fileName : fileNames){
           runBingTests(fileName);
        }
    }

    @Test
    public void convert_BingMaps_Z9Tiles() throws IOException {
        var fileNames = List.of("9-259-176", "9-265-170", "9-266-170", "9-266-171");

        for(var fileName : fileNames){
            runBingTests(fileName);
        }
    }

    @Test
    public void convert_BingMaps_Z11Tiles() throws IOException {
        var fileNames = List.of("11-603-769");

        for(var fileName : fileNames){
            runBingTests(fileName);
        }
    }


    private void runBingTests(String tileId) throws IOException {
        System.out.println(tileId + " ------------------------------------------");
        var mvtTile = MvtUtils.decodeMvt2(Paths.get(BING_MVT_PATH, tileId + ".mvt"));
        //var mvtTile2 = MvtUtils.decodeMvt(Paths.get(BING_MVT_PATH, tileId + ".mvt"));
        var mvtLayers = mvtTile.layers();

        var covTile = CovtConverter.convertMvtTile(mvtLayers, mvtTile.tileExtent(),
                CovtConverter.GeometryEncoding.ICE, CovtConverter.VertexBufferCompression.FAST_PFOR_DELTA,
                CovtConverter.TopologyStreamsCompression.FAST_PFOR_DELTA, false);
        assertTrue(covTile.length > 0);
        printStats(mvtTile, covTile);

        var covTile2 = CovtConverter.convertMvtTile(mvtLayers, mvtTile.tileExtent(),
                CovtConverter.GeometryEncoding.ICE_MORTON, CovtConverter.VertexBufferCompression.FAST_PFOR_DELTA,
                CovtConverter.TopologyStreamsCompression.FAST_PFOR_DELTA, false);
        assertTrue(covTile2.length > 0);
        printStats(mvtTile, covTile2);

        if(SAVE_GENERATED_TILES == true){
            Files.write(Paths.get(BING_COVT_PATH, tileId + ".covt" ), covTile);
        }
    }

    /* OpenMapTiles Tests --------------------------------------------------------- */

    @Test
    public void convert_OmtTileZ2_ValidCovTile() throws IOException {
        var tileId = String.format("%s_%s_%s", 4, 8, 10);
        runOmtTest(tileId);
    }

    @Test
    public void convert_OmtTileZ5_ValidCovTile() throws IOException {
        var tileId = String.format("%s_%s_%s", 5, 16, 21);
        runOmtTest(tileId);
    }

    @Test
    public void convert_OmtTilesZ2_ValidCovTile() throws IOException {
        runOmtTests(2, 2, 2 ,2, 2);
    }

    @Test
    public void convert_OmtTilesZ3_ValidCovTile() throws IOException {
        runOmtTests(3, 4, 4 ,5, 5);
    }

    @Test
    public void convert_OmtTilesZ4_ValidCovTile() throws IOException {
        runOmtTests(4, 8, 8 ,10, 10);
        runOmtTests(4, 3, 3 ,9, 9);
        /*runTest(4, 3, 3 ,10, 10);
        runTest(4, 9, 9 ,10, 10);*/
    }

    @Test
    public void convert_OmtTilesZ5_ValidCovTile() throws IOException {
        //runTest(5, 15, 17, 20, 22);
        runOmtTests(5, 16, 17, 20, 21);
    }

    @Test
    public void convert_Z6OmtTiles_ValidCovTile() throws IOException {
        //runTest(6, 32, 36, 41, 42);
        runOmtTests(6, 32, 34, 41, 42);
    }

    @Test
    public void convert_OmtTilesZ7_ValidCovTile() throws IOException {
        runOmtTests(7, 66, 68, 83, 85);
    }

    @Test
    public void convert_OmtTilesZ8_ValidCovTile() throws IOException {
        runOmtTests(8, 132, 135, 170, 171);
    }

    @Test
    public void convert_OmtTilesZ9_ValidCovTile() throws IOException {
        runOmtTests(9, 264, 266, 340, 342);
    }

    @Test
    public void convert_OmtTilesZ10_ValidCovTile() throws IOException {
        runOmtTests(10, 530, 533, 682, 684);
    }

    @Test
    public void convert_OmtTilesZ11_ValidCovTile() throws IOException {
        runOmtTests(11, 1062, 1065, 1366, 1368);
    }

    @Test
    public void convert_OmtTilesZ12_ValidCovTile() throws IOException {
        runOmtTests(12, 2130, 2134, 2733, 2734);
    }

    @Test
    public void convert_OmtTilesZ13_ValidCovTile() throws IOException {
        runOmtTests(13, 4264, 4267, 5467, 5468);
    }

    @Test
    public void convert_OmtTilesZ14_ValidCovTile() throws IOException {
        runOmtTests(14, 8296, 8300, 10748, 10749);
    }

    private static void runOmtTest(String tileId) throws IOException {
        var mvtFilePath = Paths.get(OMT_MVT_PATH, tileId + ".mvt" );
        var mvtTile = MvtUtils.decodeMvt2(mvtFilePath);
        var mvtLayers = mvtTile.layers();

        var covTile = CovtConverter.convertMvtTile(mvtLayers, mvtTile.tileExtent(),
                CovtConverter.GeometryEncoding.ICE, CovtConverter.VertexBufferCompression.FAST_PFOR_DELTA,
                CovtConverter.TopologyStreamsCompression.FAST_PFOR_DELTA, true);

        assertTrue(covTile.length > 0);

        if(SAVE_GENERATED_TILES == true){
            Files.write(Paths.get(OMT_COVT_PATH, tileId + ".covt" ), covTile);
        }
    }

    private static void runOmtTests(int zoom, int minX, int maxX, int minY, int maxY) throws IOException {
        var ratios = 0d;
        var counter = 0;
        for (var x = minX; x <= maxX; x++) {
            for (var y = minY; y <= maxY; y++) {
                var tileId = String.format("%s_%s_%s", zoom, x, y);
                var mvtFilePath = Paths.get(OMT_MVT_PATH, tileId + ".mvt" );
                var mvtTile = MvtUtils.decodeMvt(mvtFilePath);
                var mvtLayers = mvtTile.layers();

                var geometryEncoding = (zoom != 13 && zoom != 14) ? CovtConverter.GeometryEncoding.ICE :
                        CovtConverter.GeometryEncoding.PLAIN;
                var covTile = CovtConverter.convertMvtTile(mvtLayers, mvtTile.tileExtent(),
                        CovtConverter.GeometryEncoding.ICE, CovtConverter.VertexBufferCompression.FAST_PFOR_DELTA,
                        CovtConverter.TopologyStreamsCompression.FAST_PFOR_DELTA, true);

                assertTrue(covTile.length > 0);

                if(SAVE_GENERATED_TILES == true){
                    Files.write(Paths.get(OMT_COVT_PATH, tileId + ".covt"), covTile);
                }

                ratios += printStats(mvtTile, covTile);
                counter++;
            }
        }

        System.out.println("Total ratio: " + (ratios / counter));
    }

    private static double printStats(MapboxVectorTile mvtTile, byte[] covtTile) throws IOException {
        var covtGzipBuffer = EncodingUtils.gzipCompress(covtTile);
        System.out.println(String.format("MVT size: %s, Gzip MVT size: %s", mvtTile.mvtSize(), mvtTile.gzipCompressedMvtSize()));
        System.out.println(String.format("COVT size: %s, Gzip COVT size: %s", covtTile.length, covtGzipBuffer.length));
        System.out.println(String.format("Ratio uncompressed: %s, Ratio compressed: %s",
                ((double)mvtTile.mvtSize())/covtTile.length, ((double)mvtTile.gzipCompressedMvtSize())/covtGzipBuffer.length));
        var compressionRatio = (1-(1/(((double)mvtTile.mvtSize())/covtTile.length)))*100;
        var compressionRatioCompressed = (1-(1/(((double)mvtTile.gzipCompressedMvtSize())/covtGzipBuffer.length)))*100;
        System.out.println(String.format("Reduction uncompressed: %s%%, Reduction compressed: %s%%", compressionRatio, compressionRatioCompressed));
        System.out.println("------------------------------------------------------------");
        return compressionRatio;
    }

    private static double printStats2(MapboxVectorTile mvtTile, byte[] covtTile, byte[] mvtTileBuffer) throws IOException {
        var covtGzipBuffer = EncodingUtils.gzipCompress(covtTile);
        var mvtGzipBuffer = EncodingUtils.gzipCompress(mvtTileBuffer);
        System.out.println(String.format("MVT size: %s, Gzip MVT size: %s", mvtTile.mvtSize(), mvtGzipBuffer.length));
        System.out.println(String.format("COVT size: %s, Gzip COVT size: %s", covtTile.length, covtGzipBuffer.length));
        System.out.println(String.format("Ratio uncompressed: %s, Ratio compressed: %s",
                ((double)mvtTile.mvtSize())/covtTile.length, ((double)mvtTile.gzipCompressedMvtSize())/covtGzipBuffer.length));
        var compressionRatio = (1-(1/(((double)mvtTile.mvtSize())/covtTile.length)))*100;
        var compressionRatioCompressed = (1-(1/(((double)mvtTile.gzipCompressedMvtSize())/covtGzipBuffer.length)))*100;
        System.out.println(String.format("Reduction uncompressed: %s%%, Reduction compressed: %s%%", compressionRatio, compressionRatioCompressed));
        System.out.println("------------------------------------------------------------");
        return compressionRatio;
    }
}
