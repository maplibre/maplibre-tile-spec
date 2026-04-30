package org.maplibre.mlt.util;

import org.apache.commons.lang3.exception.UncheckedException;

import java.util.function.Function;

public class ExceptionUtil {
  @FunctionalInterface
  public interface ThrowingFunction<T, R, E extends Exception> {
    R apply(T t) throws E;
  }

  /// Wraps a function that throws a checked exception in a RuntimeException, allowing
  /// it to be used in contexts that don't allow checked exceptions, e.g., streams.
  public static <T, R, E extends Exception> Function<T, R> unchecked(ThrowingFunction<T, R, E> f) {
    return t -> {
      try {
        return f.apply(t);
      } catch (RuntimeException e) {
        throw e;
      } catch (Exception e) {
        throw new UncheckedException(e);
      }
    };
  }
}
