package org.maplibre.mlt.cli

import org.apache.commons.cli.DefaultParser
import org.apache.commons.cli.Option
import org.apache.commons.cli.Options
import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Assertions.assertFalse
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
        val args = arrayOf("--${EncodeCommandLine.COLUMN_MAPPING_DELIM_OPTION}", "[roads]name:")
        val cmd = DefaultParser().parse(options, args)
        val cfg = EncodeCommandLine.getColumnMappings(cmd)

        assertEquals(1, cfg.size)
        val entry = cfg.entries.first()
        // layer discriminator should create a literal pattern for 'roads'
        assertEquals("roads", entry.key.pattern())
        val m = entry.value[0]
        assertFalse(m.hasColumnNames())
        assertEquals("name", m.prefix.pattern())
        assertEquals(":", m.delimiter.pattern())
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
}
