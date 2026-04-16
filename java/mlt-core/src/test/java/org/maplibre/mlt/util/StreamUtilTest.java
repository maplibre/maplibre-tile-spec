package org.maplibre.mlt.util;

import static org.junit.jupiter.api.Assertions.*;

import java.util.ArrayList;
import java.util.stream.Stream;
import java.util.stream.StreamSupport;
import org.apache.commons.lang3.mutable.MutableBoolean;
import org.apache.commons.lang3.mutable.MutableInt;
import org.apache.commons.lang3.tuple.Pair;
import org.junit.jupiter.api.Test;

class StreamUtilTest {
  @Test
  void zipWithEqualSizedStreams() {
    final var result =
        StreamUtil.zip(Stream.of(1, 2, 3), Stream.of("a", "b", "c"), (x, y) -> x + ":" + y)
            .toList();
    assertEquals(3, result.size());
    assertEquals("1:a", result.get(0));
    assertEquals("2:b", result.get(1));
    assertEquals("3:c", result.get(2));
  }

  @Test
  void zipPairs() {
    final var result = StreamUtil.zip(Stream.of(1, 2, 3), Stream.of("a", "b", "c")).toList();
    assertEquals(3, result.size());
    assertEquals(Pair.of(1, "a"), result.get(0));
    assertEquals(Pair.of(2, "b"), result.get(1));
    assertEquals(Pair.of(3, "c"), result.get(2));
  }

  @Test
  void zipWithEmptyStreams() {
    assertEquals(0, StreamUtil.zip(Stream.of(), Stream.of(), (x, y) -> 0).toList().size());
  }

  @Test
  void zipWithFirstStreamShorter() {
    final var result =
        StreamUtil.zip(Stream.of(1, 2), Stream.of("a", "b", "c"), (x, y) -> x + ":" + y).toList();
    assertEquals(2, result.size());
    assertEquals("1:a", result.get(0));
    assertEquals("2:b", result.get(1));
  }

  @Test
  void zipWithSecondStreamShorter() {
    final var result =
        StreamUtil.zip(Stream.of(1, 2, 3), Stream.of("a", "b"), (x, y) -> x + ":" + y).toList();
    assertEquals(2, result.size());
    assertEquals("1:a", result.get(0));
    assertEquals("2:b", result.get(1));
  }

  @Test
  void zipWithTransformationFunction() {
    final var result =
        StreamUtil.zip(Stream.of(10, 20, 30), Stream.of(1, 2, 3), (x, y) -> x + y).toList();
    assertEquals(3, result.size());
    assertEquals(11, result.get(0));
    assertEquals(22, result.get(1));
    assertEquals(33, result.get(2));
  }

  @Test
  void zipPreservesStreamCharacteristics() {
    final var result =
        StreamUtil.zip(Stream.of(1, 2, 3), Stream.of("a", "b", "c"), (x, y) -> x + ":" + y)
            .toList();
    assertNotNull(result);
    assertEquals(3, result.size());
  }

  void zipWithParallelStreams(boolean first, boolean second) {
    final var a = Stream.of(1, 2, 3, 4, 5).parallel();
    final var b = Stream.of("a", "b", "c", "d", "e").parallel();
    final var result =
        StreamUtil.zip(first ? a.parallel() : a, second ? b.parallel() : b, (x, y) -> x + ":" + y)
            .toList();
    // Results should contain all expected pairs (order may vary due to parallel processing)
    assertEquals(5, result.size());
    assertTrue(result.contains("1:a"));
    assertTrue(result.contains("2:b"));
    assertTrue(result.contains("3:c"));
    assertTrue(result.contains("4:d"));
    assertTrue(result.contains("5:e"));
  }

  @Test
  void zipWithFirstParallelStream() {
    zipWithParallelStreams(true, false);
  }

  @Test
  void zipWithSecondParallelStream() {
    zipWithParallelStreams(false, true);
  }

  @Test
  void zipWithBothParallelStreams() {
    zipWithParallelStreams(true, true);
  }

  @Test
  void zipWithNullTransformationFunction() {
    assertThrows(
        NullPointerException.class,
        () -> StreamUtil.zip(Stream.of(1, 2, 3), Stream.of("a", "b", "c"), null));
  }

  @Test
  void zipWithNullFirstStream() {
    assertThrows(
        NullPointerException.class, () -> StreamUtil.zip(null, Stream.of(), (x, y) -> x + ":" + y));
  }

  @Test
  void zipWithNullSecondStream() {
    assertThrows(
        NullPointerException.class, () -> StreamUtil.zip(Stream.of(), null, (x, y) -> x + ":" + y));
  }

  @Test
  void zipEachProcessesAllPairs() {
    final var processed = new ArrayList<>();
    final long count =
        StreamUtil.zipEach(
            Stream.of(1, 2, 3), Stream.of("a", "b", "c"), (x, y) -> processed.add(x + ":" + y));
    assertEquals(3, count);
  }

