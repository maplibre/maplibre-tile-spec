package org.maplibre.mlt.cli

import org.apache.commons.lang3.mutable.MutableBoolean
import org.imintel.mbtiles4j.MBTilesReadException
import org.imintel.mbtiles4j.MBTilesReader
import org.imintel.mbtiles4j.MBTilesWriteException
import org.imintel.mbtiles4j.MBTilesWriter
import org.imintel.mbtiles4j.Tile
import org.maplibre.mlt.converter.MltConverter
import org.maplibre.mlt.metadata.tileset.MltMetadata
import org.slf4j.Logger
import org.slf4j.LoggerFactory
import java.io.ByteArrayInputStream
import java.io.File
import java.io.IOException
import java.nio.file.Files
import java.nio.file.Path
import java.sql.Connection
import java.sql.DriverManager
import java.sql.SQLException
import java.util.Optional
import java.util.concurrent.atomic.AtomicBoolean
import java.util.concurrent.atomic.AtomicLong

/** Encode the entire contents of an MBTile file of MVT tiles */
fun encodeMBTiles(
    inputMBTilesPath: String,
    outputPath: Path?,
    config: EncodeConfig,
): Boolean {
    var mbTilesReader: MBTilesReader? = null
    val success = AtomicBoolean(true)
    try {
        mbTilesReader = MBTilesReader(File(inputMBTilesPath))

        // Remove any existing output file, as SQLite will add to it instead of replacing
        if (outputPath != null) {
            Files.deleteIfExists(outputPath)
        }

        // If no target file is specified, a temporary is used.
        // mbtiles4j blocks access to in-memory databases and built-in
        // temp file support, so we have to use its temp file support.
        var mbTilesWriter: MBTilesWriter? =
            if (outputPath != null) {
                MBTilesWriter(outputPath.toFile())
            } else {
                MBTilesWriter("encode")
            }
        try {
            // Copy metadata from the input file.
            // We can't set the format, see the coda.
            val metadata = mbTilesReader.getMetadata()
            mbTilesWriter!!.addMetadataEntry(metadata)

            val pbMeta = MltMetadata.TileSetMetadata()
            pbMeta.name = metadata.tilesetName
            pbMeta.attribution = metadata.attribution
            pbMeta.description = metadata.tilesetDescription

            val bounds = metadata.tilesetBounds
            pbMeta.bounds =
                listOf(
                    bounds.left,
                    bounds.top,
                    bounds.right,
                    bounds.bottom,
                )

            val metadataJSON = MltConverter.createTilesetMetadataJSON(pbMeta)

            // Read everything on the main thread for simplicity.
            // Splitting reads by zoom level might improve performance.
            val tiles = mbTilesReader.getTiles()
            val tileCount = AtomicLong(0)
            val writerCapture = mbTilesWriter
            try {
                while (tiles.hasNext()) {
                    val tile = tiles.next()
                    try {
                        if (!config.continueOnError && !success.get()) {
                            break
                        }

                        val x = tile.column
                        val y = tile.row
                        val z = tile.zoom

                        if (z < config.minZoom || z > config.maxZoom) {
                            logger.trace("Skipping tile {}:{},{} outside zoom range", z, x, y)
                            continue
                        }

                        val data = tile.data.readAllBytes()

                        config.taskRunner.run(
                            Runnable {
                                val count = tileCount.incrementAndGet().toULong()
                                if (count.mod(Environment.tileLogInterval) == 0UL) {
                                    logger.debug(
                                        "Processing tile {} : {}:{},{}",
                                        count,
                                        z,
                                        x,
                                        y,
                                    )
                                }
                                if (!convertTile(
                                        config,
                                        data,
                                        writerCapture,
                                        tile,
                                    )
                                ) {
                                    success.set(false)
                                }
                            },
                        )
                    } catch (ex: IllegalArgumentException) {
                        success.set(false)
                        logger.error(
                            "Failed to convert tile {}:{},{}",
                            tile.zoom,
                            tile.column,
                            tile.row,
                            ex,
                        )
                    }
                }

                try {
                    config.taskRunner.shutdown()
                    config.taskRunner.awaitTermination()
                } catch (ex: InterruptedException) {
                    logger.error("Interrupted", ex)
                    return false
                }

                if (!config.continueOnError && !success.get()) {
                    return false
                }

                val dbFile = mbTilesWriter.close()
                if (outputPath == null) {
                    dbFile.deleteOnExit()
                }
                mbTilesWriter = null

                // mbtiles4j doesn't support types other than png and jpg,
                // so we have to set the format metadata the hard way.
                val connectionString = "jdbc:sqlite:" + dbFile.absolutePath
                DriverManager.getConnection(connectionString).use { connection ->
                    updateMetadata(config, connection, metadataJSON)
                }
            } finally {
                // mbtiles4j doesn't support `AutoCloseable`
                tiles.close()
            }
        } finally {
            if (mbTilesWriter != null) {
                val file = mbTilesWriter.close()
                if (outputPath == null) {
                    file.delete()
                }
            }
        }
    } catch (ex: MBTilesReadException) {
        success.set(false)
        logger.error("Failed to convert MBTiles file", ex)
    } catch (ex: IOException) {
        success.set(false)
        logger.error("Failed to convert MBTiles file", ex)
    } catch (ex: MBTilesWriteException) {
        success.set(false)
        logger.error("Failed to convert MBTiles file", ex)
    } catch (ex: SQLException) {
        success.set(false)
        logger.error("Failed to convert MBTiles file", ex)
    } finally {
        if (mbTilesReader != null) {
            mbTilesReader.close()
        }
    }
    return success.get()
}

