package org.maplibre.mlt.converter.encodings;

import jakarta.annotation.Nullable;
import java.io.IOException;
import java.nio.ByteBuffer;
import java.util.*;
import org.jetbrains.annotations.NotNull;
import org.maplibre.mlt.converter.MLTStreamObserver;
import org.maplibre.mlt.metadata.stream.*;

public class BooleanEncoder {

  private BooleanEncoder() {}

  /*
   * Combines a BitVector encoding with the Byte RLE encoding form the ORC format
   * */
  public static List<ByteBuffer> encodeBooleanStream(
      Boolean[] values,
      PhysicalStreamType streamType,
      @NotNull MLTStreamObserver streamObserver,
      @Nullable String streamName)
      throws IOException {
    final var valueStream = new BitSet(values.length);
    for (var i = 0; i < values.length; i++) {
      valueStream.set(i, values[i]);
    }

    final var encodedValueStream = EncodingUtils.encodeBooleanRle(valueStream, values.length);
    /* For Boolean RLE the additional information provided by the RleStreamMetadata class are not needed */
    final var result =
        new StreamMetadata(
                streamType,
                null,
                LogicalLevelTechnique.RLE,
                LogicalLevelTechnique.NONE,
                PhysicalLevelTechnique.NONE,
                values.length,
                encodedValueStream.length)
            .encode();

    streamObserver.observeStream(streamName, values, result, encodedValueStream);
    result.add(ByteBuffer.wrap(encodedValueStream));
    return result;
  }
}
