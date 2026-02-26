package org.maplibre.mlt.cli

import com.google.gson.Gson
import com.google.gson.JsonObject
import com.google.gson.JsonSyntaxException
import org.apache.commons.lang3.mutable.MutableBoolean
import org.slf4j.Logger
import org.slf4j.LoggerFactory
import java.io.ByteArrayInputStream
import java.io.ByteArrayOutputStream
import java.io.File
import java.io.IOException
import java.nio.charset.StandardCharsets
import java.nio.file.Files
import java.nio.file.Path
import java.nio.file.StandardCopyOption
import java.sql.Connection
import java.sql.DriverManager
import java.sql.PreparedStatement
import java.sql.SQLException
import java.util.Optional
import java.util.concurrent.atomic.AtomicBoolean
import java.util.concurrent.atomic.AtomicLong

/** Encode the MVT tiles in an offline database file */
@Throws(ClassNotFoundException::class)
fun encodeOfflineDB(
    inputPath: Path,
    outputPath: Path?,
    config: EncodeConfig,
): Boolean {
    // Start with a copy of the source file so we don't have to rebuild the complex schema
    var outputPath = outputPath
    try {
        if (outputPath == null) {
            val tempFile = File.createTempFile("encode-", "-db")
            outputPath = tempFile.toPath()
            tempFile.deleteOnExit()
        }
        logger.debug("Copying source file {} to {}", inputPath, outputPath)
        Files.copy(
            inputPath,
            outputPath,
            StandardCopyOption.REPLACE_EXISTING,
            StandardCopyOption.COPY_ATTRIBUTES,
        )
    } catch (ex: IOException) {
        logger.error("Failed to create target file", ex)
        return false
    }

    Class.forName("org.sqlite.JDBC")
    val srcConnectionString = "jdbc:sqlite:" + inputPath.toAbsolutePath()
    val dstConnectionString = "jdbc:sqlite:" + outputPath.toAbsolutePath()
    val updateSql = "UPDATE tiles SET data = ?, compressed = ? WHERE id = ?"
    val success = AtomicBoolean(true)
    try {
        DriverManager.getConnection(srcConnectionString).use { srcConnection ->
            DriverManager.getConnection(dstConnectionString).use { dstConnection ->
                srcConnection.createStatement().use { iterateStatement ->
                    iterateStatement.executeQuery("SELECT * FROM tiles").use { tileResults ->
                        dstConnection.prepareStatement(updateSql).use { updateStatement ->
                            val tileCount = AtomicLong(0)
                            while (tileResults.next()) {
                                if (!config.continueOnError && !success.get()) {
                                    break
                                }

                                // Read on the main thread. Could be split by zoom level, etc., if needed.
                                val uniqueID = tileResults.getLong("id")
                                val x = tileResults.getInt("x")
                                val y = tileResults.getInt("y")
                                val z = tileResults.getInt("z")
                                val data = tileResults.getBinaryStream("data").readAllBytes()

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
                                                z,
                                                x,
                                                y,
                                                data,
                                                uniqueID,
                                                updateStatement,
                                            )
                                        ) {
                                            success.set(false)
                                        }
                                    },
                                )
                            }
                            try {
                                config.taskRunner.shutdown()
                                config.taskRunner.awaitTermination()
                            } catch (ex: InterruptedException) {
                                logger.error("Interrupted", ex)
                                return false
                            }
                        }
                    }
                }
                if (!config.continueOnError && !success.get()) {
                    return false
                }

                updateMetadata(config, dstConnection)
                vacuumDatabase(dstConnection)
            }
        }
    } catch (ex: SQLException) {
        logger.error("Offline Database conversion failed", ex)
        return false
    } catch (ex: IOException) {
        logger.error("Offline Database conversion failed", ex)
        return false
    }
    return success.get()
}

@Throws(SQLException::class, IOException::class)
private fun updateMetadata(
    config: EncodeConfig,
    dstConnection: Connection,
) {
    val metadataKind = 2 // `mbgl::Resource::Kind::Source`
    val metadataQuerySQL = "SELECT id,data FROM resources WHERE kind = ?"
    val metadataUpdateSQL = "UPDATE resources SET data = ?, compressed = ? WHERE id = ?"
    dstConnection.prepareStatement(metadataQuerySQL).use { queryStatement ->
        dstConnection.prepareStatement(metadataUpdateSQL).use { updateStatement ->
            queryStatement.setInt(1, metadataKind)
            val metadataResults = queryStatement.executeQuery()
            while (metadataResults.next()) {
                val uniqueID = metadataResults.getLong("id")
                var data: ByteArray?
                try {
                    data = decompress(metadataResults.getBinaryStream("data"))
                } catch (ignore: IOException) {
                    logger.warn("Failed to decompress Source resource '{}', skipping", uniqueID)
                    continue
                } catch (ignore: IllegalStateException) {
                    logger.warn("Failed to decompress Source resource '{}', skipping", uniqueID)
                    continue
                }

                // Parse JSON
                var jsonString = String(data, StandardCharsets.UTF_8)
                val json: JsonObject
                try {
                    json = Gson().fromJson<JsonObject>(jsonString, JsonObject::class.java)
                } catch (ex: JsonSyntaxException) {
                    logger.warn("Source resource '{}' is not JSON, skipping", uniqueID)
                    continue
                }

                // Update the format field
                json.addProperty("format", MBTILES_METADATA_MIME_TYPE)

                // Re-serialize
                jsonString = json.toString()

                // Re-compress
                val compressed: Boolean
                ByteArrayOutputStream().use { outputStream ->
                    createCompressStream(
                        outputStream,
                        config.compressionType,
                    ).use { compressStream ->
                        compressed = (compressStream !== outputStream)
                        compressStream!!.write(jsonString.toByteArray(StandardCharsets.UTF_8))
                    }
                    data = outputStream.toByteArray()
                }
                // Update the database
                updateStatement.setBytes(1, data)
                updateStatement.setBoolean(2, compressed)
                updateStatement.setLong(3, uniqueID)
                updateStatement.execute()

                logger.debug(
                    "Updated source JSON format to '{}'",
                    MBTILES_METADATA_MIME_TYPE,
                )
            }
        }
    }
}

private fun convertTile(
    config: EncodeConfig,
    z: Int,
    x: Int,
    y: Int,
    data: ByteArray,
    uniqueID: Long,
    updateStatement: PreparedStatement,
): Boolean {
    logger.trace("Converting tile {}: {},{}", z, x, y)

    val srcTileData: ByteArray?
    try {
        srcTileData = decompress(ByteArrayInputStream(data))
    } catch (ex: IOException) {
        logger.error("Failed to decompress tile '{}'", uniqueID, ex)
        return false
    } catch (ex: IllegalStateException) {
        logger.error("Failed to decompress tile '{}'", uniqueID, ex)
        return false
    }

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
        try {
            // Parallel writes are possible, but only by creating a separate connection for each thread
            synchronized(updateStatement) {
                updateStatement.setBytes(1, tileData)
                updateStatement.setBoolean(2, didCompress.booleanValue())
                updateStatement.setLong(3, uniqueID)
                updateStatement.execute()
            }
            return true
        } catch (ex: SQLException) {
            logger.error("Failed to convert tile '{}'", uniqueID, ex)
        }
    }
    return false
}

private val logger: Logger = LoggerFactory.getLogger(Encode::class.java)
