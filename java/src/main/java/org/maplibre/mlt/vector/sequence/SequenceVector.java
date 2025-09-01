package org.maplibre.mlt.vector.sequence;

import java.nio.Buffer;
import org.maplibre.mlt.vector.Vector;

public abstract class SequenceVector<T extends Buffer, K> extends Vector<T, K> {
  protected final K delta;

  public SequenceVector(String name, T baseValueBuffer, K delta, int size) {
    super(name, baseValueBuffer, size);
    this.delta = delta;
    this.size = size;
  }
}
