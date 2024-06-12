package com.mlt.decoder.vectorized;

import com.mlt.decoder.DecodingUtils;
import com.mlt.metadata.stream.*;
import com.mlt.vector.BitVector;
import java.nio.IntBuffer;
import java.nio.LongBuffer;
import me.lemire.integercompression.IntWrapper;
import me.lemire.integercompression.differential.Delta;

public class VectorizedIntegerDecoder {

  private VectorizedIntegerDecoder() {}

  /**
   * Decode not nullable int and long streams
   * ------------------------------------------------------------------
   */
  public static IntBuffer decodeIntStream(
      byte[] data, IntWrapper offset, StreamMetadata streamMetadata, boolean isSigned) {
    var values =
        streamMetadata.physicalLevelTechnique() == PhysicalLevelTechnique.FAST_PFOR
            ? VectorizedDecodingUtils.decodeFastPfor(
                data, streamMetadata.numValues(), streamMetadata.byteLength(), offset)
            : VectorizedDecodingUtils.decodeVarint(data, offset, streamMetadata.numValues());

    return decodeIntBuffer(values.array(), streamMetadata, isSigned);
  }

  public static IntBuffer decodeLengthStreamToOffsetBuffer(
      byte[] data, IntWrapper offset, StreamMetadata streamMetadata) {
    var values =
        streamMetadata.physicalLevelTechnique() == PhysicalLevelTechnique.FAST_PFOR
            ? VectorizedDecodingUtils.decodeFastPfor(
                data, streamMetadata.numValues(), streamMetadata.byteLength(), offset)
            : VectorizedDecodingUtils.decodeVarint(data, offset, streamMetadata.numValues());

    return decodeLengthToOffsetBuffer(values.array(), streamMetadata);
  }

  public static int decodeConstIntStream(
      byte[] data, IntWrapper offset, StreamMetadata streamMetadata, boolean isSigned) {
    var values =
        streamMetadata.physicalLevelTechnique() == PhysicalLevelTechnique.FAST_PFOR
            ? VectorizedDecodingUtils.decodeFastPfor(
                data, streamMetadata.numValues(), streamMetadata.byteLength(), offset)
            : VectorizedDecodingUtils.decodeVarint(data, offset, streamMetadata.numValues());

    /**
     * Only RLE encoding or a single not encoded value in the data stream can currently produce an
     * ConstVector
     */
    if (values.capacity() == 1) {
      var value = values.get(0);
      return isSigned ? DecodingUtils.decodeZigZag(value) : value;
    }

    return isSigned
        ? VectorizedDecodingUtils.decodeZigZagConstRLE(values.array())
        : VectorizedDecodingUtils.decodeUnsignedConstRLE(values.array());
  }

  public static LongBuffer decodeLongStream(
      byte[] data, IntWrapper offset, StreamMetadata streamMetadata, boolean isSigned) {
    var values = VectorizedDecodingUtils.decodeLongVarint(data, offset, streamMetadata.numValues());
    return decodeLongBuffer(values.array(), streamMetadata, isSigned);
  }

  public static long decodeConstLongStream(
      byte[] data, IntWrapper offset, StreamMetadata streamMetadata, boolean isSigned) {
    var values = VectorizedDecodingUtils.decodeLongVarint(data, offset, streamMetadata.numValues());

    /**
     * Only RLE encoding or a single not encoded value in the data stream can currently produce an
     * ConstVector
     */
    if (values.capacity() == 1) {
      var value = values.get(0);
      return isSigned ? DecodingUtils.decodeZigZag(value) : value;
    }

    /** Only RLE encoding can currently produce an ConstVector */
    return isSigned
        ? VectorizedDecodingUtils.decodeZigZagConstRLE(values.array())
        : VectorizedDecodingUtils.decodeUnsignedConstRLE(values.array());
  }

