package org.maplibre.mlt.vector.sequence;

import java.nio.LongBuffer;

public class LongSequenceVector extends SequenceVector<LongBuffer, Long> {

  public LongSequenceVector(String name, Long baseValue, long delta, int size) {
    super(name, LongBuffer.wrap(new long[] {baseValue}), delta, size);
  }

  @Override
  protected Long getValueFromBuffer(int index) {
    return dataBuffer.get(0) + index * delta;
  }
}
