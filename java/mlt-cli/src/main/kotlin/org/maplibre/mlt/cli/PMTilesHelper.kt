package org.maplibre.mlt.cli

import com.onthegomap.planetiler.archive.TileArchiveMetadata
import com.onthegomap.planetiler.archive.TileArchiveWriter
import com.onthegomap.planetiler.archive.TileCompression
import com.onthegomap.planetiler.archive.TileEncodingResult
import com.onthegomap.planetiler.archive.TileFormat
import com.onthegomap.planetiler.archive.WriteableTileArchive
import com.onthegomap.planetiler.geo.TileCoord
import com.onthegomap.planetiler.pmtiles.Pmtiles
import com.onthegomap.planetiler.pmtiles.Pmtiles.TileType
import com.onthegomap.planetiler.pmtiles.ReadablePmtiles
import com.onthegomap.planetiler.pmtiles.WriteablePmtiles
import io.tileverse.rangereader.RangeReader
import io.tileverse.rangereader.RangeReaderFactory
import io.tileverse.rangereader.cache.CachingRangeReader
import org.apache.commons.lang3.mutable.MutableBoolean
import org.slf4j.Logger
import org.slf4j.LoggerFactory
import java.io.ByteArrayInputStream
import java.io.IOException
import java.net.URI
import java.nio.file.Path
import java.util.Optional
import java.util.OptionalLong
import java.util.concurrent.ConcurrentHashMap
import java.util.concurrent.RejectedExecutionException
import java.util.concurrent.atomic.AtomicBoolean
import java.util.concurrent.atomic.AtomicLong
import java.util.function.Function
import java.util.function.Supplier
import kotlin.math.max
import kotlin.math.min

object PMTilesHelper : ConversionHelper() {
    /** Encode the MVT tiles in a PMTiles file */
    @Throws(IOException::class)
    fun encodePMTiles(
        inputURI: URI,
        outputPath: Path,
        config: EncodeConfig,
    ): Boolean {
        val readerCacheWeight = 250 * 1024 * 1024
        RangeReaderFactory.create(inputURI).use { rawReader ->
            CachingRangeReader
                .builder(rawReader)
                .maximumWeight(readerCacheWeight.toLong())
                .build()
                .use { cachingReader ->

                    // Although `RangeReader` is thread-safe, `ReadablePmtiles` is not because
                    // it uses a single `Channel` with separate `position` and `read` operations.
                    // So use a separate instance for each thread, but share the underlying reader and cache.
                    val threadCount = config.taskRunner.threadCount
                    val readers =
                        ConcurrentHashMap<Long?, Optional<ReadablePmtiles>>(1 + threadCount)
                    val readerSupplier: Supplier<Optional<ReadablePmtiles>> =
                        Supplier {
                            readers.computeIfAbsent(
                                Thread.currentThread().threadId(),
                            ) { ignored: Long? -> tryCreateReadablePmtiles(cachingReader) }
                        }
                    val maybeReader = readerSupplier.get()
                    if (maybeReader.isEmpty()) {
                        logger.error("Failed to read PMTiles from '{}'", inputURI)
                        return false
                    }

                    val reader = maybeReader.get()

                    logger.debug("Opened PMTiles from '{}'", inputURI)

                    val header = reader.getHeader()
                    logger.debug(
                        "PMTiles header: version={}, tileType={}, tileCompression={}, minZoom={}, maxZoom={}, numAddressedTiles={}, numTileContents={} ({}%)",
                        header.specVersion(),
                        header.tileType(),
                        header.tileCompression(),
                        header.minZoom(),
                        header.maxZoom(),
                        header.numAddressedTiles(),
                        header.numTileContents(),
                        String.format(
                            "%.1f",
                            100.0 * header.numTileContents() / header.numAddressedTiles(),
                        ),
                    )

                    if (header.tileType() != TileType.MVT) {
                        logger.error(
                            "Input PMTiles tile type is {}, expected {} (MVT)",
                            header.tileType(),
                            TileType.MVT,
                        )
                        return false
                    }

                    // If a compression type is given (including none) try to use that, otherwise
                    // use the source compression type mapped to supported types.
                    val targetCompressType =
                        if (config.compressionType == null) {
                            toTileCompression(header.tileCompression())
                        } else {
                            TileCompression.fromId(config.compressionType)
                        }

                    val actualMinZoom = max(header.minZoom().toInt(), config.minZoom)
                    val actualMaxZoom = min(header.maxZoom().toInt(), config.maxZoom)

                    val oldMetadata = reader.metadata()
                    val newMetadata =
                        TileArchiveMetadata(
                            oldMetadata.name(),
                            oldMetadata.description(),
                            oldMetadata.attribution(),
                            oldMetadata.version(),
                            oldMetadata.type(),
                            TileFormat.MLT,
                            oldMetadata.bounds(),
                            oldMetadata.center(),
                            actualMinZoom,
                            actualMaxZoom,
                            oldMetadata.json(),
                            oldMetadata.others(),
                            targetCompressType,
                        )
                    try {
                        WriteablePmtiles.newWriteToFile(outputPath).use { fileWriter ->
                            fileWriter.newTileWriter().use { tileWriter ->
                                logger.debug("Opened '{}'", outputPath)
                                val taskRunner = config.taskRunner
                                val success = AtomicBoolean(true)
                                val tilesProcessed = AtomicLong(0)
                                val totalTileCount = AtomicLong(0)

                                val state =
                                    ConversionState(
                                        config,
                                        tilesProcessed,
                                        totalTileCount,
                                        AtomicBoolean(false), // directoryComplete
                                        success,
                                    )

                                val processTile =
                                    getProcessTileFunction(readerSupplier, tileWriter, state)

                                taskRunner.run(
                                    Runnable {
                                        processTiles(
                                            taskRunner,
                                            processTile,
                                            readerSupplier,
                                            state,
                                            config.minZoom,
                                            config.maxZoom,
                                        )
                                    },
                                )

                                try {
                                    // Wait for all tasks to finish
                                    taskRunner.awaitTermination()
                                } catch (ex: InterruptedException) {
                                    logger.error("Interrupted", ex)
                                    success.set(false)
                                }

                                if (success.get()) {
                                    logger.debug("Finalizing MBTiles file")
                                    fileWriter.finish(newMetadata)
                                }
                                return success.get()
                            }
                        }
                    } catch (ex: IOException) {
                        logger.error("PMTiles conversion failed", ex)
                        return false
                    }
                }
        }
    }

