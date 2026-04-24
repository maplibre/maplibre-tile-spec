package org.maplibre.mlt.converter.encodings;

import java.io.ByteArrayInputStream;
import java.io.ByteArrayOutputStream;
import java.io.DataOutputStream;
import java.io.IOException;
import java.nio.ByteBuffer;
import java.nio.ByteOrder;
import java.nio.charset.StandardCharsets;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.BitSet;
import java.util.Collection;
import java.util.zip.GZIPInputStream;
import java.util.zip.GZIPOutputStream;
import me.lemire.integercompression.Composition;
import me.lemire.integercompression.FastPFOR;
import me.lemire.integercompression.IntWrapper;
import me.lemire.integercompression.IntegerCODEC;
import me.lemire.integercompression.VariableByte;
import org.apache.commons.lang3.ArrayUtils;
import org.apache.commons.lang3.tuple.Pair;
import org.maplibre.mlt.converter.CollectionUtils;

public class EncodingUtils {

  // https://github.com/bazelbuild/bazel/blob/6ce603d8/src/main/java/com/google/devtools/build/lib/util/VarInt.java
  /** Maximum encoded size of 32-bit positive integers (in bytes) */
  public static final int MAX_VARINT_SIZE = 5;

  /** maximum encoded size of 64-bit longs, and negative 32-bit ints (in bytes) */
  public static final int MAX_VARLONG_SIZE = 10;

  public static byte[] gzip(byte[] buffer) throws IOException {
    ByteArrayOutputStream baos = new ByteArrayOutputStream();
    GZIPOutputStream gzipOut = new GZIPOutputStream(baos);
    gzipOut.write(buffer);
    gzipOut.close();
    baos.close();

    return baos.toByteArray();
  }

  public static byte[] unzip(byte[] buffer) throws IOException {
    try (var inputStream = new ByteArrayInputStream(buffer)) {
      try (var gZIPInputStream = new GZIPInputStream(inputStream)) {
        return gZIPInputStream.readAllBytes();
      }
    }
  }

  /** Convert the floats to IEEE754 floating point numbers in Little Endian byte order. */
  public static byte[] encodeFloatsLE(float[] values) {
    var buffer = ByteBuffer.allocate(values.length * 4).order(ByteOrder.LITTLE_ENDIAN);
    for (var value : values) {
      buffer.putFloat(value);
    }
    return buffer.array();
  }

  /** Convert the doubles to IEEE754 floating point numbers in Little Endian byte order. */
  public static byte[] encodeDoublesLE(double[] values) {
    var buffer = ByteBuffer.allocate(values.length * 8).order(ByteOrder.LITTLE_ENDIAN);
    for (var value : values) {
      buffer.putDouble(value);
    }
    return buffer.array();
  }

  // Source:
  // https://github.com/bazelbuild/bazel/blob/master/src/main/java/com/google/devtools/build/lib/util/VarInt.java
  public static byte[] encodeVarints(int[] values, boolean zigZagEncode, boolean deltaEncode)
      throws IOException {
    var encodedValues = values;
    if (deltaEncode) {
      encodedValues = encodeDeltas(values);
    }

    if (zigZagEncode) {
      encodedValues = encodeZigZag(encodedValues);
    }

    var varintBuffer =
        ByteBuffer.wrap(
            new byte[Arrays.stream(encodedValues).map(EncodingUtils::getVarIntSize).sum()]);
    for (var value : encodedValues) {
      putVarInt(value, varintBuffer);
    }
    return varintBuffer.array();
  }

  public static byte[] encodeVarints(long[] values, boolean zigZagEncode, boolean deltaEncode)
      throws IOException {
    var encodedValues = values;
    if (deltaEncode) {
      encodedValues = encodeDeltas(values);
    }

    if (zigZagEncode) {
      encodedValues = encodeZigZag(encodedValues);
    }

    var varintBuffer =
        ByteBuffer.wrap(
            new byte[Arrays.stream(encodedValues).mapToInt(EncodingUtils::getVarLongSize).sum()]);
    for (var value : encodedValues) {
      putVarInt(value, varintBuffer);
    }
    return varintBuffer.array();
  }

