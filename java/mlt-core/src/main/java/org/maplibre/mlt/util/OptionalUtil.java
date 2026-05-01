package org.maplibre.mlt.util;

import java.util.Comparator;
import java.util.Optional;
import java.util.function.BiConsumer;
import java.util.function.BiFunction;
import org.jetbrains.annotations.NotNull;

public class OptionalUtil {
  /// Compare two Optional values with the specified comparator.
  // Empty is not less than any value, and any value is less than empty.
  @SuppressWarnings("OptionalUsedAsFieldOrParameterType")
  public static <T extends Comparable<T>> boolean isLessThan(
      @NotNull Optional<T> a, @NotNull Optional<T> b) {
    return isLessThan(a, b, Comparator.<T>naturalOrder());
  }

  /// Compare two Optional values.
  // Empty is not less than any value, and any value is less than empty.
  @SuppressWarnings("OptionalUsedAsFieldOrParameterType")
  private static <T> boolean isLessThan(
      @NotNull Optional<T> a, @NotNull Optional<T> b, @NotNull Comparator<T> comparator) {
    return a.map(value -> b.map(t -> comparator.compare(value, t) < 0).orElse(true)).orElse(false);
  }

  /// Map two Optional values to a single Optional value using the specified function.
  /// The result is empty if either input is empty.
  @SuppressWarnings("OptionalUsedAsFieldOrParameterType")
  public static <T, U, V> Optional<V> map(
      @NotNull Optional<T> a,
      @NotNull Optional<U> b,
      @NotNull BiFunction<? super T, ? super U, ? extends V> f) {
    return a.flatMap(av -> b.map(bv -> f.apply(av, bv)));
  }

  /// Apply a function to two Optional values if both are present.
  @SuppressWarnings("OptionalUsedAsFieldOrParameterType")
  public static <T, U> void apply(
      @NotNull Optional<T> a, @NotNull Optional<U> b, @NotNull BiConsumer<? super T, ? super U> f) {
    a.flatMap(
        av ->
            b.map(
                bv -> {
                  f.accept(av, bv);
                  return 0;
                }));
  }
}
