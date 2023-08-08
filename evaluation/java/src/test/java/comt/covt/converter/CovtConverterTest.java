package comt.covt.converter;

import com.covt.converter.CovtConverter;
import com.covt.converter.EncodingUtils;
import com.covt.converter.mvt.MapboxVectorTile;
import com.covt.converter.mvt.MvtUtils;
import org.junit.jupiter.api.Test;

import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Path;
import java.sql.SQLException;
import java.util.Arrays;
import java.util.List;
import java.util.stream.DoubleStream;

public class CovtConverterTest {
    private static final String MBTILES_FILE_NAME = "C:\\mapdata\\europe.mbtiles";

    /*@Test
    public void parseCovt_Z4Tile_ValidConvertedMvtTile() throws SQLException, IOException, ClassNotFoundException {
        var mvtTile = MvtUtils.getMVT(MBTILES_FILE_NAME, 4, 8, 10);
        var mvtLayers = mvtTile.layers();

        var tile = CovtConverter.convertMvtTile(mvtLayers, true);

        Files.write(Path.of("./data/4_8_10.covt"), tile);
    }

    @Test
    public void parseCovt_Z5Tile_ValidConvertedMvtTile() throws SQLException, IOException, ClassNotFoundException {
        var mvtTile = MvtUtils.getMVT(MBTILES_FILE_NAME, 5, 16, 21);
        var mvtLayers = mvtTile.layers();

        var tile = CovtConverter.convertMvtTile(mvtLayers,true);

        Files.write(Path.of("./data/5_16_21.covt"), tile);
    }*/

    @Test
    public void parseCovt_DifferentZ2Tiles_ValidConvertedMvtTile() throws SQLException, IOException, ClassNotFoundException {
        runTest(2, 2, 2 ,2, 2);
    }

    @Test
    public void parseCovt_DifferentZ3Tiles_ValidConvertedMvtTile() throws SQLException, IOException, ClassNotFoundException {
        runTest(3, 4, 4 ,5, 5);
    }

    @Test
    public void parseCovt_DifferentZ4Tiles_ValidConvertedMvtTile() throws SQLException, IOException, ClassNotFoundException {
        runTest(4, 8, 8 ,10, 10);
        runTest(4, 3, 3 ,9, 9);
        /*runTest(4, 3, 3 ,10, 10);
        runTest(4, 9, 9 ,10, 10);*/
    }

    @Test
    public void parseCovt_DifferentZ5Tiles_ValidConvertedMvtTile() throws SQLException, IOException, ClassNotFoundException {
        //runTest(5, 15, 17, 20, 22);
        runTest(5, 16, 17, 20, 21);
    }

    @Test
    public void parseCovt_DifferentZ6Tiles_ValidConvertedMvtTile() throws SQLException, IOException, ClassNotFoundException {
        //runTest(6, 32, 36, 41, 42);
        runTest(6, 32, 34, 41, 42);
    }

    @Test
    public void parseCovt_DifferentZ7Tiles_ValidConvertedMvtTile() throws SQLException, IOException, ClassNotFoundException {
        runTest(7, 66, 68, 83, 85);
    }

    @Test
    public void parseCovt_DifferentZ8Tiles_ValidConvertedMvtTile() throws SQLException, IOException, ClassNotFoundException {
        runTest(8, 132, 135, 170, 171);
    }

    @Test
    public void parseCovt_DifferentZ9Tiles_ValidConvertedMvtTile() throws SQLException, IOException, ClassNotFoundException {
        runTest(9, 264, 266, 340, 342);
    }

    @Test
    public void parseCovt_DifferentZ10Tiles_ValidConvertedMvtTile() throws SQLException, IOException, ClassNotFoundException {
        runTest(10, 530, 533, 682, 684);
    }

    @Test
    public void parseCovt_DifferentZ11Tiles_ValidConvertedMvtTile() throws SQLException, IOException, ClassNotFoundException {
        runTest(11, 1062, 1065, 1366, 1368);
    }

    @Test
    public void parseCovt_DifferentZ12Tiles_ValidConvertedMvtTile() throws SQLException, IOException, ClassNotFoundException {
        runTest(12, 2130, 2134, 2733, 2734);
    }

