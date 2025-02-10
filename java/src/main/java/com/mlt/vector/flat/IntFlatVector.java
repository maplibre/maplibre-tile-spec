package com.mlt.vector.flat;

import com.mlt.vector.BitVector;
import com.mlt.vector.Vector;
import java.nio.IntBuffer;

public class IntFlatVector extends Vector<IntBuffer, Integer> {
  public IntFlatVector(String name, IntBuffer dataBuffer, int size) {
    super(name, dataBuffer, size);
  }

  public IntFlatVector(String name, BitVector nullabilityBuffer, IntBuffer dataBuffer) {
    super(name, nullabilityBuffer, dataBuffer);
  }

  @Override
  protected Integer getValueFromBuffer(int index) {
    return dataBuffer.get(index);
  }
}
