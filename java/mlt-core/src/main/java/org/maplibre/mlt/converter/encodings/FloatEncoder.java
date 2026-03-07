package org.maplibre.mlt.converter.encodings;

import jakarta.annotation.Nullable;
import java.io.IOException;
import java.nio.ByteBuffer;
import java.util.List;
import org.apache.commons.lang3.ArrayUtils;
import org.jetbrains.annotations.NotNull;
import org.maplibre.mlt.converter.MLTStreamObserver;
import org.maplibre.mlt.metadata.stream.LogicalLevelTechnique;
import org.maplibre.mlt.metadata.stream.PhysicalLevelTechnique;
import org.maplibre.mlt.metadata.stream.PhysicalStreamType;
import org.maplibre.mlt.metadata.stream.StreamMetadata;

public class FloatEncoder {

  private FloatEncoder() {}

  public static List<ByteBuffer> encodeFloatStream(
      List<Float> values, @NotNull MLTStreamObserver streamObserver, @Nullable String streamName)
      throws IOException {
    // TODO: add encodings -> RLE, Dictionary, PDE, ALP
    final float[] floatArray = new float[values.size()];
    for (int i = 0; i < values.size(); i++) {
      floatArray[i] = values.get(i);
    }
    final var encodedValueStream = EncodingUtils.encodeFloatsLE(floatArray);

    final var valuesMetadata =
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
        streamName, ArrayUtils.toObject(floatArray), valuesMetadata, encodedValueStream);

    valuesMetadata.add(ByteBuffer.wrap(encodedValueStream));
    return valuesMetadata;
  }
}
