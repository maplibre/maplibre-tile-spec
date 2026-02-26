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
) : ReadableTileArchive {
    val header: Pmtiles.Header

    init {
        this.header = Pmtiles.Header.fromBytes(getBytes(0, HEADER_LEN.toLong()))
    }

    @Throws(IOException::class)
    private fun getBytes(
        start: Long,
        length: Long,
    ): ByteArray = channel.readRange(start, Math.toIntExact(length)).array()

    override fun getTile(
        x: Int,
        y: Int,
        z: Int,
    ): ByteArray? {
        val tileRange = getTileRange(x, y, z)
        if (tileRange == null) {
            return null
        }
        try {
            return getBytes(tileRange.offset(), tileRange.length().toLong())
        } catch (e: IOException) {
            throw IllegalStateException("Could not get tile", e)
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
                var dirBytes = getBytes(dirOffset, dirLength.toLong())
                if (header.internalCompression() == Pmtiles.Compression.GZIP) {
                    dirBytes = Gzip.gunzip(dirBytes)
                }

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

    @get:Throws(IOException::class)
    val jsonMetadata: Pmtiles.JsonMetadata
        get() {
            var buf =
                getBytes(header.jsonMetadataOffset(), header.jsonMetadataLength().toInt().toLong())
            if (header.internalCompression() == Pmtiles.Compression.GZIP) {
                buf = Gzip.gunzip(buf)
            }
            return Pmtiles.JsonMetadata.fromBytes(buf)
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

    private fun readDir(
        offset: Long,
        length: Long,
    ): MutableList<Pmtiles.Entry> {
        try {
            var buf = getBytes(offset, length)
            if (header.internalCompression() == Pmtiles.Compression.GZIP) {
                buf = Gzip.gunzip(buf)
            }
            return Pmtiles.directoryFromBytes(buf)
        } catch (e: IOException) {
            throw UncheckedIOException(e)
        }
    }

    // Warning: this will only work on z15 or less pmtiles which planetiler creates
    private fun getTileCoords(dir: List<Pmtiles.Entry>): List<List<TileCoord>> =
        dir
            .flatMap { entry: Pmtiles.Entry ->
                if (entry.runLength() == 0) {
                    getTileCoords(
                        readDir(
                            header.leafDirectoriesOffset() + entry.offset(),
                            entry.length().toLong(),
                        ),
                    )
                } else {
                    listOf(
                        (entry.tileId()..entry.tileId() + entry.runLength())
                            .map { encoded ->
                                TileCoord.hilbertDecode(
                                    encoded.toInt(),
                                )
                            }.toList(),
                    )
                }
            }

    override fun getAllTileCoords(): CloseableIterator<TileCoord> = throw NotImplementedException()

    val allTileCoordRanges: Iterable<List<TileCoord>>
        get() = getTileCoords(readDir(header.rootDirOffset(), header.rootDirLength()))

    override fun getAllTiles(): CloseableIterator<Tile> = throw NotImplementedException()

    @Throws(IOException::class)
    override fun close() {
        channel.close()
    }

    companion object {
        const val HEADER_LEN: Int = 127 // Pmtiles.HEADER_LEN is inaccessible

        /**
         * Finds the relevant entry for a tileId in a list of entries.
         *
         * If there is an exact match for tileId, return that. Else if the tileId matches an entry's
         * tileId + runLength, return that. Else if the preceding entry is a directory (runLength = 0),
         * return that. Else return null.
         */
        fun findTile(
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
