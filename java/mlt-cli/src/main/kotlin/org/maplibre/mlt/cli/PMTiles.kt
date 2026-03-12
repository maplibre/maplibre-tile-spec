package org.maplibre.mlt.cli

import com.github.benmanes.caffeine.cache.RemovalCause
import com.github.benmanes.caffeine.cache.Scheduler
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
import io.tileverse.cache.CaffeineCache
import io.tileverse.cache.CaffeineCacheManager
import io.tileverse.io.ByteRange
import io.tileverse.rangereader.AbstractRangeReader
import io.tileverse.rangereader.RangeReader
import io.tileverse.rangereader.RangeReaderFactory
import io.tileverse.rangereader.cache.CachingRangeReader
import org.apache.commons.lang3.ArrayUtils
import org.apache.commons.lang3.mutable.MutableBoolean
import java.io.ByteArrayInputStream
import java.io.IOException
import java.lang.ref.WeakReference
import java.net.URI
import java.nio.ByteBuffer
import java.nio.file.Path
import java.util.Optional
import java.util.OptionalLong
import java.util.concurrent.ConcurrentHashMap
import java.util.concurrent.RejectedExecutionException
import java.util.concurrent.atomic.AtomicBoolean
import java.util.concurrent.atomic.AtomicLong
import kotlin.math.max
import kotlin.math.min
import kotlin.time.toJavaDuration

/** Encode the MVT tiles in a PMTiles file */
fun encodePMTiles(
    inputURI: URI,
    outputPath: Path,
    config: EncodeConfig,
): Boolean {
    val cacheManager = getCacheManager(config)
    return RangeReaderFactory.create(inputURI).use { rawReader ->
        logger.debug("Opened '{}' for reading", inputURI)
        getCachingReader(rawReader, cacheManager).use { cachingReader ->
            val adapter = getReaderAdapter(cachingReader)
            ReadablePmtiles(adapter)
                .use { pmTilesReader ->
                    WriteablePmtiles.newWriteToFile(outputPath).use { writer ->
                        logger.debug("Opened '{}' for writing", outputPath)
                        encodePMTiles(inputURI, pmTilesReader, writer, outputPath, config)
                    }
                }.also {
                    if (config.logCacheStats) {
                        rangeReaderCache.get()?.also(::logCacheStats)
                    }
                }
        }
    }
}

// To have any effect on the cache size limit, we have to entirely replace the cache setup.
// See `io.tileverse.rangereader.cache.RangeReaderCache.buildSharedCache`
private fun getCacheManager(config: EncodeConfig) =
    CaffeineCacheManager().also {
        // `RangeReaderCache.SHARED_CACHE_NAME` is private
        val cacheName = "tileverse-rangereader-cache"
        val maxMemory = Runtime.getRuntime().maxMemory()
        val cacheSize =
            if (cacheMaxHeapPercent > 0) {
                (cacheMaxHeapPercent / 100 * maxMemory).toLong()
            } else {
                cacheMaxSize
            }

        logger.debug(
            "Initializing cache with max size {}, max age {}",
            formatSize(cacheSize),
            cacheExpireAfterAccess,
        )
        rangeReaderCache =
            WeakReference(
                it.getCache(cacheName, {
                    CaffeineCache
                        .newBuilder<ByteRange, ByteBuffer>()
                        .maximumWeight(cacheSize)
                        .weigher(::weigh)
                        .scheduler(Scheduler.systemScheduler())
                        .also {
                            if (cacheAverageEntrySize > 0) {
                                it.averageWeight(weigh(cacheAverageEntrySize))
                            }
                            if (cacheExpireAfterAccess.isPositive()) {
                                it.expireAfterAccess(cacheExpireAfterAccess.toJavaDuration())
                            }
                            if (config.logCacheStats) {
                                it.recordStats()
                                it.removalListener(::onRemoval)
                            }
                        }.build()
                }),
            )
    }

private var rangeReaderCache: WeakReference<CaffeineCache<ByteRange, ByteBuffer>> = WeakReference(null)