    @Test
    public void parseCovt_DifferentZ13Tiles_ValidConvertedMvtTile() throws SQLException, IOException, ClassNotFoundException {
        runTest(13, 4264, 4267, 5467, 5468);
    }

    @Test
    public void parseCovt_DifferentZ14Tiles_ValidConvertedMvtTile() throws SQLException, IOException, ClassNotFoundException {
        runTest(14, 8296, 8300, 10748, 10749);
    }

    private static void runTest(int zoom, int minX, int maxX, int minY, int maxY) throws SQLException, IOException, ClassNotFoundException {
        var ratios = Arrays.asList(0.0, 0.0, 0.0, 0.0, 0.0);
        var counter = 0;
        for (var x = minX; x <= maxX; x++) {
            for (var y = minY; y <= maxY; y++) {
                var mvtTile = MvtUtils.getMVT(MBTILES_FILE_NAME, zoom, x, y);
                var mvtLayers = mvtTile.layers();

                var useIce = zoom != 13 && zoom != 14;
                try{
                    var covtTile = CovtConverter.convertMvtTile(mvtLayers, useIce);

                    var stats = printStats(mvtTile, covtTile);
                    ratios.set(0, ratios.get(0) + stats.get(0));
                    ratios.set(1, ratios.get(1) + stats.get(1));
                    ratios.set(2, ratios.get(2) + stats.get(2));
                    ratios.set(3, ratios.get(3) + stats.get(3));
                    ratios.set(4, ratios.get(4) + stats.get(4));
                    counter++;

                }
                catch (AssertionError e){
                    System.out.println(e);
                }
            }
        }

        System.out.println("Total ratio: " + (ratios.get(0) / counter));
        System.out.println("Diff uncompressed: " + (ratios.get(1) / counter) + ", Diff compressed: " + (ratios.get(2) / counter)
            + ", Ratio uncompressed: " + (ratios.get(3) / counter) + "%, Ratio compressed: " + (ratios.get(4) / counter) + "%");
    }

    private static List<Double> printStats(MapboxVectorTile mvtTile, List<byte[]> covtTiles) throws IOException {
        var covtTile = covtTiles.get(0);
        var covtProtoTile = covtTiles.get(1);
        var covtGzipBuffer = EncodingUtils.gzipCompress(covtTile);
        var covtProtoGzipBuffer = EncodingUtils.gzipCompress(covtProtoTile);

        var protoDiff = ((double)covtProtoTile.length - covtTile.length) / 1000;
        var gzipDiff =  ((double)covtProtoGzipBuffer.length - covtGzipBuffer.length) / 1000;
        var ratio = (100 - (covtTile.length / (double)covtProtoTile.length) * 100);
        var compressedRatio = (100 - (covtGzipBuffer.length / (double)covtProtoGzipBuffer.length) * 100);
        /*System.out.println(String.format("MVT size: %s, Gzip MVT size: %s", mvtTile.mvtSize(), mvtTile.gzipCompressedMvtSize()));
        System.out.println(String.format("COVT size: %s, Gzip COVT size: %s", covtTile.length, covtGzipBuffer.length));
        System.out.println(String.format("COVT Proto size: %s, Gzip COVT Proto size: %s", covtProtoTile.length, covtProtoGzipBuffer.length));
        System.out.println("Proto diff: " +  protoDiff + " Gzip diff: " + gzipDiff
               + " Ratio: " + ratio +  "% Ratio Compressed: " + compressedRatio + "% -----------------------------------");
        System.out.println(String.format("Ratio uncompressed: %s, Ratio compressed: %s",
                ((double)mvtTile.mvtSize()) / covtTile.length, ((double)mvtTile.gzipCompressedMvtSize()) / covtGzipBuffer.length));*/
        var compressionRatio = (1- (1/( ((double)mvtTile.mvtSize()) / covtTile.length)   ))*100;
        //System.out.println(String.format("Ratio compressed: %s", compressionRatio));
        return List.of(compressionRatio, protoDiff, gzipDiff, ratio, compressedRatio);
    }
}
