package com.mlt.vector;

import java.nio.Buffer;
import java.util.Optional;

public abstract class Vector<T extends Buffer, K> {
  protected final Optional<BitVector> nullabilityBuffer;
  protected final T dataBuffer;
  protected final String name;

  public Vector(String name, T dataBuffer) {
    this(name, null, dataBuffer);
  }

  public Vector(String name, BitVector nullabilityBuffer, T dataBuffer) {
    this.name = name;
    this.nullabilityBuffer =
        nullabilityBuffer != null ? Optional.of(nullabilityBuffer) : Optional.empty();
    this.dataBuffer = dataBuffer;
  }

  public String getName() {
    return this.name;
  }

  public Optional<K> getValue(int index) {
    return (this.nullabilityBuffer.isPresent() && !this.nullabilityBuffer.get().get(index))
        ? Optional.empty()
        : Optional.of(getValueFromBuffer(index));
  }

  public int size() {
    // TODO: change StringDictionaryVector to work with this encoding
    return this.nullabilityBuffer.map(BitVector::size).orElseGet(this.dataBuffer::capacity);
  }

  protected abstract K getValueFromBuffer(int index);
}
