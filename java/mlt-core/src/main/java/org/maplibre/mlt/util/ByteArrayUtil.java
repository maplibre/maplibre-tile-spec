package org.maplibre.mlt.util;

import java.io.ByteArrayOutputStream;
import java.io.IOException;
import java.io.OutputStream;
import java.util.Collection;

public final class ByteArrayUtil {
  private ByteArrayUtil() {}

  public static int totalLength(Collection<byte[]> buffers) {
    return buffers.stream().mapToInt(x -> x.length).sum();
  }

  public static <T extends OutputStream> T concat(T stream, Collection<byte[]> buffers)
      throws IOException {
    for (var buffer : buffers) {
      stream.write(buffer);
    }
    return stream;
  }

  public static byte[] concat(Collection<byte[]> buffers) throws IOException {
    try (final var stream = new ByteArrayOutputStream(totalLength(buffers))) {
      return concat(stream, buffers).toByteArray();
    }
  }
}
