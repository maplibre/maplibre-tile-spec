package org.springmeyer;

import java.nio.charset.StandardCharsets;
import java.util.Arrays;

/*
 * This is a partial port of https://github.com/mapbox/pbf to Java.
 */
public class Pbf {
  private byte[] buf;
  public int pos;
  public int type;
  public int length;

  public static final int Varint = 0;
  public static final int Fixed64 = 1;
  public static final int Bytes = 2;
  public static final int Fixed32 = 5;

  private static final long SHIFT_LEFT_32 = (1L << 16) * (1L << 16);

  public Pbf(byte[] buf) {
    this.buf = buf != null ? buf : new byte[0];
    this.pos = 0;
    this.type = 0;
    this.length = this.buf.length;
  }

  public <T> T readFields(ReadField<T> readField, T result, int end) {
    end = end == 0 ? this.length : end;

    while (this.pos < end) {
      int val = this.readVarint();
      int tag = val >> 3;
      int startPos = this.pos;

      this.type = val & 0x7;
      readField.read(tag, result, this);

      if (this.pos == startPos) this.skip(val);
    }
    return result;
  }

  public float readFloat() {
    float val = Float.intBitsToFloat(readInt32(this.buf, this.pos));
    this.pos += 4;
    return val;
  }

  public long readFixed64() {
    long val =
        readUInt32(this.buf, this.pos)
            + ((long) readUInt32(this.buf, this.pos + 4) * SHIFT_LEFT_32);
    this.pos += 8;
    return val;
  }

  public double readDouble() {
    double val = Double.longBitsToDouble(readFixed64());
    this.pos += 8;
    return val;
  }

  public int readVarint() {
    return this.readVarint(false);
  }

  public int readVarint(boolean isSigned) {
    int val = 0;
    int b = 0;
    var buf = this.buf;

    b = buf[this.pos++];
    val = b & 0x7f;
    if ((b & 0x80) == 0) return val;
    b = buf[this.pos++];
    val |= (b & 0x7f) << 7;
    if ((b & 0x80) == 0) return val;
    b = buf[this.pos++];
    val |= (b & 0x7f) << 14;
    if ((b & 0x80) == 0) return val;
    b = buf[this.pos++];
    val |= (b & 0x7f) << 21;
    if ((b & 0x80) == 0) return val;

    b = buf[this.pos];
    val |= (b & 0x0f) << 28;

    return readVarintRemainder(val, isSigned);
  }

  private int readVarintRemainder(int val, boolean isSigned) {
    int high = 0;
    int b = 0;

    b = this.buf[this.pos++];
    high = (b & 0x70) >> 4;
    if ((b & 0x80) == 0) return toNum(val, high, isSigned);
    b = this.buf[this.pos++];
    high |= (b & 0x7f) << 3;
    if ((b & 0x80) == 0) return toNum(val, high, isSigned);
    b = this.buf[this.pos++];
    high |= (b & 0x7f) << 10;
    if ((b & 0x80) == 0) return toNum(val, high, isSigned);
    b = this.buf[this.pos++];
    high |= (b & 0x7f) << 17;
    if ((b & 0x80) == 0) return toNum(val, high, isSigned);
    b = this.buf[this.pos++];
    high |= (b & 0x7f) << 24;
    if ((b & 0x80) == 0) return toNum(val, high, isSigned);
    b = this.buf[this.pos++];
    high |= (b & 0x01) << 31;
    if ((b & 0x80) == 0) return toNum(val, high, isSigned);

    throw new IllegalArgumentException("Expected varint not more than 10 bytes");
  }

  private static int toNum(int low, int high, boolean isSigned) {
    if (isSigned) {
      return (int) (high * 0x100000000L) + (low >>> 0);
    }
    return (int) ((high >>> 0) * 0x100000000L) + (low >>> 0);
  }

  public long readVarint64() {
    return this.readVarint(true);
  }

  public int readSVarint() {
    int num = this.readVarint();
    return num % 2 == 1 ? (num + 1) / -2 : num / 2;
  }

  public boolean readBoolean() {
    return this.readVarint() != 0;
  }

  public String readString() {
    int end = this.readVarint() + this.pos;
    int pos = this.pos;
    this.pos = end;
    return new String(Arrays.copyOfRange(this.buf, pos, end), StandardCharsets.UTF_8);
  }

  public byte[] readBytes() {
    int end = this.readVarint() + this.pos;
    byte[] buffer = new byte[end - this.pos];
    System.arraycopy(this.buf, this.pos, buffer, 0, buffer.length);
    this.pos = end;
    return buffer;
  }

  public void skip(int val) {
    int type = val & 0x7;
    if (type == Varint) {
      while (this.buf[this.pos++] > 0x7F) {}
    } else if (type == Bytes) {
      this.pos = this.readVarint() + this.pos;
    } else if (type == Fixed32) {
      this.pos += 4;
    } else if (type == Fixed64) {
      this.pos += 8;
    } else {
      throw new IllegalArgumentException("Unimplemented type: " + type);
    }
  }

  private static int readUInt32(byte[] buf, int pos) {
    return (buf[pos] & 0xFF)
        | ((buf[pos + 1] & 0xFF) << 8)
        | ((buf[pos + 2] & 0xFF) << 16)
        | ((buf[pos + 3] & 0xFF) << 24);
  }

  private static int readInt32(byte[] buf, int pos) {
    return (buf[pos] & 0xFF)
        | ((buf[pos + 1] & 0xFF) << 8)
        | ((buf[pos + 2] & 0xFF) << 16)
        | ((buf[pos + 3] & 0xFF) << 24);
  }

  @FunctionalInterface
  public interface ReadField<T> {
    void read(int tag, T result, Pbf pbf);
  }
}
