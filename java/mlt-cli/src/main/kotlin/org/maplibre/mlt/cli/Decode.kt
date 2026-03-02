package org.maplibre.mlt.cli

import org.apache.commons.cli.CommandLineParser
import org.apache.commons.cli.DefaultParser
import org.apache.commons.cli.Option
import org.apache.commons.cli.Options
import org.apache.commons.cli.ParseException
import org.apache.commons.lang3.NotImplementedException
import org.maplibre.mlt.decoder.MltDecoder
import org.slf4j.Logger
import org.slf4j.LoggerFactory
import java.io.IOException
import java.nio.charset.StandardCharsets
import java.nio.file.Files
import java.nio.file.Paths

object Decode {
    private const val FILE_NAME_ARG = "mlt"
    private const val PRINT_MLT_OPTION = "printmlt"
    private const val VECTORIZED_OPTION = "vectorized"
    private const val TIMER_OPTION = "timer"

    @JvmStatic
    fun main(args: Array<String>) {
        try {
            run(args)
        } catch (ex: Exception) {
            logger.error("Decoding failed", ex)
            System.exit(1)
        }
    }

    @Throws(ParseException::class, IOException::class)
    private fun run(args: Array<String>) {
        val options = Options()
        options.addOption(
            Option
                .builder()
                .longOpt(FILE_NAME_ARG)
                .hasArg(true)
                .desc("Path to the input MLT file to read ([REQUIRED])")
                .required(true)
                .get(),
        )
        options.addOption(
            Option
                .builder()
                .longOpt(VECTORIZED_OPTION)
                .hasArg(false)
                .desc(
                    "Use the vectorized decoding path ([OPTIONAL], default: will use non-vectorized path)",
                ).required(false)
                .get(),
        )
        options.addOption(
            Option
                .builder()
                .longOpt(PRINT_MLT_OPTION)
                .hasArg(false)
                .desc("Print the MLT tile after encoding it ([OPTIONAL], default: false)")
                .required(false)
                .get(),
        )
        options.addOption(
            Option
                .builder()
                .longOpt(TIMER_OPTION)
                .hasArg(false)
                .desc("Print the time it takes, in ms, to decode a tile ([OPTIONAL])")
                .required(false)
                .get(),
        )
        val parser: CommandLineParser = DefaultParser()
        val cmd = parser.parse(options, args)
        val fileName = cmd.getOptionValue(FILE_NAME_ARG)
        if (fileName == null || fileName.isEmpty()) {
            throw ParseException("Missing required argument: " + FILE_NAME_ARG)
        }
        val willPrintMLT = cmd.hasOption(PRINT_MLT_OPTION)
        val willUseVectorized = cmd.hasOption(VECTORIZED_OPTION)
        val willTime = cmd.hasOption(TIMER_OPTION)
        val inputTilePath = Paths.get(fileName)
        require(Files.exists(inputTilePath)) { "Input mlt tile path does not exist: " + inputTilePath }
        val mltTileBuffer = Files.readAllBytes(inputTilePath)

        val timer = Timer()
        if (willUseVectorized) {
            throw NotImplementedException("Vectorized decoding is not available")
        } else {
            val decodedTile = MltDecoder.decodeMlTile(mltTileBuffer)
            if (willTime) timer.stop("decoding")
            if (willPrintMLT) {
                System.out.write(decodedTile.toJson().toByteArray(StandardCharsets.UTF_8))
            }
        }
    }

    private val logger: Logger = LoggerFactory.getLogger(Decode::class.java)
}