private fun getCachingReader(
    rawReader: RangeReader,
    cacheManager: CacheManager,
) = CachingRangeReader
    .builder(rawReader)
    .cacheManager(cacheManager)
    .withoutHeaderBuffer()
    .blockSize(cacheBlockSize)
    .build()

private fun logCacheStats(cache: CaffeineCache<ByteRange, ByteBuffer>) {
    val stats = cache.stats()
    val runtime = Runtime.getRuntime()
    val usedMem = runtime.totalMemory() - runtime.freeMemory()
    val freeMem = runtime.maxMemory() - usedMem
    logger.debug(
        cacheMarker,
        "Cache: entries={} hits={} ({}%) misses={} averageLoadTime={} evictions={} evictionWeight={} free mem={}",
        String.format("%,d", stats.entryCount),
        String.format("%,d", stats.hitCount),
        String.format("%.1f", 100 * stats.hitRate),
        String.format("%,d", stats.missCount),
        formatNanosecDuration(stats.averageLoadTime),
        String.format("%,d", stats.evictionCount),
        formatSize(stats.evictionWeight),
        formatSize(freeMem),
    )
}

private fun getReaderAdapter(reader: AbstractRangeReader) =
    object : ReadablePmtiles.DataReader {
        override fun read(
            offset: Long,
            length: Int,
        ) = reader.readRange(offset, length).array()
    }

private fun weigh(
    key: ByteRange,
    value: ByteBuffer,
) = weigh(value.capacity())

private fun weigh(size: Int): Int {
    // Approximate overhead of the key record (long + int + object header)
    val keyWeight = 24
    // Value weight: object header + int field + the actual data in the ByteBuffer
    val valueWeight = 16 + size
    return keyWeight + valueWeight
}

private fun onRemoval(
    key: ByteRange?,
    value: ByteBuffer?,
    cause: RemovalCause,
) {
    if (logger.isTraceEnabled && cause != RemovalCause.EXPLICIT) {
        logger.trace(cacheMarker, "Evicted {}, {}, cause: {}", value, key, cause)
    }
}

