package org.maplibre.mlt.converter.encodings;

import jakarta.annotation.Nullable;
import java.io.IOException;
import java.nio.ByteBuffer;
import java.util.List;
import org.apache.commons.lang3.ArrayUtils;
import org.jetbrains.annotations.NotNull;
import org.maplibre.mlt.converter.MLTStreamObserver;
import org.maplibre.mlt.metadata.stream.*;

public class DoubleEncoder {

  private DoubleEncoder() {}

  public static List<ByteBuffer> encodeDoubleStream(
      List<Double> values, @NotNull MLTStreamObserver streamObserver, @Nullable String streamName)
      throws IOException {
    // TODO: add encodings -> RLE, Dictionary, PDE, ALP
    final double[] doubleArray = new double[values.size()];
    for (int i = 0; i < values.size(); i++) {
      doubleArray[i] = values.get(i);
    }
    final var encodedValueStream = EncodingUtils.encodeDoublesLE(doubleArray);

    final var result =
        new StreamMetadata(
                PhysicalStreamType.DATA,
                null,
                LogicalLevelTechnique.NONE,
                LogicalLevelTechnique.NONE,
                PhysicalLevelTechnique.NONE,
                values.size(),
                encodedValueStream.length)
            .encode();

    streamObserver.observeStream(
        streamName, ArrayUtils.toObject(doubleArray), result, encodedValueStream);

    result.add(ByteBuffer.wrap(encodedValueStream));
    return result;
  }
}
