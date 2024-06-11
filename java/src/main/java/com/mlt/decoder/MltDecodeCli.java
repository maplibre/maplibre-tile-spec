package com.mlt.decoder;

import org.apache.commons.cli.CommandLine;
import org.apache.commons.cli.CommandLineParser;
import org.apache.commons.cli.DefaultParser;
import org.apache.commons.cli.Option;
import org.apache.commons.cli.Options;
import org.apache.commons.cli.ParseException;

import com.mlt.decoder.MltDecoder;
import com.mlt.vector.FeatureTable;
import com.mlt.metadata.tileset.MltTilesetMetadata;
import com.mlt.data.MapLibreTile;

import java.nio.file.Files;
import java.nio.file.Paths;
import java.nio.file.Path;

public class MltDecodeCli {

    public static class Timer {
        private long startTime;

        public Timer() {
            startTime = System.nanoTime();
        }

        public void restart() {
            startTime = System.nanoTime();
        }

        public void stop(String message) {
            long endTime = System.nanoTime();
            long elapsedTime = (endTime - startTime) / 1000000; // divide by 1000000 to get milliseconds
            System.out.println("Time elapsed for " + message + ": " + elapsedTime + " milliseconds");
        }
    }

    public static void printMLT(MapLibreTile mlTile){
        var mltLayers = mlTile.layers();
        for(var i = 0; i < mltLayers.size(); i++){
            var mltLayer = mltLayers.get(i);
            System.out.println(mltLayer.name());
            var mltFeatures = mltLayer.features();
            for(var j = 0; j < mltFeatures.size(); j++){
                var mltFeature = mltFeatures.get(j);
                System.out.println("  " + mltFeature);
            }
        }
    }

    private static void printMLTVectorized(FeatureTable[] featureTables){
        for(var i = 0; i < featureTables.length; i++){
            var featureTable = featureTables[i];
            System.out.println(featureTable.getName());
            var featureIterator = featureTable.iterator();
            while (featureIterator.hasNext()) {
                var mltFeature = featureIterator.next();
                System.out.println("  " + mltFeature);
            }
        }
    }

    private static final String FILE_NAME_ARG = "mlt";
    private static final String PRINT_MLT_OPTION = "printmlt";
    private static final String VECTORIZED_OPTION = "vectorized";
    public static void main(String[] args) {
        Options options = new Options();
        options.addOption(Option.builder(FILE_NAME_ARG)
            .hasArg(true)
            .desc("Path to the input MLT file to read ([REQUIRED])")
            .required(true)
            .build());
        options.addOption(Option.builder(VECTORIZED_OPTION)
            .hasArg(false)
            .desc("Use the vectorized decoding path ([OPTIONAL], default: will use non-vectorized path)")
            .required(false)
            .build());
        options.addOption(Option.builder(PRINT_MLT_OPTION)
            .hasArg(false)
            .desc("Print the MLT tile after encoding it ([OPTIONAL], default: false)")
            .required(false)
            .build());
        CommandLineParser parser = new DefaultParser();
        try {
            CommandLine cmd = parser.parse(options, args);
            var fileName = cmd.getOptionValue(FILE_NAME_ARG);
            if (fileName == null) {
                throw new ParseException("Missing required argument: " + FILE_NAME_ARG);
            }
            var willPrintMLT = cmd.hasOption(PRINT_MLT_OPTION);
            var willUseVectorized = cmd.hasOption(VECTORIZED_OPTION);
            var inputTilePath = Paths.get(fileName);
            if (!Files.exists(inputTilePath)) {
                throw new IllegalArgumentException("Input mlt tile path does not exist: " + inputTilePath);
            }
            var inputTileName = inputTilePath.getFileName().toString();
            // var inputMetaPath = Paths.get(inputTilePath.getParent().toString(),"mltmetadata.pbf");
            var inputMetaPath = Paths.get(inputTilePath + ".meta.pbf");
            if (!Files.exists(inputMetaPath)) {
                throw new IllegalArgumentException("Input metadata path does not exist: " + inputMetaPath);
            }
            var inputMetaBuffer = Files.readAllBytes(inputMetaPath);
            var tileMetadata = MltTilesetMetadata.TileSetMetadata.parseFrom(inputMetaBuffer);
            var mltTileBuffer = Files.readAllBytes(inputTilePath);

            Timer timer = new Timer();
            if (willUseVectorized) {
                var decodedTile = MltDecoder.decodeMlTileVectorized(mltTileBuffer, tileMetadata);
                timer.stop("decoding");
                if (willPrintMLT) {
                    printMLTVectorized(decodedTile);
                }
            } else {
                var decodedTile = MltDecoder.decodeMlTile(mltTileBuffer, tileMetadata);
                timer.stop("decoding");
                if (willPrintMLT) {
                    printMLT(decodedTile);
                }
            }
        } catch (Exception e) {
            e.printStackTrace();
            System.exit(1);
        }
    }
}
