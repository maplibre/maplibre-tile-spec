package com.mlt.vector.constant;

import com.mlt.vector.BitVector;
import com.mlt.vector.Vector;
import java.nio.IntBuffer;

public class IntConstVector extends Vector<IntBuffer, Integer> {
  public IntConstVector(String name, Integer value) {
    super(name, IntBuffer.wrap(new int[] {value}));
  }

  public IntConstVector(String name, BitVector nullabilityBuffer, Integer value) {
    super(name, nullabilityBuffer, IntBuffer.wrap(new int[] {value}));
  }

  @Override
  protected Integer getValueFromBuffer(int index) {
    return dataBuffer.get(0);
  }
}
