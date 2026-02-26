package org.maplibre.mlt.cli

import org.apache.commons.cli.CommandLine
import org.apache.commons.cli.ParseException
import org.apache.commons.io.FilenameUtils
import org.apache.logging.log4j.Level
import org.apache.logging.log4j.core.config.Configurator
import org.maplibre.mlt.cli.EncodeCommandLine.getColumnMappings
import org.maplibre.mlt.compare.CompareHelper
import org.maplibre.mlt.compare.CompareHelper.CompareMode
import org.maplibre.mlt.converter.ConversionConfig
import org.maplibre.mlt.converter.MLTStreamObserver
import org.maplibre.mlt.converter.MLTStreamObserverDefault
import org.maplibre.mlt.converter.MLTStreamObserverFile
import org.maplibre.mlt.converter.MltConverter
import org.maplibre.mlt.converter.encodings.fsst.FsstEncoder
import org.maplibre.mlt.converter.encodings.fsst.FsstJni
import org.maplibre.mlt.converter.mvt.MvtUtils
import org.maplibre.mlt.decoder.MltDecoder
import org.slf4j.Logger
import org.slf4j.LoggerFactory
import java.io.File
import java.io.IOException
import java.net.URI
import java.net.URISyntaxException
import java.nio.charset.StandardCharsets
import java.nio.file.Files
import java.nio.file.InvalidPathException
import java.nio.file.Path
import java.nio.file.Paths
import java.util.concurrent.LinkedBlockingQueue
import java.util.concurrent.ThreadPoolExecutor
import java.util.concurrent.TimeUnit
import java.util.regex.Pattern
import kotlin.math.max

object Encode {
    @JvmStatic
    fun main(args: Array<String>) {
        if (!run(args)) {
            System.exit(1)
        }
    }

    fun run(args: Array<String>): Boolean {
        try {
            val cmd = EncodeCommandLine.getCommandLine(args)
            if (cmd == null) {
                return false
            }

            if (cmd.hasOption(EncodeCommandLine.SERVER_ARG)) {
                val port = cmd.getOptionValue(EncodeCommandLine.SERVER_ARG, "3001").toInt()
                // never returns
                Server().run(port)
            }

            return run(cmd)
        } catch (ex: Exception) {
            logger.error("Failed", ex)
            return false
        }
    }

