package org.maplibre.mlt.vector.flat;

import java.nio.DoubleBuffer;
import org.maplibre.mlt.vector.BitVector;
import org.maplibre.mlt.vector.Vector;

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
