package org.maplibre.mlt.converter.encodings;

import java.io.IOException;
import java.util.ArrayList;
import java.util.Collection;
import java.util.List;

import org.jetbrains.annotations.NotNull;
import org.maplibre.mlt.metadata.stream.LogicalLevelTechnique;
import org.maplibre.mlt.metadata.stream.PhysicalLevelTechnique;
import org.maplibre.mlt.metadata.stream.PhysicalStreamType;
import org.maplibre.mlt.metadata.stream.StreamMetadata;

public class DoubleEncoder {

  private DoubleEncoder() {}

  public static ArrayList<byte[]> encodeDoubleStream(@NotNull final Collection<Double> values) throws IOException {
    return encodeDoubleStream(values.size(), EncodingUtils.encodeDoublesLE(values));
  }
  public static ArrayList<byte[]> encodeDoubleStream(final double[] values) throws IOException {
    // TODO: add encodings -> RLE, Dictionary, PDE
    return encodeDoubleStream(values.length, EncodingUtils.encodeDoublesLE(values));
  }

  private static ArrayList<byte[]> encodeDoubleStream(int length, final byte[] encoded) throws IOException {
    final var result =
            new StreamMetadata(
                    PhysicalStreamType.DATA,
                    null,
                    LogicalLevelTechnique.NONE,
                    LogicalLevelTechnique.NONE,
                    PhysicalLevelTechnique.NONE,
                    length,
                    encoded.length)
                    .encode();
    result.add(encoded);
    return result;
  }
}
