package com.mlt.vector.flat;

import com.mlt.vector.BitVector;
import com.mlt.vector.Vector;
import java.nio.DoubleBuffer;

public class DoubleFlatVector extends Vector<DoubleBuffer, Double> {
  public DoubleFlatVector(String name, DoubleBuffer dataBuffer, int size) {
    super(name, dataBuffer, size);
  }

  public DoubleFlatVector(String name, BitVector nullabilityBuffer, DoubleBuffer dataBuffer) {
    super(name, nullabilityBuffer, dataBuffer);
  }

  @Override
  protected Double getValueFromBuffer(int index) {
    return dataBuffer.get(index);
  }
}