    @Throws(
        URISyntaxException::class,
        IOException::class,
        ClassNotFoundException::class,
        ParseException::class,
    )
    private fun run(cmd: CommandLine): Boolean {
        val tileFileNames = cmd.getOptionValues(EncodeCommandLine.INPUT_TILE_ARG)
        val sortFeaturesPattern =
            if (cmd.hasOption(EncodeCommandLine.SORT_FEATURES_OPTION)) {
                Pattern.compile(cmd.getOptionValue(EncodeCommandLine.SORT_FEATURES_OPTION, ".*"))
            } else {
                null
            }
        val regenIDsPattern =
            if (cmd.hasOption(EncodeCommandLine.REGEN_IDS_OPTION)) {
                Pattern.compile(cmd.getOptionValue(EncodeCommandLine.REGEN_IDS_OPTION, ".*"))
            } else {
                null
            }
        val outlineFeatureTables =
            cmd.getOptionValues(EncodeCommandLine.OUTLINE_FEATURE_TABLES_OPTION)
        val useFSSTJava = cmd.hasOption(EncodeCommandLine.FSST_ENCODING_OPTION)
        val useFSSTNative = cmd.hasOption(EncodeCommandLine.FSST_NATIVE_ENCODING_OPTION)
        val tessellateSource =
            cmd.getOptionValue(EncodeCommandLine.TESSELLATE_URL_OPTION, null as String?)
        val tessellateURI = if (tessellateSource != null) URI(tessellateSource) else null
        val tessellatePolygons =
            (tessellateSource != null) || cmd.hasOption(EncodeCommandLine.PRE_TESSELLATE_OPTION)
        val compressionType =
            cmd.getOptionValue(EncodeCommandLine.COMPRESS_OPTION, null as String?)
        val enableCoerceOnTypeMismatch = cmd.hasOption(EncodeCommandLine.ALLOW_COERCE_OPTION)
        val enableElideOnTypeMismatch = cmd.hasOption(EncodeCommandLine.ALLOW_ELISION_OPTION)
        val filterRegex =
            cmd.getOptionValue(EncodeCommandLine.FILTER_LAYERS_OPTION, null as String?)
        val filterPattern = if (filterRegex != null) Pattern.compile(filterRegex) else null
        val filterInvert = cmd.hasOption(EncodeCommandLine.FILTER_LAYERS_INVERT_OPTION)
        val columnMappings = getColumnMappings(cmd)
        val minZoom =
            cmd.getParsedOptionValue<Long?>(EncodeCommandLine.MIN_ZOOM_OPTION, 0L)!!.toInt()
        val maxZoom =
            cmd
                .getParsedOptionValue<Long?>(
                    EncodeCommandLine.MAX_ZOOM_OPTION,
                    Int.MAX_VALUE
                        .toLong(),
                )!!
                .toInt()

        val logLevel =
            if (cmd.hasOption(EncodeCommandLine.VERBOSE_OPTION)) {
                Level.toLevel(cmd.getOptionValue(EncodeCommandLine.VERBOSE_OPTION), Level.DEBUG)
            } else {
                Level.INFO
            }
        Configurator.setRootLevel(logLevel)

        // PMTiles logs stats at INFO.  Enable that only if the user has selected at least debug.
        // Note: `isLessSpecificThan` is actually less-than-or-equal.
        Configurator.setLevel(
            "com.onthegomap.planetiler.pmtiles.WriteablePmtiles",
            if (logLevel.isLessSpecificThan(Level.DEBUG)) Level.INFO else Level.OFF,
        )

        val threadCountOption =
            if (cmd.hasOption(EncodeCommandLine.PARALLEL_OPTION)) {
                cmd.getParsedOptionValue<Long?>(EncodeCommandLine.PARALLEL_OPTION, 0L)!!.toInt()
            } else {
                1
            }
        val threadCount =
            if (threadCountOption > 0) {
                threadCountOption
            } else {
                Runtime
                    .getRuntime()
                    .availableProcessors()
            }
        val taskRunner = createTaskRunner(threadCount)
        logger.debug("Using {} threads", max(1, taskRunner.threadCount))

        logger.debug(
            "Using column mappings: {}",
            if (columnMappings.isEmpty()) "none" else columnMappings.toString(),
        )

        var useFSST = false
        if (useFSSTNative) {
            if (FsstJni.isLoaded() && FsstEncoder.useNative(true)) {
                useFSST = true
            } else {
                logger.warn("Native FSST could not be loaded", FsstJni.getLoadError())
            }
        } else if (useFSSTJava) {
            logger.debug("Using Java FSST encoder")
            FsstEncoder.useNative(false)
            useFSST = true
        }

        val conversionConfig =
            ConversionConfig
                .builder()
                .includeIds(!cmd.hasOption(EncodeCommandLine.EXCLUDE_IDS_OPTION))
                .useFastPFOR(cmd.hasOption(EncodeCommandLine.FASTPFOR_ENCODING_OPTION))
                .useFSST(useFSST)
                .mismatchPolicy(enableCoerceOnTypeMismatch, enableElideOnTypeMismatch)
                .preTessellatePolygons(tessellatePolygons)
                .useMortonEncoding(!cmd.hasOption(EncodeCommandLine.NO_MORTON_OPTION))
                .outlineFeatureTableNames(
                    if (outlineFeatureTables != null) outlineFeatureTables.toList() else listOf<String>(),
                ).layerFilterPattern(filterPattern)
                .layerFilterInvert(filterInvert)
                .integerEncoding(ConversionConfig.IntegerEncodingOption.AUTO)
                .build()
        if (outlineFeatureTables != null && outlineFeatureTables.size > 0) {
            logger.debug(
                "Including outlines for layers: {}",
                outlineFeatureTables.joinToString(", "),
            )
        }

        val encodeConfig =
            EncodeConfig(
                columnMappingConfig = columnMappings,
                conversionConfig = conversionConfig,
                tessellateSource = tessellateURI,
                sortFeaturesPattern = sortFeaturesPattern,
                regenIDsPattern = regenIDsPattern,
                compressionType = compressionType,
                minZoom = minZoom,
                maxZoom = maxZoom,
                willOutput =
                    cmd.hasOption(EncodeCommandLine.OUTPUT_FILE_ARG) ||
                        cmd.hasOption(EncodeCommandLine.OUTPUT_DIR_ARG),
                willDecode = cmd.hasOption(EncodeCommandLine.DECODE_OPTION),
                willPrintMLT = cmd.hasOption(EncodeCommandLine.PRINT_MLT_OPTION),
                willPrintMVT = cmd.hasOption(EncodeCommandLine.PRINT_MVT_OPTION),
                compareProp =
                    cmd.hasOption(EncodeCommandLine.COMPARE_PROP_OPTION) ||
                        cmd.hasOption(EncodeCommandLine.COMPARE_ALL_OPTION),
                compareGeom =
                    cmd.hasOption(EncodeCommandLine.COMPARE_GEOM_OPTION) ||
                        cmd.hasOption(EncodeCommandLine.COMPARE_ALL_OPTION),
                willTime = cmd.hasOption(EncodeCommandLine.TIMER_OPTION),
                dumpStreams = cmd.hasOption(EncodeCommandLine.DUMP_STREAMS_OPTION),
                taskRunner = taskRunner,
                continueOnError = cmd.hasOption(EncodeCommandLine.CONTINUE_OPTION),
            )

        if (tileFileNames != null && tileFileNames.size > 0) {
            require(!(tileFileNames.size > 1 && cmd.hasOption(EncodeCommandLine.OUTPUT_FILE_ARG))) {
                (
                    "Multiple input files not allowed with single output file, use --" +
                        EncodeCommandLine.OUTPUT_DIR_ARG
                )
            }
            for (tileFileName in tileFileNames) {
                val outputPath = getOutputPath(cmd, tileFileName!!, "mlt")
                if (outputPath == null) {
                    continue
                }

                var streamPath: Path? = null
                if (encodeConfig.dumpStreams) {
                    val fileName = MLTStreamObserverFile.sanitizeFilename(tileFileName)
                    streamPath = getOutputPath(cmd, fileName, null, true)
                }

                logger.debug("Converting {} to {}", tileFileName, outputPath)

                encodeTile(0, 0, 0, tileFileName, outputPath, streamPath, encodeConfig)
            }
        } else if (cmd.hasOption(EncodeCommandLine.INPUT_MBTILES_ARG)) {
            // Converting all the tiles in an MBTiles file
            val inputPath = cmd.getOptionValue(EncodeCommandLine.INPUT_MBTILES_ARG)
            val outputPath = getOutputPath(cmd, inputPath, "mlt.mbtiles")
            if (!encodeMBTiles(inputPath, outputPath, encodeConfig)) {
                return false
            }
        } else if (cmd.hasOption(EncodeCommandLine.INPUT_OFFLINEDB_ARG)) {
            val inputPath = cmd.getOptionValue(EncodeCommandLine.INPUT_OFFLINEDB_ARG)
            var ext = FilenameUtils.getExtension(inputPath)
            if (!ext!!.isEmpty()) {
                ext = "." + ext
            }
            val outputPath = getOutputPath(cmd, inputPath, "mlt" + ext)
            if (!encodeOfflineDB(Path.of(inputPath), outputPath, encodeConfig)) {
                return false
            }
        } else if (cmd.hasOption(EncodeCommandLine.INPUT_PMTILES_ARG)) {
            val inputPath = cmd.getOptionValue(EncodeCommandLine.INPUT_PMTILES_ARG)
            var ext = FilenameUtils.getExtension(inputPath)
            if (!ext!!.isEmpty()) {
                ext = "." + ext
            }
            var outputPath = getOutputPath(cmd, inputPath, "mlt" + ext)
            if (outputPath == null) {
                return false
            }

            val inputURI = getInputURI(inputPath)

            outputPath = outputPath.toAbsolutePath()
            if (!encodePMTiles(inputURI, outputPath, encodeConfig)) {
                return false
            }
        }

        if (totalCompressedInput.get() > 0 && logger.isDebugEnabled) {
            val input = totalCompressedInput.get()
            val output = totalCompressedOutput.get()
            val percentStr = String.format("%.1f", 100.0 * output / input)
            logger.debug("Compressed {} bytes to {} bytes ({}%)", input, output, percentStr)
        }
        return true
    }

