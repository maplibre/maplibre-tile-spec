package org.maplibre.mlt.cli

import com.onthegomap.planetiler.pmtiles.Pmtiles
import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Assertions.assertFalse
import org.junit.jupiter.api.Assertions.assertTrue
import org.junit.jupiter.api.Test
import org.mockito.kotlin.mock
import org.mockito.kotlin.whenever

/** Tests for directory traversal logic in `ReadablePmtiles` */
class ReadablePmtilesMapDirectoryTest {
    private fun mockEntry(
        tileId: Long,
        runLength: Int,
        offset: Long = 0,
        length: Int = 1,
    ): Pmtiles.Entry {
        val entry = mock<Pmtiles.Entry>()
        whenever(entry.tileId()).thenReturn(tileId)
        whenever(entry.runLength()).thenReturn(runLength)
        whenever(entry.offset()).thenReturn(offset)
        whenever(entry.length()).thenReturn(length)
        return entry
    }

    private fun mockHeader(tileDataOffset: Long = 0): Pmtiles.Header {
        val header = mock<Pmtiles.Header>()
        whenever(header.tileDataOffset()).thenReturn(tileDataOffset)
        return header
    }

    @Test
    fun `directory reference entry calls recurse when tileId less than endTileIndex`() {
        val entry = mockEntry(tileId = 5, runLength = 0)
        val header = mockHeader()
        var recurseCalled = false
        val recurse: (Pmtiles.Entry) -> Sequence<ReadablePmtiles.TileCoordRange> = {
            recurseCalled = true
            emptySequence()
        }
        val result = ReadablePmtiles.mapDirectory(entry, 0, 10, header, recurse)
        assertTrue(recurseCalled)
        assertTrue(result.toList().isEmpty())
    }

    @Test
    fun `directory reference entry does not call recurse when tileId not less than endTileIndex`() {
        val entry = mockEntry(tileId = 10, runLength = 0)
        val header = mockHeader()
        var recurseCalled = false
        val recurse: (Pmtiles.Entry) -> Sequence<ReadablePmtiles.TileCoordRange> = {
            recurseCalled = true
            emptySequence()
        }
        val result = ReadablePmtiles.mapDirectory(entry, 0, 10, header, recurse)
        assertFalse(recurseCalled)
        assertTrue(result.toList().isEmpty())
    }

    @Test
    fun `tile range entry returns full range when within bounds`() {
        val entry = mockEntry(tileId = 2, runLength = 3, offset = 10, length = 5)
        val header = mockHeader(tileDataOffset = 100)
        val recurse: (Pmtiles.Entry) -> Sequence<ReadablePmtiles.TileCoordRange> = { emptySequence() }
        val result = ReadablePmtiles.mapDirectory(entry, 0, 10, header, recurse)
        val ranges = result.toList()
        assertEquals(1, ranges.size)
        val range = ranges[0]
        assertEquals(2, range.startTileId)
        assertEquals(3, range.tileCount)
        assertEquals(ReadablePmtiles.ByteRange(110, 5), range.byteRange)
    }

    @Test
    fun `tile range entry returns partial range when overlapping bounds`() {
        val entry = mockEntry(tileId = 5, runLength = 5, offset = 20, length = 10)
        val header = mockHeader(tileDataOffset = 50)
        val recurse: (Pmtiles.Entry) -> Sequence<ReadablePmtiles.TileCoordRange> = { emptySequence() }
        val result = ReadablePmtiles.mapDirectory(entry, 7, 12, header, recurse)
        val ranges = result.toList()
        assertEquals(1, ranges.size)
        val range = ranges[0]
        assertEquals(7, range.startTileId)
        assertEquals(3, range.tileCount)
        assertEquals(ReadablePmtiles.ByteRange(70, 10), range.byteRange)
    }

    @Test
    fun `tile range entry returns empty when out of bounds below`() {
        val entry = mockEntry(tileId = 20, runLength = 2, offset = 30, length = 5)
        val header = mockHeader(tileDataOffset = 0)
        val recurse: (Pmtiles.Entry) -> Sequence<ReadablePmtiles.TileCoordRange> = { emptySequence() }
        val result = ReadablePmtiles.mapDirectory(entry, 0, 20, header, recurse)
        assertTrue(result.toList().isEmpty())
    }

    @Test
    fun `tile range entry returns empty when out of bounds above`() {
        val entry = mockEntry(tileId = 20, runLength = 2, offset = 30, length = 5)
        val header = mockHeader(tileDataOffset = 0)
        val recurse: (Pmtiles.Entry) -> Sequence<ReadablePmtiles.TileCoordRange> = { emptySequence() }
        val result = ReadablePmtiles.mapDirectory(entry, 22, 25, header, recurse)
        assertTrue(result.toList().isEmpty())
    }
}
