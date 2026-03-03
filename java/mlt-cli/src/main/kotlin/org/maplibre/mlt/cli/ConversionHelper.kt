package org.maplibre.mlt.cli

import com.onthegomap.planetiler.geo.TileCoord
import org.apache.commons.compress.compressors.deflate.DeflateCompressorInputStream
import org.apache.commons.compress.compressors.deflate.DeflateCompressorOutputStream
import org.apache.commons.compress.compressors.deflate.DeflateParameters
import org.apache.commons.compress.compressors.gzip.GzipCompressorInputStream
import org.apache.commons.compress.compressors.gzip.GzipCompressorOutputStream
import org.apache.commons.compress.compressors.gzip.GzipParameters
import org.apache.commons.lang3.mutable.MutableBoolean
import org.maplibre.mlt.converter.ConversionConfig
import org.maplibre.mlt.converter.FeatureTableOptimizations
import org.maplibre.mlt.converter.MltConverter
import org.maplibre.mlt.converter.mvt.ColumnMapping
import org.maplibre.mlt.converter.mvt.MvtUtils
import org.maplibre.mlt.metadata.tileset.MltMetadata
import org.slf4j.Logger
import org.slf4j.LoggerFactory
import java.io.BufferedInputStream
import java.io.ByteArrayOutputStream
import java.io.IOException
import java.io.InputStream
import java.io.OutputStream
import java.sql.Connection
import java.sql.SQLException
import java.util.Optional
import java.util.concurrent.ConcurrentHashMap
import java.util.concurrent.atomic.AtomicLong
import java.util.function.Supplier
import java.util.stream.Collectors
import java.util.zip.Inflater
import java.util.zip.InflaterInputStream

@Throws(IOException::class)
fun decompress(srcStream: InputStream): ByteArray {
    var decompressInputStream: InputStream? = null
    // Check for common compression formats by looking at the header bytes
    // Buffered stream is not closed here because it would also close the underlying stream
    val readStream = BufferedInputStream(srcStream)
    if (readStream.available() > 3) {
        readStream.mark(4)
        val header = readStream.readNBytes(4)
        readStream.reset()

        if (DeflateCompressorInputStream.matches(header, header.size)) {
            // deflate with zlib header
            val inflater = Inflater(false)
            decompressInputStream = InflaterInputStream(readStream, inflater)
        } else if (header[0].toInt() == 0x1f && header[1] == 0x8b.toByte()) {
            // TODO: why doesn't GZIPInputStream work here?
            // decompressInputStream = new GZIPInputStream(readStream);
            decompressInputStream = GzipCompressorInputStream(readStream)
        }
    }

    if (decompressInputStream != null) {
        ByteArrayOutputStream().use { outputStream ->
            decompressInputStream.transferTo(outputStream)
            return outputStream.toByteArray()
        }
    }

    return readStream.readAllBytes()
}

@Throws(IOException::class)
fun createCompressStream(
    src: OutputStream,
    compressionType: String?,
): OutputStream {
    if (compressionType == "gzip") {
        val parameters = GzipParameters()
        parameters.setCompressionLevel(9)
        return GzipCompressorOutputStream(src, parameters)
    }
    if (compressionType == "deflate") {
        val parameters = DeflateParameters()
        parameters.setCompressionLevel(9)
        parameters.setWithZlibHeader(false)
        return DeflateCompressorOutputStream(src, parameters)
    }
    return src
}

@Throws(SQLException::class)
fun vacuumDatabase(connection: Connection): Boolean {
    logger.debug("Optimizing database")
    try {
        connection.createStatement().use { stmt ->
            stmt.execute("VACUUM")
            return true
        }
    } catch (ex: SQLException) {
        logger.error("Failed to optimize database", ex)
    }
    return false
}

fun getTileLabel(
    x: Long,
    y: Long,
    z: Int,
): String = String.format("%d:%d,%d", z, x, y)

fun getTileLabel(c: TileCoord): String = getTileLabel(c.x.toLong(), c.y.toLong(), c.z)

fun getTileLabels(
    coords: Iterable<TileCoord>,
    limit: Int,
): String =
    coords.joinToString(
        separator = ", ",
        limit = limit,
        truncated = "...",
        transform = ::getTileLabel,
    )