  private static IntBuffer decodeIntBuffer(
      int[] values, StreamMetadata streamMetadata, boolean isSigned) {
    /*
     * Currently the encoder uses only fixed combinations of encodings.
     * For performance reasons we also use the fixed combinations of the encodings and not a generic solution.
     * The following encodings and combinations are used:
     *   - Morton Delta -> always sorted so not ZigZag encoding needed
     *   - Delta -> currently always in combination with ZigZag encoding
     *   - Rle -> in combination with ZigZag encoding if data type is signed
     *   - Delta Rle
     *   - Componentwise Delta -> always ZigZag encoding is used
     * */
    switch (streamMetadata.logicalLevelTechnique1()) {
      case DELTA:
        if (streamMetadata.logicalLevelTechnique2().equals(LogicalLevelTechnique.RLE)) {
          var rleMetadata = (RleEncodedStreamMetadata) streamMetadata;
          values =
              VectorizedDecodingUtils.decodeUnsignedRLE(
                      values, rleMetadata.runs(), rleMetadata.numRleValues())
                  .array();
          /** Currently delta values are always ZigZag encoded */
          VectorizedDecodingUtils.decodeZigZagDelta(values);
          return IntBuffer.wrap(values);
        }

        // TODO: check if zigzag encoding is needed -> if values are sorted in ascending order no
        // need for zigzag
        // for only delta decoding without zigzag use Delta.fastinverseDelta(values) form Lemire
        VectorizedDecodingUtils.decodeZigZagDelta(values);
        return IntBuffer.wrap(values);
      case RLE:
        /** Currently no second logical level technique is used in combination with Rle */
        return VectorizedDecodingUtils.decodeRle(values, streamMetadata, isSigned);
      case MORTON:
        /**
         * Currently always used in combination with delta encoding and without ZigZag encoding
         * since the values are sorted in ascending order. The data are stored internally in
         * compressed form since they can be in parallel decompressed on the GPU.
         */
        Delta.fastinverseDelta(values);
        return IntBuffer.wrap(values);
      case COMPONENTWISE_DELTA:
        /** Currently only Vec2 is supported */
        VectorizedDecodingUtils.decodeComponentwiseDeltaVec2(values);
        return IntBuffer.wrap(values);
      case NONE:
        // TODO: merge with varint decoding
        if (isSigned) {
          DecodingUtils.decodeZigZag(values);
        }
        return IntBuffer.wrap(values);
    }

    throw new IllegalArgumentException(
        "The specified Logical level technique is not supported: "
            + streamMetadata.logicalLevelTechnique1());
  }

  private static LongBuffer decodeLongBuffer(
      long[] values, StreamMetadata streamMetadata, boolean isSigned) {
    /*
     * Currently the encoder uses only fixed combinations of encodings.
     * For performance reasons we also use the fixed combinations of the encodings and not a generic solution.
     * The following encodings and combinations are used:
     *   - Delta -> currently always in combination with ZigZag encoding
     *   - Rle -> in combination with ZigZag encoding if data type is signed
     *   - Delta Rle
     * */
    switch (streamMetadata.logicalLevelTechnique1()) {
      case DELTA:
        if (streamMetadata.logicalLevelTechnique2().equals(LogicalLevelTechnique.RLE)) {
          var rleMetadata = (RleEncodedStreamMetadata) streamMetadata;
          values =
              VectorizedDecodingUtils.decodeUnsignedRLE(
                      values, rleMetadata.runs(), rleMetadata.numRleValues())
                  .array();
          /** Currently delta values are always ZigZag encoded */
          VectorizedDecodingUtils.decodeZigZagDelta(values);
          return LongBuffer.wrap(values);
        }

        // TODO: check if zigzag encoding is needed -> if values are sorted in ascending order no
        // need for zigzag
        // for only delta decoding without zigzag use Delta.fastinverseDelta(values) form Lemire
        VectorizedDecodingUtils.decodeZigZagDelta(values);
        return LongBuffer.wrap(values);
      case RLE:
        /** Currently no second logical level technique is used in combination with Rle */
        return VectorizedDecodingUtils.decodeRle(values, streamMetadata, isSigned);
      case NONE:
        // TODO: merge with varint decoding
        if (isSigned) {
          DecodingUtils.decodeZigZag(values);
        }
        return LongBuffer.wrap(values);
    }

    throw new IllegalArgumentException(
        "The specified Logical level technique is not supported: "
            + streamMetadata.logicalLevelTechnique1());
  }

