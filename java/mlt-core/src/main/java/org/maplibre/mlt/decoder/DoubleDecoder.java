package org.maplibre.mlt.decoder;

import java.util.Arrays;
import java.util.List;
import java.util.stream.Collectors;
import java.util.stream.IntStream;
import me.lemire.integercompression.IntWrapper;
import org.maplibre.mlt.metadata.stream.StreamMetadata;

public class DoubleDecoder {
  private DoubleDecoder() {}

  public static List<Double> decodeDoubleStream(
      byte[] data, IntWrapper offset, StreamMetadata streamMetadata) {
    if ((long) streamMetadata.numValues() * Double.BYTES == streamMetadata.byteLength()) {
      final var values = DecodingUtils.decodeDoublesLE(data, offset, streamMetadata.numValues());
      return Arrays.stream(values).boxed().collect(Collectors.toUnmodifiableList());
    } else {
      // Compatibility with tilesets encoded before double support was added
      final var values = DecodingUtils.decodeFloatsLE(data, offset, streamMetadata.numValues());
      return IntStream.range(0, values.length)
          .mapToDouble(i -> values[i])
          .boxed()
          .collect(Collectors.toUnmodifiableList());
    }
  }
}
