package com.mlt.decoder.vectorized;

import com.mlt.metadata.stream.StreamMetadata;
import com.mlt.vector.BitVector;
import java.nio.ByteBuffer;
import java.nio.ByteOrder;
import java.nio.FloatBuffer;
import me.lemire.integercompression.IntWrapper;

public class VectorizedFloatDecoder {
  private VectorizedFloatDecoder() {}

  public static FloatBuffer decodeFloatStream(
      byte[] data, IntWrapper offset, StreamMetadata streamMetadata) {
    var floatBuffer =
        ByteBuffer.wrap(data, offset.get(), streamMetadata.byteLength())
            .order(ByteOrder.LITTLE_ENDIAN)
            .asFloatBuffer();
    offset.add(streamMetadata.byteLength());
    return floatBuffer;
  }

  public static FloatBuffer decodeNullableFloatStream(
      byte[] data, IntWrapper offset, StreamMetadata streamMetadata, BitVector nullabilityBuffer) {
    // TODO: refactor for performance
    var floatBuffer = decodeFloatStream(data, offset, streamMetadata);

    floatBuffer.position(0);
    var nullableFloatBuffer = new float[nullabilityBuffer.size()];
    for (var i = 0; i < nullabilityBuffer.size(); i++) {
      // TODO: or use Float.NaN -> check performance
      nullableFloatBuffer[i] = nullabilityBuffer.get(i) ? floatBuffer.get() : 0;
    }

    return FloatBuffer.wrap(nullableFloatBuffer);
  }
}