    /**  Convert a single tile from an individual file */
    @Throws(IOException::class)
    private fun encodeTile(
        x: Long,
        y: Long,
        z: Int,
        tileFileName: String,
        outputPath: Path,
        streamPath: Path?,
        config: EncodeConfig,
    ) {
        val willCompare = config.compareProp || config.compareGeom
        val inputTilePath = Paths.get(tileFileName)
        val decodedMvTile = MvtUtils.decodeMvt(inputTilePath)

        val willTime = config.willTime
        val timer = if (willTime) Timer() else null

        val isIdPresent = true
        val metadata =
            MltConverter.createTilesetMetadata(
                decodedMvTile,
                config.conversionConfig,
                config.columnMappingConfig,
                isIdPresent,
            )

        logColumnMappings(x, y, z, metadata)

        val targetConfig = applyColumnMappingsToConversionConfig(config, metadata)

        var streamObserver: MLTStreamObserver = MLTStreamObserverDefault()
        if (config.dumpStreams) {
            if (streamPath != null) {
                streamObserver = MLTStreamObserverFile(streamPath)
                Files.createDirectories(streamPath)
                logger.debug("Writing raw streams to {}", streamPath)
            }
        }
        val mlTile =
            MltConverter.convertMvt(
                decodedMvTile,
                metadata,
                targetConfig,
                config.tessellateSource,
                streamObserver,
            )
        if (willTime) {
            timer!!.stop("encoding")
        }

        if (config.willOutput) {
            logger.debug("Writing converted tile to {}", outputPath)

            try {
                Files.write(outputPath, mlTile)
            } catch (ex: IOException) {
                logger.error("Failed to write tile to {}", outputPath, ex)
            }
        }
        if (config.willPrintMVT) {
            System.out.write(decodedMvTile.toJson().toByteArray(StandardCharsets.UTF_8))
        }
        val needsDecoding = config.willDecode || willCompare || config.willPrintMLT
        if (needsDecoding) {
            logger.debug("Decoding converted tile...")
            if (willTime) {
                timer!!.restart()
            }

            val decodedTile = MltDecoder.decodeMlTile(mlTile)
            if (willTime) {
                timer!!.stop("decoding")
            }
            if (config.willPrintMLT) {
                System.out.write(decodedTile.toJson().toByteArray(StandardCharsets.UTF_8))
            }
            if (willCompare) {
                val mode =
                    if (config.compareGeom && config.compareProp) {
                        CompareMode.All
                    } else {
                        (
                            if (config.compareGeom) {
                                CompareMode.Geometry
                            } else {
                                CompareMode.Properties
                            }
                        )
                    }

                val result =
                    CompareHelper.compareTiles(
                        decodedTile,
                        decodedMvTile,
                        mode,
                        targetConfig.layerFilterPattern,
                        targetConfig.layerFilterInvert,
                    )
                if (result.isPresent) {
                    logger.warn("Tiles do not match: {}", result)
                } else {
                    logger.debug("Tiles match")
                }
            }
        }
    }

