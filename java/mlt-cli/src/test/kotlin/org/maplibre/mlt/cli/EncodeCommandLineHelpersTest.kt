package org.maplibre.mlt.cli

import org.apache.logging.log4j.Level
import org.junit.jupiter.api.Assertions.assertArrayEquals
import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Assertions.assertNotNull
import org.junit.jupiter.api.Assertions.assertTrue
import org.junit.jupiter.api.Test

/** Tests for the helper extension properties on `CommandLine` defined in `EncodeCommandLine.kt` */
class EncodeCommandLineHelpersTest {
    @Test
    fun `basic parsing of helper properties`() {
        val args =
            arrayOf(
                "--mvt",
                "input.mvt",
                "--minzoom",
                "2",
                "--maxzoom",
                "10",
                "--parallel",
                "3",
                "--verbose",
                "debug",
                "--sort-ids",
                "^foo$",
                "--regen-ids",
                "bar",
                "--outlines",
                "one,two",
                "--enable-fsst",
                "--enable-fsst-native",
                "--tessellateurl",
                "http://example/",
                "--compress",
                "gzip",
                "--coerce-mismatch",
                "--elide-mismatch",
                "--filter-layers",
                "name.*",
                "--filter-layers-invert",
            )

        val cmd = EncodeCommandLine.getCommandLine(args)
        assertNotNull(cmd)
        cmd!!

        assertEquals(2, cmd.minZoom)
        assertEquals(10, cmd.maxZoom)
        assertEquals(3, cmd.threadCount)
        assertEquals(Level.DEBUG, cmd.logLevel)

        assertEquals("^foo$", cmd.sortFeaturesPattern?.pattern())
        assertEquals("bar", cmd.regenIDsPattern?.pattern())

        val outlines = cmd.outlineFeatureTables
        assertNotNull(outlines)
        assertArrayEquals(arrayOf("one", "two"), outlines)

        assertTrue(cmd.useFSSTJava)
        assertTrue(cmd.useFSSTNative)

        assertEquals("http://example/", cmd.tessellateSource)
        assertTrue(cmd.tessellatePolygons)

        assertEquals("gzip", cmd.compressionType)

        assertTrue(cmd.enableCoerceOnTypeMismatch)
        assertTrue(cmd.enableElideOnTypeMismatch)

        assertEquals("name.*", cmd.filterRegex)
        assertEquals("name.*", cmd.filterPattern?.pattern())
        assertTrue(cmd.filterInvert)
    }

    @Test
    fun `cache stats forces debug log level`() {
        val args = arrayOf("--mvt", "input.mvt", "--cache-stats")
        val cmd = EncodeCommandLine.getCommandLine(args)
        assertNotNull(cmd)
        cmd!!
        assertEquals(Level.DEBUG, cmd.logLevel)
    }

    @Test
    fun `thread count default when not present`() {
        val args = arrayOf("--mvt", "input.mvt")
        val cmd = EncodeCommandLine.getCommandLine(args)
        assertNotNull(cmd)
        cmd!!
        assertEquals(1, cmd.threadCount)
    }

    @Test
    fun `thread count with explicit value`() {
        val args = arrayOf("--mvt", "input.mvt", "--parallel", "2")
        val cmd = EncodeCommandLine.getCommandLine(args)
        assertNotNull(cmd)
        cmd!!
        assertEquals(2, cmd.threadCount)
    }
}