  public static byte[] encodeVarints(
      Collection<Integer> values, boolean zigZagEncode, boolean deltaEncode) throws IOException {
    return encodeVarints(CollectionUtils.unboxInts(values), zigZagEncode, deltaEncode);
  }

  public static byte[] encodeLongVarints(long[] values, boolean zigZagEncode, boolean deltaEncode)
      throws IOException {
    return encodeVarints(values, zigZagEncode, deltaEncode);
  }

  public static byte[] encodeVarint(int value, boolean zigZagEncode) throws IOException {
    if (zigZagEncode) {
      value = encodeZigZag(value);
    }
    final var buffer = ByteBuffer.wrap(new byte[getVarIntSize(value)]);
    return putVarInt(value, buffer).array();
  }

  // Source:
  // https://github.com/bazelbuild/bazel/blob/master/src/main/java/com/google/devtools/build/lib/util/VarInt.java
  /**
   * Encodes an integer in a variable-length encoding, 7 bits per byte, into a destination byte[],
   * following the protocol buffer convention.
   *
   * @param v the int value to write to sink
   * @param sink the sink buffer to write to
   */
  public static ByteBuffer putVarInt(int v, ByteBuffer sink) throws IOException {
    do {
      // Encode next 7 bits + terminator bit
      final int bits = v & 0x7F;
      v >>>= 7;
      sink.put((byte) (bits + ((v != 0) ? 0x80 : 0)));
    } while (v != 0);
    return sink;
  }

  static ByteBuffer putVarInt(long v, ByteBuffer sink) throws IOException {
    do {
      // Encode next 7 bits + terminator bit
      final long bits = v & 0x7F;
      v >>>= 7;
      sink.put((byte) (bits + ((v != 0) ? 0x80 : 0)));
    } while (v != 0);
    return sink;
  }

  @SuppressWarnings("UnusedReturnValue")
  public static DataOutputStream putVarInt(DataOutputStream stream, int v) throws IOException {
    final var buffer = ByteBuffer.wrap(new byte[MAX_VARINT_SIZE]);
    putVarInt(v, buffer);
    stream.write(buffer.array(), 0, buffer.position());
    return stream;
  }

  private static final int DATA_BITS_PER_ENCODED_BYTE = 7;

  public static int getVarIntSize(int value) {
    final var bitsNeeded = Integer.SIZE - Integer.numberOfLeadingZeros(value);
    return Math.max(1, (bitsNeeded + DATA_BITS_PER_ENCODED_BYTE - 1) / DATA_BITS_PER_ENCODED_BYTE);
  }

  public static int getVarLongSize(long value) {
    final var bitsNeeded = Long.SIZE - Long.numberOfLeadingZeros(value);
    return Math.max(1, (bitsNeeded + DATA_BITS_PER_ENCODED_BYTE - 1) / DATA_BITS_PER_ENCODED_BYTE);
  }

  @SuppressWarnings("UnusedReturnValue")
  public static DataOutputStream putString(DataOutputStream stream, String s) throws IOException {
    final var bytes = s.getBytes(StandardCharsets.UTF_8);
    putVarInt(stream, bytes.length);
    stream.write(bytes);
    return stream;
  }

  public static long[] encodeZigZag(long[] values) {
    long[] result = new long[values.length];
    for (int i = 0; i < values.length; i++) {
      result[i] = encodeZigZag(values[i]);
    }
    return result;
  }

  public static int[] encodeZigZag(int[] values) {
    int[] result = new int[values.length];
    for (int i = 0; i < values.length; i++) {
      result[i] = encodeZigZag(values[i]);
    }
    return result;
  }

  public static long encodeZigZag(long value) {
    return (value << 1) ^ (value >> 63);
  }

  public static int encodeZigZag(int value) {
    return (value >> 31) ^ (value << 1);
  }

  public static long[] encodeDeltas(long[] values) {
    var deltaValues = new long[values.length];
    var previousValue = 0L;
    for (var i = 0; i < values.length; i++) {
      var value = values[i];
      deltaValues[i] = value - previousValue;
      previousValue = value;
    }
    return deltaValues;
  }

