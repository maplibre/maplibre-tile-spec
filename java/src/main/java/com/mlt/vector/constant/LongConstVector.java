package com.mlt.vector.constant;

import com.mlt.vector.BitVector;
import com.mlt.vector.Vector;
import java.nio.LongBuffer;

public class LongConstVector extends Vector<LongBuffer, Long> {
  public LongConstVector(String name, Long value) {
    super(name, LongBuffer.wrap(new long[] {value}));
  }

  public LongConstVector(String name, BitVector nullabilityBuffer, Long value) {
    super(name, nullabilityBuffer, LongBuffer.wrap(new long[] {value}));
  }

  @Override
  protected Long getValueFromBuffer(int index) {
    return dataBuffer.get(0);
  }
}
