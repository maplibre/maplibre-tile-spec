package org.maplibre.mlt.vector;

import java.nio.Buffer;
import java.util.Optional;

public abstract class Vector<T extends Buffer, K> {
  protected BitVector nullabilityBuffer;
  protected final String name;
  protected final T dataBuffer;
  protected int size;

  /**
   * @param name Name of the vector.
   * @param dataBuffer Buffer containing the data.
   * @param size Limit of how much data can be read from the Vector.
   */
  public Vector(String name, T dataBuffer, int size) {
    this.name = name;
    this.dataBuffer = dataBuffer;
    this.size = size;
  }

  public Vector(String name, BitVector nullabilityBuffer, T dataBuffer) {
    this(name, dataBuffer, nullabilityBuffer.size());
    this.nullabilityBuffer = nullabilityBuffer;
  }

  public String getName() {
    return this.name;
  }

  public Optional<K> getValue(int index) {
    return (this.nullabilityBuffer != null && !this.nullabilityBuffer.get(index))
        ? Optional.empty()
        : Optional.of(getValueFromBuffer(index));
  }

  public int size() {
    // TODO: change StringVectors to work with this encoding
    return this.size;
  }

  protected abstract K getValueFromBuffer(int index);
}
