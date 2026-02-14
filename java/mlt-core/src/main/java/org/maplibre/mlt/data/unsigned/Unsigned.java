package org.maplibre.mlt.data.unsigned;

/**
 * Represents an unsigned integer of a fixed bit width (8, 32, or 64 bits).
 *
 * <p>This interface abstracts over different unsigned integer types in Java, which does not
 * natively support unsigned primitives (except for {@link Byte#toUnsignedInt(byte)}, {@link
 * Integer#toUnsignedLong(int)}, and {@link Long#toUnsignedString(long)} for display/encoding
 * purposes).
 *
 * <p>Implementations are sealed to the following types:
 *
 * <ul>
 *   <li>{@link U8} - 8-bit unsigned integer (0..255)
 *   <li>{@link U32} - 32-bit unsigned integer (0..2^32-1)
 *   <li>{@link U64} - 64-bit unsigned integer (0..2^64-1)
 * </ul>
 *
 * <p>Use these types when you need to represent unsigned numeric values for encoding,
 * serialization, or other APIs that require explicit unsigned integers.
 */
public sealed interface Unsigned permits U8, U32, U64 {
  /**
   * Returns a debug-friendly string representation of this unsigned value.
   *
   * <p>Example outputs:
   *
   * <ul>
   *   <li>{@code u8(42)}
   *   <li>{@code u32(123456)}
   *   <li>{@code u64(0x1f4abcd123)}
   * </ul>
   *
   * <p>Intended for logging, debugging, or human-readable inspection.
   *
   * @return a string representation of the unsigned value
   */
  @Override
  String toString();
}
