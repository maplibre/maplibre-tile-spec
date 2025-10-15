package org.maplibre.mlt.converter.encodings;

import jakarta.annotation.Nullable;
import java.io.IOException;
import java.util.Arrays;
import java.util.List;
import org.apache.commons.lang3.ArrayUtils;
import org.jetbrains.annotations.NotNull;
import org.maplibre.mlt.converter.MLTStreamRecorder;
import org.maplibre.mlt.metadata.stream.*;

public class FloatEncoder {

  private FloatEncoder() {}

  public static byte[] encodeFloatStream(
      List<Float> values, @NotNull MLTStreamRecorder streamRecorder, @Nullable String streamName)
      throws IOException {
    // TODO: add encodings -> RLE, Dictionary, PDE, ALP
    float[] floatArray = new float[values.size()];
    for (int i = 0; i < values.size(); i++) {
      floatArray[i] = values.get(i);
    }
    var encodedValueStream = EncodingUtils.encodeFloatsLE(floatArray);

    var valuesMetadata =
        new StreamMetadata(
                PhysicalStreamType.DATA,
                null,
                LogicalLevelTechnique.NONE,
                LogicalLevelTechnique.NONE,
                PhysicalLevelTechnique.NONE,
                values.size(),
                encodedValueStream.length)
            .encode();

    streamRecorder.recordStream(
        streamName,
        Arrays.asList(ArrayUtils.toObject(floatArray)),
        valuesMetadata,
        encodedValueStream);
    return ArrayUtils.addAll(valuesMetadata, encodedValueStream);
  }
}
