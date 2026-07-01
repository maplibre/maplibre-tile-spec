package org.maplibre.mlt.cli

import com.onthegomap.planetiler.archive.TileArchiveMetadata
import com.onthegomap.planetiler.archive.TileCompression
import com.onthegomap.planetiler.archive.TileFormat
import com.onthegomap.planetiler.geo.TileCoord
import com.onthegomap.planetiler.pmtiles.Pmtiles
import com.onthegomap.planetiler.util.Gzip
import org.locationtech.jts.geom.Coordinate
import java.io.IOException
import java.io.UncheckedIOException

/** Potentially thread-safe partial copy of Planetiler's Pmtiles reader.
 * This class is thread-safe as long as the provided DataReader is.
 * Also exposes run-length encoded tile coordinate ranges
 * which are not currently exposed by Planetiler's API. */
class ReadablePmtiles(
    private val channel: DataReader,
    private val closeSourceChannel: Boolean = true,
) : AutoCloseable {
    data class ByteRange(
        /** The starting position of the range */
        val offset: Long,
        /** The number of bytes in the range */
        val length: Int,
    ) {
        init {
            if (offset < 0) {
                throw IllegalArgumentException("ByteRange offset must be non-negative")
            }
            if (length < 1) {
                throw IllegalArgumentException("ByteRange length must be positive")
            }
        }

        override fun equals(other: Any?) = this === other || (other is ByteRange && offset == other.offset && length == other.length)
    }

    interface DataReader : AutoCloseable {
        fun read(
            offset: Long,
            length: Int,
        ): ByteArray

        override fun close() {}
    }

    private fun getBytes(
        start: Long,
        length: Long,
    ) = channel.read(start, Math.toIntExact(length))

    fun getBytes(range: ByteRange) = getBytes(range.offset, range.length.toLong())

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

    fun getTile(
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
        Pmtiles.JsonMetadata.fromBytes(getHeaderBytes(header.jsonMetadataOffset, header.jsonMetadataLength))
    }

    fun metadata(): TileArchiveMetadata {
        val tileCompression =
            when (header.tileCompression) {
                Pmtiles.Compression.GZIP -> TileCompression.GZIP
                Pmtiles.Compression.NONE -> TileCompression.NONE
                Pmtiles.Compression.UNKNOWN -> TileCompression.UNKNOWN
            }

        val format =
            when (header.tileType) {
                Pmtiles.TileType.MVT -> TileFormat.MVT
                Pmtiles.TileType.MLT -> TileFormat.MLT
                else -> null
            }

        try {
            val jsonMetadata = this.jsonMetadata
            val map = LinkedHashMap<String, String?>(jsonMetadata.otherMetadata)
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
                    header.centerZoom.toDouble(),
                ),
                header.minZoom.toInt(),
                header.maxZoom.toInt(),
                TileArchiveMetadata.TileArchiveMetadataJson.create(jsonMetadata.vectorLayers),
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

    fun getTileCoordRanges(
        minZoom: Int? = null,
        maxZoom: Int? = null,
    ) = getTileCoordRanges(rootDir, minZoom?.let(::zoomStartIndex) ?: 0, maxZoom?.let(::zoomEndIndex) ?: Long.MAX_VALUE)

    /** Generate tile ranges for the Hilbert given tile indexes (half-closed range)
     * @param dir the directory to search
     * @param startTileIndex the first tile index to include
     * @param endTileIndex the first tile index to exclude
     * */
    private fun getTileCoordRanges(
        dir: Iterable<Pmtiles.Entry>,
        startTileIndex: Long,
        endTileIndex: Long,
    ): Sequence<TileCoordRange> =
        dir
            .asSequence()
            .flatMap<Pmtiles.Entry, TileCoordRange> { entry ->
                mapDirectory(entry, startTileIndex, endTileIndex, header) {
                    getTileCoordRanges(readDir(entry), startTileIndex, endTileIndex)
                }
            }

    /** Zoom start index, sum of previous tile counts */
    private fun zoomStartIndex(zoom: Int) = ZOOM_START_INDEX[zoom.coerceIn(0..MAX_ZOOM)]

    /** Zoom end index, first index of the next zoom */
    private fun zoomEndIndex(zoom: Int) = ZOOM_START_INDEX[zoom.coerceIn(0..MAX_ZOOM) + 1]

    val header by lazy { Pmtiles.Header.fromBytes(getBytes(0, HEADER_LEN.toLong())) }

    private val rootDir by lazy { readDir(header.rootDirOffset(), header.rootDirLength()) }

    companion object {
        const val HEADER_LEN: Int = 127 // Pmtiles.HEADER_LEN is inaccessible

        const val MAX_ZOOM = 16 // Planetiler currently only supports 15

        private val ZOOM_START_INDEX = buildZoomIndex()

        private fun buildZoomIndex(): LongArray {
            val indexes = LongArray(MAX_ZOOM + 2)
            var index = 0L
            (0..MAX_ZOOM + 1).forEach {
                indexes[it] = index
                val width = (1L shl it)
                index += width * width
                if (index < indexes[it]) {
                    throw IllegalStateException("Too many zoom levels")
                }
            }
            return indexes
        }

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

        internal fun mapDirectory(
            entry: Pmtiles.Entry,
            startTileIndex: Long,
            endTileIndex: Long,
            header: Pmtiles.Header,
            recurse: (entry: Pmtiles.Entry) -> Sequence<TileCoordRange>,
        ): Sequence<TileCoordRange> {
            // Run-length zero is a directory reference
            if (entry.runLength() == 0) {
                // continue exploring this branch?
                if (entry.tileId() < endTileIndex) {
                    return recurse(entry)
                }
            } else {
                // Run-length non-zero is a tile range
                var rangeEnd = entry.tileId() + entry.runLength()
                if (startTileIndex <= entry.tileId() && rangeEnd <= endTileIndex) {
                    // use the full range
                    return sequenceOf(TileCoordRange(entry, header))
                } else if (entry.tileId() < endTileIndex && startTileIndex < rangeEnd) {
                    // use a partial range
                    val start = entry.tileId().coerceAtLeast(startTileIndex)
                    val end = (entry.tileId() + entry.runLength()).coerceAtMost(endTileIndex)
                    return sequenceOf(
                        TileCoordRange(
                            start,
                            (end - start).toInt(),
                            ByteRange(header.tileDataOffset() + entry.offset(), entry.length()),
                        ),
                    )
                }
            }
            return sequenceOf()
        }
    }
}