    private fun processTiles(
        taskRunner: TaskRunner,
        processTile: java.util.function.Function<TileCoord?, Boolean?>,
        readerSupplier: Supplier<Optional<ReadablePmtiles>>,
        state: ConversionState,
        minZoom: Int,
        maxZoom: Int,
    ) {
        val maybeReader = readerSupplier.get()
        if (maybeReader.isEmpty()) {
            logger.error("Failed to read PMTiles file")
            state.success.set(false)
            return
        }
        val reader = maybeReader.get()
        reader
            .getAllTileCoords()
            .stream()
            .filter { tc: TileCoord? -> minZoom <= tc!!.z() && tc.z() <= maxZoom }
            .forEach { tileCoord: TileCoord? ->
                state.totalTileCount.incrementAndGet()
                if (state.encodeConfig.continueOnError || state.success.get()) {
                    try {
                        taskRunner.run(
                            Runnable {
                                if (!processTile.apply(tileCoord)!!) {
                                    state.success.set(false)
                                }
                            },
                        )
                    } catch (ignored: RejectedExecutionException) {
                        // This indicates the thread pool has been shut down due to an error
                    }
                }
            }
        state.directoryComplete.set(true)
        logger.debug(
            "Directory read complete. Processing {} tiles.",
            String.format("%,d", state.totalTileCount.get()),
        )
        taskRunner.shutdown()
    }

