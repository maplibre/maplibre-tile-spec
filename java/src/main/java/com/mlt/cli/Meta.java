package com.mlt.tools;

import org.apache.commons.cli.CommandLine;
import org.apache.commons.cli.CommandLineParser;
import org.apache.commons.cli.DefaultParser;
import org.apache.commons.cli.Option;
import org.apache.commons.cli.Options;
import org.apache.commons.cli.ParseException;
import org.apache.commons.cli.HelpFormatter;

import com.mlt.converter.MltConverter;
import com.mlt.converter.mvt.MvtUtils;
import com.mlt.converter.mvt.ColumnMapping;
import com.mlt.converter.mvt.MapboxVectorTile;

import java.nio.file.Files;
import java.nio.file.Paths;
import java.nio.file.Path;
import java.util.List;
import java.util.Optional;

public class Meta {

    private static final String FILE_NAME_ARG = "mvt";
    private static final String OUTPUT_DIR_ARG = "dir";
    private static final String OUTPUT_FILE_ARG = "meta";
    private static final String OPTION_HELP_ARG = "help";
    public static void main(String[] args) {
        Options options = new Options();
        options.addOption(Option.builder(FILE_NAME_ARG)
            .hasArg(true)
            .desc("Path to the input MVT file to read ([REQUIRED])")
            .required(true)
            .build());
        options.addOption(Option.builder(OUTPUT_DIR_ARG)
            .hasArg(true)
            .desc("Output directory to write a mltmetadata.pbf file to ([REQUIRED])")
            .required(false)
            .build());
        options.addOption(Option.builder(OUTPUT_FILE_ARG)
            .hasArg(true)
            .desc("Output file to write an MLT file to ([OPTIONAL], default: no file is written)")
            .required(false)
            .build());
        options.addOption(Option.builder(OPTION_HELP_ARG)
            .hasArg(false)
            .required(false)
            .build());
        CommandLineParser parser = new DefaultParser();
        HelpFormatter formatter = new HelpFormatter();
        formatter.setOptionComparator(null);
        try {
            CommandLine cmd = parser.parse(options, args);
            if (cmd.hasOption(OPTION_HELP_ARG)) {
                formatter.printHelp("meta", options);
                System.exit(0);
            }
            if (cmd.hasOption(OUTPUT_FILE_ARG) && cmd.hasOption(OUTPUT_DIR_ARG)) {
                throw new ParseException("Cannot specify both '-" + OUTPUT_FILE_ARG + "' and '-" + OUTPUT_DIR_ARG + "' options");
            }
            if (!cmd.hasOption(OUTPUT_FILE_ARG) && !cmd.hasOption(OUTPUT_DIR_ARG)) {
                throw new ParseException("Must specify either '-" + OUTPUT_FILE_ARG + "' or '-" + OUTPUT_DIR_ARG + "'");
            }

            System.out.println("Reading MVT: " + cmd.getOptionValue(FILE_NAME_ARG));
            var fileName = cmd.getOptionValue(FILE_NAME_ARG);
            var inputTilePath = Paths.get(fileName);
            var inputTileName = inputTilePath.getFileName().toString();
            var decodedMvTile = MvtUtils.decodeMvt(inputTilePath);

            // No ColumnMapping as support is still buggy: https://github.com/maplibre/maplibre-tile-spec/issues/59
            var columnMappings = Optional.<List<ColumnMapping>>empty();
            var isIdPresent = true;
            var tileMetadata = MltConverter.createTilesetMetadata(decodedMvTile, columnMappings, isIdPresent);
            Path outputPath = null;
            if (cmd.hasOption(OUTPUT_DIR_ARG)) {
                var outputDir = cmd.getOptionValue(OUTPUT_DIR_ARG);
                var outputTileName = String.format("%s.mlt.meta.pbf", inputTileName.split("\\.")[0]);
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
            if (Files.exists(outputPath)) {
                long existingFileSize = Files.size(outputPath);
                long newFileSize = tileMetadata.getSerializedSize();
                if (existingFileSize != newFileSize) {
                    System.out.println("Metadata size has changed. Overwriting metadata: " + outputPath);
                } else {
                    System.out.println("Metadata size has not changed. Skipping metadata overwrite.");
                    return;
                }
            }
            tileMetadata.writeTo(Files.newOutputStream(outputPath));
        } catch (Exception e) {
            formatter.printHelp("meta", "\n", options, "Error:\n  " + e.getMessage(), true);
            System.exit(1);
        }
    }
}
