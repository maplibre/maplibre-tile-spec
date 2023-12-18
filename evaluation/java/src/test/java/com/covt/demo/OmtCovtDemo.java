package com.covt.demo;

import com.covt.converter.CovtConverter;
import com.covt.converter.EncodingUtils;
import com.covt.converter.mvt.MvtUtils;
import org.jetbrains.annotations.NotNull;
import org.junit.jupiter.api.Test;
import java.io.File;
import java.io.IOException;
import java.nio.file.Paths;
import java.util.Arrays;
import java.util.stream.Collectors;

public class OmtCovtDemo {
    private static final String OMT_MVT_PATH = "..\\..\\test\\fixtures\\omt\\mvt";

    @Test
    public void omtTilesTests() throws IOException {
        File folder = new File(OMT_MVT_PATH);
        File[] files = folder.listFiles();

        var previousFileZoomLevel = -1;
        var peerZoomReduction = 0d;
        var tilesPerZoom = 0;

        var sortedFiles = Arrays.stream(files).sorted((a, b) -> {
            var zoom1 = Integer.parseInt(getTilIdFromFilename(a).split("/")[0]);
            var zoom2 = Integer.parseInt(getTilIdFromFilename(b).split("/")[0]);
            return zoom1 - zoom2;
        }).collect(Collectors.toList());

        for (File file : sortedFiles) {
            if (file.isFile()) {
                var fileName = file.getAbsoluteFile();
                String tileId = getTilIdFromFilename(file);
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

                var stats = runOmtTest(fileName.toString());
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

    private String getTilIdFromFilename(File file) {
        var tileId = file.getName().split("\\.")[0].replace("_", "/");
        return tileId;
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

    private TileStats runOmtTest(String fileName) throws IOException {
        var mvtTile = MvtUtils.decodeMvt2(Paths.get(fileName));
        var mvtLayers = mvtTile.layers();

        var covtTile = CovtConverter.convertMvtTile(mvtLayers, mvtTile.tileExtent(), CovtConverter.GeometryEncoding.ICE_MORTON,
                true, true, true, true, true);
        var varintEncodedCovtTile = CovtConverter.convertMvtTile(mvtLayers, mvtTile.tileExtent(), CovtConverter.GeometryEncoding.ICE_MORTON,
                false, false, true, true, true);

        var gzipCovtSize = EncodingUtils.gzipCompress(covtTile).length;
        var varintGzipCovtSize = EncodingUtils.gzipCompress(varintEncodedCovtTile).length;
        return new TileStats(mvtTile.mvtSize(), covtTile.length, mvtTile.gzipCompressedMvtSize(),
                gzipCovtSize < varintGzipCovtSize? gzipCovtSize : varintGzipCovtSize, mvtTile.gzipDecompressionTime(), 0, 0);
    }
}
