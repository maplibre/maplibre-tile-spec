package org.maplibre.mlt.tools;

import org.maplibre.mlt.decoder.MltDecoder;
import org.maplibre.mlt.metadata.tileset.MltTilesetMetadata;
import java.nio.file.Files;
import java.nio.file.Paths;
import org.apache.commons.cli.CommandLine;
import org.apache.commons.cli.CommandLineParser;
import org.apache.commons.cli.DefaultParser;
import org.apache.commons.cli.Option;
import org.apache.commons.cli.Options;
import org.apache.commons.cli.ParseException;

public class Decode {

  private static final String FILE_NAME_ARG = "mlt";
  private static final String PRINT_MLT_OPTION = "printmlt";
  private static final String VECTORIZED_OPTION = "vectorized";
  private static final String TIMER_OPTION = "timer";

  public static void main(String[] args) {
    Options options = new Options();
    options.addOption(
        Option.builder(FILE_NAME_ARG)
            .hasArg(true)
            .desc("Path to the input MLT file to read ([REQUIRED])")
            .required(true)
            .build());
    options.addOption(
        Option.builder(VECTORIZED_OPTION)
            .hasArg(false)
            .desc(
                "Use the vectorized decoding path ([OPTIONAL], default: will use non-vectorized path)")
            .required(false)
            .build());
    options.addOption(
        Option.builder(PRINT_MLT_OPTION)
            .hasArg(false)
            .desc("Print the MLT tile after encoding it ([OPTIONAL], default: false)")
            .required(false)
            .build());
    options.addOption(
        Option.builder(TIMER_OPTION)
            .hasArg(false)
            .desc("Print the time it takes, in ms, to decode a tile ([OPTIONAL])")
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
      var willTime = cmd.hasOption(TIMER_OPTION);
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
        var featureTables = MltDecoder.decodeMlTileVectorized(mltTileBuffer, tileMetadata);
        // Note: the vectorized result is a FeatureTable array
        // which provides an iterator to access the features.
        // Therefore, we must iterate over the FeatureTable array
        // to trigger actual decoding of the features.
        CliUtil.decodeFeatureTables(featureTables);
        if (willTime) timer.stop("decoding");
        if (willPrintMLT) {
          CliUtil.printMLTVectorized(featureTables);
        }
      } else {
        var decodedTile = MltDecoder.decodeMlTile(mltTileBuffer, tileMetadata);
        if (willTime) timer.stop("decoding");
        if (willPrintMLT) {
          CliUtil.printMLT(decodedTile);
        }
      }
    } catch (Exception e) {
      e.printStackTrace();
      System.exit(1);
    }
  }
}
