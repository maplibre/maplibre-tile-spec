package org.maplibre.mlt.converter.encodings;

import static org.junit.jupiter.api.Assertions.assertArrayEquals;
import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;

import java.io.IOException;
import java.util.BitSet;
import java.util.List;
import java.util.stream.IntStream;
import me.lemire.integercompression.IntWrapper;
import org.junit.jupiter.api.Test;
import org.maplibre.mlt.decoder.DecodingUtils;

public class EncodingUtilsTest {

  @Test
  public void encodeRle_MixedRunsAndLiterals_ValidEncoding() {
    var values = List.of(1, 1, 1, 2, 4, 5, 8, 8, 8, 8, 9, 9);
    var expectedRuns = List.of(3, 1, 1, 1, 4, 2);
    var expectedValues = List.of(1, 2, 4, 5, 8, 9);

    var actualValues = EncodingUtils.encodeRle(values.stream().mapToInt(i -> i).toArray());

    assertEquals(expectedRuns, IntStream.of(actualValues.getLeft()).boxed().toList());
    assertEquals(expectedValues, IntStream.of(actualValues.getRight()).boxed().toList());
  }

  @Test
  public void encodeRle_OnlyLiterals_ValidEncoding() {
    var values = List.of(1, 2, 3, 4, 5, 6, 7, 8);
    var expectedRuns = List.of(1, 1, 1, 1, 1, 1, 1, 1);
    var expectedValues = List.of(1, 2, 3, 4, 5, 6, 7, 8);

    var actualValues = EncodingUtils.encodeRle(values.stream().mapToInt(i -> i).toArray());

    assertEquals(expectedRuns, IntStream.of(actualValues.getLeft()).boxed().toList());
    assertEquals(expectedValues, IntStream.of(actualValues.getRight()).boxed().toList());
  }

  @Test
  public void encodeRle_OnlyRuns_ValidEncoding() {
    var values = List.of(10, 10, 10, 10, 20, 20, 40, 40, 40, 40);
    var expectedRuns = List.of(4, 2, 4);
    var expectedValues = List.of(10, 20, 40);

    var actualValues = EncodingUtils.encodeRle(values.stream().mapToInt(i -> i).toArray());

    assertEquals(expectedRuns, IntStream.of(actualValues.getLeft()).boxed().toList());
    assertEquals(expectedValues, IntStream.of(actualValues.getRight()).boxed().toList());
  }

  @Test
  public void encodeBooleanRle() throws IOException {
    var numValues = 70;
    var bitset = new BitSet();
    for (var i = 0; i < numValues; i++) {
      bitset.set(i, false);
    }

    var encodedBooleans = EncodingUtils.encodeBooleanRle(bitset, numValues);

    var decodeBooleans =
        DecodingUtils.decodeBooleanRle(
            encodedBooleans, numValues, encodedBooleans.length, new IntWrapper(0));

    for (var i = 0; i < numValues; i++) {
      assertFalse(decodeBooleans.get(i));
    }
  }

  @Test
  public void encodeFastPfor128_ReusedCodec_RoundTripsIndependentArrays() {
    var codec = EncodingUtils.createFastPforCodec();
    var first = new int[] {5, 10, 15, 20, 25, 30, 35, 40};
    var second = new int[] {1, 2, 3, 4, 100, 200, 300, 400, 500};

    var encodedFirst = EncodingUtils.encodeFastPfor128(first, false, false, codec);
    var encodedSecond = EncodingUtils.encodeFastPfor128(second, false, false, codec);

    assertArrayEquals(
        first,
        DecodingUtils.decodeFastPfor(
            encodedFirst, first.length, encodedFirst.length, new IntWrapper(0)));
    assertArrayEquals(
        second,
        DecodingUtils.decodeFastPfor(
            encodedSecond, second.length, encodedSecond.length, new IntWrapper(0)));
  }

  @Test
  public void encodeFastPfor128_NullCodec_RoundTrips() {
    var values = new int[] {5, 10, 15, 20, 25, 30, 35, 40};
    var encoded = EncodingUtils.encodeFastPfor128(values, false, false, null);

    assertArrayEquals(
        values,
        DecodingUtils.decodeFastPfor(encoded, values.length, encoded.length, new IntWrapper(0)));
  }
}
