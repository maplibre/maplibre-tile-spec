package com.mlt.vector;

import java.nio.ByteBuffer;
import java.nio.IntBuffer;

public abstract class VariableSizeVector<K> extends Vector<ByteBuffer, K> {
  protected IntBuffer lengthBuffer;

  public VariableSizeVector(String name, IntBuffer lengthBuffer, ByteBuffer dataBuffer, int size) {
    super(name, dataBuffer, size);
    this.lengthBuffer = lengthBuffer;
  }

  public VariableSizeVector(
      String name, BitVector nullabilityBuffer, IntBuffer lengthBuffer, ByteBuffer dataBuffer) {
    super(name, nullabilityBuffer, dataBuffer);
    this.lengthBuffer = lengthBuffer;
  }
}
