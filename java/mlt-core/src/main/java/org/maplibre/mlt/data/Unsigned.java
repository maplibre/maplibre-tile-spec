package org.maplibre.mlt.data;

import java.math.BigInteger;

/**
 * Generic wrapper to mark numeric values as unsigned integers.
 *
 * <p>Java doesn't have unsigned primitive types, so this wrapper signals that a value should be
 * encoded as UINT_8, UINT_32, or UINT_64.
 */
public record Unsigned<T extends Number>(T value) {
  /** Creates an unsigned 8-bit integer (range: 0-255). */
  public static Unsigned<Byte> u8(int value) {
    if (value < 0 || value > 255) {
      throw new IllegalArgumentException("Value " + value + " is out of range for UInt8 [0, 255]");
    }
    return new Unsigned<>((byte) value);
  }

  /** Creates an unsigned 32-bit integer (range: 0 to 2^32-1). */
  public static Unsigned<Integer> u32(long value) {
    if (value < 0 || value > 0xFFFFFFFFL) {
      throw new IllegalArgumentException(
          "Value " + value + " is out of range for UInt32 [0, 2^32-1]");
    }
    return new Unsigned<>((int) value);
  }

  /** Creates an unsigned 64-bit integer (range: 0 to 2^64-1). */
  public static Unsigned<Long> u64(BigInteger value) {
    final BigInteger U64_MAX_VALUE = new BigInteger("18446744073709551615");

    if (value.compareTo(BigInteger.ZERO) < 0 || value.compareTo(U64_MAX_VALUE) > 0) {
      throw new IllegalArgumentException(
          "Value " + value + " is out of range for UInt64 [0, 2^64-1]");
    }
    return new Unsigned<>(value.longValue());
  }

  @Override
  public String toString() {
    return switch (value) {
      case Byte b -> "u8(" + Byte.toUnsignedInt(b) + ")";
      case Integer i -> "u32(" + Integer.toUnsignedLong(i) + ")";
      case Long l -> "u64(" + Long.toHexString(l) + ")";
      default -> throw new IllegalArgumentException("Unsupported type");
    };
  }
}
