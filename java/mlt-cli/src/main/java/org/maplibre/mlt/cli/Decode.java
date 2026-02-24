package org.maplibre.mlt.cli;

import java.io.IOException;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Paths;
import org.apache.commons.cli.CommandLine;
import org.apache.commons.cli.CommandLineParser;
import org.apache.commons.cli.DefaultParser;
import org.apache.commons.cli.Option;
import org.apache.commons.cli.Options;
import org.apache.commons.cli.ParseException;
import org.apache.commons.lang3.NotImplementedException;
import org.maplibre.mlt.decoder.MltDecoder;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;

public class Decode {

  private static final String FILE_NAME_ARG = "mlt";
  private static final String PRINT_MLT_OPTION = "printmlt";
  private static final String VECTORIZED_OPTION = "vectorized";
  private static final String TIMER_OPTION = "timer";

  public static void main(String[] args) {
    try {
      run(args);
    } catch (Exception ex) {
      logger.error("Decoding failed", ex);
      System.exit(1);
    }
  }

  private static void run(String[] args) throws ParseException, IOException {
    Options options = new Options();
    options.addOption(
        Option.builder()
            .longOpt(FILE_NAME_ARG)
            .hasArg(true)
            .desc("Path to the input MLT file to read ([REQUIRED])")
            .required(true)
            .get());
    options.addOption(
        Option.builder()
            .longOpt(VECTORIZED_OPTION)
            .hasArg(false)
            .desc(
                "Use the vectorized decoding path ([OPTIONAL], default: will use non-vectorized path)")
            .required(false)
            .get());
    options.addOption(
        Option.builder()
            .longOpt(PRINT_MLT_OPTION)
            .hasArg(false)
            .desc("Print the MLT tile after encoding it ([OPTIONAL], default: false)")
            .required(false)
            .get());
    options.addOption(
        Option.builder()
            .longOpt(TIMER_OPTION)
            .hasArg(false)
            .desc("Print the time it takes, in ms, to decode a tile ([OPTIONAL])")
            .required(false)
            .get());
    CommandLineParser parser = new DefaultParser();
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
    var mltTileBuffer = Files.readAllBytes(inputTilePath);

    Timer timer = new Timer();
    if (willUseVectorized) {
      throw new NotImplementedException("Vectorized decoding is not available");
    } else {
      var decodedTile = MltDecoder.decodeMlTile(mltTileBuffer);
      if (willTime) timer.stop("decoding");
      if (willPrintMLT) {
        System.out.write(JsonHelper.toJson(decodedTile).getBytes(StandardCharsets.UTF_8));
      }
    }
  }

  private static final Logger logger = LoggerFactory.getLogger(Decode.class);
}
