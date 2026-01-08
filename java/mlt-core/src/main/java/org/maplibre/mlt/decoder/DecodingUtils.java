package org.maplibre.mlt.decoder;

import java.io.ByteArrayInputStream;
import java.io.IOException;
import java.io.InputStream;
import java.nio.ByteBuffer;
import java.nio.ByteOrder;
import java.nio.IntBuffer;
import java.nio.charset.StandardCharsets;
import java.util.Arrays;
import java.util.BitSet;
import me.lemire.integercompression.*;
import org.apache.commons.lang3.tuple.Pair;

public class DecodingUtils {
  private DecodingUtils() {}

  // TODO: quick and dirty -> optimize for performance
  public static int[] decodeVarints(byte[] src, IntWrapper pos, int numValues) throws IOException {
    var values = new int[numValues];
    var dstOffset = 0;
    for (var i = 0; i < numValues; i++) {
      var offset = decodeVarint(src, pos.get(), values, dstOffset);
      dstOffset++;
      pos.set(offset);
    }
    return values;
  }

  public static long[] decodeLongVarints(byte[] src, IntWrapper pos, int numValues) {
    var values = new long[numValues];
    for (var i = 0; i < numValues; i++) {
      var value = decodeLongVarint(src, pos);
      values[i] = value;
    }
    return values;
  }

  public static long decodeLongVarint(byte[] bytes, IntWrapper pos) {
    // TODO: write faster decoding method for varint
    long value = 0;
    int shift = 0;
    int index = pos.get();
    while (index < bytes.length) {
      byte b = bytes[index++];
      value |= (long) (b & 0x7F) << shift;
      if ((b & 0x80) == 0) {
        break;
      }
      shift += 7;
      if (shift > 63) {
        throw new IllegalArgumentException("Varint too long");
      }
    }

    pos.set(index);
    return value;
  }

  // Source:
  // https://github.com/bazelbuild/bazel/blob/master/src/main/java/com/google/devtools/build/lib/util/VarInt.java
  /**
   * Reads a varint from src, places its values into the first element of dst and returns the offset
   * in to src of the first byte after the varint.
   *
   * @param src source buffer to retrieve from
   * @param srcOffset offset within src
   * @param dst the resulting int values
   * @param dstOffset offset with dst
   * @return the updated offset after reading the varint
   */
  private static int decodeVarint(byte[] src, int srcOffset, int[] dst, int dstOffset)
      throws IOException {
    try (var stream = new ByteArrayInputStream(src, srcOffset, src.length - srcOffset)) {
      final var result = decodeVarintWithLength(stream);
      dst[dstOffset] = result.getLeft();
      return srcOffset + result.getRight();
    }
  }

  public static Pair<Integer, Integer> decodeVarintWithLength(InputStream stream)
      throws IOException {
    var b = (byte) stream.read();
    var bytesRead = 1;
    int value = b & 0x7f;
    if ((b & 0x80) != 0) {
      b = (byte) stream.read();
      bytesRead++;
      value |= (b & 0x7f) << 7;
      if ((b & 0x80) != 0) {
        b = (byte) stream.read();
        bytesRead++;
        value |= (b & 0x7f) << 14;
        if ((b & 0x80) != 0) {
          b = (byte) stream.read();
          bytesRead++;
          value |= (b & 0x7f) << 21;
          if ((b & 0x80) != 0) {
            b = (byte) stream.read();
            bytesRead++;
            value |= (b & 0x7f) << 28;
            if ((b & 0x80) != 0 || 15 < b) {
              throw new IOException("Varint overflow");
            }
          }
        }
      }
    }
    return Pair.of(value, bytesRead);
  }

  public static int decodeVarint(InputStream stream) throws IOException {
    return decodeVarintWithLength(stream).getLeft();
  }

  public static String decodeString(InputStream stream) throws IOException {
    var length = decodeVarint(stream);
    return new String(stream.readNBytes(length), StandardCharsets.UTF_8);
  }

  public static int decodeZigZag(int encoded) {
    return (encoded >>> 1) ^ (-(encoded & 1));
  }

  public static long decodeZigZag(long encoded) {
    return (encoded >>> 1) ^ (-(encoded & 1));
  }

