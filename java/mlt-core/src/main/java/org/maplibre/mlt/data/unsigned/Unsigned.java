package org.maplibre.mlt.data.unsigned;

/**
 * Represents an unsigned integer of a fixed bit width (8, 32, or 64 bits).
 *
 * <p>This interface abstracts over different unsigned integer types in Java, which do not natively
 * support unsigned primitives, providing a common API for working with unsigned values regardless
 * of their underlying representation.
 */
public sealed interface Unsigned permits U8, U32, U64 {
  Byte byteValue();

  Integer intValue();

  Long longValue();

  @Override
  String toString();
}
