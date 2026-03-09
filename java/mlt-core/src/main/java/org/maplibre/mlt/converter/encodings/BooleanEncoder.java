package org.maplibre.mlt.converter.encodings;

import java.io.IOException;
import java.util.ArrayList;
import java.util.BitSet;
import org.maplibre.mlt.metadata.stream.LogicalLevelTechnique;
import org.maplibre.mlt.metadata.stream.PhysicalLevelTechnique;
import org.maplibre.mlt.metadata.stream.PhysicalStreamType;
import org.maplibre.mlt.metadata.stream.StreamMetadata;

public class BooleanEncoder {

  private BooleanEncoder() {}

  /*
   * Combines a BitVector encoding with the Byte RLE encoding form the ORC format
   * */
  public static ArrayList<byte[]> encodeBooleanStream(
      Boolean[] values, PhysicalStreamType streamType) throws IOException {
    final var valueStream = new BitSet(values.length);
    for (var i = 0; i < values.length; i++) {
      valueStream.set(i, values[i]);
    }

    final var encodedValueStream = EncodingUtils.encodeBooleanRle(valueStream, values.length);
    /* For Boolean RLE the additional information provided by the RleStreamMetadata class are not needed */
    final var result =
        new StreamMetadata(
                streamType,
                null,
                LogicalLevelTechnique.RLE,
                LogicalLevelTechnique.NONE,
                PhysicalLevelTechnique.NONE,
                values.length,
                encodedValueStream.length)
            .encode();

    result.add(encodedValueStream);
    return result;
  }
}
