package org.maplibre.mlt.vector.constant;

import java.nio.Buffer;
import org.maplibre.mlt.vector.BitVector;
import org.maplibre.mlt.vector.Vector;

public abstract class ConstVector<T extends Buffer, K> extends Vector<T, K> {

  public ConstVector(String name, T buffer, int size) {
    super(name, buffer, size);
    this.size = size;
  }

  public ConstVector(String name, BitVector nullabilityBuffer, T buffer) {
    super(name, nullabilityBuffer, buffer);
  }
}
