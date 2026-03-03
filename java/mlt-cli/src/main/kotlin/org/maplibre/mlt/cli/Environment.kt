package org.maplibre.mlt.cli

import org.slf4j.LoggerFactory
import kotlin.time.Duration
import kotlin.time.Duration.Companion.seconds

class Environment {
    companion object {
        const val ENV_COMPRESSION_RATIO_THRESHOLD = "MLT_COMPRESSION_RATIO_THRESHOLD"
        const val DEFAULT_COMPRESSION_RATIO_THRESHOLD = 0.98
        val compressionRatioThreshold =
            System.getenv(ENV_COMPRESSION_RATIO_THRESHOLD)?.let { it.toDoubleOrNull() } ?: DEFAULT_COMPRESSION_RATIO_THRESHOLD

        const val ENV_COMPRESSION_FIXED_THRESHOLD = "MLT_COMPRESSION_FIXED_THRESHOLD"
        const val DEFAULT_COMPRESSION_FIXED_THRESHOLD = 20L
        val compressionFixedThreshold =
            System.getenv(ENV_COMPRESSION_FIXED_THRESHOLD)?.let { it.toLongOrNull() } ?: DEFAULT_COMPRESSION_FIXED_THRESHOLD

        const val ENV_TILE_LOG_INTERVAL = "MLT_TILE_LOG_INTERVAL"
        const val DEFAULT_TILE_LOG_INTERVAL = 10_000UL
        val tileLogInterval = System.getenv(ENV_TILE_LOG_INTERVAL)?.let { it.toULongOrNull() } ?: DEFAULT_TILE_LOG_INTERVAL

        val DEFAULT_AVERAGE_WEIGHT = 256 * 1024

        const val DEFAULT_CACHE_MAX_HEAP_PERCENT = 20
        const val ENV_CACHE_MAX_HEAP_PERCENT = "MLT_CACHE_MAX_HEAP_PERCENT"
        val cacheMaxHeapPercent = System.getenv(ENV_CACHE_MAX_HEAP_PERCENT)?.toIntOrNull() ?: DEFAULT_CACHE_MAX_HEAP_PERCENT

        const val ENV_CACHE_EXPIRE = "MLT_CACHE_EXPIRE"
        val DEFAULT_EXPIRE_AFTER_ACCESS = 60.seconds
        val cacheExpireAfterAccess = System.getenv(ENV_CACHE_EXPIRE)?.let { Duration.parseOrNull(it) } ?: DEFAULT_EXPIRE_AFTER_ACCESS

        val DEFAULT__CACHE_AVERAGE_WEIGHT = 256 * 1024

        private val logger = LoggerFactory.getLogger(Environment::class.java)
    }
}