    private fun getInputURI(inputArg: String): URI {
        val file = File(inputArg)
        return if (file.isFile) {
            file.absoluteFile.toURI().normalize()
        } else {
            URI.create(
                inputArg,
            )
        }
    }

    /** Resolve an output filename.
     * If an output filename is specified directly, use it.
     * If only an output directory is given, add the input filename and the specified extension.
     * If neither a directory nor file name is given, returns null.  This is used for testing.
     * If a path is returned and the directory doesn't already exist, it is created. */
    private fun getOutputPath(
        cmd: CommandLine,
        inputFileName: String,
        targetExt: String?,
    ): Path? = getOutputPath(cmd, inputFileName, targetExt, false)

    private fun getOutputPath(
        cmd: CommandLine,
        inputFileName: String,
        targetExt: String?,
        forceExt: Boolean,
    ): Path? {
        val ext =
            if (!targetExt.isNullOrEmpty()) {
                FilenameUtils.EXTENSION_SEPARATOR_STR + targetExt
            } else {
                ""
            }
        var outputPath: Path? = null
        if (cmd.hasOption(EncodeCommandLine.OUTPUT_FILE_ARG)) {
            outputPath = Paths.get(cmd.getOptionValue(EncodeCommandLine.OUTPUT_FILE_ARG))
        } else {
            val outputDir = cmd.getOptionValue(EncodeCommandLine.OUTPUT_DIR_ARG, "./")

            // Get the file basename without extension.  The input may be a local path or a URI (for
            // pmtiles)
            val inputURI = getInputURI(inputFileName)
            if (inputURI.path == null) {
                logger.error("Unable to determine input filename from '{}'", inputFileName)
                return null
            }

            var baseName: String?
            try {
                val inputPath = Paths.get(inputURI.path)
                baseName = FilenameUtils.getBaseName(inputPath.fileName.toString())
            } catch (ignored: InvalidPathException) {
                //  Windows can't handle getting the path part of a file URI
                baseName = FilenameUtils.getBaseName(inputFileName)
            }

            outputPath = Paths.get(outputDir, baseName + ext)
        }
        if (outputPath != null) {
            if (forceExt) {
                outputPath = Path.of(FilenameUtils.removeExtension(outputPath.toString()) + ext)
            }

            val outputDir = outputPath.toAbsolutePath().parent
            if (!Files.exists(outputDir)) {
                try {
                    Files.createDirectories(outputDir)
                } catch (ex: IOException) {
                    logger.error("Failed to create directory '{}'", outputDir, ex)
                    return null
                }
            }
        }
        return outputPath
    }

    private fun createTaskRunner(threadCount: Int): TaskRunner {
        if (threadCount <= 1) {
            return SerialTaskRunner()
        }
        // Create a thread pool with a bounded task queue that will not reject tasks when it's full.
        // Tasks beyond the limit will run on the calling thread, preventing OOM from too many tasks
        // while allowing for parallelism when the pool is available.
        return ThreadPoolTaskRunner(
            ThreadPoolExecutor(
                threadCount,
                threadCount,
                100L,
                TimeUnit.MILLISECONDS,
                LinkedBlockingQueue<Runnable?>(4 * threadCount),
                ThreadPoolExecutor.CallerRunsPolicy(),
            ),
        )
    }

    private val logger: Logger = LoggerFactory.getLogger(Encode::class.java)
}
