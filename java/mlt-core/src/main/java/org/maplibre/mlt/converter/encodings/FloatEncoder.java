package org.maplibre.mlt.converter.encodings;

import java.io.IOException;
import java.util.ArrayList;
import java.util.Collection;
import org.jetbrains.annotations.NotNull;
import org.maplibre.mlt.metadata.stream.LogicalLevelTechnique;
import org.maplibre.mlt.metadata.stream.PhysicalLevelTechnique;
import org.maplibre.mlt.metadata.stream.PhysicalStreamType;
import org.maplibre.mlt.metadata.stream.StreamMetadata;

public class FloatEncoder {

  private FloatEncoder() {}

  public static ArrayList<byte[]> encodeFloatStream(final float[] values) throws IOException {
    return encodeFloatStream(values.length, EncodingUtils.encodeFloatsLE(values));
  }

  public static ArrayList<byte[]> encodeFloatStream(@NotNull final Collection<Float> values)
      throws IOException {
    return encodeFloatStream(values.size(), EncodingUtils.encodeFloatsLE(values));
  }

  private static ArrayList<byte[]> encodeFloatStream(final int size, final byte[] encoded)
      throws IOException {
    // TODO: add encodings -> RLE, Dictionary, PDE
    final var result =
        new StreamMetadata(
                PhysicalStreamType.DATA,
                null,
                LogicalLevelTechnique.NONE,
                LogicalLevelTechnique.NONE,
                PhysicalLevelTechnique.NONE,
                size,
                encoded.length)
            .encode();
    result.add(encoded);
    return result;
  }
}
