package org.maplibre.mlt.vector.constant;

import org.maplibre.mlt.vector.BitVector;
import java.nio.IntBuffer;

public class IntConstVector extends ConstVector<IntBuffer, Integer> {

  public IntConstVector(String name, Integer value, int size) {
    super(name, IntBuffer.wrap(new int[] {value}), size);
  }

  public IntConstVector(String name, BitVector nullabilityBuffer, Integer value) {
    super(name, nullabilityBuffer, IntBuffer.wrap(new int[] {value}));
  }

  @Override
  protected Integer getValueFromBuffer(int index) {
    return dataBuffer.get(0);
  }
}
