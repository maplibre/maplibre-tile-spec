package org.maplibre.mlt.cli;

import java.io.IOException;
import java.net.URI;
import java.net.URISyntaxException;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.Collection;
import java.util.List;
import java.util.regex.Pattern;
import java.util.regex.PatternSyntaxException;
import java.util.stream.Collectors;
import java.util.stream.Stream;
import org.apache.commons.cli.CommandLine;
import org.apache.commons.cli.Converter;
import org.apache.commons.cli.DefaultParser;
import org.apache.commons.cli.Option;
import org.apache.commons.cli.Options;
import org.apache.commons.cli.ParseException;
import org.apache.commons.cli.help.HelpFormatter;
import org.apache.commons.cli.help.TextHelpAppendable;
import org.apache.commons.lang3.NotImplementedException;
import org.apache.commons.lang3.StringUtils;
import org.maplibre.mlt.converter.encodings.fsst.FsstJni;
import org.maplibre.mlt.converter.mvt.ColumnMapping;
import org.maplibre.mlt.converter.mvt.ColumnMappingConfig;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;

class EncodeCommandLine {
  static final String INPUT_TILE_ARG = "mvt";
  static final String INPUT_MBTILES_ARG = "mbtiles";
  static final String INPUT_OFFLINEDB_ARG = "offlinedb";
  static final String INPUT_PMTILES_ARG = "pmtiles";
  static final String OUTPUT_DIR_ARG = "dir";
  static final String OUTPUT_FILE_ARG = "mlt";
  static final String EXCLUDE_IDS_OPTION = "noids";
  static final String SORT_FEATURES_OPTION = "sort-ids";
  static final String REGEN_IDS_OPTION = "regen-ids";
  static final String FILTER_LAYERS_OPTION = "filter-layers";
  static final String MIN_ZOOM_OPTION = "minzoom";
  static final String MAX_ZOOM_OPTION = "maxzoom";
  static final String FILTER_LAYERS_INVERT_OPTION = "filter-layers-invert";
  static final String INCLUDE_METADATA_OPTION = "metadata";
  static final String ALLOW_COERCE_OPTION = "coerce-mismatch";
  static final String ALLOW_ELISION_OPTION = "elide-mismatch";
  static final String FASTPFOR_ENCODING_OPTION = "enable-fastpfor";
  static final String FSST_ENCODING_OPTION = "enable-fsst";
  static final String FSST_NATIVE_ENCODING_OPTION = "enable-fsst-native";
  static final String COLUMN_MAPPING_AUTO_OPTION = "colmap-auto";
  static final String COLUMN_MAPPING_DELIM_OPTION = "colmap-delim";
  static final String COLUMN_MAPPING_LIST_OPTION = "colmap-list";
  static final String NO_MORTON_OPTION = "nomorton";
  static final String PRE_TESSELLATE_OPTION = "tessellate";
  static final String TESSELLATE_URL_OPTION = "tessellateurl";
  static final String OUTLINE_FEATURE_TABLES_OPTION = "outlines";
  static final String DECODE_OPTION = "decode";
  static final String PRINT_MLT_OPTION = "printmlt";
  static final String PRINT_MVT_OPTION = "printmvt";
  static final String COMPARE_ALL_OPTION = "compare-all";
  static final String COMPARE_GEOM_OPTION = "compare-geometry";
  static final String COMPARE_PROP_OPTION = "compare-properties";
  static final String VECTORIZED_OPTION = "vectorized";
  static final String TIMER_OPTION = "timer";
  static final String COMPRESS_OPTION = "compress";
  private static final String COMPRESS_OPTION_DEFLATE = "deflate";
  private static final String COMPRESS_OPTION_GZIP = "gzip";
  private static final String COMPRESS_OPTION_NONE = "none";
  private static final String COMPRESS_OPTION_BROTLI = "brotli";
  private static final String COMPRESS_OPTION_ZSTD = "zstd";
  static final String DUMP_STREAMS_OPTION = "rawstreams";
  static final String PARALLEL_OPTION = "parallel";
  static final String VERBOSE_OPTION = "verbose";
  static final String CONTINUE_OPTION = "continue";
  static final String HELP_OPTION = "help";
  static final String SERVER_ARG = "server";

