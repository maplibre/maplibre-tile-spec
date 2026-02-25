package org.maplibre.mlt.cli

import org.apache.commons.cli.CommandLine
import org.apache.commons.cli.Converter
import org.apache.commons.cli.DefaultParser
import org.apache.commons.cli.Option
import org.apache.commons.cli.Options
import org.apache.commons.cli.ParseException
import org.apache.commons.cli.help.HelpFormatter
import org.apache.commons.cli.help.TextHelpAppendable
import org.apache.commons.lang3.NotImplementedException
import org.apache.commons.lang3.StringUtils
import org.maplibre.mlt.converter.encodings.fsst.FsstJni
import org.maplibre.mlt.converter.mvt.ColumnMapping
import org.maplibre.mlt.converter.mvt.ColumnMappingConfig
import org.slf4j.Logger
import org.slf4j.LoggerFactory
import java.io.IOException
import java.net.URI
import java.net.URISyntaxException
import java.util.Arrays
import java.util.regex.Pattern
import java.util.regex.PatternSyntaxException
import java.util.stream.Collectors
import java.util.stream.Stream
import kotlin.Array
import kotlin.Boolean
import kotlin.NumberFormatException
import kotlin.Throws

internal object EncodeCommandLine {
    const val INPUT_TILE_ARG = "mvt"
    const val INPUT_MBTILES_ARG = "mbtiles"
    const val INPUT_OFFLINEDB_ARG = "offlinedb"
    const val INPUT_PMTILES_ARG = "pmtiles"
    const val OUTPUT_DIR_ARG = "dir"
    const val OUTPUT_FILE_ARG = "mlt"
    const val EXCLUDE_IDS_OPTION = "noids"
    const val SORT_FEATURES_OPTION = "sort-ids"
    const val REGEN_IDS_OPTION = "regen-ids"
    const val FILTER_LAYERS_OPTION = "filter-layers"
    const val MIN_ZOOM_OPTION = "minzoom"
    const val MAX_ZOOM_OPTION = "maxzoom"
    const val FILTER_LAYERS_INVERT_OPTION = "filter-layers-invert"
    const val INCLUDE_METADATA_OPTION = "metadata"
    const val ALLOW_COERCE_OPTION = "coerce-mismatch"
    const val ALLOW_ELISION_OPTION = "elide-mismatch"
    const val FASTPFOR_ENCODING_OPTION = "enable-fastpfor"
    const val FSST_ENCODING_OPTION = "enable-fsst"
    const val FSST_NATIVE_ENCODING_OPTION = "enable-fsst-native"
    const val COLUMN_MAPPING_AUTO_OPTION = "colmap-auto"
    const val COLUMN_MAPPING_DELIM_OPTION = "colmap-delim"
    const val COLUMN_MAPPING_LIST_OPTION = "colmap-list"
    const val NO_MORTON_OPTION = "nomorton"
    const val PRE_TESSELLATE_OPTION = "tessellate"
    const val TESSELLATE_URL_OPTION = "tessellateurl"
    const val OUTLINE_FEATURE_TABLES_OPTION = "outlines"
    const val DECODE_OPTION = "decode"
    const val PRINT_MLT_OPTION = "printmlt"
    const val PRINT_MVT_OPTION = "printmvt"
    const val COMPARE_ALL_OPTION = "compare-all"
    const val COMPARE_GEOM_OPTION = "compare-geometry"
    const val COMPARE_PROP_OPTION = "compare-properties"
    const val VECTORIZED_OPTION = "vectorized"
    const val TIMER_OPTION = "timer"
    const val COMPRESS_OPTION = "compress"
    private const val COMPRESS_OPTION_DEFLATE = "deflate"
    private const val COMPRESS_OPTION_GZIP = "gzip"
    private const val COMPRESS_OPTION_NONE = "none"
    private const val COMPRESS_OPTION_BROTLI = "brotli"
    private const val COMPRESS_OPTION_ZSTD = "zstd"
    const val DUMP_STREAMS_OPTION = "rawstreams"
    const val PARALLEL_OPTION = "parallel"
    const val VERBOSE_OPTION = "verbose"
    const val CONTINUE_OPTION = "continue"
    const val HELP_OPTION = "help"
    const val SERVER_ARG = "server"

