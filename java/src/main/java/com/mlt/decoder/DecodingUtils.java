package com.mlt.decoder;

import com.mlt.converter.geometry.ZOrderCurve;
import java.io.IOException;
import java.io.InputStream;
import java.nio.ByteBuffer;
import java.nio.ByteOrder;
import java.nio.IntBuffer;
import java.nio.charset.StandardCharsets;
import java.util.Arrays;
import java.util.BitSet;
import java.util.List;
import me.lemire.integercompression.*;
import org.apache.orc.impl.BufferChunk;
import org.apache.orc.impl.InStream;
import org.apache.orc.impl.RunLengthByteReader;

public class DecodingUtils {
  private DecodingUtils() {}

  // TODO: quick and dirty -> optimize for performance
  public static int[] decodeVarint(byte[] src, IntWrapper pos, int numValues) {
    var values = new int[numValues];
    var dstOffset = 0;
    for (var i = 0; i < numValues; i++) {
      var offset = decodeVarint(src, pos.get(), values, dstOffset);
      dstOffset++;
      pos.set(offset);
    }
    return values;
  }

  public static long[] decodeLongVarint(byte[] src, IntWrapper pos, int numValues) {
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
      if (shift >= 64) {
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
   * @param offset offset within src
   * @param dst the resulting int values
   * @return the updated offset after reading the varint
   */
  private static int decodeVarint(byte[] src, int offset, int[] dst) {
    var dstOffset = 0;

    /*
     * Max 4 bytes supported.
     * */
    var b = src[offset++];
    var value = b & 0x7f;
    if ((b & 0x80) == 0) {
      dst[dstOffset] = value;
      return offset;
    }

    b = src[offset++];
    value |= (b & 0x7f) << 7;
    if ((b & 0x80) == 0) {
      dst[dstOffset] = value;
      return offset;
    }

    b = src[offset++];
    value |= (b & 0x7f) << 14;
    if ((b & 0x80) == 0) {
      dst[dstOffset] = value;
      return offset;
    }

    b = src[offset++];
    value |= (b & 0x7f) << 21;
    dst[dstOffset] = value;
    return offset;
  }

  private static int decodeVarint(byte[] src, int offset, int[] dst, int dstOffset) {
    /*
     * Max 4 bytes supported.
     * */
    var b = src[offset++];
    var value = b & 0x7f;
    if ((b & 0x80) == 0) {
      dst[dstOffset] = value;
      return offset;
    }

    b = src[offset++];
    value |= (b & 0x7f) << 7;
    if ((b & 0x80) == 0) {
      dst[dstOffset] = value;
      return offset;
    }

    b = src[offset++];
    value |= (b & 0x7f) << 14;
    if ((b & 0x80) == 0) {
      dst[dstOffset] = value;
      return offset;
    }

    b = src[offset++];
    value |= (b & 0x7f) << 21;
    dst[dstOffset] = value;
    return offset;
  }

  public static int decodeVarint(InputStream stream) throws IOException {
    var b = (byte) stream.read();
    var value = b & 0x7f;
    if ((b & 0x80) != 0) {
      b = (byte) stream.read();
      value |= (b & 0x7f) << 7;
      if ((b & 0x80) != 0) {
        b = (byte) stream.read();
        value |= (b & 0x7f) << 14;
        if ((b & 0x80) != 0) {
          b = (byte) stream.read();
          value |= (b & 0x7f) << 21;
        }
      }
    }
    return value;
  }

  public static String decodeString(InputStream stream) throws IOException {
    var length = decodeVarint(stream);
    return new String(stream.readNBytes(length), StandardCharsets.UTF_8);
  }

  public static int decodeZigZag(int encoded) {
    return (encoded >>> 1) ^ (-(encoded & 1));
  }

  public static void decodeZigZag(int[] encoded) {
    for (var i = 0; i < encoded.length; i++) {
      encoded[i] = decodeZigZag(encoded[i]);
    }
  }

  public static long decodeZigZag(long encoded) {
    return (encoded >>> 1) ^ (-(encoded & 1));
  }

  public static void decodeZigZag(long[] encoded) {
    for (var i = 0; i < encoded.length; i++) {
      encoded[i] = decodeZigZag(encoded[i]);
    }
  }

  // TODO: quick and dirty -> optimize for performance
  private static int[] decodeVarint(byte[] src, IntWrapper pos) {
    var values = new int[1];
    var offset = decodeVarint(src, pos.get(), values);
    pos.set(offset);
    return values;
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
    int[] intValues = new int[(int) Math.ceil(byteLength / 4)];
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
    int[] intValues = new int[(int) Math.ceil(byteLength / 4)];
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

  public static void decodeFastPforOptimized(
      byte[] buffer, IntWrapper offset, int byteLength, int[] decodedValues) {
    // TODO: get rid of that conversion
    IntBuffer intBuf =
        ByteBuffer.wrap(buffer, offset.get(), byteLength)
            // TODO: change to little endian
            .order(ByteOrder.BIG_ENDIAN)
            .asIntBuffer();
    int[] intValues = new int[(int) Math.ceil(byteLength / 4)];
    for (var i = 0; i < intValues.length; i++) {
      intValues[i] = intBuf.get(i);
    }

    IntegerCODEC ic = new Composition(new FastPFOR(), new VariableByte());
    ic.uncompress(intValues, new IntWrapper(0), intValues.length, decodedValues, new IntWrapper(0));

    offset.add(byteLength);
  }

  public static byte[] decodeByteRle(byte[] buffer, int numBytes, int byteSize, IntWrapper pos)
      throws IOException {
    var inStream =
        InStream.create(
            "test", new BufferChunk(ByteBuffer.wrap(buffer), 0), pos.get(), buffer.length);
    var reader = new RunLengthByteReader(inStream);

    var values = new byte[numBytes];
    for (var i = 0; i < numBytes; i++) {
      values[i] = reader.next();
    }

    pos.add(byteSize);
    return values;
  }

  public static BitSet decodeBooleanRle(
      byte[] buffer, int numBooleans, int byteSize, IntWrapper pos) throws IOException {
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

  public static int[] decodeMortonCode(List<Integer> mortonCodes, ZOrderCurve zOrderCurve) {
    var vertexBuffer = new int[mortonCodes.size() * 2];
    for (var i = 0; i < mortonCodes.size(); i++) {
      var mortonCode = mortonCodes.get(i);
      var vertex = zOrderCurve.decode(mortonCode);
      vertexBuffer[i * 2] = vertex[0];
      vertexBuffer[i * 2 + 1] = vertex[1];
    }

    return vertexBuffer;
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
