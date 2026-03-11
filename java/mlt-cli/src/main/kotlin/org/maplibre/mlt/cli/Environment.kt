package org.maplibre.mlt.cli

import kotlin.time.Duration
import kotlin.time.Duration.Companion.seconds

const val ENV_COMPRESSION_RATIO_THRESHOLD = "MLT_COMPRESSION_RATIO_THRESHOLD"
const val DEFAULT_COMPRESSION_RATIO_THRESHOLD = 0.98
val compressionRatioThreshold by lazy {
    resolveConfigValue(ENV_COMPRESSION_RATIO_THRESHOLD, DEFAULT_COMPRESSION_RATIO_THRESHOLD, String::toDouble).also {
        if (it < 0.0) {
            throw IllegalArgumentException("Compression ratio threshold must be non-negative")
        }
    }
}

const val ENV_COMPRESSION_FIXED_THRESHOLD = "MLT_COMPRESSION_FIXED_THRESHOLD"
const val DEFAULT_COMPRESSION_FIXED_THRESHOLD = 20L

val compressionFixedThreshold by lazy {
    resolveConfigValue(ENV_COMPRESSION_FIXED_THRESHOLD, DEFAULT_COMPRESSION_FIXED_THRESHOLD, String::toLong)
}

const val ENV_TILE_LOG_INTERVAL = "MLT_TILE_LOG_INTERVAL"
const val DEFAULT_TILE_LOG_INTERVAL = 10_000UL
val tileLogInterval by lazy {
    resolveConfigValue(ENV_TILE_LOG_INTERVAL, DEFAULT_TILE_LOG_INTERVAL, String::toULong).let {
        // treat zero as "never"
        if (it < 1UL) ULong.MAX_VALUE else it
    }
}

const val ENV_CACHE_MAX_HEAP = "MLT_CACHE_MAX_HEAP"
const val DEFAULT_CACHE_MAX_HEAP = 0UL
val cacheMaxHeap by lazy {
    resolveConfigValue(ENV_CACHE_MAX_HEAP, DEFAULT_CACHE_MAX_HEAP, String::toULong).let {
        if (it > Long.MAX_VALUE.toULong()) {
            // Cache API uses `Long`, so don't let it go negative
            throw IllegalArgumentException("Cache maximum heap size must be at most $Long.MAX_VALUE")
        }
        if (it > 0UL) it else null
    }
}

const val DEFAULT_CACHE_MAX_HEAP_PERCENT = 20.0
const val ENV_CACHE_MAX_HEAP_PERCENT = "MLT_CACHE_MAX_HEAP_PERCENT"
val cacheMaxHeapPercent by lazy {
    resolveConfigValue(ENV_CACHE_MAX_HEAP_PERCENT, DEFAULT_CACHE_MAX_HEAP_PERCENT, String::toDouble).also {
        if (!it.isFinite() || !(0.0 <= it && it <= 100.0)) {
            throw IllegalArgumentException("Cache max heap percent must be between 0 and 100")
        }
    }
}

const val ENV_CACHE_EXPIRE = "MLT_CACHE_EXPIRE"
val DEFAULT_CACHE_EXPIRE = 10.seconds
val cacheExpireAfterAccess by lazy {
    resolveConfigValue(ENV_CACHE_EXPIRE, DEFAULT_CACHE_EXPIRE, Duration::parse).also {
        if (it.isNegative()) {
            throw IllegalArgumentException("Cache expire duration must be non-negative")
        }
    }
}

const val ENV_THREAD_QUEUE_SIZE = "MLT_THREAD_QUEUE_SIZE"
const val DEFAULT_THREAD_QUEUE_SIZE = 10_000
val threadQueueSize by lazy {
    resolveConfigValue(ENV_THREAD_QUEUE_SIZE, DEFAULT_THREAD_QUEUE_SIZE, String::toInt).also {
        if (it < 1) {
            throw IllegalArgumentException("Thread queue size must be positive")
        }
    }
}

const val ENV_PMTILES_DEDUP = "MLT_PMTILES_DEDUP"
const val DEFAULT_PMTILES_DEDUP = false
val pmTilesDedup by lazy {
    resolveConfigValue(ENV_PMTILES_DEDUP, DEFAULT_PMTILES_DEDUP) {
        // Accept empty (just defined), true, or non-zero to enable
        it.isEmpty() || it.toBoolean() || (it.toIntOrNull() ?: 0) > 0
    }
}

/** Parse using the provided function, use default on null or error */
private fun <T> resolveConfigValue(
    name: String,
    def: T,
    parser: (String) -> T,
): T =
    try {
        System.getenv(name)?.let { parser(it) } ?: def
    } catch (e: Exception) {
        logger.warn("Failed to parse {}, using default value {}", name, def, e)
        def
    }

const val DEFAULT_CACHE_AVERAGE_WEIGHT = 256 * 1024