  private static IntBuffer decodeLengthToOffsetBuffer(int[] values, StreamMetadata streamMetadata) {
    if (streamMetadata.logicalLevelTechnique1().equals(LogicalLevelTechnique.DELTA)
        && streamMetadata.logicalLevelTechnique2().equals(LogicalLevelTechnique.NONE)) {
      var decodedValues = VectorizedDecodingUtils.zigZagDeltaOfDeltaDecoding(values);
      return IntBuffer.wrap(decodedValues);
    }

    if (streamMetadata.logicalLevelTechnique1().equals(LogicalLevelTechnique.RLE)
        && streamMetadata.logicalLevelTechnique2().equals(LogicalLevelTechnique.NONE)) {
      var rleMetadata = (RleEncodedStreamMetadata) streamMetadata;
      var decodedValues =
          VectorizedDecodingUtils.rleDeltaDecoding(
              values, rleMetadata.runs(), rleMetadata.numRleValues());
      return decodedValues;
    }

    if (streamMetadata.logicalLevelTechnique1().equals(LogicalLevelTechnique.NONE)
        && streamMetadata.logicalLevelTechnique2().equals(LogicalLevelTechnique.NONE)) {
      // TODO: optimize performance
      Delta.fastinverseDelta(values);
      var offsets = new int[streamMetadata.numValues() + 1];
      offsets[0] = 0;
      System.arraycopy(values, 0, offsets, 1, streamMetadata.numValues());
      return IntBuffer.wrap(offsets);
    }

    if (streamMetadata.logicalLevelTechnique1().equals(LogicalLevelTechnique.DELTA)
        && streamMetadata.logicalLevelTechnique2().equals(LogicalLevelTechnique.RLE)) {
      var rleMetadata = (RleEncodedStreamMetadata) streamMetadata;
      var decodedValues =
          VectorizedDecodingUtils.zigZagRleDeltaDecoding(
              values, rleMetadata.runs(), rleMetadata.numRleValues());
      Delta.fastinverseDelta(decodedValues.array());
      return decodedValues;
    }

    throw new IllegalArgumentException(
        "Only delta encoding is supported for transforming length to offset streams yet.");
  }

  /**
   * Decode nullable int and long data streams
   * -------------------------------------------------------------------
   */
  public static IntBuffer decodeNullableIntStream(
      byte[] data,
      IntWrapper offset,
      StreamMetadata streamMetadata,
      boolean isSigned,
      BitVector bitVector) {
    var values =
        streamMetadata.physicalLevelTechnique() == PhysicalLevelTechnique.FAST_PFOR
            ? VectorizedDecodingUtils.decodeFastPfor(
                data, streamMetadata.numValues(), streamMetadata.byteLength(), offset)
            : VectorizedDecodingUtils.decodeVarint(data, offset, streamMetadata.numValues());

    return decodeNullableIntBuffer(values.array(), streamMetadata, isSigned, bitVector);
  }

  public static LongBuffer decodeNullableLongStream(
      byte[] data,
      IntWrapper offset,
      StreamMetadata streamMetadata,
      boolean isSigned,
      BitVector bitVector) {

    var values = VectorizedDecodingUtils.decodeLongVarint(data, offset, streamMetadata.numValues());
    return decodeNullableLongBuffer(values.array(), streamMetadata, isSigned, bitVector);
  }

