package org.maplibre.mlt.cli

import com.onthegomap.planetiler.archive.ReadableTileArchive
import com.onthegomap.planetiler.archive.Tile
import com.onthegomap.planetiler.archive.TileArchiveMetadata
import com.onthegomap.planetiler.archive.TileCompression
import com.onthegomap.planetiler.archive.TileFormat
import com.onthegomap.planetiler.geo.TileCoord
import com.onthegomap.planetiler.pmtiles.Pmtiles
import com.onthegomap.planetiler.util.CloseableIterator
import com.onthegomap.planetiler.util.Gzip
import io.tileverse.io.ByteRange
import io.tileverse.rangereader.RangeReader
import org.apache.commons.lang3.NotImplementedException
import org.locationtech.jts.geom.Coordinate
import java.io.IOException
import java.io.UncheckedIOException

/** Thread-safe partial copy of Planetiler's Pmtiles reader.
 * Also exposes run-length encoded tile coordinate ranges
 * which are not currently exposed by Planetiler's API. */
class ReadablePmtiles(
    private val channel: RangeReader,
    private val closeSourceChannel: Boolean = true,
) : ReadableTileArchive {
    private fun getBytes(
        start: Long,
        length: Long,
    ) = channel.readRange(start, Math.toIntExact(length)).array()

    fun getBytes(range: ByteRange) = getBytes(range.offset(), range.length().toLong())

    private fun getHeaderBytes(
        offset: Long,
        length: Long,
    ) = getBytes(offset, length).let {
        if (header.internalCompression() == Pmtiles.Compression.GZIP) {
            Gzip.gunzip(it)
        } else {
            it
        }
    }

    fun getTileRange(
        x: Int,
        y: Int,
        z: Int,
    ): ByteRange? {
        try {
            val tileId = TileCoord.ofXYZ(x, y, z).hilbertEncoded()

            var dirOffset = header.rootDirOffset()
            var dirLength = header.rootDirLength().toInt()

            for (depth in 0..3) {
                val dirBytes = getHeaderBytes(dirOffset, dirLength.toLong())
                val dir = Pmtiles.directoryFromBytes(dirBytes)
                val entry = findTile(dir, tileId.toLong())
                if (entry != null) {
                    if (entry.runLength() > 0) {
                        return ByteRange(header.tileDataOffset() + entry.offset(), entry.length())
                    } else {
                        dirOffset = header.leafDirectoriesOffset() + entry.offset()
                        dirLength = entry.length()
                    }
                } else {
                    return null
                }
            }
        } catch (e: IOException) {
            throw IllegalStateException("Could not get tile", e)
        }

        return null
    }

    override fun getAllTileCoords(): CloseableIterator<TileCoord> = throw NotImplementedException()

    override fun getAllTiles(): CloseableIterator<Tile> = throw NotImplementedException()

    override fun getTile(
        x: Int,
        y: Int,
        z: Int,
    ) = getTileRange(x, y, z)?.let(::getBytes)

    override fun close() {
        if (closeSourceChannel) {
            channel.close()
        }
    }

    val jsonMetadata by lazy {
        Pmtiles.JsonMetadata.fromBytes(getHeaderBytes(header.jsonMetadataOffset(), header.jsonMetadataLength().toInt().toLong()))
    }

    override fun metadata(): TileArchiveMetadata {
        val tileCompression =
            when (header.tileCompression()) {
                Pmtiles.Compression.GZIP -> TileCompression.GZIP
                Pmtiles.Compression.NONE -> TileCompression.NONE
                Pmtiles.Compression.UNKNOWN -> TileCompression.UNKNOWN
            }

        val format =
            when (header.tileType()) {
                Pmtiles.TileType.MVT -> TileFormat.MVT
                Pmtiles.TileType.MLT -> TileFormat.MLT
                else -> null
            }

        try {
            val jsonMetadata = this.jsonMetadata
            val map = LinkedHashMap<String, String?>(jsonMetadata.otherMetadata())
            return TileArchiveMetadata(
                map.remove(TileArchiveMetadata.NAME_KEY),
                map.remove(TileArchiveMetadata.DESCRIPTION_KEY),
                map.remove(TileArchiveMetadata.ATTRIBUTION_KEY),
                map.remove(TileArchiveMetadata.VERSION_KEY),
                map.remove(TileArchiveMetadata.TYPE_KEY),
                format,
                header.bounds(),
                Coordinate(
                    header.center().getX(),
                    header.center().getY(),
                    header.centerZoom().toDouble(),
                ),
                header.minZoom().toInt(),
                header.maxZoom().toInt(),
                TileArchiveMetadata.TileArchiveMetadataJson.create(jsonMetadata.vectorLayers()),
                map,
                tileCompression,
            )
        } catch (e: IOException) {
            throw UncheckedIOException(e)
        }
    }

    private fun readDir(entry: Pmtiles.Entry) = readDir(header.leafDirectoriesOffset() + entry.offset(), entry.length().toLong())

    private fun readDir(
        offset: Long,
        length: Long,
    ) = Pmtiles.directoryFromBytes(getHeaderBytes(offset, length))

    data class TileCoordRange(
        val startTileId: Long,
        val tileCount: Int,
        val byteRange: ByteRange,
    ) {
        constructor(entry: Pmtiles.Entry, header: Pmtiles.Header) : this(
            entry.tileId(),
            entry.runLength(),
            ByteRange(header.tileDataOffset() + entry.offset(), entry.length()),
        )

        init {
            if (tileCount < 1) {
                throw IllegalArgumentException("tileCount must be positive")
            }
            if (byteRange.offset < 0) {
                throw IllegalArgumentException("byteRange offset must be non-negative")
            }
            if (byteRange.length < 1) {
                throw IllegalArgumentException("byteRange length must be positive")
            }
        }

        val tileIds get() = (startTileId until startTileId + tileCount)

        // Warning: this will only work on z15 or less pmtiles which planetiler creates
        // TODO: Will extending `hilbertDecode` to Longs solve this?
        val tileCoords get() = tileIds.map { TileCoord.hilbertDecode(it.toInt()) }
    }

    private fun getTileCoordRanges(dir: Iterable<Pmtiles.Entry>): Sequence<TileCoordRange> =
        dir
            .asSequence()
            .flatMap<Pmtiles.Entry, TileCoordRange> { entry ->
                if (entry.runLength() == 0) {
                    getTileCoordRanges(readDir(entry))
                } else {
                    sequenceOf(TileCoordRange(entry, header))
                }
            }

    val allTileCoordRanges get() = getTileCoordRanges(rootDir)

    val header by lazy { Pmtiles.Header.fromBytes(getBytes(0, HEADER_LEN.toLong())) }

    private val rootDir by lazy { readDir(header.rootDirOffset(), header.rootDirLength()) }

    companion object {
        const val HEADER_LEN: Int = 127 // Pmtiles.HEADER_LEN is inaccessible

        /**
         * Finds the relevant entry for a tileId in a list of entries.
         *
         * If there is an exact match for tileId, return that. Else if the tileId matches an entry's
         * tileId + runLength, return that. Else if the preceding entry is a directory (runLength = 0),
         * return that. Else return null.
         */
        private fun findTile(
            entries: List<Pmtiles.Entry>,
            tileId: Long,
        ): Pmtiles.Entry? {
            var m = 0
            var n = entries.size - 1
            while (m <= n) {
                val k = (n + m) shr 1
                val entry = entries.get(k)
                val cmp = tileId - entry.tileId()
                if (cmp > 0) {
                    m = k + 1
                } else if (cmp < 0) {
                    n = k - 1
                } else {
                    return entry
                }
            }
            if (n >= 0) {
                val entry = entries.get(n)
                if ((entry.runLength() == 0 || tileId - entry.tileId() < entry.runLength())) {
                    return entry
                }
            }
            return null
        }
    }
}
