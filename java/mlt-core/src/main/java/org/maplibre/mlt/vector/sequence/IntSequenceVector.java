package org.maplibre.mlt.vector.sequence;

import java.nio.IntBuffer;

public class IntSequenceVector extends SequenceVector<IntBuffer, Integer> {

  public IntSequenceVector(String name, Integer baseValue, int delta, int size) {
    super(name, IntBuffer.wrap(new int[] {baseValue}), delta, size);
  }

  @Override
  protected Integer getValueFromBuffer(int index) {
    return dataBuffer.get(0) + index * delta;
  }
}
