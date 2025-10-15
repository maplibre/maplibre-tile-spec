package org.maplibre.mlt.vector.flat;

import java.nio.IntBuffer;
import org.maplibre.mlt.vector.BitVector;
import org.maplibre.mlt.vector.Vector;

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