    private fun getAllowedCompressions(cmd: CommandLine): Stream<String> {
        val extra = if (cmd.hasOption(INPUT_PMTILES_ARG)) Stream.of(COMPRESS_OPTION_BROTLI, COMPRESS_OPTION_ZSTD) else Stream.of()
        return Stream.concat(
            Stream.of(
                COMPRESS_OPTION_GZIP,
                COMPRESS_OPTION_DEFLATE,
                COMPRESS_OPTION_NONE,
            ),
            extra,
        )
    }

    private fun validateCompression(
        cmd: CommandLine,
        option: String?,
    ): Boolean = getAllowedCompressions(cmd).anyMatch { x -> x == option }

    @Throws(IOException::class)
    @JvmStatic
    fun getCommandLine(args: Array<String>): CommandLine? {
        try {
            val options = Options()
            options.addOption(
                Option
                    .builder()
                    .longOpt(INPUT_TILE_ARG)
                    .hasArgs()
                    .argName("file")
                    .desc("Path to the input MVT file")
                    .required(false)
                    .get(),
            )
            options.addOption(
                Option
                    .builder()
                    .longOpt(INPUT_MBTILES_ARG)
                    .hasArg(true)
                    .argName("file")
                    .desc("Path of the input MBTiles file.")
                    .required(false)
                    .get(),
            )
            options.addOption(
                Option
                    .builder()
                    .longOpt(INPUT_PMTILES_ARG)
                    .hasArg(true)
                    .argName("fileOrUri")
                    .desc("Location of the input PMTiles file.")
                    .required(false)
                    .get(),
            )
            options.addOption(
                Option
                    .builder()
                    .longOpt(INPUT_OFFLINEDB_ARG)
                    .hasArg(true)
                    .argName("file")
                    .desc("Path of the input offline database file.")
                    .required(false)
                    .get(),
            )
            options.addOption(
                Option
                    .builder()
                    .longOpt(OUTPUT_DIR_ARG)
                    .hasArg(true)
                    .argName("dir")
                    .desc(
                        "Directory where the output is written, using the input file basename (OPTIONAL).",
                    ).required(false)
                    .get(),
            )
            options.addOption(
                Option
                    .builder()
                    .longOpt(OUTPUT_FILE_ARG)
                    .hasArg(true)
                    .argName("file")
                    .desc(
                        "Filename where the output will be written. Overrides --" + OUTPUT_DIR_ARG + ".",
                    ).required(false)
                    .get(),
            )
            options.addOption(
                Option
                    .builder()
                    .longOpt(EXCLUDE_IDS_OPTION)
                    .hasArg(false)
                    .desc("Don't include feature IDs.")
                    .required(false)
                    .get(),
            )
            options.addOption(
                Option
                    .builder()
                    .longOpt(SORT_FEATURES_OPTION)
                    .hasArg(true)
                    .optionalArg(true)
                    .argName("pattern")
                    .desc(
                        "Reorder features of matching layers (default all) by ID, for optimal encoding of ID values.",
                    ).required(false)
                    .get(),
            )
            options.addOption(
                Option
                    .builder()
                    .longOpt(REGEN_IDS_OPTION)
                    .hasArg(true)
                    .optionalArg(true)
                    .argName("pattern")
                    .desc(
                        "Re-generate ID values of matching layers (default all).  Sequential values are assigned for optimal encoding, when ID values have no special meaning.",
                    ).required(false)
                    .get(),
            )
            options.addOption(
                Option
                    .builder()
                    .longOpt(FILTER_LAYERS_OPTION)
                    .hasArg(true)
                    .desc("Filter layers by regex")
                    .required(false)
                    .get(),
            )
            options.addOption(
                Option
                    .builder()
                    .longOpt(FILTER_LAYERS_INVERT_OPTION)
                    .hasArg(false)
                    .desc("Invert the result of --" + FILTER_LAYERS_OPTION)
                    .required(false)
                    .get(),
            )
            options.addOption(
                Option
                    .builder()
                    .longOpt(MIN_ZOOM_OPTION)
                    .hasArg(true)
                    .optionalArg(false)
                    .argName("level")
                    .desc(
                        (
                            "Minimum zoom level to encode tiles.  Only applies with --" +
                                INPUT_MBTILES_ARG +
                                " or --" +
                                INPUT_PMTILES_ARG
                        ),
                    ).required(false)
                    .converter(Converter.NUMBER)
                    .get(),
            )
            options.addOption(
                Option
                    .builder()
                    .longOpt(MAX_ZOOM_OPTION)
                    .hasArg(true)
                    .optionalArg(false)
                    .argName("level")
                    .desc(
                        (
                            "Maximum zoom level to encode tiles.  Only applies with --" +
                                INPUT_MBTILES_ARG +
                                " or --" +
                                INPUT_PMTILES_ARG
                        ),
                    ).required(false)
                    .converter(Converter.NUMBER)
                    .get(),
            )
            options.addOption(
                Option
                    .builder()
                    .longOpt(INCLUDE_METADATA_OPTION)
                    .hasArg(false)
                    .deprecated()
                    .desc(
                        (
                            "Write tile metadata alongside the output tile (adding '.meta'). " +
                                "Only applies with --" +
                                INPUT_TILE_ARG +
                                "."
                        ),
                    ).required(false)
                    .get(),
            )
            options.addOption(
                Option
                    .builder()
                    .longOpt(ALLOW_COERCE_OPTION)
                    .hasArg(false)
                    .desc("Allow coercion of property values")
                    .required(false)
                    .get(),
            )
            options.addOption(
                Option
                    .builder()
                    .longOpt(ALLOW_ELISION_OPTION)
                    .hasArg(false)
                    .desc("Allow elision of mismatched property types")
                    .required(false)
                    .get(),
            )
            options.addOption(
                Option
                    .builder()
                    .longOpt(FASTPFOR_ENCODING_OPTION)
                    .hasArg(false)
                    .desc("Enable FastPFOR encodings of integer columns")
                    .required(false)
                    .get(),
            )
            options.addOption(
                Option
                    .builder()
                    .longOpt(FSST_ENCODING_OPTION)
                    .hasArg(false)
                    .desc("Enable FSST encodings of string columns (Java implementation)")
                    .required(false)
                    .get(),
            )
            options.addOption(
                Option
                    .builder()
                    .longOpt(FSST_NATIVE_ENCODING_OPTION)
                    .hasArg(false)
                    .desc(
                        (
                            "Enable FSST encodings of string columns (Native implementation: " +
                                (if (FsstJni.isLoaded()) "" else "Not ") +
                                " available)"
                        ),
                    ).required(false)
                    .get(),
            )
            options.addOption(
                Option
                    .builder()
                    .longOpt(COLUMN_MAPPING_AUTO_OPTION)
                    .hasArg(true)
                    .optionalArg(false)
                    .argName("columns")
                    .valueSeparator(',')
                    .desc(
                        """
Automatic column mapping for the specified layers (Not implemented)
  Layer specifications may be regular expressions if surrounded by '/'.
  An empty set of layers applies to all layers.
                        """.trimIndent(),
                    ).required(false)
                    .get(),
            )
            options.addOption(
                Option
                    .builder()
                    .longOpt(COLUMN_MAPPING_DELIM_OPTION)
                    .hasArg(true)
                    .optionalArg(false)
                    .argName("map")
                    .desc(
                        """
Add a separator-based column mapping:
  '[<layer>,...]<name><separator>'
  e.g. '[][:]name' combines 'name' and 'name:de' on all layers.
                        """.trimIndent(),
                    ).required(false)
                    .get(),
            )
            options.addOption(
                Option
                    .builder()
                    .longOpt(COLUMN_MAPPING_LIST_OPTION)
                    .hasArg(true)
                    .optionalArg(false)
                    .argName("map")
                    .desc(
                        """
Add an explicit column mapping on the specified layers:
  '[<layer>,...]<name>,...'
  e.g. '[]name,name:de' combines 'name' and 'name:de' on all layers.
                        """.trimIndent(),
                    ).required(false)
                    .get(),
            )
            options.addOption(
                Option
                    .builder()
                    .longOpt(NO_MORTON_OPTION)
                    .hasArg(false)
                    .desc("Disable Morton encoding.")
                    .required(false)
                    .get(),
            )
            options.addOption(
                Option
                    .builder()
                    .longOpt(PRE_TESSELLATE_OPTION)
                    .hasArg(false)
                    .desc("Include tessellation data in converted tiles.")
                    .required(false)
                    .get(),
            )
            options.addOption(
                Option
                    .builder()
                    .longOpt(TESSELLATE_URL_OPTION)
                    .hasArg(true)
                    .desc("Use a tessellation server (implies --" + PRE_TESSELLATE_OPTION + ").")
                    .required(false)
                    .get(),
            )
            options.addOption(
                Option
                    .builder()
                    .longOpt(OUTLINE_FEATURE_TABLES_OPTION)
                    .hasArgs()
                    .desc(
                        "The feature tables for which outlines are included " +
                            "([OPTIONAL], comma-separated, 'ALL' for all, default: none).",
                    ).valueSeparator(',')
                    .argName("tables")
                    .required(false)
                    .optionalArg(false)
                    .get(),
            )
            options.addOption(
                Option
                    .builder()
                    .longOpt(DECODE_OPTION)
                    .hasArg(false)
                    .desc(
                        (
                            "Test decoding the tile after encoding it. " +
                                "Only applies with --" +
                                INPUT_TILE_ARG +
                                "."
                        ),
                    ).required(false)
                    .get(),
            )
            options.addOption(
                Option
                    .builder()
                    .longOpt(PRINT_MLT_OPTION)
                    .hasArg(false)
                    .desc(
                        (
                            "Print the MLT tile to stdout after encoding it. " +
                                "Only applies with --" +
                                INPUT_TILE_ARG +
                                "."
                        ),
                    ).required(false)
                    .get(),
            )
            options.addOption(
                Option
                    .builder()
                    .longOpt(PRINT_MVT_OPTION)
                    .hasArg(false)
                    .desc(
                        (
                            "Print the round-tripped MVT tile to stdout. " +
                                "Only applies with --" +
                                INPUT_TILE_ARG +
                                "."
                        ),
                    ).required(false)
                    .get(),
            )
            options.addOption(
                Option
                    .builder()
                    .longOpt(COMPARE_GEOM_OPTION)
                    .hasArg(false)
                    .desc(
                        (
                            "Assert that geometry in the decoded tile is the same as the input tile. " +
                                "Only applies with --" +
                                INPUT_TILE_ARG +
                                "."
                        ),
                    ).required(false)
                    .get(),
            )
            options.addOption(
                Option
                    .builder()
                    .longOpt(COMPARE_PROP_OPTION)
                    .hasArg(false)
                    .desc(
                        (
                            "Assert that properties in the decoded tile is the same as the input tile. " +
                                "Only applies with --" +
                                INPUT_TILE_ARG +
                                "."
                        ),
                    ).required(false)
                    .get(),
            )
            options.addOption(
                Option
                    .builder()
                    .longOpt(COMPARE_ALL_OPTION)
                    .hasArg(false)
                    .desc("Equivalent to --" + COMPARE_GEOM_OPTION + " --" + COMPARE_PROP_OPTION + ".")
                    .required(false)
                    .get(),
            )
            options.addOption(
                Option
                    .builder()
                    .longOpt(VECTORIZED_OPTION)
                    .hasArg(false)
                    .desc("Use the vectorized decoding path.")
                    .required(false)
                    .deprecated()
                    .get(),
            )
            options.addOption(
                Option
                    .builder()
                    .longOpt(DUMP_STREAMS_OPTION)
                    .hasArg(false)
                    .desc(
                        (
                            "Dump the raw contents of the individual streams. " +
                                "Only applies with --" +
                                INPUT_TILE_ARG +
                                "."
                        ),
                    ).required(false)
                    .get(),
            )
            options.addOption(
                Option
                    .builder()
                    .longOpt(TIMER_OPTION)
                    .hasArg(false)
                    .desc("Print the time it takes, in ms, to decode a tile.")
                    .required(false)
                    .get(),
            )
            options.addOption(
                Option
                    .builder()
                    .longOpt(COMPRESS_OPTION)
                    .hasArg(true)
                    .argName("algorithm")
                    .desc(
                        (
                            "Compress tile data with one of 'deflate', 'gzip', or 'none'. " +
                                "Only applies with --" +
                                INPUT_MBTILES_ARG +
                                ", --" +
                                INPUT_PMTILES_ARG +
                                " or --" +
                                INPUT_OFFLINEDB_ARG +
                                "." +
                                " Default: none for MBTiles and offline database, keep existing for PMTiles."
                        ),
                    ).required(false)
                    .get(),
            )
            options.addOption(
                Option
                    .builder()
                    .option("j")
                    .longOpt(PARALLEL_OPTION)
                    .hasArg(true)
                    .optionalArg(true)
                    .argName("threads")
                    .desc(
                        (
                            "Enable parallel encoding of tiles.  Only applies with --" +
                                INPUT_MBTILES_ARG +
                                ", --" +
                                INPUT_OFFLINEDB_ARG +
                                ", or --" +
                                INPUT_PMTILES_ARG
                        ),
                    ).required(false)
                    .converter(Converter.NUMBER)
                    .get(),
            )
            options.addOption(
                Option
                    .builder()
                    .longOpt(CONTINUE_OPTION)
                    .hasArg(false)
                    .desc("Continue on error when possible")
                    .required(false)
                    .get(),
            )
            options.addOption(
                Option
                    .builder()
                    .option("v")
                    .longOpt(VERBOSE_OPTION)
                    .hasArg(true)
                    .optionalArg(true)
                    .argName("level")
                    .desc(
                        (
                            "Select output verbosity for status and information on stderr. " +
                                "Optionally specify a level: off, fatal, error, warn, info, debug, trace. " +
                                "Default is info, or debug if --" +
                                VERBOSE_OPTION +
                                " is specified without a level."
                        ),
                    ).required(false)
                    .get(),
            )
            options.addOption(
                Option
                    .builder()
                    .option("h")
                    .longOpt(HELP_OPTION)
                    .hasArg(false)
                    .desc("Show this output.")
                    .required(false)
                    .get(),
            )
            options.addOption(
                Option
                    .builder()
                    .longOpt(SERVER_ARG)
                    .hasArg(true)
                    .optionalArg(true)
                    .argName("port")
                    .desc("Start encoding server")
                    .required(false)
                    .get(),
            )

            val cmd = DefaultParser().parse(options, args)

            val tessellateSource = cmd.getOptionValue(TESSELLATE_URL_OPTION, null as String?)
            if (tessellateSource != null) {
                // throw if it's not a valid URI
                val ignored = URI(tessellateSource)
            }

            val filterRegex = cmd.getOptionValue(FILTER_LAYERS_OPTION, null as String?)
            if (filterRegex != null) {
                // throw if it's not a valid regex
                val ignored = Pattern.compile(filterRegex)
            }

            val compressOption = cmd.getOptionValue(COMPRESS_OPTION, COMPRESS_OPTION_NONE)

            if (cmd.hasOption(SERVER_ARG)) {
                return cmd
            } else if (cmd.getOptions().size == 0 || cmd.hasOption(HELP_OPTION)) {
                printHelp(options)
            } else if (Stream
                    .of<Boolean?>(
                        cmd.hasOption(INPUT_TILE_ARG),
                        cmd.hasOption(INPUT_MBTILES_ARG),
                        cmd.hasOption(INPUT_OFFLINEDB_ARG),
                        cmd.hasOption(INPUT_PMTILES_ARG),
                    ).filter { x: Boolean? -> x!! }
                    .count()
                != 1L
            ) {
                logger.error(
                    "Exactly one of --{}, --{}, --{}, or --{} must be used",
                    INPUT_TILE_ARG,
                    INPUT_MBTILES_ARG,
                    INPUT_OFFLINEDB_ARG,
                    INPUT_PMTILES_ARG,
                )
            } else if (cmd.hasOption(OUTPUT_FILE_ARG) && cmd.hasOption(OUTPUT_DIR_ARG)) {
                logger.error(
                    "Cannot specify both --{} and --{} options.",
                    OUTPUT_FILE_ARG,
                    OUTPUT_DIR_ARG,
                )
            } else if (!validateCompression(cmd, compressOption)) {
                logger.error(
                    "Not a valid compression type: '{}'.  Valid options are: {}",
                    compressOption,
                    getAllowedCompressions(cmd).collect(Collectors.joining(", ")),
                )
            } else {
                return cmd
            }
        } catch (ex: IOException) {
            logger.error("Failed to parse command line arguments", ex)
        } catch (ex: ParseException) {
            logger.error("Failed to parse command line arguments", ex)
        } catch (ex: URISyntaxException) {
            logger.error("Invalid tessellation URL", ex)
        }
        return null
    }

