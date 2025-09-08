package org.maplibre.mlt.vector.flat;

import java.nio.FloatBuffer;
import org.maplibre.mlt.vector.BitVector;
import org.maplibre.mlt.vector.Vector;

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