  public static int[] decodeFastPfor(
      byte[] encodedValues, int numValues, int byteLength, IntWrapper pos) {
    var encodedValuesSlice = Arrays.copyOfRange(encodedValues, pos.get(), pos.get() + byteLength);
    // TODO: get rid of that conversion
    IntBuffer intBuf =
        ByteBuffer.wrap(encodedValuesSlice)
            // TODO: change to little endian
            .order(ByteOrder.BIG_ENDIAN)
            .asIntBuffer();
    int[] intValues = new int[(int) Math.ceil(byteLength / 4d)];
    for (var i = 0; i < intValues.length; i++) {
      intValues[i] = intBuf.get(i);
    }

    int[] decodedValues = new int[numValues];
    var inputOffset = new IntWrapper(0);
    var outputOffset = new IntWrapper(0);
    IntegerCODEC ic = new Composition(new FastPFOR(), new VariableByte());
    ic.uncompress(intValues, inputOffset, intValues.length, decodedValues, outputOffset);

    pos.add(byteLength);
    return decodedValues;
  }

  public static int[] decodeFastPforDeltaCoordinates(
      byte[] encodedValues, int numValues, int byteLength, IntWrapper pos) {
    var encodedValuesSlice = Arrays.copyOfRange(encodedValues, pos.get(), pos.get() + byteLength);
    // TODO: get rid of that conversion
    IntBuffer intBuf =
        ByteBuffer.wrap(encodedValuesSlice)
            // TODO: change to little endian
            .order(ByteOrder.BIG_ENDIAN)
            .asIntBuffer();
    int[] intValues = new int[(int) Math.ceil(byteLength / 4d)];
    for (var i = 0; i < intValues.length; i++) {
      intValues[i] = intBuf.get(i);
    }

    int[] decompressedValues = new int[numValues];
    var inputOffset = new IntWrapper(0);
    var outputOffset = new IntWrapper(0);
    IntegerCODEC ic = new Composition(new FastPFOR(), new VariableByte());
    ic.uncompress(intValues, inputOffset, intValues.length, decompressedValues, outputOffset);

    var decodedValues = new int[numValues];
    for (var i = 0; i < numValues; i++) {
      var zigZagValue = decompressedValues[i];
      decodedValues[i] = (zigZagValue >>> 1) ^ (-(zigZagValue & 1));
    }

    pos.set(pos.get() + byteLength);

    var values = new int[numValues];
    var previousValueX = 0;
    var previousValueY = 0;
    for (var i = 0; i < numValues; i += 2) {
      var deltaX = decodedValues[i];
      var deltaY = decodedValues[i + 1];
      var x = previousValueX + deltaX;
      var y = previousValueY + deltaY;
      values[i] = x;
      values[i + 1] = y;

      previousValueX = x;
      previousValueY = y;
    }

    return values;
  }

  public static byte[] decodeByteRle(byte[] buffer, int numBytes, int byteSize, IntWrapper pos) {
    var reader = new ByteRleDecoder(buffer, pos.get(), byteSize);

    var values = new byte[numBytes];
    for (var i = 0; i < numBytes; i++) {
      values[i] = reader.next();
    }

    pos.add(byteSize);
    return values;
  }

  public static BitSet decodeBooleanRle(
      byte[] buffer, int numBooleans, int byteSize, IntWrapper pos) {
    var numBytes = (int) Math.ceil(numBooleans / 8d);
    var byteStream = decodeByteRle(buffer, numBytes, byteSize, pos);
    // TODO: get rid of that conversion
    return BitSet.valueOf(byteStream);
  }

  public static int[] decodeUnsignedRLE(int[] data, int numRuns, int numTotalValues) {
    var values = new int[numTotalValues];
    var offset = 0;
    for (var i = 0; i < numRuns; i++) {
      var runLength = data[i];
      var value = data[i + numRuns];
      for (var j = offset; j < offset + runLength; j++) {
        values[j] = value;
      }

      offset += runLength;
    }
    return values;
  }

  public static long[] decodeUnsignedRLE(long[] data, int numRuns, int numTotalValues) {
    var values = new long[numTotalValues];
    var offset = 0L;
    for (var i = 0; i < numRuns; i++) {
      var runLength = data[i];
      var value = data[i + numRuns];
      for (var j = offset; j < offset + runLength; j++) {
        values[(int) j] = value;
      }

      offset += runLength;
    }
    return values;
  }

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
}
