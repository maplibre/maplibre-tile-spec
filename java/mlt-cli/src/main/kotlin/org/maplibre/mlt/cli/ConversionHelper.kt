package org.maplibre.mlt.cli

import org.apache.commons.compress.compressors.deflate.DeflateCompressorInputStream
import org.apache.commons.compress.compressors.deflate.DeflateCompressorOutputStream
import org.apache.commons.compress.compressors.deflate.DeflateParameters
import org.apache.commons.compress.compressors.gzip.GzipCompressorInputStream
import org.apache.commons.compress.compressors.gzip.GzipCompressorOutputStream
import org.apache.commons.compress.compressors.gzip.GzipParameters
import org.slf4j.LoggerFactory
import java.io.BufferedInputStream
import java.io.ByteArrayOutputStream
import java.io.IOException
import java.io.InputStream
import java.io.OutputStream
import java.sql.Connection
import java.sql.SQLException
import java.util.zip.Inflater
import java.util.zip.InflaterInputStream
import kotlin.Boolean
import kotlin.ByteArray
import kotlin.Exception
import kotlin.String
import kotlin.Throws
import kotlin.io.use
import kotlin.use

/**  Base class for sharing between PMTiles, MBTiles and offline database conversions */
open class ConversionHelper {
    companion object {
        @JvmStatic
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

        @JvmStatic
        @Throws(IOException::class)
        fun createCompressStream(
            src: OutputStream,
            compressionType: String?,
        ): OutputStream? {
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

        @JvmStatic
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

        public const val DEFAULT_COMPRESSION_RATIO_THRESHOLD = 0.98

        @JvmField
        protected var compressionRatioThreshold = DEFAULT_COMPRESSION_RATIO_THRESHOLD
        public const val DEFAULT_COMPRESSION_FIXED_THRESHOLD = 20L

        @JvmField
        protected var compressionFixedThreshold = DEFAULT_COMPRESSION_FIXED_THRESHOLD

        public const val DEFAULT_TILE_LOG_INTERVAL = 10_000L

        @JvmField
        protected var tileLogInterval = DEFAULT_TILE_LOG_INTERVAL

        private val logger = LoggerFactory.getLogger(ConversionHelper::class.java)

        init {
            try {
                val str = System.getenv("MLT_TILE_LOG_INTERVAL")
                if (str != null) {
                    val value = str.toULong().toLong()
                    if (value > 0) {
                        logger.trace("Setting tile logging interval to {}", value)
                        tileLogInterval = value
                    } else {
                        logger.warn("Invalid value for MLT_TILE_LOG_INTERVAL, using default value of {}", DEFAULT_TILE_LOG_INTERVAL)
                    }
                }
            } catch (ex: Exception) {
                logger.warn(
                    "Failed to parse MLT_TILE_LOG_INTERVAL, using default value of {}",
                    tileLogInterval,
                    ex,
                )
            }
            try {
                val str = System.getenv("MLT_COMPRESSION_RATIO_THRESHOLD")
                if (str != null) {
                    val value = str.toDouble()
                    if (value > 0 && value <= 1) {
                        logger.trace("Setting compression ratio threshold to {}", value)
                        compressionRatioThreshold = value
                    } else {
                        logger.warn(
                            "Invalid value for MLT_COMPRESSION_RATIO_THRESHOLD: {}, using default value of {}",
                            value,
                            compressionRatioThreshold,
                        )
                    }
                }
            } catch (ex: Exception) {
                logger.warn(
                    "Failed to parse MLT_COMPRESSION_RATIO_THRESHOLD, using default value of {}",
                    compressionRatioThreshold,
                    ex,
                )
            }
            try {
                val str = System.getenv("MLT_COMPRESSION_FIXED_THRESHOLD")
                if (str != null) {
                    val value = str.toLong()
                    logger.trace("Setting compression fixed threshold to {}", value)
                    compressionFixedThreshold = value
                }
            } catch (ex: Exception) {
                logger.warn(
                    "Failed to parse MLT_COMPRESSION_FIXED_THRESHOLD, using default value of {}",
                    compressionFixedThreshold,
                    ex,
                )
            }
        }
    }
}
