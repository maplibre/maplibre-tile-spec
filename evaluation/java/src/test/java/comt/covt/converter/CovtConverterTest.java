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

public class CovtConverterTest {
    private static final String MBTILES_FILE_NAME = "C:\\mapdata\\europe.mbtiles";

    @Test
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
    }

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
        var ratios = 0d;
        var counter = 0;
        for (var x = minX; x <= maxX; x++) {
            for (var y = minY; y <= maxY; y++) {
                var mvtTile = MvtUtils.getMVT(MBTILES_FILE_NAME, zoom, x, y);
                var mvtLayers = mvtTile.layers();

                var useIce = zoom != 13 && zoom != 14;
                var covtTile = CovtConverter.convertMvtTile(mvtLayers, useIce);

                ratios += printStats(mvtTile, covtTile);
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
                ((double)mvtTile.mvtSize()) / covtTile.length, ((double)mvtTile.gzipCompressedMvtSize()) / covtGzipBuffer.length));
        var compressionRatio = (1- (1/( ((double)mvtTile.mvtSize()) / covtTile.length)   ))*100;
        System.out.println(String.format("Ratio compressed: %s", compressionRatio));
        return compressionRatio;
    }
}
