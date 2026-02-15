package org.maplibre.mlt.data.unsigned;

/**
 * Represents an unsigned integer of a fixed bit width (8, 32, or 64 bits).
 *
 * <p>This interface abstracts over different unsigned integer types in Java, which do not natively
 * support unsigned primitives
 */
public sealed interface Unsigned permits U8, U32, U64 {
  default Byte byteValue() {
    if (this instanceof U8 u) {
      return u.value();
    } else if (this instanceof U32 u) {
      final var v = u.value();
      if ((byte) v == v) {
        return (byte) v;
      }
    } else if (this instanceof U64 u) {
      final var v = u.value();
      if ((byte) v == v) {
        return (byte) v;
      }
    }
    return null;
  }

  default Integer intValue() {
    if (this instanceof U8 u) {
      return (int) u.value();
    } else if (this instanceof U32 u) {
      return u.value();
    } else if (this instanceof U64 u) {
      final var v = u.value();
      if ((int) v == v) {
        return (int) v;
      }
    }
    return null;
  }

  default Long longValue() {
    if (this instanceof U8 u) {
      return (long) u.value();
    } else if (this instanceof U32 u) {
      return (long) u.value();
    } else if (this instanceof U64 u) {
      return u.value();
    }
    return null;
  }

  @Override
  String toString();
}
