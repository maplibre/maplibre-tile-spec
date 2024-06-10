package com.mlt.vector;

import java.nio.ByteBuffer;

public class BitVector {

  private final ByteBuffer values;
  private final int size;

  // TODO: check how BitSet is ordered
  /**
   * @param values The byte buffer containing the bit values in least-significant bit (LSB)
   *     numbering
   */
  public BitVector(ByteBuffer values, int size) {
    this.values = values;
    this.size = size;
  }

  public boolean get(int index) {
    int byteIndex = index / 8;
    int bitIndex = index % 8;
    byte b = values.get(byteIndex);
    return ((b >> bitIndex) & 1) == 1;
  }

  public int getInt(int index) {
    int byteIndex = index / 8;
    int bitIndex = index % 8;
    byte b = values.get(byteIndex);
    return (b >> bitIndex) & 1;
  }

  public int size() {
    return size;
  }

  public ByteBuffer getBuffer() {
    return this.values;
  }
}
