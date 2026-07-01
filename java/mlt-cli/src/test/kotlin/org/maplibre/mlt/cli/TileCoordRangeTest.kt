package org.maplibre.mlt.cli

import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Assertions.assertThrows
import org.junit.jupiter.api.Test

class TileCoordRangeTest {
    @Test
    fun `allows zero offset byte range`() {
        val range = ReadablePmtiles.TileCoordRange(42L, 3, ReadablePmtiles.ByteRange(0, 7))
        assertEquals(42L, range.startTileId)
        assertEquals(3, range.tileCount)
        assertEquals(0L, range.byteRange.offset)
    }

    @Test
    fun `rejects negative offset byte range`() {
        assertThrows(IllegalArgumentException::class.java) {
            ReadablePmtiles.TileCoordRange(0L, 1, ReadablePmtiles.ByteRange(-1, 7))
        }
    }

    @Test
    fun `rejects non-positive tile count`() {
        assertThrows(IllegalArgumentException::class.java) {
            ReadablePmtiles.TileCoordRange(0L, 0, ReadablePmtiles.ByteRange(0, 7))
        }
    }
}
