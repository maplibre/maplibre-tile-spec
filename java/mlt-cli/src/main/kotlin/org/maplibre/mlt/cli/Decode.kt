package org.maplibre.mlt.cli

import org.apache.commons.cli.CommandLine
import org.apache.commons.cli.DefaultParser
import org.apache.commons.cli.Option
import org.apache.commons.cli.Options
import org.apache.commons.cli.ParseException
import org.apache.commons.cli.help.HelpFormatter
import org.maplibre.mlt.data.MapLibreTile
import org.maplibre.mlt.decoder.MltDecoder
import org.slf4j.Logger
import org.slf4j.LoggerFactory
import java.nio.charset.StandardCharsets
import java.nio.file.Files
import java.nio.file.Paths

object Decode {
    private const val FILE_NAME_ARG = "mlt"
    private const val PRINT_MLT_OPTION = "printmlt"
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
                .hasArg(true)
                .optionalArg(true)
                .argName("count")
                .desc("Print the time it takes, in ms, to decode a tile <count> times ([OPTIONAL], default 1)")
                .required(false)
                .get(),
        )

        if (args.isEmpty()) {
            showHelp(options)
            return
        }
        try {
            val cmd = DefaultParser().parse(options, args)
            run(cmd)
            System.out.println("x")
        } catch (ex: ParseException) {
            System.err.println(ex.message)
            showHelp(options)
        } catch (ex: Exception) {
            System.err.println("Failed: " + ex.message)
            ex.printStackTrace(System.err)
        }
    }

    private fun run(cmd: CommandLine) {
        val fileName = cmd.getOptionValue(FILE_NAME_ARG)
        if (fileName == null || fileName.isEmpty()) {
            throw ParseException("Missing required argument: " + FILE_NAME_ARG)
        }
        val willPrintMLT = cmd.hasOption(PRINT_MLT_OPTION)
        val willTime = cmd.hasOption(TIMER_OPTION)
        val decodeIterations = cmd.getOptionValue(TIMER_OPTION, "1").toInt().coerceAtLeast(1)
        val inputTilePath = Paths.get(fileName)
        require(Files.exists(inputTilePath)) { "Input mlt tile path does not exist: " + inputTilePath }
        val mltTileBuffer = Files.readAllBytes(inputTilePath)

        val timer = Timer()
        var decodedTile: MapLibreTile? = null
        for (i in 0 until decodeIterations) {
            decodedTile = MltDecoder.decodeMlTile(mltTileBuffer)
        }
        if (willTime) timer.stop("decoding")
        if (willPrintMLT && decodedTile != null) {
            System.out.write(decodedTile.toJson().toByteArray(StandardCharsets.UTF_8))
        }
    }

    private fun showHelp(options: Options) {
        HelpFormatter
            .builder()
            .setShowSince(false)
            .get()
            .printHelp(Decode::class.java.name, "", options, null, true)
    }

    private val logger: Logger = LoggerFactory.getLogger(Decode::class.java)
}
