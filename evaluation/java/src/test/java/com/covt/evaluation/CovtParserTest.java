package com.covt.evaluation;


import com.covt.decoder.CovtParser;
import com.covt.evaluation.compression.IntegerCompression;
import org.junit.jupiter.api.Test;

import java.io.IOException;
import java.sql.SQLException;
import java.util.ArrayList;
import java.util.List;
import java.util.Optional;

import static org.junit.jupiter.api.Assertions.assertEquals;


public class CovtParserTest {
    /* private static final String MBTILES_FILE_NAME = "C:\\mapdata\\europe.mbtiles";
    private static final boolean USE_ICE_ENCODING = true;
    private static final boolean USE_FAST_PFOR = false;
    private static final boolean USE_STRING_DICTIONARY_CODING = true;
    private static final boolean REMOVE_CLOSING_POLYGON_VERTEX = true;

    @Test
    public void parseCovt_Z5Tile_ValidParsedTile() throws SQLException, IOException, ClassNotFoundException {
        var mvtTile = MvtUtils.getMVT(MBTILES_FILE_NAME, 5, 16, 21);
        var mvtLayers = mvtTile.layers();
        var covtTile = MvtConverter.createCovtTile(mvtLayers, USE_ICE_ENCODING, USE_FAST_PFOR, USE_STRING_DICTIONARY_CODING, REMOVE_CLOSING_POLYGON_VERTEX);

        var covtLayers = CovtParser.parseCovt(covtTile, USE_ICE_ENCODING, USE_FAST_PFOR, USE_STRING_DICTIONARY_CODING);

        compareTiles(mvtLayers, covtLayers);

        printStats(mvtTile, covtTile);
    }

    @Test
    public void parseCovt_DifferentZ2Tiles_ValidParsedTile() throws SQLException, IOException, ClassNotFoundException {
        runTest(2, 2, 2 ,2, 2);
    }

    @Test
    public void parseCovt_DifferentZ3Tiles_ValidParsedTile() throws SQLException, IOException, ClassNotFoundException {
        runTest(3, 4, 4 ,5, 5);
    }

    @Test
    public void parseCovt_DifferentZ4Tiles_ValidParsedTile() throws SQLException, IOException, ClassNotFoundException {
        runTest(4, 8, 8 ,10, 10);
        runTest(4, 3, 3 ,9, 9);
    }

    @Test
    public void parseCovt_DifferentZ5Tiles_ValidParsedTile() throws SQLException, IOException, ClassNotFoundException {
        //runTest(5, 15, 17, 20, 22);
        runTest(5, 16, 17, 20, 21);
    }

    @Test
    public void parseCovt_DifferentZ6Tiles_ValidParsedTile() throws SQLException, IOException, ClassNotFoundException {
        //runTest(6, 32, 36, 41, 42);
        runTest(6, 32, 34, 41, 42);
    }

    @Test
    public void parseCovt_DifferentZ7Tiles_ValidParsedTile() throws SQLException, IOException, ClassNotFoundException {
        runTest(7, 66, 68, 83, 85);
    }

    @Test
    public void parseCovt_DifferentZ8Tiles_ValidParsedTile() throws SQLException, IOException, ClassNotFoundException {
        runTest(8, 132, 135, 170, 171);
    }

    @Test
    public void parseCovt_DifferentZ9Tiles_ValidParsedTile() throws SQLException, IOException, ClassNotFoundException {
        runTest(9, 264, 266, 340, 342);
    }

    @Test
    public void parseCovt_DifferentZ10Tiles_ValidParsedTile() throws SQLException, IOException, ClassNotFoundException {
        runTest(10, 530, 533, 682, 684);
    }

    @Test
    public void parseCovt_DifferentZ11Tiles_ValidParsedTile() throws SQLException, IOException, ClassNotFoundException {
        runTest(11, 1062, 1065, 1366, 1368);
    }

    @Test
    public void parseCovt_DifferentZ12Tiles_ValidParsedTile() throws SQLException, IOException, ClassNotFoundException {
        runTest(12, 2130, 2134, 2733, 2734);
    }

    @Test
    public void parseCovt_DifferentZ13Tiles_ValidParsedTile() throws SQLException, IOException, ClassNotFoundException {
        runTest(13, 4264, 4267, 5467, 5468);
    }

    @Test
    public void parseCovt_DifferentZ14Tiles_ValidParsedTile() throws SQLException, IOException, ClassNotFoundException {
        runTest(14, 8296, 8300, 10748, 10749);
    }

    private static void runTest(int zoom, int minX, int maxX, int minY, int maxY) throws SQLException, IOException, ClassNotFoundException {
        var ratios = new ArrayList<double[]>();
        for(var x = minX; x <= maxX; x++){
            for(var y = minY; y <= maxY; y++){
                System.out.println("x: " + x + ", y: " + y);
                var mvtTile = MvtUtils.getMVT(MBTILES_FILE_NAME, zoom, x, y);
                var mvtLayers = mvtTile.layers();
                var covtTile = MvtConverter.createCovtTile(mvtLayers, USE_ICE_ENCODING, USE_FAST_PFOR, USE_STRING_DICTIONARY_CODING, REMOVE_CLOSING_POLYGON_VERTEX);

                //var covtLayers = CovtParser.parseCovt(covtTile, USE_ICE_ENCODING, USE_FAST_PFOR, USE_STRING_DICTIONARY_CODING);

                //compareTiles(mvtLayers, covtLayers);

                printStats(mvtTile, covtTile);
                var ratio = getRatio(mvtTile, covtTile);
                ratios.add(ratio);
            }
        }

        printAverageStats(ratios);
    }

    private static void printAverageStats(ArrayList<double[]> ratios){
        var totalUncompressedRatio = 0d;
        var totalCompressedRatio = 0d;
        for(var ratio : ratios){
            totalUncompressedRatio += ratio[0];
            totalCompressedRatio += ratio[1];
        }
        System.out.println(String.format("Total ratio uncompressed: %s, Total ratio compressed: %s",
                (1 - 1 / (totalUncompressedRatio / ratios.size())) * 100,
                (1 - 1 / (totalCompressedRatio / ratios.size())) * 100
        ));
    }

    private static double[] getRatio(MapboxVectorTile mvtTile, byte[] covtTile) throws IOException {
        var covtGzipBuffer = IntegerCompression.gzipCompress(covtTile);
        var uncompressedRatio = ((double)mvtTile.mvtSize()) / covtTile.length;
        var compressedRatio = ((double)mvtTile.gzipCompressedMvtSize()) / covtGzipBuffer.length;
        return new double[]{uncompressedRatio, compressedRatio};
    }

    private static void printStats(MapboxVectorTile mvtTile, byte[] covtTile) throws IOException {
        var covtGzipBuffer = IntegerCompression.gzipCompress(covtTile);
        System.out.println(String.format("MVT size: %s, Gzip MVT size: %s", mvtTile.mvtSize(), mvtTile.gzipCompressedMvtSize()));
        System.out.println(String.format("COVT size: %s, Gzip COVT size: %s", covtTile.length, covtGzipBuffer.length));
        System.out.println(String.format("Ratio uncompressed: %s, Ratio compressed: %s",
                ((double)mvtTile.mvtSize()) / covtTile.length, ((double)mvtTile.gzipCompressedMvtSize()) / covtGzipBuffer.length));
    }

    private static void compareTiles(List<Layer> mvtLayers, List<Layer> covtLayers){
        assertEquals(mvtLayers.size(), covtLayers.size());
        for(var i = 0; i < mvtLayers.size(); i++){
            var mvtLayer = mvtLayers.get(i);
            var covtLayer = covtLayers.get(i);

            assertEquals(mvtLayer.name(), covtLayer.name());

            var mvtFeatures = mvtLayer.features();
            var covtFeatures = covtLayer.features();
            for(var j = 0; j < mvtFeatures.size(); j++){
                var mvtFeature = mvtFeatures.get(j);
                var covtFeature = covtFeatures.get(j);

                assertEquals(mvtFeature.id(), covtFeature.id());
                assertEquals(mvtFeature.geometry(), covtFeature.geometry());

                var mvtProperties = mvtFeature.properties();
                var covtProperties = covtFeature.properties();
                for(var propertyKey : covtProperties.keySet()){
                    var mvtProperty = mvtProperties.get(propertyKey);
                    var optionalCovtProperty = ((Optional)covtProperties.get(propertyKey));
                    var covtProperty = optionalCovtProperty.isPresent() ? optionalCovtProperty.get() : null;

                    //TODO: handle long values properly in the conversion
                    if(mvtProperty instanceof Long){
                        var intMvtProperty = ((Long)mvtProperty).intValue();
                        assertEquals(intMvtProperty, covtProperty);
                    }
                    else{
                        assertEquals(mvtProperty, covtProperty);
                    }
                }
            }
        }
    }*/

}
