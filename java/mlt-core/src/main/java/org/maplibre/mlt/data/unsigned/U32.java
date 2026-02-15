package org.maplibre.mlt.data.unsigned;

public record U32(int value) implements Unsigned {

  public static U32 of(long value) {
    if (value < 0 || value > 0xFFFFFFFFL) {
      throw new IllegalArgumentException("Out of range for u32");
    }
    return new U32((int) value);
  }

  @Override
  public String toString() {
    return "u32(" + Integer.toUnsignedLong(value) + ")";
  }
}
