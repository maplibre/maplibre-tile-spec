package org.maplibre.mlt.cli;

import java.nio.file.Files;
import java.nio.file.Paths;
import org.apache.commons.cli.CommandLine;
import org.apache.commons.cli.CommandLineParser;
import org.apache.commons.cli.DefaultParser;
import org.apache.commons.cli.Option;
import org.apache.commons.cli.Options;
import org.apache.commons.cli.ParseException;
import org.maplibre.mlt.converter.MltConverter;
import org.maplibre.mlt.converter.mvt.MvtUtils;

public class Meta {

  private static final String FILE_NAME_ARG = "mvt";
  private static final String OUTPUT_FILE_ARG = "output";

  public static void main(String[] args) {
    Options options = new Options();
    options.addOption(
        Option.builder()
            .longOpt(FILE_NAME_ARG)
            .hasArg(true)
            .desc("Path to the input MVT file to read ([REQUIRED])")
            .required(true)
            .get());
    options.addOption(
        Option.builder()
            .longOpt(OUTPUT_FILE_ARG)
            .hasArg(true)
            .desc("Path to the output metadata file ([OPTIONAL], default: stdout)")
            .required(false)
            .get());

    CommandLineParser parser = new DefaultParser();
    try {
      CommandLine cmd = parser.parse(options, args);
      var fileName = cmd.getOptionValue(FILE_NAME_ARG);
      if (fileName == null) {
        throw new ParseException("Missing required argument: " + FILE_NAME_ARG);
      }

      var inputTilePath = Paths.get(fileName);
      if (!Files.exists(inputTilePath)) {
        throw new IllegalArgumentException("Input MVT tile path does not exist: " + inputTilePath);
      }

      var decodedMvTile = MvtUtils.decodeMvt(inputTilePath);
      var metadata =
          MltConverter.createTilesetMetadata(
              decodedMvTile,
              java.util.List.of(), // No column mappings
              true, // Include IDs
              false, // Don't coerce property values
              false // Don't elide on type mismatch
              );
      var metadataJSON = MltConverter.createTilesetMetadataJSON(metadata);

      var outputPath = cmd.getOptionValue(OUTPUT_FILE_ARG);
      if (outputPath != null) {
        Files.writeString(Paths.get(outputPath), metadataJSON);
        System.out.println("Metadata written to: " + outputPath);
      } else {
        System.out.println(metadataJSON);
      }
    } catch (Exception e) {
      System.err.println("Metadata generation failed: ");
      e.printStackTrace(System.err);
      System.exit(1);
    }
  }
}