  private static List<String> getAllowedCompressions(CommandLine cmd) {
    final var allowed =
        new ArrayList<>(
            Arrays.asList(COMPRESS_OPTION_GZIP, COMPRESS_OPTION_DEFLATE, COMPRESS_OPTION_NONE));
    if (cmd.hasOption(INPUT_PMTILES_ARG)) {
      allowed.add(COMPRESS_OPTION_BROTLI);
      allowed.add(COMPRESS_OPTION_ZSTD);
    }
    return allowed;
  }

  private static boolean validateCompression(CommandLine cmd) {
    return getAllowedCompressions(cmd)
        .contains(cmd.getOptionValue(COMPRESS_OPTION, COMPRESS_OPTION_NONE));
  }

  static CommandLine getCommandLine(String[] args) throws IOException {
    try {
      Options options = new Options();
      options.addOption(
          Option.builder()
              .longOpt(INPUT_TILE_ARG)
              .hasArgs()
              .argName("file")
              .desc("Path to the input MVT file")
              .required(false)
              .get());
      options.addOption(
          Option.builder()
              .longOpt(INPUT_MBTILES_ARG)
              .hasArg(true)
              .argName("file")
              .desc("Path of the input MBTiles file.")
              .required(false)
              .get());
      options.addOption(
          Option.builder()
              .longOpt(INPUT_PMTILES_ARG)
              .hasArg(true)
              .argName("fileOrUri")
              .desc("Location of the input PMTiles file.")
              .required(false)
              .get());
      options.addOption(
          Option.builder()
              .longOpt(INPUT_OFFLINEDB_ARG)
              .hasArg(true)
              .argName("file")
              .desc("Path of the input offline database file.")
              .required(false)
              .get());
      options.addOption(
          Option.builder()
              .longOpt(OUTPUT_DIR_ARG)
              .hasArg(true)
              .argName("dir")
              .desc(
                  "Directory where the output is written, using the input file basename (OPTIONAL).")
              .required(false)
              .get());
      options.addOption(
          Option.builder()
              .longOpt(OUTPUT_FILE_ARG)
              .hasArg(true)
              .argName("file")
              .desc(
                  "Filename where the output will be written. Overrides --" + OUTPUT_DIR_ARG + ".")
              .required(false)
              .get());
      options.addOption(
          Option.builder()
              .longOpt(EXCLUDE_IDS_OPTION)
              .hasArg(false)
              .desc("Don't include feature IDs.")
              .required(false)
              .get());
      options.addOption(
          Option.builder()
              .longOpt(SORT_FEATURES_OPTION)
              .hasArg(true)
              .optionalArg(true)
              .argName("pattern")
              .desc(
                  "Reorder features of matching layers (default all) by ID, for optimal encoding of ID values.")
              .required(false)
              .get());
      options.addOption(
          Option.builder()
              .longOpt(REGEN_IDS_OPTION)
              .hasArg(true)
              .optionalArg(true)
              .argName("pattern")
              .desc(
                  "Re-generate ID values of matching layers (default all).  Sequential values are assigned for optimal encoding, when ID values have no special meaning.")
              .required(false)
              .get());
      options.addOption(
          Option.builder()
              .longOpt(FILTER_LAYERS_OPTION)
              .hasArg(true)
              .desc("Filter layers by regex")
              .required(false)
              .get());
      options.addOption(
          Option.builder()
              .longOpt(FILTER_LAYERS_INVERT_OPTION)
              .hasArg(false)
              .desc("Invert the result of --" + FILTER_LAYERS_OPTION)
              .required(false)
              .get());
      options.addOption(
          Option.builder()
              .longOpt(MIN_ZOOM_OPTION)
              .hasArg(true)
              .optionalArg(false)
              .argName("level")
              .desc(
                  "Minimum zoom level to encode tiles.  Only applies with --"
                      + INPUT_MBTILES_ARG
                      + " or --"
                      + INPUT_PMTILES_ARG)
              .required(false)
              .converter(Converter.NUMBER)
              .get());
      options.addOption(
          Option.builder()
              .longOpt(MAX_ZOOM_OPTION)
              .hasArg(true)
              .optionalArg(false)
              .argName("level")
              .desc(
                  "Maximum zoom level to encode tiles.  Only applies with --"
                      + INPUT_MBTILES_ARG
                      + " or --"
                      + INPUT_PMTILES_ARG)
              .required(false)
              .converter(Converter.NUMBER)
              .get());
      options.addOption(
          Option.builder()
              .longOpt(INCLUDE_METADATA_OPTION)
              .hasArg(false)
              .deprecated()
              .desc(
                  "Write tile metadata alongside the output tile (adding '.meta'). "
                      + "Only applies with --"
                      + INPUT_TILE_ARG
                      + ".")
              .required(false)
              .get());
      options.addOption(
          Option.builder()
              .longOpt(ALLOW_COERCE_OPTION)
              .hasArg(false)
              .desc("Allow coercion of property values")
              .required(false)
              .get());
      options.addOption(
          Option.builder()
              .longOpt(ALLOW_ELISION_OPTION)
              .hasArg(false)
              .desc("Allow elision of mismatched property types")
              .required(false)
              .get());
      options.addOption(
          Option.builder()
              .longOpt(FASTPFOR_ENCODING_OPTION)
              .hasArg(false)
              .desc("Enable FastPFOR encodings of integer columns")
              .required(false)
              .get());
      options.addOption(
          Option.builder()
              .longOpt(FSST_ENCODING_OPTION)
              .hasArg(false)
              .desc("Enable FSST encodings of string columns (Java implementation)")
              .required(false)
              .get());
      options.addOption(
          Option.builder()
              .longOpt(FSST_NATIVE_ENCODING_OPTION)
              .hasArg(false)
              .desc(
                  "Enable FSST encodings of string columns (Native implementation: "
                      + (FsstJni.isLoaded() ? "" : "Not ")
                      + " available)")
              .required(false)
              .get());
      options.addOption(
          Option.builder()
              .longOpt(COLUMN_MAPPING_AUTO_OPTION)
              .hasArg(true)
              .optionalArg(false)
              .argName("columns")
              .valueSeparator(',')
              .desc(
"""
Automatic column mapping for the specified layers (Not implemented)
  Layer specifications may be regular expressions if surrounded by '/'.
  An empty set of layers applies to all layers.""")
              .required(false)
              .get());
      options.addOption(
          Option.builder()
              .longOpt(COLUMN_MAPPING_DELIM_OPTION)
              .hasArg(true)
              .optionalArg(false)
              .argName("map")
              .desc(
"""
Add a separator-based column mapping:
  '[<layer>,...]<name><separator>'
  e.g. '[][:]name' combines 'name' and 'name:de' on all layers.""")
              .required(false)
              .get());
      options.addOption(
          Option.builder()
              .longOpt(COLUMN_MAPPING_LIST_OPTION)
              .hasArg(true)
              .optionalArg(false)
              .argName("map")
              .desc(
"""
Add an explicit column mapping on the specified layers:
  '[<layer>,...]<name>,...'
  e.g. '[]name,name:de' combines 'name' and 'name:de' on all layers.""")
              .required(false)
              .get());
      options.addOption(
          Option.builder()
              .longOpt(NO_MORTON_OPTION)
              .hasArg(false)
              .desc("Disable Morton encoding.")
              .required(false)
              .get());
      options.addOption(
          Option.builder()
              .longOpt(PRE_TESSELLATE_OPTION)
              .hasArg(false)
              .desc("Include tessellation data in converted tiles.")
              .required(false)
              .get());
      options.addOption(
          Option.builder()
              .longOpt(TESSELLATE_URL_OPTION)
              .hasArg(true)
              .desc("Use a tessellation server (implies --" + PRE_TESSELLATE_OPTION + ").")
              .required(false)
              .get());
      options.addOption(
          Option.builder()
              .longOpt(OUTLINE_FEATURE_TABLES_OPTION)
              .hasArgs()
              .desc(
                  "The feature tables for which outlines are included "
                      + "([OPTIONAL], comma-separated, 'ALL' for all, default: none).")
              .valueSeparator(',')
              .argName("tables")
              .required(false)
              .optionalArg(false)
              .get());
      options.addOption(
          Option.builder()
              .longOpt(DECODE_OPTION)
              .hasArg(false)
              .desc(
                  "Test decoding the tile after encoding it. "
                      + "Only applies with --"
                      + INPUT_TILE_ARG
                      + ".")
              .required(false)
              .get());
      options.addOption(
          Option.builder()
              .longOpt(PRINT_MLT_OPTION)
              .hasArg(false)
              .desc(
                  "Print the MLT tile to stdout after encoding it. "
                      + "Only applies with --"
                      + INPUT_TILE_ARG
                      + ".")
              .required(false)
              .get());
      options.addOption(
          Option.builder()
              .longOpt(PRINT_MVT_OPTION)
              .hasArg(false)
              .desc(
                  "Print the round-tripped MVT tile to stdout. "
                      + "Only applies with --"
                      + INPUT_TILE_ARG
                      + ".")
              .required(false)
              .get());
      options.addOption(
          Option.builder()
              .longOpt(COMPARE_GEOM_OPTION)
              .hasArg(false)
              .desc(
                  "Assert that geometry in the decoded tile is the same as the input tile. "
                      + "Only applies with --"
                      + INPUT_TILE_ARG
                      + ".")
              .required(false)
              .get());
      options.addOption(
          Option.builder()
              .longOpt(COMPARE_PROP_OPTION)
              .hasArg(false)
              .desc(
                  "Assert that properties in the decoded tile is the same as the input tile. "
                      + "Only applies with --"
                      + INPUT_TILE_ARG
                      + ".")
              .required(false)
              .get());
      options.addOption(
          Option.builder()
              .longOpt(COMPARE_ALL_OPTION)
              .hasArg(false)
              .desc("Equivalent to --" + COMPARE_GEOM_OPTION + " --" + COMPARE_PROP_OPTION + ".")
              .required(false)
              .get());
      options.addOption(
          Option.builder()
              .longOpt(VECTORIZED_OPTION)
              .hasArg(false)
              .desc("Use the vectorized decoding path.")
              .required(false)
              .deprecated()
              .get());
      options.addOption(
          Option.builder()
              .longOpt(DUMP_STREAMS_OPTION)
              .hasArg(false)
              .desc(
                  "Dump the raw contents of the individual streams. "
                      + "Only applies with --"
                      + INPUT_TILE_ARG
                      + ".")
              .required(false)
              .get());
      options.addOption(
          Option.builder()
              .longOpt(TIMER_OPTION)
              .hasArg(false)
              .desc("Print the time it takes, in ms, to decode a tile.")
              .required(false)
              .get());
      options.addOption(
          Option.builder()
              .longOpt(COMPRESS_OPTION)
              .hasArg(true)
              .argName("algorithm")
              .desc(
                  "Compress tile data with one of 'deflate', 'gzip', or 'none'. "
                      + "Only applies with --"
                      + INPUT_MBTILES_ARG
                      + ", --"
                      + INPUT_PMTILES_ARG
                      + " or --"
                      + INPUT_OFFLINEDB_ARG
                      + "."
                      + " Default: none for MBTiles and offline database, keep existing for PMTiles.")
              .required(false)
              .get());
      options.addOption(
          Option.builder()
              .option("j")
              .longOpt(PARALLEL_OPTION)
              .hasArg(true)
              .optionalArg(true)
              .argName("threads")
              .desc(
                  "Enable parallel encoding of tiles.  Only applies with --"
                      + INPUT_MBTILES_ARG
                      + ", --"
                      + INPUT_OFFLINEDB_ARG
                      + ", or --"
                      + INPUT_PMTILES_ARG)
              .required(false)
              .converter(Converter.NUMBER)
              .get());
      options.addOption(
          Option.builder()
              .longOpt(CONTINUE_OPTION)
              .hasArg(false)
              .desc("Continue on error when possible")
              .required(false)
              .get());
      options.addOption(
          Option.builder()
              .option("v")
              .longOpt(VERBOSE_OPTION)
              .hasArg(true)
              .optionalArg(true)
              .argName("level")
              .desc(
                  "Select output verbosity for status and information on stderr. "
                      + "Optionally specify a level: off, fatal, error, warn, info, debug, trace. "
                      + "Default is info, or debug if --"
                      + VERBOSE_OPTION
                      + " is specified without a level.")
              .required(false)
              .get());
      options.addOption(
          Option.builder()
              .option("h")
              .longOpt(HELP_OPTION)
              .hasArg(false)
              .desc("Show this output.")
              .required(false)
              .get());
      options.addOption(
          Option.builder()
              .longOpt(SERVER_ARG)
              .hasArg(true)
              .optionalArg(true)
              .argName("port")
              .desc("Start encoding server")
              .required(false)
              .get());

      var cmd = new DefaultParser().parse(options, args);

      var tessellateSource = cmd.getOptionValue(TESSELLATE_URL_OPTION, (String) null);
      if (tessellateSource != null) {
        // throw if it's not a valid URI
        var ignored = new URI(tessellateSource);
      }

      final var filterRegex = cmd.getOptionValue(FILTER_LAYERS_OPTION, (String) null);
      if (filterRegex != null) {
        // throw if it's not a valid regex
        var ignored = Pattern.compile(filterRegex);
      }

      if (cmd.hasOption(SERVER_ARG)) {
        return cmd;
      } else if (cmd.getOptions().length == 0 || cmd.hasOption(HELP_OPTION)) {
        printHelp(options);
      } else if (Stream.of(
                  cmd.hasOption(INPUT_TILE_ARG),
                  cmd.hasOption(INPUT_MBTILES_ARG),
                  cmd.hasOption(INPUT_OFFLINEDB_ARG),
                  cmd.hasOption(INPUT_PMTILES_ARG))
              .filter(x -> x)
              .count()
          != 1) {
        logger.error(
            "Exactly one of --{}, --{}, --{}, or --{} must be used",
            INPUT_TILE_ARG,
            INPUT_MBTILES_ARG,
            INPUT_OFFLINEDB_ARG,
            INPUT_PMTILES_ARG);
      } else if (cmd.hasOption(OUTPUT_FILE_ARG) && cmd.hasOption(OUTPUT_DIR_ARG)) {
        logger.error("Cannot specify both --{} and --{} options.", OUTPUT_FILE_ARG, OUTPUT_DIR_ARG);
      } else if (!validateCompression(cmd)) {
        logger.error(
            "Not a valid compression type: '{}'.  Valid options are: {}",
            cmd.getOptionValue(COMPRESS_OPTION),
            String.join(", ", getAllowedCompressions(cmd)));
      } else {
        return cmd;
      }
    } catch (IOException | ParseException ex) {
      logger.error("Failed to parse command line arguments", ex);
    } catch (URISyntaxException ex) {
      logger.error("Invalid tessellation URL", ex);
    }
    return null;
  }

