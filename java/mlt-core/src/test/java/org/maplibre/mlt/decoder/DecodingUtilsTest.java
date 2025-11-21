package org.maplibre.mlt.decoder;

import static org.junit.jupiter.api.Assertions.assertArrayEquals;

import java.io.IOException;
import me.lemire.integercompression.IntWrapper;
import org.junit.jupiter.api.Test;
import org.locationtech.jts.util.Assert;
import org.maplibre.mlt.converter.encodings.EncodingUtils;
import org.maplibre.mlt.decoder.vectorized.VectorizedDecodingUtils;

public class DecodingUtilsTest {

  @Test
  public void decodeLongVarints() throws IOException {
    var value = (long) Math.pow(2, 54);
    var value2 = (long) Math.pow(2, 17);
    var value3 = (long) Math.pow(2, 57);
    var encodedValues =
        EncodingUtils.encodeVarints(new long[] {value, value2, value3}, false, false);

    final var pos = new IntWrapper(0);
    final var decodedValue = DecodingUtils.decodeLongVarints(encodedValues, pos, 3);

    Assert.equals(value, decodedValue[0]);
    Assert.equals(value2, decodedValue[1]);
    Assert.equals(value3, decodedValue[2]);

    Assert.equals(encodedValues.length, pos.get());
  }

  @Test
  public void decodeZigZag_LongValue() {
    var value = (long) Math.pow(2, 54);
    var value2 = (long) -Math.pow(2, 44);
    var encodedValue = EncodingUtils.encodeZigZag(value);
    var encodedValue2 = EncodingUtils.encodeZigZag(value2);

    var decodedValue = DecodingUtils.decodeZigZag(encodedValue);
    var decodedValue2 = DecodingUtils.decodeZigZag(encodedValue2);

    Assert.equals(value, decodedValue);
    Assert.equals(value2, decodedValue2);
  }

  @Test
  void decodeFastPfor() {
    var values = new int[] {5, 10, 15, 20, 25, 30, 35, 40};
    var encoded = EncodingUtils.encodeFastPfor128(values, false, false);
    var pos = new IntWrapper(0);
    var decoded =
        VectorizedDecodingUtils.decodeFastPfor(encoded, values.length, encoded.length, pos);

    assertArrayEquals(values, decoded.array());
  }
}
