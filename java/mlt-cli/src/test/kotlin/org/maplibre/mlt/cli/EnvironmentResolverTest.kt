package org.maplibre.mlt.cli

import org.apache.logging.log4j.Level
import org.apache.logging.log4j.core.config.Configurator
import org.junit.jupiter.api.AfterEach
import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Assertions.assertNull
import org.junit.jupiter.api.Assertions.assertThrows
import org.junit.jupiter.api.BeforeEach
import org.junit.jupiter.api.Test
import kotlin.time.Duration.Companion.seconds

class EnvironmentResolverTest {
    var savedResolver = envResolver

    @BeforeEach
    fun setUp() {
        savedResolver = envResolver
        Configurator.setLevel(logger.name, Level.ERROR) // suppress expected warning logs
    }

    @AfterEach
    fun tearDown() {
        envResolver = savedResolver
    }

    @Test
    fun `override compression ratio with valid value`() {
        envResolver = { name -> if (name == ENV_COMPRESSION_RATIO_THRESHOLD) "0.5" else null }
        assertEquals(0.5, computeCompressionRatioThreshold())
    }

    @Test
    fun `override compression ratio with invalid negative throws`() {
        envResolver = { name -> if (name == ENV_COMPRESSION_RATIO_THRESHOLD) "-1" else null }
        assertThrows(illegalArgType) {
            computeCompressionRatioThreshold()
        }.andContains("non-negative")
    }

    /** When the parser throws (invalid format), resolveConfigValue should return the default */
    @Test
    fun `parser throws returns default`() {
        Configurator.setLevel(logger.name, Level.ERROR) // suppress expected warning logs;
        envResolver = { name -> if (name == ENV_COMPRESSION_RATIO_THRESHOLD) "not-a-number" else null }
        assertEquals(DEFAULT_COMPRESSION_RATIO_THRESHOLD, computeCompressionRatioThreshold())
    }

    @Test
    fun `computeCompressionFixedThreshold success`() {
        envResolver = { name -> if (name == ENV_COMPRESSION_FIXED_THRESHOLD) "123" else null }
        assertEquals(123L, computeCompressionFixedThreshold())
    }

    @Test
    fun `computeCompressionFixedThreshold parser failure returns default`() {
        envResolver = { name -> if (name == ENV_COMPRESSION_FIXED_THRESHOLD) "bad" else null }
        assertEquals(DEFAULT_COMPRESSION_FIXED_THRESHOLD, computeCompressionFixedThreshold())
    }

    @Test
    fun `computeTileLogInterval success`() {
        envResolver = { name -> if (name == ENV_TILE_LOG_INTERVAL) "12345" else null }
        assertEquals(12345UL, computeTileLogInterval())
    }

    @Test
    fun `computeTileLogInterval parser failure returns default`() {
        envResolver = { name -> if (name == ENV_TILE_LOG_INTERVAL) "bad" else null }
        assertEquals(DEFAULT_TILE_LOG_INTERVAL, computeTileLogInterval())
    }

    @Test
    fun `computeTileLogInterval zero treated as never`() {
        envResolver = { name -> if (name == ENV_TILE_LOG_INTERVAL) "0" else null }
        assertEquals(ULong.MAX_VALUE, computeTileLogInterval())
    }

    @Test
    fun `cache expire accepts ISO duration`() {
        envResolver = { name -> if (name == ENV_CACHE_EXPIRE) "PT20S" else null }
        assertEquals(20.seconds, computeCacheExpireAfterAccess())
    }

    @Test
    fun `cache expire negative throws`() {
        envResolver = { name -> if (name == ENV_CACHE_EXPIRE) "-PT1S" else null }
        assertThrows(illegalArgType) { computeCacheExpireAfterAccess() }.andContains("non-negative")
    }

    @Test
    fun `thread queue size positive`() {
        envResolver = { name -> if (name == ENV_THREAD_QUEUE_SIZE) "5" else null }
        assertEquals(5, computeThreadQueueSize())
    }

    @Test
    fun `thread queue size invalid throws`() {
        envResolver = { name -> if (name == ENV_THREAD_QUEUE_SIZE) "0" else null }
        assertThrows(illegalArgType) { computeThreadQueueSize() }.andContains("positive")
    }

    @Test
    fun `cache max heap valid`() {
        envResolver = { name -> if (name == ENV_CACHE_MAX_HEAP) "12345" else null }
        assertEquals(12345UL, computeCacheMaxHeap())
    }

    @Test
    fun `cache max heap zero produces null`() {
        envResolver = { name -> if (name == ENV_CACHE_MAX_HEAP) "0" else null }
        assertNull(computeCacheMaxHeap())
    }

    @Test
    fun `cache max heap too large throws`() {
        // larger than Long.MAX_VALUE
        val big = (Long.MAX_VALUE.toULong() + 1UL).toString()
        envResolver = { name -> if (name == ENV_CACHE_MAX_HEAP) big else null }
        assertThrows(illegalArgType) { computeCacheMaxHeap() }.andContains("at most")
    }

    @Test
    fun `cache max heap percent valid range accepts valid`() {
        envResolver = { name -> if (name == ENV_CACHE_MAX_HEAP_PERCENT) "99.9" else null }
        assertEquals(99.9, computeCacheMaxHeapPercent())
    }

    @Test
    fun `cache max heap percent invalid throws`() {
        envResolver = { name -> if (name == ENV_CACHE_MAX_HEAP_PERCENT) "-1" else null }
        assertThrows(illegalArgType) { computeCacheMaxHeapPercent() }.andContains("between 0 and 100")
        envResolver = { name -> if (name == ENV_CACHE_MAX_HEAP_PERCENT) "100" else null }
        assertThrows(illegalArgType) { computeCacheMaxHeapPercent() }.andContains("between 0 and 100")
    }

    @Test
    fun `cache max heap percent detects special values`() {
        envResolver = { name -> if (name == ENV_CACHE_MAX_HEAP_PERCENT) "NaN" else null }
        assertThrows(illegalArgType) { computeCacheMaxHeapPercent() }.andContains("between 0 and 100")
        envResolver = { name -> if (name == ENV_CACHE_MAX_HEAP_PERCENT) "Infinity" else null }
        assertThrows(illegalArgType) { computeCacheMaxHeapPercent() }.andContains("between 0 and 100")
        envResolver = { name -> if (name == ENV_CACHE_MAX_HEAP_PERCENT) "-Infinity" else null }
        assertThrows(illegalArgType) { computeCacheMaxHeapPercent() }.andContains("between 0 and 100")
    }

    @Test
    fun `computeCacheAverageEntrySize success`() {
        envResolver = { name -> if (name == ENV_CACHE_AVERAGE_WEIGHT) "2048" else null }
        assertEquals(2048, computeCacheAverageEntrySize())
    }

    @Test
    fun `computeCacheAverageEntrySize parser failure returns default`() {
        envResolver = { name -> if (name == ENV_CACHE_AVERAGE_WEIGHT) "x" else null }
        assertEquals(DEFAULT_CACHE_AVERAGE_WEIGHT, computeCacheAverageEntrySize())
    }

    @Test
    fun `computeCacheAverageEntrySize negative throws`() {
        envResolver = { name -> if (name == ENV_CACHE_AVERAGE_WEIGHT) "-1" else null }
        assertThrows(illegalArgType) { computeCacheAverageEntrySize() }.andContains("not be negative")
    }

    private val illegalArgType = IllegalArgumentException::class.java
}
