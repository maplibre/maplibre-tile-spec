package com.covt.demo;

import com.covt.converter.CovtConverter;
import com.covt.converter.EncodingUtils;
import com.covt.converter.mvt.Layer;
import com.covt.converter.mvt.MvtUtils;
import com.covt.decoder.CovtParser;
import org.junit.jupiter.api.Test;

import java.io.File;
import java.io.IOException;
import java.nio.file.Paths;
import java.util.List;
import java.util.Optional;

import static org.junit.jupiter.api.Assertions.assertEquals;

record TileStats(double mvtSize, double covtSize, double gzipMvtSize, double gzipCovtSize, double gzipMvtDecompressionTime){}

public class BingCovtDemo {
    private static final String BING_MVT_PATH = "..\\..\\test\\fixtures\\bing\\mvt";

    @Test
    public void bingTilesTests() throws IOException {
        File folder = new File(BING_MVT_PATH);
        File[] listOfFiles = folder.listFiles();

        var previousFileZoomLevel = -1;
        var peerZoomReduction = 0d;
        var tilesPerZoom = 0;
        for (File file : listOfFiles) {
            if (file.isFile()) {
                var fileName = file.getAbsoluteFile();
                var tileId = file.getName().split("\\.")[0].replace("-", "/");
                var zoom = Integer.parseInt(tileId.split("/")[0]);

                if(previousFileZoomLevel != -1 && previousFileZoomLevel != zoom){
                    System.out.printf("------------------------------------------------------------------------------------------%n");
                    System.out.printf("Total reduction for zoom %d: %d%%%n", previousFileZoomLevel,
                            Math.round(peerZoomReduction / tilesPerZoom));
                    System.out.printf("------------------------------------------------------------------------------------------%n");
                    tilesPerZoom = 0;
                    peerZoomReduction = 0;
                }

                System.out.printf("Tile %s -------------------------------------------------------------------%n", tileId);

                var stats = runBingTest(fileName.toString());
                var reduction = printStats(stats);
                peerZoomReduction += reduction;

                previousFileZoomLevel = zoom;
                tilesPerZoom++;
            }
        }

        System.out.printf("------------------------------------------------------------------------------------------%n");
        System.out.printf("Total reduction for zoom %d: %d%%%n", previousFileZoomLevel, Math.round(peerZoomReduction / tilesPerZoom));
        System.out.printf("------------------------------------------------------------------------------------------%n");
    }

    private double printStats(TileStats stats){
        var reduction = (1-(1/((stats.mvtSize())/stats.covtSize())))*100;
        System.out.printf("MVT size: %dkb, COVT size: %dkb, Reduction: %.2f%%%n", Math.round(stats.mvtSize()/1000),
                Math.round(stats.covtSize()/1000), reduction);
        var gzipReduction = (1-(1/((stats.gzipMvtSize())/ stats.gzipCovtSize())))*100;
        System.out.printf("Gzip MVT size: %dkb, Gzip COVT size: %dkb, Gzip Reduction: %.2f%%%n", Math.round(stats.gzipMvtSize()/1000),
                Math.round(stats.gzipCovtSize()/1000),
                gzipReduction);
        System.out.printf("COVT uncompressed to MVT Gzip compressed reduction: %.2fkb, MVT Gzip decompression time for tile: %.4f milliseconds%n",
                (stats.gzipMvtSize() - stats.covtSize()) / 1000, stats.gzipMvtDecompressionTime());
        return reduction;
    }

    private TileStats runBingTest(String fileName) throws IOException {
        var mvtTile = MvtUtils.decodeMvt2(Paths.get(fileName));
        var mvtLayers = mvtTile.layers();

        var covtTile = CovtConverter.convertMvtTile(mvtLayers, mvtTile.tileExtent(), CovtConverter.GeometryEncoding.ICE_MORTON,
                    true, true, false, false);
        var varintEncodedCovtTile = CovtConverter.convertMvtTile(mvtLayers, mvtTile.tileExtent(), CovtConverter.GeometryEncoding.ICE_MORTON,
                false, false, false, false);

        var covtLayers = CovtParser.decodeCovt(covtTile);

        try{
            compareTiles(mvtLayers, covtLayers);
            System.out.println("Tests passed. Tiles contain the same information");
        }
        catch(Error e){
            System.err.println(e);
        }

        var gzipCovtSize = EncodingUtils.gzipCompress(covtTile).length;
        var varintGzipCovtSize = EncodingUtils.gzipCompress(varintEncodedCovtTile).length;
        return new TileStats(mvtTile.mvtSize(), covtTile.length, mvtTile.gzipCompressedMvtSize(),
                gzipCovtSize < varintGzipCovtSize? gzipCovtSize : varintGzipCovtSize, mvtTile.gzipDecompressionTime());
    }

    private void compareTiles(List<Layer> mvtLayers, List<Layer> covtLayers){
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
                for(var propertyKey : mvtProperties.keySet()){
                    var mvtProperty = mvtProperties.get(propertyKey);
                    var optionalCovtProperty = ((Optional)covtProperties.get(propertyKey));
                    var covtProperty = optionalCovtProperty.isPresent() ? optionalCovtProperty.get() : null;

                    assertEquals(mvtProperty, covtProperty);
                }
            }
        }
    }
}
