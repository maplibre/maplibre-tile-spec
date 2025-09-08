package org.maplibre.mlt.decoder.vectorized;

import java.nio.*;
import java.util.BitSet;
import me.lemire.integercompression.*;
import org.apache.commons.lang3.tuple.Pair;
import org.maplibre.mlt.decoder.DecodingUtils;
import org.maplibre.mlt.metadata.stream.LogicalLevelTechnique;
import org.maplibre.mlt.metadata.stream.RleEncodedStreamMetadata;
import org.maplibre.mlt.metadata.stream.StreamMetadata;
import org.maplibre.mlt.vector.BitVector;
import org.maplibre.mlt.vector.VectorType;

/* the redundant implementations in this class are mainly to avoid branching and therefore speed up the decoding */
public class VectorizedDecodingUtils {

  private static IntegerCODEC ic;
  private static IntegerCODEC icVectorized;

  public static ByteBuffer decodeBooleanRle(byte[] buffer, int numBooleans, IntWrapper pos) {
    var numBytes = (int) Math.ceil(numBooleans / 8d);
    return decodeByteRle(buffer, numBytes, pos);
  }

  public static ByteBuffer decodeNullableBooleanRle(
      byte[] buffer, int numBooleans, IntWrapper pos, BitVector nullabilityBuffer) {
    // TODO: refactor quick and dirty solution -> use vectorized solution in one pass
    var numBytes = (int) Math.ceil(numBooleans / 8d);
    var values = decodeByteRle(buffer, numBytes, pos);
    var bitVector = new BitVector(values, numBooleans);

    var nullableBitset = new BitSet(nullabilityBuffer.size());
    var valueCounter = 0;
    for (var i = 0; i < nullabilityBuffer.size(); i++) {
      if (nullabilityBuffer.get(i)) {
        var value = bitVector.get(valueCounter++);
        nullableBitset.set(i, value);
      } else {
        nullableBitset.set(i, false);
      }
    }

    return ByteBuffer.wrap(nullableBitset.toByteArray());
  }

  public static ByteBuffer decodeByteRle(byte[] buffer, int numBytesResult, IntWrapper pos) {
    ByteBuffer values = ByteBuffer.allocate(numBytesResult);

    var offset = pos.get();
    int valueOffset = 0;
    while (valueOffset < numBytesResult) {
      int header = buffer[offset++] & 0xFF;
      if (header <= 0x7F) {
        /* Runs */
        int numRuns = header + 3;
        byte value = buffer[offset++];
        int endValueOffset = valueOffset + numRuns;
        for (int i = valueOffset; i < endValueOffset; i++) {
          values.put(i, value);
        }
        valueOffset = endValueOffset;
      } else {
        /* Literals */
        int numLiterals = 256 - header;
        for (int i = 0; i < numLiterals; i++) {
          byte value = buffer[offset++];
          values.put(valueOffset++, value);
        }
        // TODO: use System.arrayCopy
        // System.arraycopy(buffer, offset, values.array(), valueOffset, numLiterals);
      }
    }

    pos.set(offset);
    return values;
  }

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

