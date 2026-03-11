package org.maplibre.mlt.cli

import org.apache.commons.cli.DefaultParser
import org.apache.commons.cli.Option
import org.apache.commons.cli.Options
import org.apache.logging.log4j.Level
import org.junit.jupiter.api.Assertions.assertArrayEquals
import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Assertions.assertFalse
import org.junit.jupiter.api.Assertions.assertNotNull
import org.junit.jupiter.api.Assertions.assertTrue
import org.junit.jupiter.api.Test

class EncodeCommandLineTest {
    @Test
    fun `getColumnMappings list`() {
        val options = Options()
        options.addOption(
            Option
                .builder()
                .longOpt(EncodeCommandLine.COLUMN_MAPPING_LIST_OPTION)
                .hasArg(true)
                .argName("map")
                .get(),
        )
        val args = arrayOf("--${EncodeCommandLine.COLUMN_MAPPING_LIST_OPTION}", "[]name,name:de")
        val cmd = DefaultParser().parse(options, args)
        val cfg = EncodeCommandLine.getColumnMappings(cmd)

        assertEquals(1, cfg.size)
        val entry = cfg.entries.first()
        // empty layer list ([]) should match all
        assertEquals(".*", entry.key.pattern())
        val mappings = entry.value
        assertEquals(1, mappings.size)
        val m = mappings[0]
        assertTrue(m.hasColumnNames())
        assertTrue(m.columnNames.contains("name"))
        assertTrue(m.columnNames.contains("name:de"))
    }

    @Test
    fun `getColumnMappings delimited`() {
        val options = Options()
        options.addOption(
            Option
                .builder()
                .longOpt(EncodeCommandLine.COLUMN_MAPPING_DELIM_OPTION)
                .hasArg(true)
                .argName("map")
                .get(),
        )
        val args = arrayOf("--${EncodeCommandLine.COLUMN_MAPPING_DELIM_OPTION}", "[roads]name/[:_]/")
        val cmd = DefaultParser().parse(options, args)
        val cfg = EncodeCommandLine.getColumnMappings(cmd)

        assertEquals(1, cfg.size)
        val entry = cfg.entries.first()
        // layer discriminator should create a literal pattern for 'roads'
        assertEquals("roads", entry.key.pattern())
        val m = entry.value[0]
        assertFalse(m.hasColumnNames())
        assertEquals("name", m.prefix.pattern())
        assertEquals("[:_]", m.delimiter.pattern())
    }

    @Test
    fun `getAllowedCompressions Pmtiles`() {
        val options =
            Options().addOption(
                Option
                    .builder()
                    .longOpt(EncodeCommandLine.INPUT_PMTILES_ARG)
                    .hasArg(true)
                    .get(),
            )
        val cmd = DefaultParser().parse(options, arrayOf("--${EncodeCommandLine.INPUT_PMTILES_ARG}", "input.pmtiles"))
        val list = EncodeCommandLine.getAllowedCompressions(cmd).toList()
        // should contain the standard compressors plus brotli/zstd when pmtiles option present
        assertTrue(list.containsAll(listOf("gzip", "deflate", "none", "brotli", "zstd")))
    }

    @Test
    fun `getAllowedCompressions not Pmtiles`() {
        val cmd = DefaultParser().parse(Options(), arrayOf<String>())
        val list = EncodeCommandLine.getAllowedCompressions(cmd).toList()
        assertTrue(list.containsAll(listOf("gzip", "deflate", "none")))
        assertFalse(list.contains("brotli") || list.contains("zstd"))
    }

    /** Tests for the helper extension properties on `CommandLine` defined in `EncodeCommandLine.kt` */
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
        val args = arrayOf("--" + EncodeCommandLine.INPUT_PMTILES_ARG, "input.pmtiles", "--" + EncodeCommandLine.CACHE_STATS_OPTION, "--" + EncodeCommandLine.VERBOSE_OPTION, "error")
        val cmd = EncodeCommandLine.getCommandLine(args)
        assertNotNull(cmd)
        cmd!!
        assertEquals(Level.DEBUG, cmd.logLevel)
    }

    @Test
    fun `log level isn't forced otherwise`() {
        val args = arrayOf("--" + EncodeCommandLine.INPUT_PMTILES_ARG, "input.pmtiles", "--" + EncodeCommandLine.VERBOSE_OPTION, "error")
        val cmd = EncodeCommandLine.getCommandLine(args)
        assertNotNull(cmd)
        cmd!!
        assertEquals(Level.ERROR, cmd.logLevel)
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
