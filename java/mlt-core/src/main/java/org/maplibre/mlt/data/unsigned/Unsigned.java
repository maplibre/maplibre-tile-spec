package org.maplibre.mlt.data.unsigned;

/**
 * Represents an unsigned integer of a fixed bit width (8, 32, or 64 bits).
 *
 * <p>This interface abstracts over different unsigned integer types in Java, which do not natively
 * support unsigned primitives
 */
public sealed interface Unsigned permits U8, U32, U64 {
  @Override
  String toString();

  /**
   * @return the value of this unsigned integer as a Byte or null if the value is out of range
   */
  default Byte byteValue() {
    final var v = this.longValue().longValue();
    if ((byte) v == v) {
      return (byte) v;
    }
    return null;
  }

  /**
   * @return the value of this unsigned integer as a Integer or null if the value is out of range
   */
  default Integer intValue() {
    final var v = this.longValue().longValue();
    if ((int) v == v) {
      return (int) v;
    }
    return null;
  }

  /**
   * @return the value of this unsigned integer as a Long or null if the value is out of range
   */
  Long longValue();
}
