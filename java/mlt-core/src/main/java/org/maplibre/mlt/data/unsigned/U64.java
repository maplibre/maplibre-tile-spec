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
  public String toString() {
    return "u64(" + Long.toUnsignedString(value) + ")";
  }

  @Override
  public Long longValue() {
    return value;
  }
}