  private static void printHelp(Options options) throws IOException {
    final var autoUsage = true;
    final var header =
        "Convert an MVT tile or a container file containing MVT tiles to MLT format.\n";
    final var footer =
        "\nExample usages:\n"
            + " Encode a single tile:\n"
            + "    encode --mvt input.mvt --mlt output.mlt\n"
            + " Encode all tiles in an MBTiles file, with compression and parallel encoding:\n"
            + "    encode --mbtiles input.mbtiles --dir output_dir --compress gzip -j\n"
            + " Start an encoding server on port 8080:\n"
            + "    encode --server 8080\n"
            + "\nEnvironment variables:\n"
            + "  MLT_TILE_LOG_INTERVAL: Number of tiles between status log messages (default: 10000)\n"
            + "  MLT_COMPRESSION_RATIO_THRESHOLD: Minimum compression ratio to apply compression (default: "
            + ConversionHelper.DEFAULT_COMPRESSION_RATIO_THRESHOLD
            + ").\n"
            + "  MLT_COMPRESSION_FIXED_THRESHOLD: Minimum size in bytes for a tile to be compressed (default: "
            + ConversionHelper.DEFAULT_COMPRESSION_FIXED_THRESHOLD
            + ")\n";

    final var target = new TextHelpAppendable(System.err);

    final var widthStr = System.getenv("COLUMNS");
    if (widthStr != null) {
      try {
        target.getTextStyleBuilder().setMaxWidth(Integer.parseInt(widthStr));
      } catch (NumberFormatException ignore) {
      }
    }

    final var formatter =
        HelpFormatter.builder().setShowSince(false).setHelpAppendable(target).get();
    formatter.printHelp(Encode.class.getName(), header, options, null, autoUsage);

    // Passing this as the footer to `printHelp` causes it to be formatted incorrectly.
    target.append(footer);
  }