  private static IntBuffer decodeNullableIntBuffer(
      int[] values, StreamMetadata streamMetadata, boolean isSigned, BitVector bitVector) {
    /*
     * Currently the encoder uses only fixed combinations of encodings.
     * For performance reasons we also use the fixed combinations of the encodings and not a generic solution.
     * The following encodings and combinations are used:
     *   - Morton Delta -> always sorted so not ZigZag encoding needed
     *   - Delta -> currently always in combination with ZigZag encoding
     *   - Rle -> in combination with ZigZag encoding if data type is signed
     *   - Delta Rle
     *   - Componentwise Delta -> always ZigZag encoding is used
     * */
    switch (streamMetadata.logicalLevelTechnique1()) {
      case DELTA:
        if (streamMetadata.logicalLevelTechnique2().equals(LogicalLevelTechnique.RLE)) {
          var rleMetadata = (RleEncodedStreamMetadata) streamMetadata;
          values =
              VectorizedDecodingUtils.decodeUnsignedRLE(
                      values, rleMetadata.runs(), rleMetadata.numRleValues())
                  .array();
        }
        /** Currently delta values are always ZigZag encoded */
        // TODO: check if zigzag encoding is needed -> if values are sorted in ascending order no
        // need for zigzag
        var decodedValues = VectorizedDecodingUtils.decodeNullableZigZagDelta(bitVector, values);
        return IntBuffer.wrap(decodedValues);
      case RLE:
        /** Currently no second logical level technique is used in combination with Rle */
        return VectorizedDecodingUtils.decodeNullableRle(
            values, streamMetadata, isSigned, bitVector);
      case MORTON:
        /**
         * Currently always used in combination with delta encoding and without ZigZag encoding
         * since the values are sorted in ascending order. The data are stored internally in
         * compressed form since they can be in parallel decompressed on the GPU.
         */
        Delta.fastinverseDelta(values);
        return IntBuffer.wrap(values);
      case COMPONENTWISE_DELTA:
        /**
         * Currently only Vec2 is supported -> no null values are supported currently in this
         * encoding
         */
        VectorizedDecodingUtils.decodeComponentwiseDeltaVec2(values);
        return IntBuffer.wrap(values);
      case NONE:
        values =
            isSigned
                ? VectorizedDecodingUtils.padZigZagWithZeros(bitVector, values)
                : VectorizedDecodingUtils.padWithZeros(bitVector, values);
        return IntBuffer.wrap(values);
    }

    throw new IllegalArgumentException("The specified Logical level technique is not supported");
  }

  private static LongBuffer decodeNullableLongBuffer(
      long[] values, StreamMetadata streamMetadata, boolean isSigned, BitVector bitVector) {
    /*
     * Currently the encoder uses only fixed combinations of encodings.
     * For performance reasons we also use the fixed combinations of the encodings and not a generic solution.
     * The following encodings and combinations are used:
     *   - Delta -> currently always in combination with ZigZag encoding
     *   - Rle -> in combination with ZigZag encoding if data type is signed
     *   - Delta Rle
     * */
    switch (streamMetadata.logicalLevelTechnique1()) {
      case DELTA:
        if (streamMetadata.logicalLevelTechnique2().equals(LogicalLevelTechnique.RLE)) {
          var rleMetadata = (RleEncodedStreamMetadata) streamMetadata;
          values =
              VectorizedDecodingUtils.decodeUnsignedRLE(
                      values, rleMetadata.runs(), rleMetadata.numRleValues())
                  .array();
        }

        /** Currently delta values are always ZigZag encoded */
        var decodedValues = VectorizedDecodingUtils.decodeNullableZigZagDelta(bitVector, values);
        return LongBuffer.wrap(decodedValues);
      case RLE:
        /** Currently no second logical level technique is used in combination with Rle */
        return VectorizedDecodingUtils.decodeNullableRle(
            values, streamMetadata, isSigned, bitVector);
      case NONE:
        values =
            isSigned
                ? VectorizedDecodingUtils.padZigZagWithZeros(bitVector, values)
                : VectorizedDecodingUtils.padWithZeros(bitVector, values);
        return LongBuffer.wrap(values);
    }

    throw new IllegalArgumentException("The specified Logical level technique is not supported");
  }

  /**
   * Decode length streams to offsets streams for random access by adding an additional delta
   * decoding step -----
   */
}
