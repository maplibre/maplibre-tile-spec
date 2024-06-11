package com.mlt.vector.flat;

import com.mlt.vector.BitVector;
import com.mlt.vector.Vector;
import java.nio.Buffer;

enum VectorType {
  FIXED_SIZE,
  VARIABLE_SIZE
}

public abstract class FlatVector<T extends Buffer, K> extends Vector<T, K> {
  public FlatVector(String name, BitVector nullabilityBuffer, T dataBuffer) {
    super(name, nullabilityBuffer, dataBuffer);
  }

  public FlatVector(String name, T dataBuffer) {
    super(name, dataBuffer);
  }

  /*public static <T extends Buffer>FlatVector createVariableSizeVector(BitVector nullabilityBuffer,
                                                                      T dataBuffer, T offsetBuffer){
      return new FlatVector(nullabilityBuffer, dataBuffer);
  }

  public static <T extends Buffer>FlatVector createFixedSizeVector(BitVector nullabilityBuffer,
                                                 T dataBuffer){
      return new FlatVector(nullabilityBuffer, dataBuffer);
  }*/

}