fun logColumnMappings(
    x: Long,
    y: Long,
    z: Int,
    metadata: MltMetadata.TileSetMetadata,
) {
    if (!logger.isTraceEnabled) {
        return
    }

    for (table in metadata.featureTables) {
        for (column in table.columns) {
            if (column.complexType != null &&
                column.complexType.physicalType == MltMetadata.ComplexType.STRUCT
            ) {
                val mappings =
                    column.complexType.children
                        .map { child -> column.name + child.name }
                        .toSortedSet()
                        .joinToString(", ")
                val added = MutableBoolean(false)
                val entry =
                    loggedColumnMappings.computeIfAbsent(mappings, { _ ->
                        added.setTrue()
                        loggedColumnMappingId.incrementAndGet()
                    })

                if (added.isTrue) {
                    logger.trace("{}:{},{} Found new column mapping {} for {}: {}", z, x, y, entry, table.name, mappings)
                } else {
                    logger.trace("{}:{},{} Found existing column mapping for {}: {}", z, x, y, table.name, entry)
                }
            }
        }
    }
}

/**
 * Converts a tile from MVT to MLT, handling de- and re-compression
 *
 * @param x The x-coordinate of the tile.
 * @param y The y-coordinate of the tile.
 * @param z The zoom level of the tile.
 * @param srcTileData The source tile data as a byte array.
 * @param config The configuration for the conversion process
 * @param compressionRatioThreshold An optional threshold for the compression ratio to determine
 * whether compression is worth the need to decompress it. The compressed version will only be
 * used if its size is less than the original size multiplied by this ratio. If not present, a
 * compressed result will be used even if it's larger than the original.
 * @param compressionFixedThreshold An optional fixed byte threshold to determine whether
 * compression is worth the need to decompress it. The compressed version will only be used if
 * its size is at least this many bytes smaller than the original size. If not present, a
 * compressed result will be used even if it's larger than the original.
 * @param didCompress A mutable boolean to indicate whether compression was applied
 * @return The converted tile data as a byte array, or null if the conversion failed.
 */
fun convertTile(
    x: Long,
    y: Long,
    z: Int,
    srcTileData: ByteArray,
    config: EncodeConfig,
    compressionRatioThreshold: Optional<Double>,
    compressionFixedThreshold: Optional<Long>,
    didCompress: MutableBoolean?,
): ByteArray? {
    try {
        // Decode the source tile data into an intermediate representation.
        val decodedMvTile = MvtUtils.decodeMvt(srcTileData)

        val isIdPresent = true
        val metadata =
            MltConverter.createTilesetMetadata(
                decodedMvTile,
                config.conversionConfig,
                config.columnMappingConfig,
                isIdPresent,
            )

        // Print column mappings if verbosity level is high.
        logColumnMappings(x, y, z, metadata)

        // Apply column mappings and update the conversion configuration.
        val targetConfig = applyColumnMappingsToConversionConfig(config, metadata)

        // Convert the tile using the updated configuration and tessellation source.
        var tileData =
            MltConverter.convertMvt(
                decodedMvTile,
                metadata,
                targetConfig,
                config.tessellateSource,
            )

        // Apply compression if specified.
        if (config.compressionType != null) {
            ByteArrayOutputStream().use { outputStream ->
                createCompressStream(
                    outputStream,
                    config.compressionType,
                ).use { compressStream ->
                    compressStream.write(tileData)
                }
                // Evaluate whether the compressed version is worth using.
                if ((
                        compressionFixedThreshold.isEmpty ||
                            outputStream.size() <= tileData.size - compressionFixedThreshold.get()
                    ) &&
                    (
                        compressionRatioThreshold.isEmpty ||
                            outputStream.size() <= tileData.size * compressionRatioThreshold.get()
                    )
                ) {
                    if (logger.isTraceEnabled) {
                        val compressedSize = outputStream.size()
                        val originalSize = tileData.size
                        logger
                            .atTrace()
                            .setMessage("{} compressed {} to {} bytes ({}%)")
                            .addArgument(getTileLabel(x, y, z))
                            .addArgument(originalSize)
                            .addArgument(compressedSize)
                            .addArgument(
                                Supplier {
                                    String.format(
                                        "%.1f",
                                        100.0 * compressedSize / originalSize,
                                    )
                                },
                            ).log()
                    }
                    totalCompressedInput.addAndGet(tileData.size.toLong())
                    totalCompressedOutput.addAndGet(outputStream.size().toLong())
                    if (didCompress != null) {
                        didCompress.setTrue()
                    }
                    tileData = outputStream.toByteArray()
                } else {
                    if (logger.isTraceEnabled) {
                        val compressedSize = outputStream.size()
                        val originalSize = tileData.size
                        logger
                            .atTrace()
                            .setMessage(
                                "Compression of {}:{},{} not effective ({} vs {} bytes, {}%), using uncompressed",
                            ).addArgument(z)
                            .addArgument(x)
                            .addArgument(y)
                            .addArgument(originalSize)
                            .addArgument(compressedSize)
                            .addArgument(
                                Supplier {
                                    String.format(
                                        "%.1f",
                                        100.0 * compressedSize / originalSize,
                                    )
                                },
                            ).log()
                    }
                    if (didCompress != null) {
                        didCompress.setFalse()
                    }
                }
            }
        }
        return tileData
    } catch (ex: IOException) {
        // Log an error message if tile conversion fails.
        logger.error("Failed to process tile {}:{},{}", z, x, y, ex)
    }
    return null
}

