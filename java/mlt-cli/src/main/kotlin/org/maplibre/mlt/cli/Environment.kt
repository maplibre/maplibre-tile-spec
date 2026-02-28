package org.maplibre.mlt.cli

import kotlin.time.Duration
import kotlin.time.Duration.Companion.seconds

const val ENV_COMPRESSION_RATIO_THRESHOLD = "MLT_COMPRESSION_RATIO_THRESHOLD"
const val DEFAULT_COMPRESSION_RATIO_THRESHOLD = 0.98
val compressionRatioThreshold by lazy {
    parse(ENV_COMPRESSION_RATIO_THRESHOLD, DEFAULT_COMPRESSION_RATIO_THRESHOLD, String::toDouble)
}

const val ENV_COMPRESSION_FIXED_THRESHOLD = "MLT_COMPRESSION_FIXED_THRESHOLD"
const val DEFAULT_COMPRESSION_FIXED_THRESHOLD = 20L

val compressionFixedThreshold by lazy { parse(ENV_COMPRESSION_FIXED_THRESHOLD, DEFAULT_COMPRESSION_FIXED_THRESHOLD, String::toLong) }

const val ENV_TILE_LOG_INTERVAL = "MLT_TILE_LOG_INTERVAL"
const val DEFAULT_TILE_LOG_INTERVAL = 10_000UL
val tileLogInterval by lazy { parse(ENV_TILE_LOG_INTERVAL, DEFAULT_TILE_LOG_INTERVAL, String::toULong) }

const val DEFAULT_CACHE_MAX_HEAP_PERCENT = 20
const val ENV_CACHE_MAX_HEAP_PERCENT = "MLT_CACHE_MAX_HEAP_PERCENT"
val cacheMaxHeapPercent by lazy { parse(ENV_CACHE_MAX_HEAP_PERCENT, DEFAULT_CACHE_MAX_HEAP_PERCENT, String::toInt) }

const val ENV_CACHE_EXPIRE = "MLT_CACHE_EXPIRE"
val DEFAULT_CACHE_EXPIRE = 10.seconds
val cacheExpireAfterAccess by lazy { parse(ENV_CACHE_EXPIRE, DEFAULT_CACHE_EXPIRE, Duration::parse) }

const val ENV_THREAD_QUEUE_SIZE = "MLT_THREAD_QUEUE_SIZE"
const val DEFAULT_THREAD_QUEUE_SIZE = 10_000
val threadQueueSize by lazy { parse(ENV_THREAD_QUEUE_SIZE, DEFAULT_THREAD_QUEUE_SIZE, String::toInt) }

private fun <T> parse(
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
