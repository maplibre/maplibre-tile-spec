package org.maplibre.mlt.converter.encodings;

import java.io.IOException;
import java.util.ArrayList;
import java.util.List;
import org.maplibre.mlt.metadata.stream.LogicalLevelTechnique;
import org.maplibre.mlt.metadata.stream.PhysicalLevelTechnique;
import org.maplibre.mlt.metadata.stream.PhysicalStreamType;
import org.maplibre.mlt.metadata.stream.StreamMetadata;

public class FloatEncoder {

  private FloatEncoder() {}

  public static ArrayList<byte[]> encodeFloatStream(List<Float> values) throws IOException {
    // TODO: add encodings -> RLE, Dictionary, PDE
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

    valuesMetadata.add(encodedValueStream);
    return valuesMetadata;
  }
}
