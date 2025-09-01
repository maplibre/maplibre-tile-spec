package org.maplibre.mlt.vector.flat;

import org.maplibre.mlt.vector.BitVector;
import org.maplibre.mlt.vector.Vector;
import java.nio.LongBuffer;

public class LongFlatVector extends Vector<LongBuffer, Long> {
  public LongFlatVector(String name, LongBuffer dataBuffer, int size) {
    super(name, dataBuffer, size);
  }

  public LongFlatVector(String name, BitVector nullabilityBuffer, LongBuffer dataBuffer) {
    super(name, nullabilityBuffer, dataBuffer);
  }

  @Override
  protected Long getValueFromBuffer(int index) {
    return dataBuffer.get(index);
  }
}
