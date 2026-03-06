package org.maplibre.mlt.util;

import java.nio.ByteBuffer;
import java.util.Iterator;
import java.util.stream.Stream;
import java.util.stream.StreamSupport;

public class ByteBufferUtil {
  /// Combine (by copying) ByteBuffer objects into a new, read-only ByteBuffer
  public static ByteBuffer concat(ByteBuffer... buffers) {
    return concat(
        () ->
            new Iterator<ByteBuffer>() {
              int current = 0;
              final int end = buffers.length;

              @Override
              public boolean hasNext() {
                return current < end;
              }

              @Override
              public ByteBuffer next() {
                return buffers[current++];
              }
            });
  }

  /// Combine (by copying) ByteBuffer objects into a new ByteBuffer
  /// NOTE: traverses the iterable twice
  /// The result is not marked as read-only because that disables `.array()`
  public static ByteBuffer concat(Iterable<ByteBuffer> buffers) {
    final var result = ByteBuffer.wrap(new byte[totalLength(buffers)]);
    streamOf(buffers)
        .forEach(
            b -> {
              // Use `duplicate` to avoid changing the position of the input buffers
              result.put(b.duplicate());
            });
    return result.flip();
  }

  public static int totalLength(Iterable<ByteBuffer> buffers) {
    return streamOf(buffers).mapToInt(ByteBuffer::remaining).sum();
  }

  private static <T> Stream<T> streamOf(Iterable<T> items) {
    return StreamSupport.stream(items.spliterator(), /* parallel= */ false);
  }
}
