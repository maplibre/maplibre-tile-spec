package com.covt.demo;

import com.covt.converter.CovtConverter;
import com.covt.converter.EncodingUtils;
import com.covt.converter.mvt.Layer;
import com.covt.converter.mvt.MvtUtils;
import com.covt.converter.tilejson.TileJson;
import com.covt.decoder.CovtParser;
import com.fasterxml.jackson.databind.ObjectMapper;
import org.junit.jupiter.api.Test;

import java.io.File;
import java.io.FileWriter;
import java.io.IOException;
import java.nio.file.Paths;
import java.util.Arrays;
import java.util.Collections;
import java.util.List;
import java.util.Optional;
import java.util.stream.Collectors;

import static org.junit.jupiter.api.Assertions.assertEquals;

record TileStats(double mvtSize, double covtSize, double gzipMvtSize, double gzipCovtSize,
        double gzipMvtDecompressionTime,
        double unoptimizedCovtSize, double unoptimizedGzipCovtSize) {
}

public class BingCovtDemo {
    private static final String BING_MVT_PATH = "..\\..\\test\\fixtures\\bing\\mvt";

    @Test
    public void bingTilesTests() throws IOException {
        File folder = new File(BING_MVT_PATH);
        File[] files = folder.listFiles();

        var sortedFiles = Arrays.stream(files).filter(f -> !f.getName().contains("readme.md")).sorted((a, b) -> {
            var zoom1 = Integer.parseInt(getTilIdFromFilename(a).split("-")[0]);
            var zoom2 = Integer.parseInt(getTilIdFromFilename(b).split("-")[0]);
            return zoom1 - zoom2;
        }).collect(Collectors.toList());
        Collections.reverse(sortedFiles);

        var previousFileZoomLevel = -1;
        var peerZoomReduction = 0d;
        var tilesPerZoom = 0;
        var csvBuilder = new StringBuilder();

        // row 1: headers
        csvBuilder.append(
                ",zoom, mvt size, covt size, reduction rate, gzip mvt size, gzip covt size, gzip mvt decompressition time, unoptimized covt size, unoptimzied gzip covt size\r\n");

        var rowCounter = 2;
        for (File file : sortedFiles) {
            if (file.isFile()) {
                var fileName = file.getAbsoluteFile();
                var tileId = file.getName().split("\\.")[0].replace("-", "/");
                var zoom = Integer.parseInt(tileId.split("/")[0]);

                if (previousFileZoomLevel != -1 && previousFileZoomLevel != zoom) {
                    System.out.printf(
                            "------------------------------------------------------------------------------------------%n");
                    System.out.printf("Total reduction with optimized metadata for zoom %d: %f%%%n",
                            previousFileZoomLevel,
                            // Math.round(peerZoomReduction / tilesPerZoom));
                            peerZoomReduction / tilesPerZoom);
                    System.out.printf(
                            "------------------------------------------------------------------------------------------%n");
                    tilesPerZoom = 0;
                    peerZoomReduction = 0;
                }

                System.out.printf("Tile %s -------------------------------------------------------------------%n",
                        tileId);

                try {
                    var stats = runBingTest(fileName.toString());
                    var reduction = printStats(stats);
                    peerZoomReduction += reduction;

                    previousFileZoomLevel = zoom;
                    tilesPerZoom++;

                    csvBuilder.append(file.getName() + "," + // column A
                            zoom + "," + // column B
                            stats.mvtSize() + "," + // column C
                            stats.covtSize() + "," + // D
                            String.format("=(1 - (1 / (C%d / D%d))) * 100, ", rowCounter, rowCounter) +
                            stats.gzipMvtSize() + "," +
                            stats.gzipCovtSize() + "," +
                            stats.gzipMvtDecompressionTime() + "," +
                            stats.unoptimizedCovtSize() + "," +
                            stats.unoptimizedGzipCovtSize() + "\r\n");
                    rowCounter++;
                }
                catch(Error e){
                    System.err.println("Test failed.");
                }
            }
        }

        System.out
                .printf("------------------------------------------------------------------------------------------%n");
        System.out.printf("Total reduction with optimized metadata for zoom %d: %d%%%n", previousFileZoomLevel,
                Math.round(peerZoomReduction / tilesPerZoom));
        System.out
                .printf("------------------------------------------------------------------------------------------%n");

        // last row, only need column E to average all redution rates
        csvBuilder.append(String.format(",,,,=AVERAGE(E2:E%d)\r\n", rowCounter - 1));

        File csvFileExporter = new File("result.csv");
        FileWriter writer = new FileWriter(csvFileExporter, false); // false to overwrite.
        writer.write(csvBuilder.toString());
        writer.close();
    }

