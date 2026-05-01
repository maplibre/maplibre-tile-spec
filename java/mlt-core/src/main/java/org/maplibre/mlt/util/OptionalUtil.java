package org.maplibre.mlt.util;

import java.util.Comparator;
import java.util.Optional;
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
    if (a.isEmpty()) {
      return false;
    }
    if (b.isEmpty()) {
      return true;
    }
    return comparator.compare(a.get(), b.get()) < 0;
  }

  /// Map two Optional values to a single Optional value using the specified function.
  /// The result is empty if either input is empty.
  @SuppressWarnings("OptionalUsedAsFieldOrParameterType")
  public static <T, U, V> Optional<V> map(
      Optional<T> a, Optional<U> b, BiFunction<? super T, ? super U, ? extends V> f) {
    return a.flatMap(av -> b.map(bv -> f.apply(av, bv)));
  }
}