  public static int[] encodeDeltas(int[] values) {
    var deltaValues = new int[values.length];
    var previousValue = 0;
    for (var i = 0; i < values.length; i++) {
      var value = values[i];
      deltaValues[i] = value - previousValue;
      previousValue = value;
    }
    return deltaValues;
  }

  /**
   * @return Pair of runs and values.
   */
  public static Pair<int[], int[]> encodeRle(int[] values) {
    var valueBuffer = new ArrayList<Integer>();
    var runsBuffer = new ArrayList<Integer>();
    var previousValue = 0;
    var runs = 0;
    for (var i = 0; i < values.length; i++) {
      var value = values[i];
      if (previousValue != value && i != 0) {
        valueBuffer.add(previousValue);
        runsBuffer.add(runs);
        runs = 0;
      }

      runs++;
      previousValue = value;
    }

    valueBuffer.add(values[values.length - 1]);
    runsBuffer.add(runs);

    return Pair.of(CollectionUtils.unboxInts(runsBuffer), CollectionUtils.unboxInts(valueBuffer));
  }

  /**
   * @return Pair of runs and values.
   */
  // TODO: merge this method with the int variant
  public static Pair<long[], long[]> encodeRle(long[] values) {
    var valueBuffer = new ArrayList<Long>();
    var runsBuffer = new ArrayList<Integer>();
    var previousValue = 0L;
    var runs = 0;
    for (var i = 0; i < values.length; i++) {
      var value = values[i];
      if (previousValue != value && i != 0) {
        valueBuffer.add(previousValue);
        runsBuffer.add(runs);
        runs = 0;
      }

      runs++;
      previousValue = value;
    }

    valueBuffer.add(values[values.length - 1]);
    runsBuffer.add(runs);
    return Pair.of(CollectionUtils.unboxLongs(runsBuffer), CollectionUtils.unboxLongs(valueBuffer));
  }

  public static byte[] encodeFastPfor128(int[] values, boolean zigZagEncode, boolean deltaEncode) {
    /*
     * Note that this does not use differential coding: if you are working on sorted lists,
     * you should first compute deltas, @see me.lemire.integercompression.differential.Delta#delta
     * */
    var encodedValues = values;
    if (deltaEncode) {
      encodedValues = encodeDeltas(values);
    }

    if (zigZagEncode) {
      encodedValues = encodeZigZag(encodedValues);
    }

    IntegerCODEC ic = new Composition(new FastPFOR(), new VariableByte());
    IntWrapper inputoffset = new IntWrapper(0);
    IntWrapper outputoffset = new IntWrapper(0);
    final int[] compressed = new int[encodedValues.length + 1024];
    ic.compress(encodedValues, inputoffset, encodedValues.length, compressed, outputoffset);
    final var totalSize = outputoffset.intValue() * 4;

    final var compressedBuffer = new byte[totalSize];
    var valueCounter = 0;
    for (var i = 0; i < totalSize; i += 4) {
      var value = compressed[valueCounter++];
      compressedBuffer[i] = (byte) (value >>> 24);
      compressedBuffer[i + 1] = (byte) (value >>> 16);
      compressedBuffer[i + 2] = (byte) (value >>> 8);
      compressedBuffer[i + 3] = (byte) value;
    }

    return compressedBuffer;
  }

  public static byte[] encodeByteRle(byte[] values) throws IOException {
    return ByteRleEncoder.encode(values);
  }

  public static byte[] encodeBooleanRle(BitSet bitSet, int numValues) throws IOException {
    var presentStream = bitSet.toByteArray();
    /* The BitSet only returns the bytes until the last set bit */
    var numMissingBytes = (int) Math.ceil(numValues / 8d) - (int) Math.ceil(bitSet.length() / 8d);
    if (numMissingBytes != 0) {
      var paddingBytes = new byte[numMissingBytes];
      Arrays.fill(paddingBytes, (byte) 0);
      presentStream = ArrayUtils.addAll(presentStream, paddingBytes);
    }

    return EncodingUtils.encodeByteRle(presentStream);
  }
}
