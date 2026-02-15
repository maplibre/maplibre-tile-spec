package org.maplibre.mlt.data.unsigned;

/**
 * Represents an unsigned integer of a fixed bit width (8, 32, or 64 bits).
 *
 * <p>This interface abstracts over different unsigned integer types in Java, which do not
 * natively support unsigned primitives
 */
public sealed interface Unsigned permits U8, U32, U64 {
  @Override
  String toString();
}
