package org.maplibre.mlt.converter.encodings;

import java.io.ByteArrayInputStream;
import java.io.IOException;
import java.nio.ByteBuffer;
import java.util.Arrays;
import java.util.stream.IntStream;
import java.util.stream.LongStream;
import me.lemire.integercompression.IntWrapper;
import org.junit.jupiter.api.Test;
import org.locationtech.jts.util.Assert;
import org.maplibre.mlt.decoder.DecodingUtils;

public class VarintTest {
  private static final int[] intTestValues = {
    0,
    1,
    0x7F,
    0x80,
    0xFE,
    0xFF,
    0x100,
    0xFFF,
    0x1000,
    0x4000,
    0xFFFF,
    0x10000,
    0x40000,
    0xFFFFF,
    0x100000,
    0x1FFFFF,
    0x400000,
    0xFFFFFFF,
    0x10000000,
    0x40000000,
    0xFFFFFFFF,
    Integer.MAX_VALUE
  };
  private static final long[] longTestValues = {
    0x100000000L,
    0x400000000L,
    0xFFFFFFFFFL,
    0x1000000000000000L,
    0x4000000000000000L,
    Long.MAX_VALUE
  };

  @Test
  public void testVarIntEncoding() throws IOException {
    for (int value : Arrays.stream(intTestValues).flatMap(x -> IntStream.of(x, -x)).toArray()) {
      final var encoded =
          EncodingUtils.putVarInt(value, ByteBuffer.wrap(new byte[EncodingUtils.MAX_VARINT_SIZE]))
              .flip();
      Assert.equals(EncodingUtils.getVarIntSize(value), encoded.limit());
      final var decoded1 = DecodingUtils.decodeVarints(encoded.array(), new IntWrapper(0), 1)[0];
      Assert.equals(value, decoded1);
      final var decoded2 = DecodingUtils.decodeVarint(new ByteArrayInputStream(encoded.array()));
      Assert.equals(value, decoded2);
      final var decoded3 =
          DecodingUtils.decodeVarintWithLength(new ByteArrayInputStream(encoded.array()));
      Assert.equals(value, decoded3.getLeft());
      Assert.equals(encoded.limit(), decoded3.getRight());
    }
  }

  @Test
  public void testVarLongEncoding() throws IOException {
    final var ints = Arrays.stream(intTestValues).asLongStream().flatMap(x -> LongStream.of(x, -x));
    final var longs = Arrays.stream(longTestValues).flatMap(x -> LongStream.of(x, -x));
    for (long value : LongStream.concat(ints, longs).toArray()) {
      final var encoded =
          EncodingUtils.putVarInt(value, ByteBuffer.wrap(new byte[EncodingUtils.MAX_VARLONG_SIZE]))
              .flip();
      Assert.equals(EncodingUtils.getVarLongSize(value), encoded.limit());
      final var decoded = DecodingUtils.decodeLongVarint(encoded.array(), new IntWrapper(0));
      Assert.equals(value, decoded);
    }
  }
}