    private double printStats(TileStats stats) {
        var reduction = (1 - (1 / ((stats.mvtSize()) / stats.covtSize()))) * 100;
        System.out.printf("MVT size: %dkb, COVT size with optimized metadta: %dkb, Reduction: %.2f%%%n",
                Math.round(stats.mvtSize() / 1000),
                Math.round(stats.covtSize() / 1000), reduction);

        /*
         * var unoptimizedMetadataReduction =
         * (1-(1/((stats.mvtSize())/stats.unoptimizedCovtSize())))*100;
         * System.out.
         * printf("MVT size: %dkb, Unoptimized Metadata COVT size: %dkb, Reduction: %.2f%%%n"
         * , Math.round(stats.mvtSize()/1000),
         * Math.round(stats.unoptimizedCovtSize()/1000), unoptimizedMetadataReduction);
         */

         var gzipReduction = (1-(1/((stats.gzipMvtSize())/
         stats.gzipCovtSize())))*100;
         System.out.printf("Gzip MVT size: %dkb, Optimized Metadata Gzip COVT size: %dkb, Gzip Reduction: %.2f%%%n",
                         Math.round(stats.gzipMvtSize()/1000), Math.round(stats.gzipCovtSize()/1000), gzipReduction);


        /*
         * var unoptimizedGzipReduction = (1-(1/((stats.gzipMvtSize())/
         * stats.unoptimizedGzipCovtSize())))*100;
         * System.out.
         * printf("Gzip MVT size: %dkb, Unoptimized Metadata Gzip COVT size: %dkb, Gzip Reduction: %.2f%%%n"
         * , Math.round(stats.gzipMvtSize()/1000),
         * Math.round(stats.unoptimizedGzipCovtSize()/1000),
         * unoptimizedGzipReduction);
         */
        // System.out.printf("COVT uncompressed to MVT Gzip compressed reduction:
        // %.2fkb, MVT Gzip decompression time for tile: %.4f milliseconds%n",
        // (stats.gzipMvtSize() - stats.covtSize()) / 1000,
        // stats.gzipMvtDecompressionTime());

        var gzipOptimizedGzipReduction = (1-(1/((stats.gzipCovtSize() / stats.unoptimizedGzipCovtSize()))))*100;
        var gzipOptimizedGzipToMvtReduction = (1-(1/((stats.gzipMvtSize() / stats.unoptimizedGzipCovtSize()))))*100;
        System.out.printf("Optimized Gzip COVT ratio %.2f%%%n", gzipOptimizedGzipReduction);
        System.out.printf("Optimized Gzip COVT to MVT ratio %.2f%%%n", gzipOptimizedGzipToMvtReduction);

        return reduction;
    }

    private TileStats runBingTest(String fileName) throws IOException {
        var mvtTile = MvtUtils.decodeMvt2(Paths.get(fileName));
        var mvtLayers = mvtTile.layers();

        var data = CovtConverter.convertMvtTile2(mvtLayers, mvtTile.tileExtent(),
                CovtConverter.GeometryEncoding.ICE_MORTON,
                true, true, false, false, true);
        var covtTile = data.getRight();
        var objectMapper = new ObjectMapper();
        var tileJson = objectMapper.readValue(data.getLeft(), TileJson.class);

        //var covtLayers = CovtParser.decodeCovt(covtTile, tileJson);

        //compareTiles(mvtLayers, covtLayers);
        //System.out.println("Tests passed. Tiles contain the same information");

        var unoptimizedData = CovtConverter.convertMvtTile2(mvtLayers, mvtTile.tileExtent(),
                CovtConverter.GeometryEncoding.ICE_MORTON,
                true, true, false, false, false);

        var gzipOptimizedData = CovtConverter.convertMvtTile2(mvtLayers, mvtTile.tileExtent(),
                CovtConverter.GeometryEncoding.ICE_MORTON,
                false, false, false, false, false);

        var gzipCovtSize = EncodingUtils.gzipCompress(covtTile).length;
        /*
         * var varintGzipCovtSize =
         * EncodingUtils.gzipCompress(varintEncodedCovtTile).length;
         * return new TileStats(mvtTile.mvtSize(), covtTile.length,
         * mvtTile.gzipCompressedMvtSize(),
         * gzipCovtSize < varintGzipCovtSize? gzipCovtSize : varintGzipCovtSize,
         * mvtTile.gzipDecompressionTime());
         */
        /*return new TileStats(mvtTile.mvtSize(), covtTile.length, mvtTile.gzipCompressedMvtSize(), gzipCovtSize,
                mvtTile.gzipDecompressionTime(), unoptimizedData.getRight().length,
                EncodingUtils.gzipCompress(unoptimizedData.getRight()).length);*/
        return new TileStats(mvtTile.mvtSize(), covtTile.length, mvtTile.gzipCompressedMvtSize(), gzipCovtSize,
                mvtTile.gzipDecompressionTime(), gzipOptimizedData.getRight().length,
                EncodingUtils.gzipCompress(gzipOptimizedData.getRight()).length);
    }

    private void compareTiles(List<Layer> mvtLayers, List<Layer> covtLayers) {
        assertEquals(mvtLayers.size(), covtLayers.size());
        for (var i = 0; i < mvtLayers.size(); i++) {
            var mvtLayer = mvtLayers.get(i);
            var covtLayer = covtLayers.get(i);

            assertEquals(mvtLayer.name(), covtLayer.name());

            var mvtFeatures = mvtLayer.features();
            var covtFeatures = covtLayer.features();
            for (var j = 0; j < mvtFeatures.size(); j++) {
                var mvtFeature = mvtFeatures.get(j);
                var covtFeature = covtFeatures.get(j);

                assertEquals(mvtFeature.id(), covtFeature.id());
                assertEquals(mvtFeature.geometry(), covtFeature.geometry());

                var mvtProperties = mvtFeature.properties();
                var covtProperties = covtFeature.properties();
                for (var propertyKey : mvtProperties.keySet()) {
                    var mvtProperty = mvtProperties.get(propertyKey);
                    var optionalCovtProperty = ((Optional) covtProperties.get(propertyKey));
                    var covtProperty = optionalCovtProperty.isPresent() ? optionalCovtProperty.get() : null;

                    assertEquals(mvtProperty, covtProperty);
                }
            }
        }
    }

    private String getTilIdFromFilename(File file) {
        var tileId = file.getName().split("\\.")[0].replace("_", "/");
        return tileId;
    }
}