private fun encodePMTiles(
    inputURI: URI,
    reader: ReadablePmtiles,
    writer: WriteablePmtiles,
    outputPath: Path,
    config: EncodeConfig,
): Boolean {
    val timer = if (config.willTime) Timer() else null
    val header = reader.header

    if (logger.isDebugEnabled) {
        val tilesPresentPct =
            if (header.numAddressedTiles > 0) {
                String.format(
                    " (%.1f%%)",
                    100.0 * header.numTileContents / header.numAddressedTiles,
                )
            } else {
                ""
            }

        logger.debug(
            "PMTiles: version={}, tileType={}, tileCompression={}, minZoom={}, maxZoom={}, numAddressedTiles={}, numTileContents={}{}",
            header.specVersion,
            header.tileType,
            header.tileCompression,
            header.minZoom,
            header.maxZoom,
            String.format("%,d", header.numAddressedTiles),
            String.format("%,d", header.numTileContents),
            tilesPresentPct,
        )
    }

    if (header.tileType != TileType.MVT) {
        logger.error(
            "Input PMTiles tile type is {}, expected {}",
            header.tileType,
            TileType.MVT,
        )
        return false
    }

    // If a compression type is given (including none) try to use that, otherwise
    // use the source compression type mapped to supported types.
    val targetCompressType =
        if (config.compressionType == null) {
            toTileCompression(header.tileCompression)
        } else {
            TileCompression.fromId(config.compressionType)
        }

    val minZoom = max(header.minZoom.toInt(), config.minZoom)
    val maxZoom = min(header.maxZoom.toInt(), config.maxZoom)

    // re-create the config with the resolved compression type
    val targetConfig = config.copy(compressionType = toCompressionOption(targetCompressType))

    val newMetadata =
        updateMetadata(reader.metadata(), minZoom, maxZoom, targetCompressType)
    val state = ConversionState(targetConfig)

    try {
        writer.newTileWriter().use { tileWriter ->
            // The root task which creates other tasks for converting each tile coordinate range.
            // Completes when the directory has been fully read and all tasks have been created.
            config.taskRunner.run(
                {
                    processTiles(
                        reader.getTileCoordRanges(minZoom, maxZoom),
                        reader,
                        tileWriter,
                        config.taskRunner,
                        config,
                        state,
                    )
                    config.taskRunner.shutdown()
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
                logger.debug("Finalizing PMTiles file")
                writer.finish(newMetadata)
            }

            timer?.stop("PMTiles")

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

private fun processTiles(
    tileSeq: Sequence<ReadablePmtiles.TileCoordRange>,
    reader: ReadablePmtiles,
    tileWriter: WriteableTileArchive.TileWriter,
    taskRunner: TaskRunner,
    config: EncodeConfig,
    state: ConversionState,
) {
    tileSeq.forEach { tileRange ->
        if (state.encodeConfig.continueOnError || state.success.get()) {
            state.totalTileCount.addAndGet(tileRange.tileCount.toLong())
            try {
                taskRunner.run(
                    {
                        if (!processTileRange(tileRange, reader, tileWriter, config, state)) {
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
        "Directory read complete. Processing {} total tiles.",
        String.format("%,d", state.totalTileCount.get()),
    )
}

/** Convert a range of tile coordinates which have the same contents */
private fun processTileRange(
    tileRef: ReadablePmtiles.TileCoordRange,
    reader: ReadablePmtiles,
    writer: WriteableTileArchive.TileWriter,
    config: EncodeConfig,
    state: ConversionState,
): Boolean {
    val tileCoords = tileRef.tileCoords
    val firstTileCoord = tileCoords.first()
    val tileLabel = getTileLabel(firstTileCoord)
    val prevTileCount = state.tilesProcessed.getAndAdd(tileCoords.size.toLong())
    val curTileCount = prevTileCount + tileCoords.size

    // Log every Nth tile at debug level
    if (logger.isDebugEnabled) {
        val prevBatch = prevTileCount / tileLogInterval
        val curBatch = curTileCount / tileLogInterval
        // If we're the range that crosses a log interval boundary, log the progress.
        if (curBatch != prevBatch || (state.directoryComplete.get() && curTileCount == state.totalTileCount.get())) {
            if (!state.directoryComplete.get()) {
                // Still fetching tile coordinates, we can't show a percentage.
                logger.debug(
                    "Converted {} unique of {} tiles : {}",
                    String.format("%,d", state.tilesConverted.get()),
                    String.format("%,d", curTileCount),
                    tileLabel,
                )
            } else {
                val totalTiles = state.totalTileCount.get()
                logger.debug(
                    "Converted {} unique of {} encountered of {} total tiles ({}%) : {}",
                    String.format("%,d", state.tilesConverted.get()),
                    String.format("%,d", curTileCount),
                    String.format("%,d", totalTiles),
                    String.format("%.1f", 100.0 * curTileCount / totalTiles),
                    tileLabel,
                )
            }
            if (config.logCacheStats) {
                rangeReaderCache.get()?.also(::logCacheStats)
            }
        }
    }

    if (tileRef.byteRange.length <= maxTileTrackSize) {
        state.offsetToHashMap.get(tileRef.byteRange.offset)?.let { tileHash ->
            // Already processed this input tile for a different tile coordinate range.
            // Pass an empty result with the same hash to the writer.  Rely on
            // `WritablePmtiles.DeduplicatingTileArchiveWriter` to locate the previous entry and
            // never write this empty result, though it provides no feedback we can use to check.
            writeTiles(ArrayUtils.EMPTY_BYTE_ARRAY, OptionalLong.of(tileHash), tileCoords, writer)
            // nothing more to do for this tile range
            return@processTileRange true
        }
    }

    // Get the raw tile contents
    var tileData = reader.getBytes(tileRef.byteRange)

    // Decompress the raw tile data
    val compressedMVTSize = tileData.size
    try {
        tileData = decompress(ByteArrayInputStream(tileData))
    } catch (ex: IOException) {
        logger.error("Failed to decompress tile {}", tileLabel, ex)
        return false
    }

    val uncompressedMVTSize = tileData.size
    if (logger.isTraceEnabled) {
        val comp =
            if (compressedMVTSize != uncompressedMVTSize) {
                ", $compressedMVTSize compressed"
            } else {
                ""
            }
        logger.trace(
            readMarker,
            "{} loaded, {} bytes{}",
            tileLabel,
            uncompressedMVTSize,
            comp,
        )
    }

    // Convert to MLT and optionally re-compress
    val didCompress = MutableBoolean()
    val mltData =
        convertTile(
            firstTileCoord.x().toLong(),
            firstTileCoord.y().toLong(),
            firstTileCoord.z(),
            tileData,
            state.encodeConfig,
            Optional.empty<Double>(), // compress even if it increases the size
            Optional.empty<Long>(),
            didCompress,
        )
    state.tilesConverted.incrementAndGet()

    // Write the result for all the tile coordinates in the range
    if (mltData != null && mltData.size > 0) {
        val hash = OptionalLong.of(TileArchiveWriter.generateContentHash(mltData))

        val existingHash =
            if (tileRef.byteRange.length <= maxTileTrackSize) {
                state.offsetToHashMap.putIfAbsent(tileRef.byteRange.offset, hash.asLong)
            } else {
                null
            }
        if (existingHash != null) {
            // The value was already present.  This indicates that another thread processed the
            // same input tile since we checked above, and this thread's result will be ignored.
            if (hash.asLong == existingHash) {
                // As above, we write with no data and the same hash to reference the previous result
                writeTiles(ArrayUtils.EMPTY_BYTE_ARRAY, hash, tileCoords, writer)
                return@processTileRange true
            } else {
                logger.warn(
                    "Hash collision for {} : {} != {}. This will cause incorrect de-duplication.",
                    tileLabel,
                    existingHash,
                    hash.asLong,
                )
                return@processTileRange false
            }
        }

        writeTiles(mltData, hash, tileCoords, writer)

        if (logger.isTraceEnabled) {
            val extraCoords =
                if (tileCoords.size > 1) {
                    "(+ ${tileCoords.size - 1}: ${getTileLabels(tileCoords.drop(1), 10)})"
                } else {
                    ""
                }
            logger.trace(
                writeMarker,
                "Writing {}{}, {} bytes{} hash {}",
                tileLabel,
                extraCoords,
                mltData.size,
                if (didCompress.isTrue) " (compressed)" else "",
                hash.asLong,
            )
        }
    } else {
        logger.warn("Tile {} produced empty output, skipping", tileLabel)
    }
    return true
}

private fun writeTiles(
    tileData: ByteArray,
    tileHash: OptionalLong,
    tileCoords: Collection<TileCoord>,
    writer: WriteableTileArchive.TileWriter,
) = synchronized(writer) {
    // The writer is not thread-safe
    tileCoords.forEach { tileCoord ->
        writer.write(TileEncodingResult(tileCoord, tileData, tileHash))
    }
}

private fun toTileCompression(compression: Pmtiles.Compression): TileCompression =
    when (compression) {
        Pmtiles.Compression.NONE -> TileCompression.NONE
        Pmtiles.Compression.GZIP -> TileCompression.GZIP
        else -> throw IllegalArgumentException("Unsupported compression type: $compression")
    }

private fun toCompressionOption(compression: TileCompression): String? =
    when (compression) {
        TileCompression.NONE -> null
        TileCompression.GZIP -> "gzip"
        else -> throw IllegalArgumentException("Unsupported compression type: $compression")
    }

private data class ConversionState(
    // Options passed to each tile conversion
    val encodeConfig: EncodeConfig,
    // Used to track tile hashes to identify repeated tiles in separate tile ranges.
    // Maps from the byte offset in the source file to the hash of the generated contents.
    val offsetToHashMap: ConcurrentHashMap<Long, Long> = ConcurrentHashMap(),
    // The number of tiles processed so far
    val tilesProcessed: AtomicLong = AtomicLong(0),
    val tilesConverted: AtomicLong = AtomicLong(0),
    // The total number of tiles seen in the directory so far
    val totalTileCount: AtomicLong = AtomicLong(0),
    // Whether we've finished reading the directory
    val directoryComplete: AtomicBoolean = AtomicBoolean(false),
    // Whether the conversion is successful so far
    val success: AtomicBoolean = AtomicBoolean(true),
)
