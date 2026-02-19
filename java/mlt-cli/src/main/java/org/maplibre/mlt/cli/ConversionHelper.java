package org.maplibre.mlt.cli;

import java.io.BufferedInputStream;
import java.io.ByteArrayOutputStream;
import java.io.IOException;
import java.io.InputStream;
import java.io.OutputStream;
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

public class ConversionHelper {
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

    return srcStream.readAllBytes();
  }

  public static OutputStream createCompressStream(
      OutputStream src, @Nullable String compressionType) {
    if (Objects.equals(compressionType, "gzip")) {
      try {
        var parameters = new GzipParameters();
        parameters.setCompressionLevel(9);
        return new GzipCompressorOutputStream(src, parameters);
      } catch (IOException ex) {
        System.err.println(
            "Failed to create GzipCompressorOutputStream, falling back to uncompressed or alternative compression.");
        ex.printStackTrace(System.err);
      }
    }
    if (Objects.equals(compressionType, "deflate")) {
      var parameters = new DeflateParameters();
      parameters.setCompressionLevel(9);
      parameters.setWithZlibHeader(false);
      return new DeflateCompressorOutputStream(src);
    }
    return src;
  }
}
