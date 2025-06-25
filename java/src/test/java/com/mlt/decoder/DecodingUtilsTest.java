package com.mlt.decoder;

import static org.junit.jupiter.api.Assertions.assertArrayEquals;
import static org.junit.jupiter.api.Assertions.assertTrue;

import com.mlt.converter.encodings.EncodingUtils;
import com.mlt.decoder.vectorized.VectorizedDecodingUtils;
import com.mlt.vector.BitVector;
import java.nio.ByteBuffer;
import java.util.Arrays;
import me.lemire.integercompression.IntWrapper;
import org.apache.commons.lang3.ArrayUtils;
import org.junit.jupiter.api.Disabled;
import org.junit.jupiter.api.Test;
import org.locationtech.jts.util.Assert;

public class DecodingUtilsTest {

  @Test
  public void decodeLongVarint() {
    var value = (long) Math.pow(2, 54);
    var value2 = (long) Math.pow(2, 17);
    var value3 = (long) Math.pow(2, 57);
    var encodedValues =
        EncodingUtils.encodeVarints(new long[] {value, value2, value3}, false, false);

    var pos = new IntWrapper(0);
    var decodedValue = DecodingUtils.decodeLongVarint(encodedValues, pos, 3);

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
  @Disabled
  public void decodeNullableDelta() {
    var values = new int[] {10, 14, 16, 19};
    var expectedValues = new int[] {10, 14, 14, 16, 16, 19, 19};
    var deltaValues = new int[] {10, 4, 2, 3};
    // 0101011
    var byteBuffer = ByteBuffer.wrap(new byte[] {0x2B});
    var bitVector = new BitVector(byteBuffer, 7);

    var decodedData = VectorizedDecodingUtils.decodeNullableZigZagDelta(bitVector, deltaValues);

    assertArrayEquals(expectedValues, decodedData);
  }

  @Test
  void deltaOfDeltaDecoding() {
    var offsets = new int[] {2, 6, 8, 14};
    var numParts = new int[] {2, 4, 2, 6}; // delta coded
    var deltaCodedNumParts = new int[] {2, 2, -2, 4}; // delta of delta coded
    var zigZagEncodedValues = EncodingUtils.encodeZigZag(deltaCodedNumParts);

    var decodedValues = VectorizedDecodingUtils.zigZagDeltaOfDeltaDecoding(zigZagEncodedValues);

    assertTrue(Arrays.equals(ArrayUtils.addAll(new int[] {0}, offsets), decodedValues));
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
