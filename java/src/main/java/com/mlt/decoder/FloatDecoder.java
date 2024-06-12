package com.mlt.decoder;

import com.mlt.metadata.stream.StreamMetadata;
import java.util.ArrayList;
import java.util.List;
import me.lemire.integercompression.IntWrapper;

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