    /** Produce a callable function for processing a single tile, capturing everything
     * it needs, suitable for submitting to a thread pool */
    private fun getProcessTileFunction(
        readerSupplier: Supplier<Optional<ReadablePmtiles>>,
        writer: WriteableTileArchive.TileWriter,
        state: ConversionState,
    ): java.util.function.Function<TileCoord?, Boolean?> {
        return Function { tileCoord: TileCoord? ->
            val tileLabel =
                String.format("%d:%d,%d", tileCoord!!.z(), tileCoord.x(), tileCoord.y())
            val tileCount = state.tilesProcessed.incrementAndGet()

            if (logger.isDebugEnabled()) {
                if (!state.directoryComplete.get()) {
                    // Still fetching tile coordinates, we can't show a percentage
                    if (tileCount < 2 || (tileCount % tileLogInterval == 0L)) {
                        logger
                            .atDebug()
                            .setMessage("Processing tile {} : {}")
                            .addArgument(Supplier { String.format("%,d", tileCount) })
                            .addArgument(tileLabel)
                            .log()
                    }
                } else {
                    val totalTiles = state.totalTileCount.get()
                    val progress = 100.0 * tileCount / totalTiles
                    val prevProgress = 100.0 * max(0, tileCount - 1) / totalTiles
                    if ((tileCount % tileLogInterval == 0L) ||
                        Math.round(progress * 10.0).toInt() !=
                        Math
                            .round(prevProgress * 10.0)
                            .toInt()
                    ) {
                        logger
                            .atDebug()
                            .setMessage("Processing tiles: {} / {} ({}%) : {}")
                            .addArgument(Supplier { String.format("%,d", tileCount) })
                            .addArgument(Supplier { String.format("%,d", totalTiles) })
                            .addArgument(Supplier { String.format("%.1f", progress) })
                            .addArgument(tileLabel)
                            .log()
                    }
                }
            }

            val maybeReader = readerSupplier.get()
            if (maybeReader.isEmpty()) {
                logger.error("Failed to read PMTiles for tile {}", tileLabel)
                return@Function false
            }

            val reader = maybeReader.get()
            var tileData = reader.getTile(tileCoord)
            if (tileData == null) {
                logger.warn("Tile {} is missing from PMTiles file", tileLabel)
                return@Function false
            }

            val compressedSize = tileData.size
            try {
                tileData = decompress(ByteArrayInputStream(tileData))
            } catch (ex: IOException) {
                logger.error("Failed to decompress tile {}", tileLabel, ex)
                return@Function false
            }

            val uncompressedSize = tileData.size
            logger
                .atTrace()
                .setMessage("Loaded {}, {} bytes{}")
                .addArgument(tileLabel)
                .addArgument(uncompressedSize)
                .addArgument(
                    Supplier {
                        if (compressedSize != uncompressedSize) {
                            String.format(
                                ", %d compressed",
                                compressedSize,
                            )
                        } else {
                            ""
                        }
                    },
                ).log()

            val didCompress: MutableBoolean? = null
            val mltData =
                Encode.convertTile(
                    tileCoord.x().toLong(),
                    tileCoord.y().toLong(),
                    tileCoord.z(),
                    tileData,
                    state.encodeConfig,
                    Optional.empty<Double>(), // compress even if it increases the size
                    Optional.empty<Long>(),
                    didCompress,
                )

            if (mltData != null && mltData.size > 0) {
                val hash = TileArchiveWriter.generateContentHash(mltData)
                synchronized(writer) {
                    writer.write(TileEncodingResult(tileCoord, mltData, OptionalLong.of(hash)))
                }
            } else {
                logger.warn("Tile {} produced empty output, skipping", tileLabel)
            }
            true
        }
    }

    private fun toTileCompression(compression: Pmtiles.Compression): TileCompression =
        when (compression) {
            Pmtiles.Compression.NONE -> {
                TileCompression.NONE
            }

            Pmtiles.Compression.GZIP -> {
                TileCompression.GZIP
            }

            else -> {
                throw IllegalArgumentException("Unsupported PMTiles compression type: " + compression)
            }
        }

    private fun tryCreateReadablePmtiles(reader: RangeReader): Optional<ReadablePmtiles> {
        try {
            return Optional.of(ReadablePmtiles(reader.asByteChannel()))
        } catch (ignored: IOException) {
        }
        return Optional.empty()
    }

    private val logger: Logger = LoggerFactory.getLogger(PMTilesHelper::class.java)

    @JvmRecord
    private data class ConversionState(
        val encodeConfig: EncodeConfig,
        val tilesProcessed: AtomicLong,
        val totalTileCount: AtomicLong,
        val directoryComplete: AtomicBoolean,
        val success: AtomicBoolean,
    )
}