  /**
   * Varint decoding
   * ----------------------------------------------------------------------------------------
   */
  public static IntBuffer decodeVarint(byte[] src, IntWrapper pos, int numValues) {
    var values = new int[numValues];
    for (var i = 0; i < numValues; i++) {
      var offset = decodeVarint(src, pos.get(), values, i);
      pos.set(offset);
    }
    return IntBuffer.wrap(values);
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

  // TODO: refactor for performance reasons
  public static LongBuffer decodeLongVarint(byte[] src, IntWrapper pos, int numValues) {
    var values = new long[numValues];
    for (var i = 0; i < numValues; i++) {
      long value = 0;
      int shift = 0;
      int index = pos.get();
      while (index < src.length) {
        byte b = src[index++];
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
      values[i] = value;
    }

    return LongBuffer.wrap(values);
  }

  /**
   * Rle decoding
   * --------------------------------------------------------------------------------------
   */
  public static IntBuffer decodeRle(int[] data, StreamMetadata streamMetadata, boolean isSigned) {
    var rleMetadata = (RleEncodedStreamMetadata) streamMetadata;
    return isSigned
        ? VectorizedDecodingUtils.decodeZigZagRLE(
            data, rleMetadata.runs(), rleMetadata.numRleValues())
        : VectorizedDecodingUtils.decodeUnsignedRLE(
            data, rleMetadata.runs(), rleMetadata.numRleValues());
  }

  // TODO: use vectorized solution which is 2x faster in the tests
  public static IntBuffer decodeUnsignedRLE(int[] data, int numRuns, int numTotalValues) {
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

    return IntBuffer.wrap(values);
  }

  public static int[] decodeUnsignedRLE2(int[] data, int numRuns, int numTotalValues) {
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

  public static IntBuffer decodeZigZagRLE(int[] data, int numRuns, int numTotalValues) {
    var values = new int[numTotalValues];
    var offset = 0;
    for (var i = 0; i < numRuns; i++) {
      var runLength = data[i];
      var value = data[i + numRuns];
      value = (value >>> 1) ^ ((value << 31) >> 31);
      for (var j = offset; j < offset + runLength; j++) {
        values[j] = value;
      }

      offset += runLength;
    }

    return IntBuffer.wrap(values);
  }

  public static LongBuffer decodeRle(long[] data, StreamMetadata streamMetadata, boolean isSigned) {
    var rleMetadata = (RleEncodedStreamMetadata) streamMetadata;
    return isSigned
        ? VectorizedDecodingUtils.decodeZigZagRLE(
            data, rleMetadata.runs(), rleMetadata.numRleValues())
        : VectorizedDecodingUtils.decodeUnsignedRLE(
            data, rleMetadata.runs(), rleMetadata.numRleValues());
  }

  public static LongBuffer decodeUnsignedRLE(long[] data, int numRuns, int numTotalValues) {
    var values = new long[numTotalValues];
    var offset = 0;
    for (var i = 0; i < numRuns; i++) {
      var runLength = data[i];
      var value = data[i + numRuns];
      for (var j = offset; j < offset + runLength; j++) {
        values[j] = value;
      }

      offset += (int) runLength;
    }

    return LongBuffer.wrap(values);
  }

  public static LongBuffer decodeZigZagRLE(long[] data, int numRuns, int numTotalValues) {
    var values = new long[numTotalValues];
    var offset = 0;
    for (var i = 0; i < numRuns; i++) {
      var runLength = data[i];
      var value = data[i + numRuns];
      value = (value >>> 1) ^ ((value << 63) >> 63);
      for (var j = offset; j < offset + runLength; j++) {
        values[j] = value;
      }

      offset += (int) runLength;
    }

    return LongBuffer.wrap(values);
  }

  /**
   * Nullable Rle decoding
   * --------------------------------------------------------------------------------------
   */
  public static IntBuffer decodeNullableRle(
      int[] data, StreamMetadata streamMetadata, boolean isSigned, BitVector bitVector) {
    var rleMetadata = (RleEncodedStreamMetadata) streamMetadata;
    return isSigned
        ? VectorizedDecodingUtils.decodeNullableZigZagRLE(bitVector, data, rleMetadata.runs())
        : VectorizedDecodingUtils.decodeNullableUnsignedRLE(bitVector, data, rleMetadata.runs());
  }

  public static IntBuffer decodeNullableUnsignedRLE(BitVector bitVector, int[] data, int numRuns) {
    var values = new int[bitVector.size()];
    var offset = 0;
    for (var i = 0; i < numRuns; i++) {
      var runLength = data[i];
      var value = data[i + numRuns];
      for (var j = offset; j < offset + runLength; j++) {
        /* There can be null values in a run */
        if (bitVector.get(j)) {
          values[j] = value;
        } else {
          values[j] = 0;
          offset++;
        }
      }
      offset += runLength;
    }

    return IntBuffer.wrap(values);
  }

  public static IntBuffer decodeNullableZigZagRLE(BitVector bitVector, int[] data, int numRuns) {
    var values = new int[bitVector.size()];
    var offset = 0;
    for (var i = 0; i < numRuns; i++) {
      var runLength = data[i];
      var value = data[i + numRuns];
      value = (value >>> 1) ^ ((value << 31) >> 31);
      for (var j = offset; j < offset + runLength; j++) {
        /* There can be null values in a run */
        if (bitVector.get(j)) {
          values[j] = value;
        } else {
          values[j] = 0;
          offset++;
        }
      }
      offset += runLength;
    }

    return IntBuffer.wrap(values);
  }

  public static LongBuffer decodeNullableRle(
      long[] data, StreamMetadata streamMetadata, boolean isSigned, BitVector bitVector) {
    var rleMetadata = (RleEncodedStreamMetadata) streamMetadata;
    return isSigned
        ? VectorizedDecodingUtils.decodeNullableZigZagRLE(bitVector, data, rleMetadata.runs())
        : VectorizedDecodingUtils.decodeNullableUnsignedRLE(bitVector, data, rleMetadata.runs());
  }

  public static LongBuffer decodeNullableUnsignedRLE(
      BitVector bitVector, long[] data, int numRuns) {
    var values = new long[bitVector.size()];
    var offset = 0;
    for (var i = 0; i < numRuns; i++) {
      var runLength = data[i];
      var value = data[i + numRuns];
      for (var j = offset; j < offset + runLength; j++) {
        /* There can be null values in a run */
        if (bitVector.get(j)) {
          values[j] = value;
        } else {
          values[j] = 0;
          offset++;
        }
      }
      offset += (int) runLength;
    }

    return LongBuffer.wrap(values);
  }

  public static LongBuffer decodeNullableZigZagRLE(BitVector bitVector, long[] data, int numRuns) {
    var values = new long[bitVector.size()];
    var offset = 0;
    for (var i = 0; i < numRuns; i++) {
      var runLength = data[i];
      var value = data[i + numRuns];
      value = (value >>> 1) ^ ((value << 63) >> 63);
      for (var j = offset; j < offset + runLength; j++) {
        /* There can be null values in a run */
        if (bitVector.get(j)) {
          values[j] = value;
        } else {
          values[j] = 0;
          offset++;
        }
      }
      offset += (int) runLength;
    }

    return LongBuffer.wrap(values);
  }

  public static int decodeUnsignedConstRLE(int[] data) {
    return data[1];
  }

  public static int decodeZigZagConstRLE(int[] data) {
    var value = data[1];
    return (value >>> 1) ^ ((value << 31) >> 31);
  }

  public static Pair<Integer, Integer> decodeZigZagSequenceRLE(int[] data) {
    /* base value and delta value are equal */
    if (data.length == 2) {
      var value = DecodingUtils.decodeZigZag(data[1]);
      return Pair.of(value, value);
    }

    /* base value and delta value are not equal -> 2 runs and 2 values*/
    return Pair.of(DecodingUtils.decodeZigZag(data[2]), DecodingUtils.decodeZigZag(data[3]));
  }

  public static Pair<Long, Long> decodeZigZagSequenceRLE(long[] data) {
    /* base value and delta value are equal */
    if (data.length == 2) {
      var value = DecodingUtils.decodeZigZag(data[1]);
      return Pair.of(value, value);
    }

    /* base value and delta value are not equal -> 2 runs and 2 values*/
    return Pair.of(DecodingUtils.decodeZigZag(data[2]), DecodingUtils.decodeZigZag(data[3]));
  }

  public static long decodeUnsignedConstRLE(long[] data) {
    return data[1];
  }

  public static long decodeZigZagConstRLE(long[] data) {
    var value = data[1];
    return (value >>> 1) ^ ((value << 63) >> 63);
  }

  /* Delta encoding  ------------------------------------------------------------------------------*/

  /*
   * In place decoding of the zigzag encoded delta values.
   * Inspired by https://github.com/lemire/JavaFastPFOR/blob/master/src/main/java/me/lemire/integercompression/differential/Delta.java
   */
  public static void decodeZigZagDelta(int[] data) {
    data[0] = (data[0] >>> 1) ^ ((data[0] << 31) >> 31);
    int sz0 = data.length / 4 * 4;
    int i = 1;
    if (sz0 >= 4) {
      for (; i < sz0 - 4; i += 4) {
        var data1 = data[i];
        var data2 = data[i + 1];
        var data3 = data[i + 2];
        var data4 = data[i + 3];

        data[i] = ((data1 >>> 1) ^ ((data1 << 31) >> 31)) + data[i - 1];
        data[i + 1] = ((data2 >>> 1) ^ ((data2 << 31) >> 31)) + data[i];
        data[i + 2] = ((data3 >>> 1) ^ ((data3 << 31) >> 31)) + data[i + 1];
        data[i + 3] = ((data4 >>> 1) ^ ((data4 << 31) >> 31)) + data[i + 2];
      }
    }

    for (; i != data.length; ++i) {
      data[i] = ((data[i] >>> 1) ^ ((data[i] << 31) >> 31)) + data[i - 1];
    }
  }

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

  public static void decodeZigZagDelta(long[] data) {
    data[0] = (data[0] >>> 1) ^ ((data[0] << 63) >> 63);
    int sz0 = data.length / 4 * 4;
    int i = 1;
    if (sz0 >= 4) {
      for (; i < sz0 - 4; i += 4) {
        var data1 = data[i];
        var data2 = data[i + 1];
        var data3 = data[i + 2];
        var data4 = data[i + 3];

        data[i] = ((data1 >>> 1) ^ ((data1 << 63) >> 63)) + data[i - 1];
        data[i + 1] = ((data2 >>> 1) ^ ((data2 << 63) >> 63)) + data[i];
        data[i + 2] = ((data3 >>> 1) ^ ((data3 << 63) >> 63)) + data[i + 1];
        data[i + 3] = ((data4 >>> 1) ^ ((data4 << 63) >> 63)) + data[i + 2];
      }
    }

    for (; i != data.length; ++i) {
      data[i] = ((data[i] >>> 1) ^ ((data[i] << 63) >> 63)) + data[i - 1];
    }
  }

  public static int[] decodeNullableZigZagDelta(BitVector bitVector, int[] data) {
    var decodedData = new int[bitVector.size()];
    var dataCounter = 0;
    if (bitVector.get(0)) {
      decodedData[0] = bitVector.get(0) ? ((data[0] >>> 1) ^ ((data[0] << 31) >> 31)) : 0;
      dataCounter = 1;
    } else {
      decodedData[0] = 0;
    }

    var i = 1;
    for (; i != decodedData.length; ++i) {
      decodedData[i] =
          bitVector.get(i)
              ? decodedData[i - 1]
                  + ((data[dataCounter] >>> 1) ^ ((data[dataCounter++] << 31) >> 31))
              : decodedData[i - 1];
    }

    return decodedData;
  }

  public static long[] decodeNullableZigZagDelta(BitVector bitVector, long[] data) {
    var decodedData = new long[bitVector.size()];
    var dataCounter = 0;
    if (bitVector.get(0)) {
      decodedData[0] = bitVector.get(0) ? ((data[0] >>> 1) ^ ((data[0] << 63) >> 63)) : 0;
      dataCounter = 1;
    } else {
      decodedData[0] = 0;
    }

    var i = 1;
    for (; i != decodedData.length; ++i) {
      decodedData[i] =
          bitVector.get(i)
              ? decodedData[i - 1]
                  + ((data[dataCounter] >>> 1) ^ ((data[dataCounter++] << 63) >> 63))
              : decodedData[i - 1];
    }

    return decodedData;
  }

  /**
   * Transform data to allow random access
   * ---------------------------------------------------------------------
   */
  public static int[] zigZagDeltaOfDeltaDecoding(int[] data) {
    var decodedData = new int[data.length + 1];
    decodedData[0] = 0;
    decodedData[1] = (data[0] >>> 1) ^ ((data[0] << 31) >> 31);
    var deltaSum = decodedData[1];
    int i = 2;
    for (; i != decodedData.length; ++i) {
      var zigZagValue = data[i - 1];
      var delta = (zigZagValue >>> 1) ^ ((zigZagValue << 31) >> 31);
      deltaSum += delta;
      decodedData[i] = decodedData[i - 1] + deltaSum;
    }

    return decodedData;
  }

  public static IntBuffer zigZagRleDeltaDecoding(int[] data, int numRuns, int numTotalValues) {
    var values = new int[numTotalValues + 1];
    values[0] = 0;
    var offset = 1;
    var previousValue = values[0];
    for (var i = 0; i < numRuns; i++) {
      var runLength = data[i];
      var value = data[i + numRuns];
      value = (value >>> 1) ^ ((value << 31) >> 31);
      for (var j = offset; j < offset + runLength; j++) {
        values[j] = value + previousValue;
        previousValue = values[j];
      }

      offset += runLength;
    }

    return IntBuffer.wrap(values);
  }

  public static IntBuffer rleDeltaDecoding(int[] data, int numRuns, int numTotalValues) {
    var values = new int[numTotalValues + 1];
    values[0] = 0;
    var offset = 1;
    var previousValue = values[0];
    for (var i = 0; i < numRuns; i++) {
      var runLength = data[i];
      var value = data[i + numRuns];
      for (var j = offset; j < offset + runLength; j++) {
        values[j] = value + previousValue;
        previousValue = values[j];
      }

      offset += runLength;
    }

    return IntBuffer.wrap(values);
  }

  public static int[] padWithZeros(BitVector bitVector, int[] data) {
    var decodedData = new int[bitVector.size()];
    var dataCounter = 0;
    var i = 0;
    for (; i != decodedData.length; ++i) {
      decodedData[i] = bitVector.get(i) ? data[dataCounter++] : 0;
    }

    return decodedData;
  }

  public static int[] padZigZagWithZeros(BitVector bitVector, int[] data) {
    var decodedData = new int[bitVector.size()];
    var dataCounter = 0;
    var i = 0;
    for (; i != decodedData.length; ++i) {
      if (bitVector.get(i)) {
        var value = data[dataCounter++];
        decodedData[i] = (value >>> 1) ^ ((value << 31) >> 31);
      } else {
        decodedData[i] = 0;
      }
    }

    return decodedData;
  }

  public static long[] padWithZeros(BitVector bitVector, long[] data) {
    var decodedData = new long[bitVector.size()];
    var dataCounter = 0;
    var i = 0;
    for (; i != decodedData.length; ++i) {
      decodedData[i] = bitVector.get(i) ? data[dataCounter++] : 0;
    }

    return decodedData;
  }

  public static long[] padZigZagWithZeros(BitVector bitVector, long[] data) {
    var decodedData = new long[bitVector.size()];
    var dataCounter = 0;
    var i = 0;
    for (; i != decodedData.length; ++i) {
      if (bitVector.get(i)) {
        var value = data[dataCounter++];
        decodedData[i] = (value >>> 1) ^ ((value << 63) >> 63);
      } else {
        decodedData[i] = 0;
      }
    }

    return decodedData;
  }

  public static VectorType getVectorTypeIntStream(StreamMetadata streamMetadata) {
    var logicalLevelTechnique1 = streamMetadata.logicalLevelTechnique1();
    if (logicalLevelTechnique1.equals(LogicalLevelTechnique.RLE)) {
      return ((RleEncodedStreamMetadata) streamMetadata).runs() == 1
          ? VectorType.CONST
          : VectorType.FLAT;
    }

    if (logicalLevelTechnique1.equals(LogicalLevelTechnique.DELTA)
        && streamMetadata.logicalLevelTechnique2().equals(LogicalLevelTechnique.RLE)
        /* If base value equals delta value then one run else two runs */
        && (((RleEncodedStreamMetadata) streamMetadata).runs() == 1
            || ((RleEncodedStreamMetadata) streamMetadata).runs() == 2)) {
      return VectorType.SEQUENCE;
    }

    return streamMetadata.numValues() == 1 ? VectorType.CONST : VectorType.FLAT;
  }

  public static VectorType getVectorTypeBooleanStream(
      int numFeatures, int byteLength, byte[] data, IntWrapper offset) {
    var valuesPerRun = 131;
    // TODO: use VectorType metadata field for to test which VectorType is used
    return (Math.ceil((double) numFeatures / valuesPerRun) * 2 == byteLength)
            &&
            /* Test the first value byte if all bits are set to true */
            (data[offset.get() + 1] & 0xFF) == ((Integer.bitCount(numFeatures) << 2) - 1)
        ? VectorType.CONST
        : VectorType.FLAT;
  }
}
