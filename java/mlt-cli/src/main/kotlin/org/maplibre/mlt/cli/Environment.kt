package org.maplibre.mlt.cli

import kotlin.time.Duration
import kotlin.time.Duration.Companion.seconds

const val ENV_COMPRESSION_RATIO_THRESHOLD = "MLT_COMPRESSION_RATIO_THRESHOLD"
const val DEFAULT_COMPRESSION_RATIO_THRESHOLD = 0.98
val compressionRatioThreshold by lazy { computeCompressionRatioThreshold() }

const val ENV_COMPRESSION_FIXED_THRESHOLD = "MLT_COMPRESSION_FIXED_THRESHOLD"
const val DEFAULT_COMPRESSION_FIXED_THRESHOLD = 20L
val compressionFixedThreshold by lazy { computeCompressionFixedThreshold() }

const val ENV_TILE_LOG_INTERVAL = "MLT_TILE_LOG_INTERVAL"
const val DEFAULT_TILE_LOG_INTERVAL = 10_000L
val tileLogInterval by lazy { computeTileLogInterval() }

const val ENV_CACHE_MAX = "MLT_CACHE_MAX"
val DEFAULT_CACHE_MAX = 64L * 1024L * 1024L
val cacheMaxSize by lazy { computeCacheMaxSize() }

// Default percentage is zero; a non-zero percentage will override the fixed size.
const val DEFAULT_CACHE_MAX_HEAP_PERCENT = 0.0
const val ENV_CACHE_MAX_HEAP_PERCENT = "MLT_CACHE_MAX_HEAP_PERCENT"
val cacheMaxHeapPercent by lazy { computeCacheMaxHeapPercent() }

const val ENV_CACHE_EXPIRE = "MLT_CACHE_EXPIRE"
val DEFAULT_CACHE_EXPIRE = 0.seconds
val cacheExpireAfterAccess by lazy { computeCacheExpireAfterAccess() }

const val ENV_THREAD_QUEUE_SIZE = "MLT_THREAD_QUEUE_SIZE"
const val DEFAULT_THREAD_QUEUE_SIZE = 10_000
val threadQueueSize by lazy { computeThreadQueueSize() }

const val ENV_CACHE_AVERAGE_WEIGHT = "MLT_CACHE_AVERAGE_SIZE"
const val DEFAULT_CACHE_AVERAGE_WEIGHT = 4 * 1024
val cacheAverageEntrySize by lazy { computeCacheAverageEntrySize() }

const val ENV_CACHE_BLOCK_SIZE = "MLT_CACHE_BLOCK_SIZE"
const val DEFAULT_CACHE_BLOCK_SIZE = 16 * 1024
val cacheBlockSize by lazy { computeCacheBlockSize() }

val defaultEnvResolver: (String) -> String? = { name -> System.getenv(name) }

/** Use the provided function to resolve environment variables.
 */
var envResolver = defaultEnvResolver

private fun <T> resolveConfigValue(
    name: String,
    def: T,
    parser: (String) -> T,
): T =
    try {
        envResolver(name)?.let { parser(it) } ?: def
    } catch (e: Exception) {
        logger.warn("Failed to parse {}, using default value {}", name, def, e)
        def
    }

internal fun computeCompressionRatioThreshold() =
    resolveConfigValue(ENV_COMPRESSION_RATIO_THRESHOLD, DEFAULT_COMPRESSION_RATIO_THRESHOLD, String::toDouble).also {
        if (it < 0.0) {
            throw IllegalArgumentException("Compression ratio threshold must be non-negative")
        }
    }

internal fun computeCompressionFixedThreshold() =
    resolveConfigValue(ENV_COMPRESSION_FIXED_THRESHOLD, DEFAULT_COMPRESSION_FIXED_THRESHOLD, String::toLong)

internal fun computeTileLogInterval() =
    resolveConfigValue(ENV_TILE_LOG_INTERVAL, DEFAULT_TILE_LOG_INTERVAL, String::toLong).let {
        // treat zero or less as "never"
        if (it < 1L) Long.MAX_VALUE else it
    }

internal fun computeCacheMaxSize() =
    resolveConfigValue(ENV_CACHE_MAX, DEFAULT_CACHE_MAX, String::toLong).let {
        if (it > 0L) it else throw IllegalArgumentException("Cache maximum size must be positive")
    }

internal fun computeCacheMaxHeapPercent(): Double =
    resolveConfigValue(ENV_CACHE_MAX_HEAP_PERCENT, DEFAULT_CACHE_MAX_HEAP_PERCENT, String::toDouble).also {
        if (!it.isFinite() || !(0.0 <= it && it < 100.0)) {
            throw IllegalArgumentException("Cache max heap percent must be between 0 and 100")
        }
    }

internal fun computeCacheExpireAfterAccess() =
    resolveConfigValue(ENV_CACHE_EXPIRE, DEFAULT_CACHE_EXPIRE, Duration::parse).also {
        if (it.isNegative()) {
            throw IllegalArgumentException("Cache expire duration must be non-negative")
        }
    }

internal fun computeThreadQueueSize() =
    resolveConfigValue(ENV_THREAD_QUEUE_SIZE, DEFAULT_THREAD_QUEUE_SIZE, String::toInt).also {
        if (it < 1) {
            throw IllegalArgumentException("Thread queue size must be positive")
        }
    }

internal fun computeCacheAverageEntrySize() =
    resolveConfigValue(ENV_CACHE_AVERAGE_WEIGHT, DEFAULT_CACHE_AVERAGE_WEIGHT, String::toInt).also {
        if (it < 0) {
            throw IllegalArgumentException("Cache average weight must not be negative")
        }
    }

internal fun computeCacheBlockSize() =
    resolveConfigValue(ENV_CACHE_BLOCK_SIZE, DEFAULT_CACHE_BLOCK_SIZE, String::toInt).also {
        if (it < 0) {
            throw IllegalArgumentException("Cache block size must not be negative")
        }
    }
