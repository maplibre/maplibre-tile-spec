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
import com.onthegomap.planetiler.pmtiles.WriteablePmtiles
import io.tileverse.cache.CacheManager
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
import java.util.concurrent.RejectedExecutionException
import java.util.concurrent.atomic.AtomicBoolean
import java.util.concurrent.atomic.AtomicLong
import kotlin.math.max
import kotlin.math.min

object PMTilesHelper : ConversionHelper() {
    /** Encode the MVT tiles in a PMTiles file */
    @Throws(IOException::class)
    fun encodePMTiles(
        inputURI: URI,
        outputPath: Path,
        config: EncodeConfig,
    ): Boolean =
        RangeReaderFactory.create(inputURI).use { rawReader ->
            CachingRangeReader
                .builder(rawReader)
                .cacheManager(CacheManager.getDefault())
                .withHeaderBuffer()
                .withBlockAlignment()
                .build()
                .use { cachingReader ->
                    // This reader is thread-safe
                    ReadablePmtiles(cachingReader).use { reader ->
                        logger.debug("Opened '{}' for reading", inputURI)
                        WriteablePmtiles.newWriteToFile(outputPath).use { writer ->
                            logger.debug("Opened '{}' for writing", outputPath)
                            encodePMTiles(inputURI, reader, writer, outputPath, config)
                        }
                    }
                }
        }

    @Throws(IOException::class)
    private fun encodePMTiles(
        inputURI: URI,
        reader: ReadablePmtiles,
        writer: WriteablePmtiles,
        outputPath: Path,
        config: EncodeConfig,
    ): Boolean {
        val header = reader.header
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
                "Input PMTiles tile type is {}, expected {}",
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

        val minZoom = max(header.minZoom().toInt(), config.minZoom)
        val maxZoom = min(header.maxZoom().toInt(), config.maxZoom)

        val newMetadata =
            updateMetadata(reader.metadata(), minZoom, maxZoom, targetCompressType)
        try {
            writer.newTileWriter().use { tileWriter ->
                val state =
                    ConversionState(
                        config,
                        AtomicLong(0),
                        AtomicLong(0),
                        AtomicBoolean(false),
                        AtomicBoolean(true),
                        minZoom,
                        maxZoom,
                    )

                config.taskRunner.run(
                    {
                        processAllTiles(
                            config.taskRunner,
                            { coords ->
                                processTileRange(coords, reader, tileWriter, state)
                            },
                            reader,
                            state,
                        )
                    },
                )

                try {
                    // Wait for all tasks to finish
                    config.taskRunner.awaitTermination()
                } catch (ex: InterruptedException) {
                    logger.error("Interrupted", ex)
                    state.success.set(false)
                }

                if (state.success.get()) {
                    logger.debug("Finalizing MBTiles file")
                    writer.finish(newMetadata)
                }
                return state.success.get()
            }
        } catch (ex: IOException) {
            logger.error("PMTiles conversion failed", ex)
            return false
        }
    }

    private fun updateMetadata(
        oldMetadata: TileArchiveMetadata,
        minZoom: Int,
        maxZoom: Int,
        targetCompressType: TileCompression?,
    ): TileArchiveMetadata =
        TileArchiveMetadata(
            oldMetadata.name(),
            oldMetadata.description(),
            oldMetadata.attribution(),
            oldMetadata.version(),
            oldMetadata.type(),
            TileFormat.MLT,
            oldMetadata.bounds(),
            oldMetadata.center(),
            minZoom,
            maxZoom,
            oldMetadata.json(),
            oldMetadata.others(),
            targetCompressType,
        )

    private fun processAllTiles(
        taskRunner: TaskRunner,
        processTileRange: (List<TileCoord>) -> Boolean,
        reader: ReadablePmtiles,
        state: ConversionState,
    ) {
        reader
            .allTileCoordRanges
            .forEach { tileCoords ->
                if (state.encodeConfig.continueOnError || state.success.get()) {
                    state.totalTileCount.addAndGet(tileCoords.size.toLong())
                    if (!tileCoords.any { coord -> coord.z() in state.minZoom..state.maxZoom }) {
                        return@forEach
                    }
                    try {
                        taskRunner.run(
                            {
                                if (!processTileRange(tileCoords)) {
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

    /** Convert a range of tile coordinates which have the same contents */
    private fun processTileRange(
        tileCoords: List<TileCoord>,
        reader: ReadablePmtiles,
        writer: WriteableTileArchive.TileWriter,
        state: ConversionState,
    ): Boolean {
        if (tileCoords.isEmpty()) {
            return true
        }
        val firstTileCoord = tileCoords.first()
        val tileLabel =
            String.format("%d:%d,%d", firstTileCoord.z(), firstTileCoord.x(), firstTileCoord.y())
        val tileCount = state.tilesProcessed.addAndGet(tileCoords.size.toLong())

        if (logger.isDebugEnabled()) {
            if (!state.directoryComplete.get()) {
                // Still fetching tile coordinates, we can't show a percentage
                if (tileCount < 2 || (tileCount % tileLogInterval == 0L)) {
                    logger
                        .atDebug()
                        .setMessage("Processing tile {} : {}")
                        .addArgument({ String.format("%,d", tileCount) })
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
                        .addArgument({ String.format("%,d", tileCount) })
                        .addArgument({ String.format("%,d", totalTiles) })
                        .addArgument({ String.format("%.1f", progress) })
                        .addArgument(tileLabel)
                        .log()
                }
            }
        }

        var tileData = reader.getTile(firstTileCoord)
        if (tileData == null) {
            logger.warn("Tile {} is missing from PMTiles file", tileLabel)
            return false
        }

        val compressedSize = tileData.size
        try {
            tileData = decompress(ByteArrayInputStream(tileData))
        } catch (ex: IOException) {
            logger.error("Failed to decompress tile {}", tileLabel, ex)
            return false
        }

        val uncompressedSize = tileData.size
        logger
            .atTrace()
            .setMessage("Loaded {}, {} bytes{}")
            .addArgument(tileLabel)
            .addArgument(uncompressedSize)
            .addArgument(
                {
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
                firstTileCoord.x().toLong(),
                firstTileCoord.y().toLong(),
                firstTileCoord.z(),
                tileData,
                state.encodeConfig,
                Optional.empty<Double>(), // compress even if it increases the size
                Optional.empty<Long>(),
                didCompress,
            )

        if (mltData != null && mltData.size > 0) {
            val hash = OptionalLong.of(TileArchiveWriter.generateContentHash(mltData))
            // Write the result for all tile coordinates in this range
            synchronized(writer) {
                tileCoords.forEach { tileCoord ->
                    if (tileCoord.z() in state.minZoom..state.maxZoom) {
                        writer.write(TileEncodingResult(tileCoord, mltData, hash))
                    }
                }
            }
        } else {
            logger.warn("Tile {} produced empty output, skipping", tileLabel)
        }
        return true
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
                throw IllegalArgumentException("Unsupported compression type: " + compression)
            }
        }

    private val logger: Logger = LoggerFactory.getLogger(PMTilesHelper::class.java)

    private data class ConversionState(
        val encodeConfig: EncodeConfig,
        val tilesProcessed: AtomicLong,
        val totalTileCount: AtomicLong,
        val directoryComplete: AtomicBoolean,
        val success: AtomicBoolean,
        val minZoom: Int,
        val maxZoom: Int,
    )
}
