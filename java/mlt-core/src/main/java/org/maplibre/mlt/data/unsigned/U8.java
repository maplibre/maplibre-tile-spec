package org.maplibre.mlt.data.unsigned;

public record U8(byte value) implements Unsigned {
  public static U8 of(int value) {
    if (value < 0 || value > 255) {
      throw new IllegalArgumentException("Out of range for u8");
    }
    return new U8((byte) value);
  }

  @Override
  public String toString() {
    return "u8(" + Byte.toUnsignedInt(value) + ")";
  }

  @Override
  public Long longValue() {
    return Long.valueOf(Byte.toUnsignedInt(value));
  }
}
