package org.maplibre.mlt.converter.encodings;

import java.io.IOException;
import java.util.ArrayList;
import java.util.BitSet;
import java.util.Collection;
import java.util.stream.Stream;

import org.jetbrains.annotations.NotNull;
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
      final boolean[] values, @NotNull final PhysicalStreamType streamType) throws IOException {
    final var valueStream = new BitSet(values.length);
    for (var i = 0; i < values.length; i++) {
      valueStream.set(i, values[i]);
    }
    return encodeBooleanStream(values.length, valueStream, streamType);
  }

  public static ArrayList<byte[]> encodeBooleanStream(
          final Boolean[] values, @NotNull final PhysicalStreamType streamType) throws IOException {
    final var valueStream = new BitSet(values.length);
    for (var i = 0; i < values.length; i++) {
      valueStream.set(i, values[i]);
    }
    return encodeBooleanStream(values.length, valueStream, streamType);
  }

  public static ArrayList<byte[]> encodeBooleanStream(
          @NotNull final Collection<Boolean> values, @NotNull final PhysicalStreamType streamType) throws IOException {
    return encodeBooleanStream(values.size(), values, streamType);
  }

  public static ArrayList<byte[]> encodeBooleanStream(
          final int count,
          @NotNull final Iterable<Boolean> values, @NotNull final PhysicalStreamType streamType) throws IOException {
    final var valueStream = new BitSet(count);
    int index = 0;
    for (var value : values) {
      valueStream.set(index++, value);
    }
    return encodeBooleanStream(count, valueStream, streamType);
  }

  private static ArrayList<byte[]> encodeBooleanStream(
          final int count, @NotNull final BitSet values, @NotNull final PhysicalStreamType streamType) throws IOException {
    final var encodedValueStream = EncodingUtils.encodeBooleanRle(values, count);
    /* For Boolean RLE the additional information provided by the RleStreamMetadata class are not needed */
    final var result =
            new StreamMetadata(
                    streamType,
                    null,
                    LogicalLevelTechnique.RLE,
                    LogicalLevelTechnique.NONE,
                    PhysicalLevelTechnique.NONE,
                    count,
                    encodedValueStream.length)
                    .encode();

    result.add(encodedValueStream);
    return result;
  }
}
