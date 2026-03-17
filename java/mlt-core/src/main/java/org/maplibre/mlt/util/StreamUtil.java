package org.maplibre.mlt.util;

import java.util.Iterator;
import java.util.Objects;
import java.util.Spliterator;
import java.util.Spliterators;
import java.util.function.BiConsumer;
import java.util.function.BiFunction;
import java.util.stream.Stream;
import java.util.stream.StreamSupport;
import org.apache.commons.lang3.tuple.Pair;

public final class StreamUtil {
  private StreamUtil() {}

  /// Combine two streams into one by applying a function to pairs of elements from the input
  /// streams.
  /// The resulting stream will have the same number of elements as the shorter input stream, and
  /// any remaining elements in the longer stream will be ignored.
  /// The resulting stream:
  /// - Is lazy and will only compute elements as needed, allowing for efficient processing of large
  /// or infinite streams.
  /// - Is parallel if either of the input streams is parallel.
  /// - Has the intersection of the characteristics of input streams with DISTINCT and SORTED
  /// removed
  /// - Is sized if both input streams are sized, the size being the minimum of the sizes of the
  /// input streams
  /// @param a the first input stream, providing the first argument to the function
  /// @param b the second input stream, providing the second argument to the function
  /// @param f the function to apply to pairs of elements from the input streams
  /// @return a stream of the results of applying the function to pairs of elements from the input
  /// streams
  /// @throws NullPointerException if any of the input streams or the function is null
  // Really, this isn't in `Stream` anywhere?
  public static <A, B, C> Stream<C> zip(
      Stream<? extends A> a,
      Stream<? extends B> b,
      BiFunction<? super A, ? super B, ? extends C> f) {
    Objects.requireNonNull(f);
    final var spliterA = Objects.requireNonNull(a).spliterator();
    final var spliterB = Objects.requireNonNull(b).spliterator();
    final var cIterator = new ZipIterator<A, B, C>(spliterA, spliterB, f);

    // Eliminate DISTINCT and SORTED characteristics
    final int characteristics =
        spliterA.characteristics()
            & spliterB.characteristics()
            & ~(Spliterator.DISTINCT | Spliterator.SORTED);

    // the zipped result will be the size of the smaller stream
    final var sizeIfKnown =
        ((characteristics & Spliterator.SIZED) != 0)
            ? Math.min(spliterA.getExactSizeIfKnown(), spliterB.getExactSizeIfKnown())
            : -1;

    return StreamSupport.stream(
        (sizeIfKnown < 0)
            ? Spliterators.spliteratorUnknownSize(cIterator, characteristics)
            : Spliterators.spliterator(cIterator, sizeIfKnown, characteristics),
        a.isParallel() || b.isParallel());
  }

  private static final class ZipIterator<A, B, C> implements Iterator<C> {
    private final Iterator<A> aIterator;
    private final Iterator<B> bIterator;
    private final BiFunction<? super A, ? super B, ? extends C> function;

    ZipIterator(
        Spliterator<? extends A> a,
        Spliterator<? extends B> b,
        BiFunction<? super A, ? super B, ? extends C> f) {
      aIterator = Spliterators.iterator(a);
      bIterator = Spliterators.iterator(b);
      function = f;
    }

    @Override
    public boolean hasNext() {
      // Stop when either stream is exhausted, ignoring remaining elements in the longer stream
      return aIterator.hasNext() && bIterator.hasNext();
    }

    @Override
    public C next() {
      return function.apply(aIterator.next(), bIterator.next());
    }
  }

  /// Return a lazy stream of `Pair<>` objects
  public static <A, B> Stream<Pair<A, B>> zip(Stream<? extends A> a, Stream<? extends B> b) {
    return zip(a, b, Pair::of);
  }

  /// Run the given function for each pair of elements from the two streams.
  /// @return the number of pairs processed
  public static <A, B> long zipEach(
      Stream<? extends A> a, Stream<? extends B> b, BiConsumer<? super A, ? super B> f) {
    return zip(
            a,
            b,
            (x, y) -> {
              f.accept(x, y);
              return 1L;
            })
        .reduce(0L, Long::sum);
  }
}
