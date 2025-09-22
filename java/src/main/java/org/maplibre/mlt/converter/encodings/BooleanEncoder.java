package org.maplibre.mlt.converter.encodings;

import java.io.IOException;
import java.util.*;
import javax.annotation.Nullable;
import org.apache.commons.lang3.ArrayUtils;
import org.apache.commons.lang3.tuple.Triple;
import org.maplibre.mlt.metadata.stream.*;

public class BooleanEncoder {

  private BooleanEncoder() {}

  /*
   * Combines a BitVector encoding with the Byte RLE encoding form the ORC format
   * */
  public static byte[] encodeBooleanStream(
      List<Boolean> values,
      PhysicalStreamType streamType,
      @Nullable Map<String, Triple<byte[], byte[], String>> rawStreamData,
      @Nullable String streamName)
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

    GeometryEncoder.recordStream(
        streamName, values, valuesMetadata, encodedValueStream, rawStreamData);
    return ArrayUtils.addAll(valuesMetadata, encodedValueStream);
  }

  /*
   * Combines a BitVector encoding with the Byte RLE encoding form the ORC format
   * */
  public static byte[] encodeBooleanStreamOptimized(
      List<Boolean> values,
      PhysicalStreamType streamType,
      @Nullable Map<String, Triple<byte[], byte[], String>> rawStreamData,
      @Nullable String streamName)
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

    GeometryEncoder.recordStream(
        streamName, values, valuesMetadata, encodedValueStream, rawStreamData);
    return ArrayUtils.addAll(valuesMetadata, encodedValueStream);
  }
}
