#pragma once

#include <common.hpp>

namespace mlt::util::decoding {

#if 0
  public static float[] decodeFloatsLE(byte[] encodedValues, IntWrapper pos, int numValues) {
    var fb =
        ByteBuffer.wrap(encodedValues, pos.get(), numValues * Float.BYTES)
            .order(ByteOrder.LITTLE_ENDIAN)
            .asFloatBuffer();
    pos.set(pos.get() + numValues * Float.BYTES);
    var decodedValues = new float[fb.limit()];
    fb.get(decodedValues);
    return decodedValues;
  }
#endif

}