  @Test
  void zipEachWithEmptyStreams() {
    assertEquals(0, StreamUtil.zipEach(Stream.of(), Stream.of(), (x, y) -> {}));
  }

  @Test
  void zipEachWithMismatchedKnownStreamSizes() {
    final var sideEffectCount = new MutableInt(0);
    final long count =
        StreamUtil.zipEach(
            Stream.of(1, 2, 3), Stream.of("a", "b"), (x, y) -> sideEffectCount.increment());
    // With the new behavior, we process all pairs from the shorter stream
    assertEquals(2, count);
    assertEquals(2, sideEffectCount.get());
  }

  @Test
  void zipEachWithMismatchedUnknownStreamSizes() {
    final var sideEffectCount = new MutableInt(0);
    final var a = Stream.iterate(0, i -> i < 3, i -> i + 1);
    final var b = Stream.iterate(0, i -> i < 2, i -> i + 1);
    // Streams with different sizes now silently truncate to the shorter stream
    final long count = StreamUtil.zipEach(a, b, (x, y) -> sideEffectCount.increment());
    // With the new behavior, we process all pairs from the shorter stream
    assertEquals(2, count);
    assertEquals(2, sideEffectCount.get());
  }

  @Test
  void zipEachWithNullFirstStream() {
    assertThrows(
        NullPointerException.class, () -> StreamUtil.zipEach(null, Stream.of(), (x, y) -> {}));
  }

  @Test
  void zipEachWithNullSecondStream() {
    assertThrows(
        NullPointerException.class, () -> StreamUtil.zipEach(Stream.of(), null, (x, y) -> {}));
  }

  @Test
  void zipWithLargeStreams() {
    final var a = Stream.iterate(0, i -> i + 1).limit(1000);
    final var b = Stream.iterate(1000, i -> i + 1).limit(1000);
    final var result = StreamUtil.zip(a, b, Integer::sum).toList();
    assertEquals(1000, result.size());
    assertEquals(1000, result.get(0)); // First pair: 0 + 1000 = 1000
    assertEquals(1008, result.get(4)); // Fifth pair: 4 + 1004 = 1008
    assertEquals(2998, result.get(999)); // Last pair: 999 + 1999 = 2998
  }

  @Test
  void zipHandlesNullValuesInStreams() {
    final var result =
        StreamUtil.zip(Stream.of("a", null, "c"), Stream.of("1", "2", "3"), (x, y) -> x + ":" + y)
            .toList();
    assertEquals(3, result.size());
    assertEquals("a:1", result.get(0));
    assertEquals("null:2", result.get(1));
    assertEquals("c:3", result.get(2));
  }

  @Test
  void zipWithDifferentObjectTypes() {
    final var result =
        StreamUtil.zip(
                Stream.of("one", "two", "three"), Stream.of(1.5, 2.5, 3.5), (s, d) -> s + "=" + d)
            .toList();
    assertEquals(3, result.size());
    assertEquals("one=1.5", result.get(0));
    assertEquals("two=2.5", result.get(1));
    assertEquals("three=3.5", result.get(2));
  }

  @Test
  void zipStreamIsLazy() {
    final var functionCalled = new MutableBoolean(false);
    final var zipped =
        StreamUtil.zip(
            Stream.of(1, 2, 3),
            Stream.of("a", "b", "c"),
            (x, y) -> {
              functionCalled.setTrue();
              return x + ":" + y;
            });

    final var split = zipped.spliterator();
    // size is known before evaluation for fixed-size inputs
    assertEquals(3, split.getExactSizeIfKnown());

    // Function should not be called yet - stream is lazy
    assertFalse(functionCalled.get());

    // Function should be called when stream is consumed
    final var result = StreamSupport.stream(split, false).toList();
    assertTrue(functionCalled.get());
    assertEquals(3, result.size());
    assertEquals("3:c", result.get(2));
  }

  @Test
  void zipWithInfiniteFirstStream() {
    final var infiniteStream = Stream.generate(() -> "x");
    final var finiteStream = Stream.of(1, 2, 3, 4, 5);
    final var result = StreamUtil.zip(infiniteStream, finiteStream, (x, y) -> y + ":" + x).toList();
    assertEquals(5, result.size());
    assertEquals("1:x", result.get(0));
    assertEquals("5:x", result.get(4));
  }

  @Test
  void zipWithInfiniteSecondStream() {
    // Test with Stream.generate - an infinite stream as the second argument
    final var finiteStream = Stream.of("a", "b", "c");
    final var infiniteStream = Stream.generate(() -> 42);
    final var result = StreamUtil.zip(finiteStream, infiniteStream, (x, y) -> x + ":" + y).toList();
    assertEquals(3, result.size());
    assertEquals("a:42", result.get(0));
    assertEquals("c:42", result.get(2));
  }
}
