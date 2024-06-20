package com.mlt.converter.encodings;

import com.mlt.metadata.stream.*;
import java.io.IOException;
import java.util.*;
import org.apache.commons.lang3.ArrayUtils;

public class BooleanEncoder {

  private BooleanEncoder() {}

  /*
   * Combines a BitVector encoding with the Byte RLE encoding form the ORC format
   * */
  public static byte[] encodeBooleanStream(List<Boolean> values, PhysicalStreamType streamType)
      throws IOException {
    var valueStream = new BitSet(values.size());
    for (var i = 0; i < values.size(); i++) {
      var value = values.get(i);
      valueStream.set(i, value);
    }

    var encodedValueStream = EncodingUtils.encodeBooleanRle(valueStream, values.size());
    /* For Boolean RLE the additional information provided by the RleStreamMetadata class are not needed */
    var valuesMetadata =
        new StreamMetadata(
                streamType,
                null,
                LogicalLevelTechnique.RLE,
                LogicalLevelTechnique.NONE,
                PhysicalLevelTechnique.NONE,
                values.size(),
                encodedValueStream.length)
            .encode();

    return ArrayUtils.addAll(valuesMetadata, encodedValueStream);
  }

  /*
   * Combines a BitVector encoding with the Byte RLE encoding form the ORC format
   * */
  public static byte[] encodeBooleanStreamOptimized(
      List<Boolean> values, PhysicalStreamType streamType) throws IOException {
    var valueStream = new BitSet(values.size());
    for (var i = 0; i < values.size(); i++) {
      var value = values.get(i);
      valueStream.set(i, value);
    }

    var encodedValueStream = EncodingUtils.encodeBooleanRle(valueStream, values.size());
    /* For Boolean RLE the additional information provided by the RleStreamMetadata class are not needed */
    var valuesMetadata =
        new StreamMetadata(
                streamType,
                null,
                LogicalLevelTechnique.RLE,
                LogicalLevelTechnique.NONE,
                PhysicalLevelTechnique.NONE,
                values.size(),
                encodedValueStream.length)
            .encode();

    return ArrayUtils.addAll(valuesMetadata, encodedValueStream);
  }
}
