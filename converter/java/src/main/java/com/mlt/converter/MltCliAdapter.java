package com.mlt.converter;

import java.io.IOException;

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

import java.nio.file.Files;
import java.nio.file.Paths;
import java.util.HashMap;
import java.util.List;
import java.util.Map;
import java.util.Optional;

public class MltCliAdapter {
    private static final String FILE_NAME_ARG = "mvt";
    private static final String OUTPUT_DIR_ARG = "output";

    public static void main(String[] args) {
        Options options = new Options();
        options.addOption(Option.builder(FILE_NAME_ARG)
            .hasArg(true)
            .desc("Path to the MVT file to convert ([REQUIRED])")
            .required(true)
            .build());
        options.addOption(Option.builder(OUTPUT_DIR_ARG)
            .hasArg(true)
            .desc("Output directory to write the converted MLT file to ([REQUIRED])")
            .required(true)
            .build());
        CommandLineParser parser = new DefaultParser();
        try {
            CommandLine cmd = parser.parse(options, args);
            var fileName = cmd.getOptionValue(FILE_NAME_ARG);
            if (fileName == null) {
                throw new ParseException("Missing required argument: " + FILE_NAME_ARG);
            }
            var outputDir = cmd.getOptionValue(OUTPUT_DIR_ARG);
            if (outputDir == null) {
                throw new ParseException("Missing required argument: " + OUTPUT_DIR_ARG);
            }
            var inputTilePath = Paths.get(fileName);
            var inputTileName = inputTilePath.getFileName().toString();
            var outputTileName = String.format("%s.mlt", inputTileName.split("\\.")[0]);
            var outputPath = Paths.get(outputDir, outputTileName);
            System.out.println("Converting " + fileName + " to " + outputPath.toString());
            var outputDirPath = outputPath.getParent();
            if (!Files.exists(outputDirPath)) {
                System.out.println("Creating directory: " + outputDirPath);
                Files.createDirectories(outputDirPath);
            }                
            // TODO: fails with ../../test/fixtures/bing/mvt/4-12-6.mvt
            // java.lang.IndexOutOfBoundsException: Index 6 out of bounds for length 6
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
            var useAdvancedEncodingSchemes = false;
            var includeIds = true;
            var conversionConfig = new ConversionConfig(includeIds, useAdvancedEncodingSchemes, optimizations);
            System.out.println("converting data now...");
            long startTime = System.nanoTime();
            var mlTile = MltConverter.convertMvt(decodedMvTile, conversionConfig, tileMetadata);
            long endTime = System.nanoTime();            
            long duration = (endTime - startTime);  //divide by 1000000 to get milliseconds.
            System.out.println("Conversion took: " + duration / 1000000 + "ms");
            System.out.println("Writing converted tile to " + outputPath);
            Files.write(outputPath, mlTile);
        } catch (Exception e) {
            e.printStackTrace();
            System.exit(1);
        }
    }
}
