package org.maplibre.mlt.util;

import java.io.ByteArrayOutputStream;
import java.io.IOException;
import java.util.Collection;

public final class ByteArrayUtil {
  private ByteArrayUtil() {}

  public static int totalLength(Collection<byte[]> buffers) {
    return buffers.stream().mapToInt(x -> x.length).sum();
  }

  public static byte[] concat(Collection<byte[]> buffers) throws IOException {
    final var size = buffers.stream().map(b -> b.length).reduce(0, Integer::sum);
    final var stream = new ByteArrayOutputStream(size);
    for (var buffer : buffers) {
      stream.write(buffer);
    }
    return stream.toByteArray();
  }
}
