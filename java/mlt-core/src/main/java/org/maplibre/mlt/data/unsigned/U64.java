package org.maplibre.mlt.data.unsigned;

import java.math.BigInteger;

public record U64(long value) implements Unsigned {

  public static U64 of(BigInteger value) {
    if (value.signum() < 0 || value.bitLength() > 64) {
      throw new IllegalArgumentException("Out of range for u64");
    }
    return new U64(value.longValue());
  }

  @Override
  public Byte byteValue() {
    final var v = value;
    if ((byte) v == v) {
      return (byte) v;
    }
    return null;
  }

  @Override
  public Integer intValue() {
    final var v = value;
    if ((int) v == v) {
      return (int) v;
    }
    return null;
  }

  @Override
  public Long longValue() {
    return value;
  }

  @Override
  public String toString() {
    return "u64(" + Long.toUnsignedString(value) + ")";
  }
}
