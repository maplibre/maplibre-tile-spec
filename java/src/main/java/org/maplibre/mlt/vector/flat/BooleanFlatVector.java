package org.maplibre.mlt.vector.flat;

import org.maplibre.mlt.vector.BitVector;
import org.maplibre.mlt.vector.Vector;
import java.nio.ByteBuffer;

public class BooleanFlatVector extends Vector<ByteBuffer, Boolean> {
  private final BitVector dataVector;

  public BooleanFlatVector(String name, BitVector dataVector, int size) {
    super(name, dataVector.getBuffer(), size);
    this.dataVector = dataVector;
  }

  public BooleanFlatVector(String name, BitVector nullabilityBuffer, BitVector dataVector) {
    super(name, nullabilityBuffer, dataVector.getBuffer());
    this.dataVector = dataVector;
  }

  @Override
  protected Boolean getValueFromBuffer(int index) {
    return this.dataVector.get(index);
  }

  public int size() {
    return this.dataVector.size();
  }
}
