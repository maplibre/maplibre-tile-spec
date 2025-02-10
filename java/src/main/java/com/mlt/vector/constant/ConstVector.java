package com.mlt.vector.constant;

import com.mlt.vector.BitVector;
import com.mlt.vector.Vector;
import java.nio.Buffer;

public abstract class ConstVector<T extends Buffer, K> extends Vector<T, K> {

  public ConstVector(String name, T buffer, int size) {
    super(name, buffer, size);
    this.size = size;
  }

  public ConstVector(String name, BitVector nullabilityBuffer, T buffer) {
    super(name, nullabilityBuffer, buffer);
  }
}