  // matches a layer discriminator, [] = all, [regex] = match a regex
  private static final String layerPatternPrefix = "^(?:\\[]|\\[([^]].+)])?";
  // A layer discriminator followed by prefix and delimiter.
  // Delimiter may be a pattern if surrounded by slashes.
  private static final Pattern colMapSeparatorPattern1 =
      Pattern.compile(layerPatternPrefix + "(.+)(/.+/)$");
  private static final Pattern colMapSeparatorPattern2 =
      Pattern.compile(layerPatternPrefix + "(.+)(.)$");
  // a layer discriminator followed by a comma-separated list of columns
  private static final Pattern colMapListPattern = Pattern.compile(layerPatternPrefix + "(.+)$");
  private static final Pattern colMapPatternPattern = Pattern.compile("^/(.*)/$");
  private static final Pattern colMapMatchAll = Pattern.compile(".*");

  public static ColumnMappingConfig getColumnMappings(CommandLine cmd) {
    final var result = new ColumnMappingConfig();
    if (cmd.hasOption(EncodeCommandLine.COLUMN_MAPPING_AUTO_OPTION)) {
      throw new NotImplementedException("Auto column mappings are not implemented yet");
    }
    if (cmd.hasOption(EncodeCommandLine.COLUMN_MAPPING_LIST_OPTION)) {
      final var strings = cmd.getOptionValues(EncodeCommandLine.COLUMN_MAPPING_LIST_OPTION);
      if (strings != null) {
        for (var item : strings) {
          var matcher = colMapListPattern.matcher(item);
          if (matcher.matches()) {
            // matcher doesn't support multiple group matches, split them separately
            final var layers = parseLayerPatterns(matcher.group(1));
            final var list = matcher.group(2);
            final var columnNames =
                Arrays.stream(list.split(","))
                    .map(String::trim)
                    .filter(s -> !s.isEmpty())
                    .collect(Collectors.toList());
            result.merge(
                layers,
                List.of(new ColumnMapping(columnNames, true)),
                (oldList, newList) ->
                    Stream.of(oldList, newList).flatMap(Collection::stream).toList());
          } else {
            logger.warn(
                "Invalid column mapping ignored: '{}'. Expected pattern is: {}",
                item,
                colMapListPattern);
          }
        }
      }
    }
    final var strings = cmd.getOptionValues(EncodeCommandLine.COLUMN_MAPPING_DELIM_OPTION);
    if (strings != null) {
      for (var item : strings) {
        var matcher = colMapSeparatorPattern1.matcher(item);
        if (!matcher.matches()) {
          matcher = colMapSeparatorPattern2.matcher(item);
        }
        if (matcher.matches()) {
          // matcher doesn't support multiple group matches, split them separately
          final var layers = parseLayerPatterns(matcher.group(1));
          final var prefix = parsePattern(matcher.group(2));
          final var delimiter = parsePattern(matcher.group(3));
          result.merge(
              layers,
              List.of(new ColumnMapping(prefix, delimiter, true)),
              (oldList, newList) ->
                  Stream.of(oldList, newList).flatMap(Collection::stream).toList());
        } else {
          logger.warn(
              "Invalid column mapping ignored: '{}'. Expected pattern is: {} or {}",
              item,
              colMapSeparatorPattern1,
              colMapSeparatorPattern2);
        }
      }
    }
    return result;
  }

  private static Pattern parseLayerPatterns(String pattern) {
    return StringUtils.isBlank(pattern) ? colMapMatchAll : parsePattern(pattern);
  }

  private static Pattern parsePattern(String pattern) {
    final var matcher = colMapPatternPattern.matcher(pattern);
    try {
      if (matcher.matches()) {
        // A regex surrounded by slashes, return the compiled pattern
        return Pattern.compile(matcher.group(1));
      } else {
        // Just a regular string
        return Pattern.compile(pattern, Pattern.LITERAL);
      }
    } catch (PatternSyntaxException ex) {
      logger.error("Invalid regex pattern: '{}'.  Matching all input.", pattern, ex);
      return colMapMatchAll;
    }
  }

  private static final Logger logger = LoggerFactory.getLogger(EncodeCommandLine.class);
}
