package org.maplibre.mlt.decoder;

import java.util.ArrayList;
import java.util.List;
import me.lemire.integercompression.IntWrapper;
import org.maplibre.mlt.metadata.stream.StreamMetadata;

public class DoubleDecoder {
  private DoubleDecoder() {}

  public static List<Double> decodeDoubleStream(
      byte[] data, IntWrapper offset, StreamMetadata streamMetadata) {
    var values = DecodingUtils.decodeDoublesLE(data, offset, streamMetadata.numValues());
    var valuesList = new ArrayList<Double>(values.length);
    for (var value : values) {
      valuesList.add(value);
    }
    return valuesList;
  }
}
