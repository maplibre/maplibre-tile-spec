package com.mlt.converter;

import org.apache.commons.cli.CommandLine;
import org.apache.commons.cli.CommandLineParser;
import org.apache.commons.cli.DefaultParser;
import org.apache.commons.cli.Option;
import org.apache.commons.cli.Options;
import org.apache.commons.cli.ParseException;

import com.mlt.converter.MltConverter;
import com.mlt.decoder.MltDecoder;
import com.mlt.converter.mvt.MvtUtils;
import com.mlt.converter.mvt.ColumnMapping;
import com.mlt.data.MapLibreTile;
import com.mlt.converter.mvt.MapboxVectorTile;

import java.nio.file.Files;
import java.nio.file.Paths;
import java.nio.file.Path;
import java.util.HashMap;
import java.util.List;
import java.util.Optional;

public class MltCliAdapter {

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

    public static void printMVT(MapboxVectorTile mvTile){
        var mvtLayers = mvTile.layers();
        for(var i = 0; i < mvtLayers.size(); i++){
            var mvtLayer = mvtLayers.get(i);
            System.out.println(mvtLayer.name());
            var mvtFeatures = mvtLayer.features();
            for(var j = 0; j < mvtFeatures.size(); j++){
                var mvtFeature = mvtFeatures.get(j);
                System.out.println("  " + mvtFeature);
            }
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

    public static void compare(MapLibreTile mlTile, MapboxVectorTile mvTile){
        var mltLayers = mlTile.layers();
        var mvtLayers = mvTile.layers();
        if (mltLayers.size() != mvtLayers.size()) {
            throw new RuntimeException("Number of layers in MLT and MVT tiles do not match");
        }
        for(var i = 0; i < mvtLayers.size(); i++){
            var mltLayer = mltLayers.get(i);
            var mvtLayer = mvtLayers.get(i);
            var mltFeatures = mltLayer.features();
            var mvtFeatures = mvtLayer.features();
            if (mltFeatures.size() != mvtFeatures.size()) {
                throw new RuntimeException("Number of features in MLT and MVT layers do not match");
            }
            for(var j = 0; j < mvtFeatures.size(); j++){
                var mvtFeature = mvtFeatures.get(j);
                var mltFeature = mltFeatures.stream().filter(f -> f.id() == mvtFeature.id()).findFirst().get();
                if (mvtFeature.id() != mltFeature.id()) {
                    throw new RuntimeException("Feature IDs in MLT and MVT layers do not match: " + mvtFeature.id() + " != " + mltFeature.id());
                }
                var mltGeometry = mltFeature.geometry();
                var mvtGeometry = mvtFeature.geometry();
                if (!mltGeometry.equals(mvtGeometry)) {
                    throw new RuntimeException("Geometries in MLT and MVT layers do not match: " + mvtGeometry + " != " + mltGeometry);
                }
                var mltProperties = mltFeature.properties();
                var mvtProperties = mvtFeature.properties();
                if (mvtProperties.size() != mltProperties.size()) {
                    throw new RuntimeException("Number of properties in MLT and MVT features do not match");
                }
                var mvtPropertyKeys = mvtProperties.keySet();
                var mltPropertyKeys = mltProperties.keySet();
                // compare keys
                if (!mvtPropertyKeys.equals(mltPropertyKeys)) {
                    throw new RuntimeException("Property keys in MLT and MVT features do not match");
                }
                // compare values
                var equalValues = mvtProperties.keySet()
                    .stream()
                    .allMatch(key -> mvtProperties.get(key).equals(mltProperties.get(key)));
                if (!equalValues) {
                    throw new RuntimeException("Property values in MLT and MVT features do not match: \n'" + mvtProperties + "'\n'" + mltProperties + "'");
                }
            }
        }
    }

    private static final String FILE_NAME_ARG = "mvt";
    private static final String OUTPUT_DIR_ARG = "dir";
    private static final String OUTPUT_FILE_ARG = "mlt";
    private static final String INCLUDE_TILESET_METADATA_OPTION = "metadata";
    private static final String ADVANCED_ENCODING_OPTION = "advanced";
    private static final String DECODE_OPTION = "decode";
    private static final String PRINT_MLT_OPTION = "printmlt";
    private static final String PRINT_MVT_OPTION = "printmvt";
    private static final String COMPARE_OPTION = "compare";
    public static void main(String[] args) {
        Options options = new Options();
        options.addOption(Option.builder(FILE_NAME_ARG)
            .hasArg(true)
            .desc("Path to the input MVT file to read ([REQUIRED])")
            .required(true)
            .build());
        options.addOption(Option.builder(OUTPUT_DIR_ARG)
            .hasArg(true)
            .desc("Output directory to write an MLT file to ([OPTIONAL], default: no file is written)")
            .required(false)
            .build());
        options.addOption(Option.builder(OUTPUT_FILE_ARG)
            .hasArg(true)
            .desc("Output file to write an MLT file to ([OPTIONAL], default: no file is written)")
            .required(false)
            .build());
        options.addOption(Option.builder(INCLUDE_TILESET_METADATA_OPTION)
            .hasArg(false)
            .desc("Include tileset metadata in the output ([OPTIONAL], default: false)")
            .required(false)
            .build());
        options.addOption(Option.builder(ADVANCED_ENCODING_OPTION)
            .hasArg(false)
            .desc("Enable advanced encodings (fsst & fastpfor) ([OPTIONAL], default: false")
            .required(false)
            .build());
        options.addOption(Option.builder(DECODE_OPTION)
            .hasArg(false)
            .desc("Test decoding the tile after encoding it ([OPTIONAL], default: false)")
            .required(false)
            .build());
        options.addOption(Option.builder(PRINT_MLT_OPTION)
            .hasArg(false)
            .desc("Print the MLT tile after encoding it ([OPTIONAL], default: false)")
            .required(false)
            .build());
        options.addOption(Option.builder(PRINT_MVT_OPTION)
            .hasArg(false)
            .desc("Print the round tripped MVT tile ([OPTIONAL], default: false)")
            .required(false)
            .build());
        options.addOption(Option.builder(COMPARE_OPTION)
            .hasArg(false)
            .desc("Assert that data in the the decoded tile is the same as the data in the input tile ([OPTIONAL], default: false)")
            .required(false)
            .build());
        CommandLineParser parser = new DefaultParser();
        try {
            CommandLine cmd = parser.parse(options, args);
            var fileName = cmd.getOptionValue(FILE_NAME_ARG);
            if (fileName == null) {
                throw new ParseException("Missing required argument: " + FILE_NAME_ARG);
            }
            if (cmd.hasOption(OUTPUT_FILE_ARG) && cmd.hasOption(OUTPUT_DIR_ARG)) {
                throw new ParseException("Cannot specify both '-" + OUTPUT_FILE_ARG + "' and '-" + OUTPUT_DIR_ARG + "' options");
            }
            var willOutput = cmd.hasOption(OUTPUT_FILE_ARG) || cmd.hasOption(OUTPUT_DIR_ARG);
            var willIncludeTilesetMetadata = cmd.hasOption(INCLUDE_TILESET_METADATA_OPTION);
            var useAdvancedEncodingSchemes = cmd.hasOption(ADVANCED_ENCODING_OPTION);
            var willDecode = cmd.hasOption(DECODE_OPTION);
            var willPrintMLT = cmd.hasOption(PRINT_MLT_OPTION);
            var willPrintMVT = cmd.hasOption(PRINT_MVT_OPTION);
            var willCompare = cmd.hasOption(COMPARE_OPTION);
            var inputTilePath = Paths.get(fileName);
            var inputTileName = inputTilePath.getFileName().toString();
            var decodedMvTile = MvtUtils.decodeMvt(inputTilePath);

            // No ColumnMapping as support is still buggy: https://github.com/maplibre/maplibre-tile-spec/issues/59
            var columnMappings = Optional.<List<ColumnMapping>>empty();
            var isIdPresent = true;
            var tileMetadata = MltConverter.createTilesetMetadata(decodedMvTile, columnMappings, isIdPresent);

            var allowIdRegeneration = true;
            var allowSorting = false;
            var optimization = new FeatureTableOptimizations(allowSorting, allowIdRegeneration, columnMappings);
            // No FeatureTableOptimizations
            var optimizations = new HashMap<String, FeatureTableOptimizations>();
            var includeIds = true;
            var conversionConfig = new ConversionConfig(includeIds, useAdvancedEncodingSchemes, optimizations);
            Timer timer = new Timer();
            var mlTile = MltConverter.convertMvt(decodedMvTile, conversionConfig, tileMetadata);
            timer.stop("encoding");
            if (willOutput) {
                Path outputPath = null;
                if (cmd.hasOption(OUTPUT_DIR_ARG)) {
                    var outputDir = cmd.getOptionValue(OUTPUT_DIR_ARG);
                    var outputTileName = String.format("%s.mlt", inputTileName.split("\\.")[0]);
                    outputPath = Paths.get(outputDir, outputTileName);
                } else if (cmd.hasOption(OUTPUT_FILE_ARG)) {
                    outputPath = Paths.get(cmd.getOptionValue(OUTPUT_FILE_ARG));
                }
                var outputDirPath = outputPath.toAbsolutePath().getParent();
                if (!Files.exists(outputDirPath)) {
                    System.out.println("Creating directory: " + outputDirPath);
                    Files.createDirectories(outputDirPath);
                }
                System.out.println("Writing converted tile to " + outputPath);
                Files.write(outputPath, mlTile);

                if (willIncludeTilesetMetadata) {
                    Path outputMetadataPath = Paths.get(outputPath.toString() + ".meta.pbf");
                    System.out.println("Writing metadata to " + outputMetadataPath);
                    tileMetadata.writeTo(Files.newOutputStream(outputMetadataPath));
                }
            }
            if (willPrintMVT) {
                printMVT(decodedMvTile);
            }
            var needsDecoding = willDecode || willCompare || willPrintMLT;
            if (needsDecoding) {
                timer.restart();
                // convert MLT wire format to an MapLibreTile object
                var decodedTile = MltDecoder.decodeMlTile(mlTile, tileMetadata);
                timer.stop("decoding");
                if (willPrintMLT) {
                    printMLT(decodedTile);
                }
                if (willCompare) {
                    compare(decodedTile, decodedMvTile);
                }
            }
        } catch (Exception e) {
            e.printStackTrace();
            System.exit(1);
        }
    }
}
