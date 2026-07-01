package org.maplibre.mlt.data.unsigned;

import java.math.BigInteger;
import java.util.Objects;

/**
 * Represents an unsigned integer of a fixed bit width (8, 32, or 64 bits).
 *
 * <p>This interface abstracts over different unsigned integer types in Java, which do not natively
 * support unsigned primitives, providing a common API for working with unsigned values regardless
 * of their underlying representation.
 */
public sealed interface Unsigned extends Comparable<Unsigned> permits U8, U32, U64 {
  Byte byteValue();

  Integer intValue();

  BigInteger bigIntValue();

  Long longValue();

  @Override
  default int compareTo(Unsigned other) {
    Objects.requireNonNull(other, "other");
    final var valueComparison = Long.compareUnsigned(longValue(), other.longValue());
    if (valueComparison != 0) {
      return valueComparison;
    }

    // Keep natural ordering consistent with equals across implementations.
    return Integer.compare(typeRank(this), typeRank(other));
  }

  private static int typeRank(Unsigned value) {
    return switch (value) {
      case U8 ignored -> 0;
      case U32 ignored -> 1;
      case U64 ignored -> 2;
    };
  }

  @Override
  String toString();
}
