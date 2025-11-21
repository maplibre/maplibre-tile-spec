package org.maplibre.mlt.decoder.vectorized;

import java.nio.*;
import me.lemire.integercompression.*;

/* the redundant implementations in this class are mainly to avoid branching and therefore speed up the decoding */
public class VectorizedDecodingUtils {

  private static IntegerCODEC ic;

  public static IntBuffer decodeFastPfor(
      byte[] buffer, int numValues, int byteLength, IntWrapper offset) {
    if (ic == null) {
      ic = new Composition(new FastPFOR(), new VariableByte());
    }

    /* Create a vectorized conversion from the ByteBuffer to the IntBuffer */
    // TODO: get rid of that conversion
    IntBuffer intBuf =
        ByteBuffer.wrap(buffer, offset.get(), byteLength).order(ByteOrder.BIG_ENDIAN).asIntBuffer();
    var bufferSize = (int) Math.ceil(byteLength / 4d);
    int[] intValues = new int[bufferSize];
    for (var i = 0; i < intValues.length; i++) {
      intValues[i] = intBuf.get(i);
    }

    int[] decodedValues = new int[numValues];
    ic.uncompress(intValues, new IntWrapper(0), intValues.length, decodedValues, new IntWrapper(0));

    offset.add(byteLength);
    return IntBuffer.wrap(decodedValues);
  }

  /* Delta encoding  ------------------------------------------------------------------------------*/

  /*
   * In place decoding of the zigzag delta encoded Vec2.
   * Inspired by https://github.com/lemire/JavaFastPFOR/blob/master/src/main/java/me/lemire/integercompression/differential/Delta.java
   */
  public static void decodeComponentwiseDeltaVec2(int[] data) {
    data[0] = (data[0] >>> 1) ^ ((data[0] << 31) >> 31);
    data[1] = (data[1] >>> 1) ^ ((data[1] << 31) >> 31);
    int sz0 = data.length / 4 * 4;
    int i = 2;
    if (sz0 >= 4) {
      for (; i < sz0 - 4; i += 4) {
        var x1 = data[i];
        var y1 = data[i + 1];
        var x2 = data[i + 2];
        var y2 = data[i + 3];

        data[i] = ((x1 >>> 1) ^ ((x1 << 31) >> 31)) + data[i - 2];
        data[i + 1] = ((y1 >>> 1) ^ ((y1 << 31) >> 31)) + data[i - 1];
        data[i + 2] = ((x2 >>> 1) ^ ((x2 << 31) >> 31)) + data[i];
        data[i + 3] = ((y2 >>> 1) ^ ((y2 << 31) >> 31)) + data[i + 1];
      }
    }

    for (; i != data.length; i += 2) {
      data[i] = ((data[i] >>> 1) ^ ((data[i] << 31) >> 31)) + data[i - 2];
      data[i + 1] = ((data[i + 1] >>> 1) ^ ((data[i + 1] << 31) >> 31)) + data[i - 1];
    }
  }
}
