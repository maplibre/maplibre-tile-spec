package org.maplibre.mlt.decoder;

import java.io.IOException;
import java.math.BigDecimal;
import java.math.BigInteger;
import java.util.List;
import java.util.Map;
import org.junit.jupiter.api.Test;
import org.maplibre.mlt.data.unsigned.U32;
import org.maplibre.mlt.data.unsigned.U64;

/// Unit tests for MAP property encoder/decoder
class MapDecoderTest {

  @Test
  void nestedNull() throws IOException {
    // empty is distinct from null
    final var features =
        List.of(
            PropertyDecoderTest.feature(0, Map.of("a", Map.of("b", Map.of()))),
            PropertyDecoderTest.feature(1, Map.of()));

    PropertyDecoderTest.assertRoundTripEncoding(features);
  }

  @Test
  void nestedList() throws IOException {
    // can contain lists of mixed types, list-within-map and map-within-list
    final var features =
        List.of(
            PropertyDecoderTest.feature(
                0, Map.of("a", Map.of("b", List.of(1, "2", Map.of("c", List.of(3.0, 4)))))));

    PropertyDecoderTest.assertRoundTripEncoding(features);
  }

  @Test
  void nestedListRootWithMixedTypes() throws IOException {
    final var features =
        List.of(
            PropertyDecoderTest.feature(
                0,
                Map.of(
                    "a",
                    List.of(
                        true,
                        1,
                        (long) Integer.MAX_VALUE + 2,
                        U32.of(Integer.MAX_VALUE + 3L),
                        U64.of(BigInteger.valueOf(Long.MAX_VALUE).add(BigInteger.valueOf(4L))),
                        "5",
                        6.0,
                        (double) Float.MAX_VALUE + 7))));

    PropertyDecoderTest.assertRoundTripEncoding(features);
  }

  @Test
  void nestedBig() throws IOException {
    // BigInteger and BigDecimal support
    final var features =
        List.of(
            PropertyDecoderTest.feature(
                0,
                Map.of(
                    "a",
                    List.of(
                        BigInteger.valueOf(1),
                        BigInteger.valueOf(-2),
                        BigInteger.valueOf(Integer.MAX_VALUE + 3L),
                        BigInteger.valueOf(Long.MAX_VALUE).add(BigInteger.valueOf(5L)),
                        BigDecimal.valueOf(6.0f),
                        BigDecimal.valueOf(7.0),
                        BigDecimal.valueOf(Math.nextUp(8.0))))));

    PropertyDecoderTest.assertRoundTripEncoding(features);
  }

  @Test
  void nestedSpecials() throws IOException {
    // Make sure special float values are supported in nested properties
    final var features =
        List.of(
            PropertyDecoderTest.feature(
                0,
                Map.of(
                    "a",
                    List.of(
                        Float.NaN,
                        Double.NaN,
                        Float.POSITIVE_INFINITY,
                        Float.NEGATIVE_INFINITY,
                        Double.POSITIVE_INFINITY,
                        Double.NEGATIVE_INFINITY))));

    PropertyDecoderTest.assertRoundTripEncoding(features);
  }

  @Test
  void nestedMixedRoot() throws IOException {
    // Mixed scalar types in the same column as a nested value are supported
    final var features =
        List.of(
            PropertyDecoderTest.feature(0, Map.of("a", "b")),
            PropertyDecoderTest.feature(1, Map.of("a", Map.of("b", "c"))),
            PropertyDecoderTest.feature(2, Map.of("a", Math.PI)));

    PropertyDecoderTest.assertRoundTripEncodingWithCoercion(features);
  }

  @Test
  void nestedShared() throws IOException {
    // Multiple nested columns share dictionaries
    final var features =
        List.of(
            PropertyDecoderTest.feature(0, Map.of("name:a", "b")),
            PropertyDecoderTest.feature(1, Map.of("name:a", Map.of("b", "c"))),
            PropertyDecoderTest.feature(2, Map.of("name:b", Map.of("c", List.of("b")))),
            PropertyDecoderTest.feature(3, Map.of("name:b", "b")),
            PropertyDecoderTest.feature(4, Map.of("name:c", Map.of("b", Math.PI))),
            PropertyDecoderTest.feature(5, Map.of("name:c", Math.PI)));

    PropertyDecoderTest.assertRoundTripEncodingWithCoercion(features);
  }
}
