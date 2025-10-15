package org.maplibre.mlt.decoder;

import java.util.ArrayList;
import java.util.List;
import me.lemire.integercompression.IntWrapper;
import org.maplibre.mlt.metadata.stream.StreamMetadata;

public class FloatDecoder {
  private FloatDecoder() {}

  public static List<Float> decodeFloatStream(
      byte[] data, IntWrapper offset, StreamMetadata streamMetadata) {
    var values = DecodingUtils.decodeFloatsLE(data, offset, streamMetadata.numValues());
    var valuesList = new ArrayList<Float>(values.length);
    for (var value : values) {
      valuesList.add(value);
    }
    return valuesList;
  }
}
