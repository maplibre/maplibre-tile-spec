package org.maplibre.mlt.util;

import java.util.Iterator;
import java.util.Objects;
import java.util.Spliterator;
import java.util.Spliterators;
import java.util.function.BiConsumer;
import java.util.function.BiFunction;
import java.util.stream.Stream;
import java.util.stream.StreamSupport;

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

    // Get sizes, if both are available.  Zipped result is the smaller of the two sizes.
    final long zipSize =
        ((characteristics & Spliterator.SIZED) != 0)
            ? Math.min(spliterA.getExactSizeIfKnown(), spliterB.getExactSizeIfKnown())
            : 0;

    final var cIterator =
        new Iterator<C>() {
          private final Iterator<A> aIterator = Spliterators.iterator(spliterA);
          private final Iterator<B> bIterator = Spliterators.iterator(spliterB);

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
            return f.apply(aIterator.next(), bIterator.next());
          }
        };

    final var spliterator =
        (zipSize > 0)
            ? Spliterators.spliterator(cIterator, zipSize, characteristics)
            : Spliterators.spliteratorUnknownSize(cIterator, characteristics);
    return StreamSupport.stream(spliterator, a.isParallel() || b.isParallel());
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
              return null;
            })
        .count();
  }
}