/** In batch conversion, we don't have the column names yet when we set up the config, so
we need to apply the mappings to an existing immutable config object using the column
names from a decoded tile metadata. */
fun applyColumnMappingsToConversionConfig(
    config: EncodeConfig,
    metadata: MltMetadata.TileSetMetadata,
): ConversionConfig {
    val conversionConfig = config.conversionConfig
    // If the config already has optimizations, don't modify it
    if (!conversionConfig.optimizations.isEmpty()) {
        return conversionConfig
    }

    // Warn if both patterns match on any tables
    val warnTables =
        metadata.featureTables
            .stream()
            .filter { table: MltMetadata.FeatureTable? ->
                config.sortFeaturesPattern != null &&
                    config.sortFeaturesPattern.matcher(table!!.name).matches()
            }.filter { table: MltMetadata.FeatureTable? ->
                config.regenIDsPattern != null &&
                    config.regenIDsPattern.matcher(table!!.name).matches()
            }.map<String?> { table: MltMetadata.FeatureTable? -> table!!.name }
            .collect(Collectors.joining(","))
    if (!warnTables.isEmpty()) {
        logger.warn(
            "The --{} and --{} options are incompatible: {}",
            EncodeCommandLine.SORT_FEATURES_OPTION,
            EncodeCommandLine.REGEN_IDS_OPTION,
            warnTables,
        )
    }

    // Now that we have the actual column names from the metadata, we can determine which column
    // mappings apply to which tables and create the optimizations for each table accordingly.
    val optimizationMap =
        metadata.featureTables
            .stream()
            .collect(
                Collectors.toUnmodifiableMap(
                    { table: MltMetadata.FeatureTable -> table.name },
                    { table: MltMetadata.FeatureTable ->
                        FeatureTableOptimizations(
                            config.sortFeaturesPattern != null &&
                                config.sortFeaturesPattern
                                    .matcher(table.name)
                                    .matches(),
                            config.regenIDsPattern != null &&
                                config.regenIDsPattern.matcher(table.name).matches(),
                            config.columnMappingConfig.entries
                                .stream()
                                .filter { entry ->
                                    entry
                                        .key
                                        .matcher(
                                            table.name,
                                        ).matches()
                                }.flatMap<ColumnMapping> { entry ->
                                    entry.value.stream()
                                }.toList(),
                        )
                    },
                ),
            )

    // re-create the config with the applied column mappings
    return conversionConfig.asBuilder().optimizations(optimizationMap).build()
}

val totalCompressedInput = AtomicLong(0L)
val totalCompressedOutput = AtomicLong(0L)

val loggedColumnMappingId = AtomicLong(0L)
val loggedColumnMappings = ConcurrentHashMap<String, Long>()

private val logger: Logger = LoggerFactory.getLogger(Encode::class.java)

const val MBTILES_METADATA_MIME_TYPE = "application/vnd.maplibre-vector-tile"