@Throws(SQLException::class)
private fun updateMetadata(
    config: EncodeConfig,
    connection: Connection,
    metadataJSON: String,
) {
    logger.debug("Updating metadata")
    logger.trace("Setting tile MIME type to '{}'", MBTILES_METADATA_MIME_TYPE)

    var sql = "UPDATE metadata SET value = ? WHERE name = ?"
    connection.prepareStatement(sql).use { statement ->
        statement.setString(1, MBTILES_METADATA_MIME_TYPE)
        statement.setString(2, "format")
        statement.execute()
    }
    // Put the global metadata in a custom metadata key.
    // Could also be in a custom key within the standard `json` entry...
    logger.trace("Adding tileset metadata JSON: {}", metadataJSON)
    sql = "INSERT OR REPLACE INTO metadata (name, value) VALUES (?, ?)"
    connection.prepareStatement(sql).use { statement ->
        statement.setString(1, "mln-json")
        statement.setString(2, metadataJSON)
        statement.execute()
    }
    vacuumDatabase(connection)
}

private fun convertTile(
    config: EncodeConfig,
    data: ByteArray,
    writerCapture: MBTilesWriter,
    tile: Tile,
): Boolean {
    try {
        val x = tile.column
        val y = tile.row
        val z = tile.zoom
        val srcTileData = decompress(ByteArrayInputStream(data))
        val didCompress = MutableBoolean(false)
        val tileData =
            convertTile(
                x.toLong(),
                y.toLong(),
                z,
                srcTileData,
                config,
                Optional.of(Environment.compressionRatioThreshold),
                Optional.of(Environment.compressionFixedThreshold),
                didCompress,
            )

        if (tileData != null) {
            synchronized(writerCapture) {
                writerCapture.addTile(tileData, z.toLong(), x.toLong(), y.toLong())
            }
        }
        return true
    } catch (ex: IOException) {
        logger.error(
            "Failed to convert tile {}:{},{}",
            tile.zoom,
            tile.column,
            tile.row,
            ex,
        )
    } catch (ex: MBTilesWriteException) {
        logger.error(
            "Failed to convert tile {}:{},{}",
            tile.zoom,
            tile.column,
            tile.row,
            ex,
        )
    }
    return false
}

private val logger: Logger = LoggerFactory.getLogger(Encode::class.java)
