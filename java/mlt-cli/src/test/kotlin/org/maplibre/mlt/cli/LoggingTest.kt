package org.maplibre.mlt.cli

import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Test

class LoggingTest {
    @Test
    fun `formatSize returns zero for non-positive`() {
        assertEquals("0", formatSize(0))
        assertEquals("0", formatSize(-1))
    }

    @Test
    fun `formatSize formats bytes correctly`() {
        assertEquals("1 B", formatSize(1))
        assertEquals("1,023 B", formatSize(1023))
    }

    @Test
    fun `formatSize formats kibibytes and beyond`() {
        assertEquals("1 kiB", formatSize(1024))
        assertEquals("1.5 kiB", formatSize(1536))
        assertEquals("1 MiB", formatSize(1024L * 1024))
        assertEquals("5 GiB", formatSize(5L * 1024 * 1024 * 1024))
    }

    @Test
    fun `formatNanosecDuration formats ns, us, ms, s and zero`() {
        // nanoseconds
        assertEquals("0.50ns", formatNanosecDuration(0.5))
        assertEquals("1.00us", formatNanosecDuration(1e3))
        assertEquals("1.00ms", formatNanosecDuration(1e6))
        assertEquals("1.00s", formatNanosecDuration(1e9))
        assertEquals("2.35s", formatNanosecDuration(2.345e9))
        assertEquals("0", formatNanosecDuration(0.0))
    }
}
