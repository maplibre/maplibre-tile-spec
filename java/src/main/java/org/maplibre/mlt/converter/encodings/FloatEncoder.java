package org.maplibre.mlt.converter.encodings;

import jakarta.annotation.Nullable;
import java.util.Arrays;
import java.util.List;
import java.util.Map;
import org.apache.commons.lang3.ArrayUtils;
import org.apache.commons.lang3.tuple.Triple;
import org.maplibre.mlt.metadata.stream.*;

public class FloatEncoder {

  private FloatEncoder() {}

  public static byte[] encodeFloatStream(
      List<Float> values,
      @Nullable Map<String, Triple<byte[], byte[], String>> rawStreamData,
      @Nullable String streamName) {
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

    if (rawStreamData != null && streamName != null) {
      GeometryEncoder.recordStream(
          streamName,
          Arrays.asList(ArrayUtils.toObject(floatArray)),
          valuesMetadata,
          encodedValueStream,
          rawStreamData);
    }
    return ArrayUtils.addAll(valuesMetadata, encodedValueStream);
  }
}
