package com.mlt.vector.flat;

import com.mlt.vector.BitVector;
import com.mlt.vector.Vector;
import java.nio.FloatBuffer;

public class FloatFlatVector extends Vector<FloatBuffer, Float> {
  public FloatFlatVector(String name, FloatBuffer dataBuffer, int size) {
    super(name, dataBuffer, size);
  }

  public FloatFlatVector(String name, BitVector nullabilityBuffer, FloatBuffer dataBuffer) {
    super(name, nullabilityBuffer, dataBuffer);
  }

  @Override
  protected Float getValueFromBuffer(int index) {
    return dataBuffer.get(index);
  }
}
