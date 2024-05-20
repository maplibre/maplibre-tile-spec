package com.mlt.vector.flat;

import com.mlt.vector.BitVector;
import com.mlt.vector.Vector;

import java.nio.LongBuffer;

public class LongFlatVector extends Vector<LongBuffer, Long> {
    public LongFlatVector(String name, LongBuffer dataBuffer) {
        super(name, dataBuffer);
    }

    public LongFlatVector(String name, BitVector nullabilityBuffer, LongBuffer dataBuffer) {
        super(name, nullabilityBuffer, dataBuffer);
    }

    @Override
    protected Long getValueFromBuffer(int index) {
        return dataBuffer.get(index);
    }

}