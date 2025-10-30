package org.maplibre.mlt.converter.encodings;

import java.io.ByteArrayInputStream;
import java.io.IOException;
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
      final var encoded = new byte[EncodingUtils.MAX_VARINT_SIZE];
      final var offset = EncodingUtils.putVarInt(value, encoded, 0);
      Assert.equals(EncodingUtils.getVarIntSize(value), offset);
      final var decoded1 = DecodingUtils.decodeVarints(encoded, new IntWrapper(0), 1)[0];
      Assert.equals(value, decoded1);
      final var decoded2 = DecodingUtils.decodeVarint(new ByteArrayInputStream(encoded));
      Assert.equals(value, decoded2);
      final var decoded3 = DecodingUtils.decodeVarintWithLength(new ByteArrayInputStream(encoded));
      Assert.equals(value, decoded3.getLeft());
      Assert.equals(offset, decoded3.getRight());
    }
  }

  @Test
  public void testVarLongEncoding() throws IOException {
    final var ints = Arrays.stream(intTestValues).asLongStream().flatMap(x -> LongStream.of(x, -x));
    final var longs = Arrays.stream(longTestValues).flatMap(x -> LongStream.of(x, -x));
    for (long value : LongStream.concat(ints, longs).toArray()) {
      final var encoded = new byte[EncodingUtils.MAX_VARLONG_SIZE];
      final var offset = EncodingUtils.putVarInt(value, encoded, 0);
      Assert.equals(EncodingUtils.getVarLongSize(value), offset);
      final var decoded = DecodingUtils.decodeLongVarint(encoded, new IntWrapper(0));
      Assert.equals(value, decoded);
    }
  }
}
