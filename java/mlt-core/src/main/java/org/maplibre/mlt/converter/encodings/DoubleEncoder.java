package org.maplibre.mlt.converter.encodings;

import java.io.IOException;
import java.util.ArrayList;
import java.util.List;
import org.maplibre.mlt.metadata.stream.LogicalLevelTechnique;
import org.maplibre.mlt.metadata.stream.PhysicalLevelTechnique;
import org.maplibre.mlt.metadata.stream.PhysicalStreamType;
import org.maplibre.mlt.metadata.stream.StreamMetadata;

public class DoubleEncoder {

  private DoubleEncoder() {}

  public static ArrayList<byte[]> encodeDoubleStream(List<Double> values) throws IOException {
    // TODO: add encodings -> RLE, Dictionary, PDE
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

    result.add(encodedValueStream);
    return result;
  }
}