    @Throws(IOException::class)
    private fun printHelp(options: Options?) {
        val autoUsage = true
        val header =
            "Convert an MVT tile or a container file containing MVT tiles to MLT format.\n"
        val footer =
            (
                "\nExample usages:\n" +
                    " Encode a single tile:\n" +
                    "    encode --mvt input.mvt --mlt output.mlt\n" +
                    " Encode all tiles in an MBTiles file, with compression and parallel encoding:\n" +
                    "    encode --mbtiles input.mbtiles --dir output_dir --compress gzip -j\n" +
                    " Start an encoding server on port 8080:\n" +
                    "    encode --server 8080\n" +
                    "\nEnvironment variables:\n" +
                    "  MLT_TILE_LOG_INTERVAL: Number of tiles between status log messages (default: 10000)\n" +
                    "  MLT_COMPRESSION_RATIO_THRESHOLD: Minimum compression ratio to apply compression (default: " +
                    ConversionHelper.DEFAULT_COMPRESSION_RATIO_THRESHOLD +
                    ").\n" +
                    "  MLT_COMPRESSION_FIXED_THRESHOLD: Minimum size in bytes for a tile to be compressed (default: " +
                    ConversionHelper.DEFAULT_COMPRESSION_FIXED_THRESHOLD +
                    ")\n"
            )

        val target = TextHelpAppendable(System.err)

        val widthStr = System.getenv("COLUMNS")
        if (widthStr != null) {
            try {
                target.getTextStyleBuilder().setMaxWidth(widthStr.toInt())
            } catch (ignore: NumberFormatException) {
            }
        }

        val formatter =
            HelpFormatter
                .builder()
                .setShowSince(false)
                .setHelpAppendable(target)
                .get()
        formatter.printHelp(Encode::class.java.getName(), header, options, null, autoUsage)

        // Passing this as the footer to `printHelp` causes it to be formatted incorrectly.
        target.append(footer)
    }

