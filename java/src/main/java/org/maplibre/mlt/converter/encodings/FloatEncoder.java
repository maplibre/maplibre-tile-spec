package org.maplibre.mlt.converter.encodings;

import java.util.List;
import org.apache.commons.lang3.ArrayUtils;
import org.maplibre.mlt.metadata.stream.*;

public class FloatEncoder {

  private FloatEncoder() {}

  public static byte[] encodeFloatStream(List<Float> values) {
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

    return ArrayUtils.addAll(valuesMetadata, encodedValueStream);
  }
}
