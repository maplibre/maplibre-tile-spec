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

  // Really, this isn't in `Stream` anywhere?
  public static <A, B, C> Stream<C> zip(
      Stream<? extends A> a,
      Stream<? extends B> b,
      BiFunction<? super A, ? super B, ? extends C> f) {
    Objects.requireNonNull(f);
    final var spliterA = Objects.requireNonNull(a).spliterator();
    final var spliterB = Objects.requireNonNull(b).spliterator();

    // Eliminate DISTINCT and SORTED characteristics
    final int characteristics =
        spliterA.characteristics()
            & spliterB.characteristics()
            & ~(Spliterator.DISTINCT | Spliterator.SORTED);

    // If both streams are SIZED, they must have the same size
    final var sized = (characteristics & Spliterator.SIZED) != 0;
    if (sized && spliterA.getExactSizeIfKnown() != spliterB.getExactSizeIfKnown()) {
      throw new IllegalStateException("Streams have different sizes");
    }

    final var cIterator = new ZipIterator<A, B, C>(spliterA, spliterB, f);

    // Get sizes, if both are available.  Zipped result is the smaller of the two sizes.
    final var spliterator =
        sized
            ? Spliterators.spliterator(
                cIterator,
                Math.min(spliterA.getExactSizeIfKnown(), spliterB.getExactSizeIfKnown()),
                characteristics)
            : Spliterators.spliteratorUnknownSize(cIterator, characteristics);
    return StreamSupport.stream(spliterator, a.isParallel() || b.isParallel());
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
      final var aHasNext = aIterator.hasNext();
      final var bHasNext = bIterator.hasNext();
      if (aHasNext != bHasNext) {
        throw new IllegalStateException("Streams have different sizes");
      }
      return aHasNext && bHasNext;
    }

    @Override
    public C next() {
      return function.apply(aIterator.next(), bIterator.next());
    }
  }

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
              return 1;
            })
        .reduce(0, Integer::sum);
  }
}