    // matches a layer discriminator, [] = all, [regex] = match a regex
    private const val LAYER_PATTERN_PREFIX = "^(?:\\[]|\\[([^]].+)])?"

    // A layer discriminator followed by prefix and delimiter.
    // Delimiter may be a pattern if surrounded by slashes.
    private val colMapSeparatorPattern1: Pattern =
        Pattern.compile(LAYER_PATTERN_PREFIX + "(.+)(/.+/)$")
    private val colMapSeparatorPattern2: Pattern = Pattern.compile(LAYER_PATTERN_PREFIX + "(.+)(.)$")

    // a layer discriminator followed by a comma-separated list of columns
    private val colMapListPattern: Pattern = Pattern.compile(LAYER_PATTERN_PREFIX + "(.+)$")
    private val colMapPatternPattern: Pattern = Pattern.compile("^/(.*)/$")
    private val colMapMatchAll: Pattern = Pattern.compile(".*")

    @JvmStatic
    fun getColumnMappings(cmd: CommandLine): ColumnMappingConfig {
        val result = ColumnMappingConfig()
        if (cmd.hasOption(COLUMN_MAPPING_AUTO_OPTION)) {
            throw NotImplementedException("Auto column mappings are not implemented yet")
        }
        if (cmd.hasOption(COLUMN_MAPPING_LIST_OPTION)) {
            val strings = cmd.getOptionValues(COLUMN_MAPPING_LIST_OPTION)
            if (strings != null) {
                for (item in strings) {
                    val matcher = colMapListPattern.matcher(item)
                    if (matcher.matches()) {
                        // matcher doesn't support multiple group matches, split them separately
                        val layers = parseLayerPatterns(matcher.group(1))
                        val list = matcher.group(2)
                        val columnNames =
                            Arrays
                                .stream<kotlin.String>(
                                    list
                                        .split(",".toRegex())
                                        .dropLastWhile { it.isEmpty() }
                                        .toTypedArray(),
                                ).map<kotlin.String?> { obj: kotlin.String? -> obj!!.trim { it <= ' ' } }
                                .filter { s: kotlin.String? -> !s!!.isEmpty() }
                                .collect(Collectors.toList())
                        result.merge(
                            layers,
                            listOf(ColumnMapping(columnNames, true)),
                        ) { oldList: MutableList<ColumnMapping?>?, newList: MutableList<ColumnMapping?>? ->
                            Stream
                                .of<MutableList<ColumnMapping?>?>(
                                    oldList,
                                    newList,
                                ).flatMap<ColumnMapping?> { obj: MutableList<ColumnMapping?>? -> obj!!.stream() }
                                .toList()
                        }
                    } else {
                        logger.warn(
                            "Invalid column mapping ignored: '{}'. Expected pattern is: {}",
                            item,
                            colMapListPattern,
                        )
                    }
                }
            }
        }
        val strings = cmd.getOptionValues(COLUMN_MAPPING_DELIM_OPTION)
        if (strings != null) {
            for (item in strings) {
                var matcher = colMapSeparatorPattern1.matcher(item)
                if (!matcher.matches()) {
                    matcher = colMapSeparatorPattern2.matcher(item)
                }
                if (matcher.matches()) {
                    // matcher doesn't support multiple group matches, split them separately
                    val layers = parseLayerPatterns(matcher.group(1))
                    val prefix = parsePattern(matcher.group(2))
                    val delimiter = parsePattern(matcher.group(3))
                    result.merge(
                        layers,
                        listOf(ColumnMapping(prefix, delimiter, true)),
                    ) { oldList: MutableList<ColumnMapping?>?, newList: MutableList<ColumnMapping?>? ->
                        Stream
                            .of<MutableList<ColumnMapping?>?>(
                                oldList,
                                newList,
                            ).flatMap<ColumnMapping?> { obj: MutableList<ColumnMapping?>? -> obj!!.stream() }
                            .toList()
                    }
                } else {
                    logger.warn(
                        "Invalid column mapping ignored: '{}'. Expected pattern is: {} or {}",
                        item,
                        colMapSeparatorPattern1,
                        colMapSeparatorPattern2,
                    )
                }
            }
        }
        return result
    }

    private fun parseLayerPatterns(pattern: kotlin.String?): Pattern? =
        if (StringUtils.isBlank(pattern)) {
            colMapMatchAll
        } else {
            EncodeCommandLine.parsePattern(
                pattern!!,
            )
        }

    private fun parsePattern(pattern: kotlin.String): Pattern {
        val matcher = colMapPatternPattern.matcher(pattern)
        try {
            if (matcher.matches()) {
                // A regex surrounded by slashes, return the compiled pattern
                return Pattern.compile(matcher.group(1))
            } else {
                // Just a regular string
                return Pattern.compile(pattern, Pattern.LITERAL)
            }
        } catch (ex: PatternSyntaxException) {
            logger.error("Invalid regex pattern: '{}'.  Matching all input.", pattern, ex)
            return colMapMatchAll
        }
    }

    private val logger: Logger = LoggerFactory.getLogger(EncodeCommandLine::class.java)
}
