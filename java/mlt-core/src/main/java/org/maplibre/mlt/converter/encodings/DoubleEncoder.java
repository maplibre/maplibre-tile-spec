package org.maplibre.mlt.converter.encodings;

import jakarta.annotation.Nullable;
import java.io.IOException;
import java.util.Arrays;
import java.util.List;
import org.apache.commons.lang3.ArrayUtils;
import org.jetbrains.annotations.NotNull;
import org.maplibre.mlt.converter.MLTStreamObserver;
import org.maplibre.mlt.metadata.stream.*;

public class DoubleEncoder {

  private DoubleEncoder() {}

  public static byte[] encodeDoubleStream(
      List<Double> values, @NotNull MLTStreamObserver streamObserver, @Nullable String streamName)
      throws IOException {
    // TODO: add encodings -> RLE, Dictionary, PDE, ALP
    double[] doubleArray = new double[values.size()];
    for (int i = 0; i < values.size(); i++) {
      doubleArray[i] = values.get(i);
    }
    var encodedValueStream = EncodingUtils.encodeDoublesLE(doubleArray);

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

    streamObserver.observeStream(
        streamName,
        Arrays.asList(ArrayUtils.toObject(doubleArray)),
        valuesMetadata,
        encodedValueStream);
    return ArrayUtils.addAll(valuesMetadata, encodedValueStream);
  }
}
