package org.maplibre.mlt.cli;

import java.io.BufferedInputStream;
import java.io.ByteArrayOutputStream;
import java.io.IOException;
import java.io.InputStream;
import java.io.OutputStream;
import java.sql.Connection;
import java.sql.SQLException;
import java.util.Objects;
import java.util.zip.Inflater;
import java.util.zip.InflaterInputStream;
import org.apache.commons.compress.compressors.deflate.DeflateCompressorInputStream;
import org.apache.commons.compress.compressors.deflate.DeflateCompressorOutputStream;
import org.apache.commons.compress.compressors.deflate.DeflateParameters;
import org.apache.commons.compress.compressors.gzip.GzipCompressorInputStream;
import org.apache.commons.compress.compressors.gzip.GzipCompressorOutputStream;
import org.apache.commons.compress.compressors.gzip.GzipParameters;
import org.jetbrains.annotations.Nullable;
import org.jspecify.annotations.NonNull;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;

///  Base class for sharing between PMTiles, MBTiles and offline database conversions
class ConversionHelper {
  public static byte[] decompress(InputStream srcStream) throws IOException {
    InputStream decompressInputStream = null;
    // Check for common compression formats by looking at the header bytes
    // Buffered stream is not closed here because it would also close the underlying stream
    final var readStream = new BufferedInputStream(srcStream);
    if (readStream.available() > 3) {
      readStream.mark(4);
      final var header = readStream.readNBytes(4);
      readStream.reset();

      if (DeflateCompressorInputStream.matches(header, header.length)) {
        // deflate with zlib header
        final var inflater = new Inflater(/* nowrap= */ false);
        decompressInputStream = new InflaterInputStream(readStream, inflater);
      } else if (header[0] == 0x1f && header[1] == (byte) 0x8b) {
        // TODO: why doesn't GZIPInputStream work here?
        // decompressInputStream = new GZIPInputStream(readStream);
        decompressInputStream = new GzipCompressorInputStream(readStream);
      }
    }

    if (decompressInputStream != null) {
      try (final var outputStream = new ByteArrayOutputStream()) {
        decompressInputStream.transferTo(outputStream);
        return outputStream.toByteArray();
      }
    }

    return readStream.readAllBytes();
  }

  public static OutputStream createCompressStream(
      OutputStream src, @Nullable String compressionType) throws IOException {
    if (Objects.equals(compressionType, "gzip")) {
      var parameters = new GzipParameters();
      parameters.setCompressionLevel(9);
      return new GzipCompressorOutputStream(src, parameters);
    }
    if (Objects.equals(compressionType, "deflate")) {
      var parameters = new DeflateParameters();
      parameters.setCompressionLevel(9);
      parameters.setWithZlibHeader(false);
      return new DeflateCompressorOutputStream(src);
    }
    return src;
  }

  static boolean vacuumDatabase(@NonNull Connection connection) throws SQLException {
    logger.debug("Optimizing database");
    try (final var stmt = connection.createStatement()) {
      stmt.execute("VACUUM");
      return true;
    } catch (SQLException ex) {
      logger.error("Failed to optimize database", ex);
    }
    return false;
  }

  protected static final double DEFAULT_COMPRESSION_RATIO_THRESHOLD = 0.98;
  protected static double COMPRESSION_RATIO_THRESHOLD = DEFAULT_COMPRESSION_RATIO_THRESHOLD;
  protected static final long DEFAULT_COMPRESSION_FIXED_THRESHOLD = 20L;
  protected static long COMPRESSION_FIXED_THRESHOLD = DEFAULT_COMPRESSION_FIXED_THRESHOLD;

  protected static long TILE_LOG_INTERVAL = 10_000L;

  private static final Logger logger = LoggerFactory.getLogger(ConversionHelper.class);

  static {
    try {
      final var str = System.getenv("MLT_TILE_LOG_INTERVAL");
      if (str != null) {
        final var value = Long.parseUnsignedLong(str);
        if (value > 0) {
          logger.trace("Setting tile logging interval to {}", value);
          TILE_LOG_INTERVAL = value;
        }
      }
    } catch (Exception ex) {
      logger.warn(
          "Failed to parse MLT_TILE_LOG_INTERVAL, using default value of {}",
          TILE_LOG_INTERVAL,
          ex);
    }
    try {
      final var str = System.getenv("MLT_COMPRESSION_RATIO_THRESHOLD");
      if (str != null) {
        final var value = Double.parseDouble(str);
        if (value > 0 && value <= 1) {
          logger.trace("Setting compression ratio threshold to {}", value);
          COMPRESSION_RATIO_THRESHOLD = value;
        }
      }
    } catch (Exception ex) {
      logger.warn(
          "Failed to parse MLT_COMPRESSION_RATIO_THRESHOLD, using default value of {}",
          COMPRESSION_RATIO_THRESHOLD,
          ex);
    }
    try {
      final var str = System.getenv("MLT_COMPRESSION_FIXED_THRESHOLD");
      if (str != null) {
        final var value = Long.parseLong(str);
        logger.trace("Setting compression fixed threshold to {}", value);
        COMPRESSION_FIXED_THRESHOLD = value;
      }
    } catch (Exception ex) {
      logger.warn(
          "Failed to parse MLT_COMPRESSION_FIXED_THRESHOLD, using default value of {}",
          COMPRESSION_FIXED_THRESHOLD,
          ex);
    }
  }
}
