package com.mlt.vector.sequence;

import com.mlt.vector.Vector;
import java.nio.Buffer;

public abstract class SequenceVector<T extends Buffer, K> extends Vector<T, K> {
  protected final K delta;

  public SequenceVector(String name, T baseValueBuffer, K delta, int size) {
    super(name, baseValueBuffer, size);
    this.delta = delta;
    this.size = size;
  }
}
