package org.maplibre.mlt.decoder;

import java.io.IOException;
import java.util.List;
import me.lemire.integercompression.IntWrapper;
import org.junit.jupiter.api.Test;
import org.locationtech.jts.util.Assert;
import org.maplibre.mlt.converter.encodings.DoubleEncoder;
import org.maplibre.mlt.converter.encodings.FloatEncoder;
import org.maplibre.mlt.metadata.stream.StreamMetadata;
import org.maplibre.mlt.util.ByteArrayUtil;

public class DoubleDecoderTest {

  @Test
  public void decodeDoubleStream_DoubleEncodedValues_ReturnsExactValues() throws IOException {
    final var encoded = ByteArrayUtil.concat(DoubleEncoder.encodeDoubleStream(f64Values));
    final var offset = new IntWrapper(0);
    final var streamMetadata = StreamMetadata.decode(encoded, offset);
    final var decoded = DoubleDecoder.decodeDoubleStream(encoded, offset, streamMetadata);
    Assert.equals(f64Values, decoded);
  }

  @Test
  public void decodeDoubleStream_FloatEncodedValues_ReturnsConvertedDoubleValues()
      throws IOException {
    final var f32Values = f64Values.stream().map(Double::floatValue).toList();
    final var encoded = ByteArrayUtil.concat(FloatEncoder.encodeFloatStream(f32Values));
    final var offset = new IntWrapper(0);
    final var streamMetadata = StreamMetadata.decode(encoded, offset);
    final var decoded = DoubleDecoder.decodeDoubleStream(encoded, offset, streamMetadata);
    Assert.equals(f32Values.stream().map(Double::valueOf).toList(), decoded);
  }

  @Test
  public void decodeDoubleStream_EmptyStream_ReturnsEmptyList() throws IOException {
    final var values = List.<Double>of();
    final var encoded = ByteArrayUtil.concat(DoubleEncoder.encodeDoubleStream(values));
    final var offset = new IntWrapper(0);
    final var streamMetadata = StreamMetadata.decode(encoded, offset);
    final var decoded = DoubleDecoder.decodeDoubleStream(encoded, offset, streamMetadata);
    Assert.equals(values, decoded);
  }

  final List<Double> f64Values =
      List.of(
          1.25,
          -3.5,
          6.02214076e23,
          -0.0,
          123456.789,
          Double.POSITIVE_INFINITY,
          Double.NEGATIVE_INFINITY,
          Double.NaN);
}
