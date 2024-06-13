package com.mlt.decoder;

import com.mlt.decoder.vectorized.VectorizedDecodingUtils;
import com.mlt.metadata.stream.*;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.List;
import java.util.stream.Collectors;
import me.lemire.integercompression.IntWrapper;

public class IntegerDecoder {

  private IntegerDecoder() {}

  public static List<Integer> decodeMortonStream(
      byte[] data, IntWrapper offset, MortonEncodedStreamMetadata streamMetadata) {
    int[] values;
    if (streamMetadata.physicalLevelTechnique() == PhysicalLevelTechnique.FAST_PFOR) {
      // TODO: numValues is not right if rle or delta rle is used -> add separate flag in
      // StreamMetadata
      values =
          DecodingUtils.decodeFastPfor(
              data, streamMetadata.numValues(), streamMetadata.byteLength(), offset);
    } else if (streamMetadata.physicalLevelTechnique() == PhysicalLevelTechnique.VARINT) {
      values = DecodingUtils.decodeVarint(data, offset, streamMetadata.numValues());
    } else {
      throw new IllegalArgumentException(
          "Specified physical level technique not yet supported: "
              + streamMetadata.physicalLevelTechnique());
    }

    return decodeMortonDelta(values, streamMetadata.numBits(), streamMetadata.coordinateShift());
  }

  private static List<Integer> decodeMortonDelta(int[] data, int numBits, int coordinateShift) {
    var vertices = new ArrayList<Integer>(data.length * 2);
    var previousMortonCode = 0;
    for (var deltaCode : data) {
      var mortonCode = previousMortonCode + deltaCode;
      var vertex = decodeMortonCode(mortonCode, numBits, coordinateShift);
      vertices.add(vertex[0]);
      vertices.add(vertex[1]);
      previousMortonCode = mortonCode;
    }

    return vertices;
  }

  private static List<Integer> decodeMortonCodes(int[] data, int numBits, int coordinateShift) {
    var vertices = new ArrayList<Integer>(data.length * 2);
    for (var mortonCode : data) {
      var vertex = decodeMortonCode(mortonCode, numBits, coordinateShift);
      vertices.add(vertex[0]);
      vertices.add(vertex[1]);
    }

    return vertices;
  }

  private static int[] decodeMortonCode(int mortonCode, int numBits, int coordinateShift) {
    int x = decodeMorton(mortonCode, numBits) - coordinateShift;
    int y = decodeMorton(mortonCode >> 1, numBits) - coordinateShift;
    return new int[] {x, y};
  }

  private static int decodeMorton(int code, int numBits) {
    int coordinate = 0;
    for (int i = 0; i < numBits; i++) {
      coordinate |= (code & (1L << (2 * i))) >> i;
    }
    return coordinate;
  }

  public static List<Integer> decodeIntStream(
      byte[] data, IntWrapper offset, StreamMetadata streamMetadata, boolean isSigned) {
    int[] values = null;
    if (streamMetadata.physicalLevelTechnique() == PhysicalLevelTechnique.FAST_PFOR) {
      values =
          DecodingUtils.decodeFastPfor(
              data, streamMetadata.numValues(), streamMetadata.byteLength(), offset);
    } else if (streamMetadata.physicalLevelTechnique() == PhysicalLevelTechnique.VARINT) {
      values = DecodingUtils.decodeVarint(data, offset, streamMetadata.numValues());
    } else {
      throw new IllegalArgumentException(
          "Specified physical level technique not yet supported: "
              + streamMetadata.physicalLevelTechnique());
    }

    return decodeIntArray(values, streamMetadata, isSigned);
  }

  private static List<Integer> decodeIntArray(
      int[] values, StreamMetadata streamMetadata, boolean isSigned) {
    switch (streamMetadata.logicalLevelTechnique1()) {
      case DELTA:
        if (streamMetadata.logicalLevelTechnique2().equals(LogicalLevelTechnique.RLE)) {
          var rleMetadata = (RleEncodedStreamMetadata) streamMetadata;
          values =
              DecodingUtils.decodeUnsignedRLE(
                  values, rleMetadata.runs(), rleMetadata.numRleValues());
          return decodeZigZagDelta(values);
        }
        return decodeZigZagDelta(values);
      case RLE:
        {
          var rleMetadata = (RleEncodedStreamMetadata) streamMetadata;
          var decodedValues = decodeRLE(values, rleMetadata.runs(), rleMetadata.numRleValues());
          return isSigned
              ? decodeZigZag(decodedValues.stream().mapToInt(i -> i).toArray())
              : decodedValues;
        }
      case NONE:
        {
          if (isSigned) {
            return decodeZigZag(values);
          }
          return Arrays.stream(values).boxed().collect(Collectors.toList());
        }
      case COMPONENTWISE_DELTA:
        VectorizedDecodingUtils.decodeComponentwiseDeltaVec2(values);
        return Arrays.stream(values).boxed().collect(Collectors.toList());
      case MORTON:
        // TODO: zig-zag decode when morton second logical level technique
        return decodeMortonCodes(
            values,
            ((MortonEncodedStreamMetadata) streamMetadata).numBits(),
            ((MortonEncodedStreamMetadata) streamMetadata).coordinateShift());
    }

    throw new IllegalArgumentException(
        "The specified logical level technique is not supported for integers: "
            + streamMetadata.logicalLevelTechnique1());
  }

  public static List<Long> decodeLongStream(
      byte[] data, IntWrapper offset, StreamMetadata streamMetadata, boolean isSigned) {
    var values = DecodingUtils.decodeLongVarint(data, offset, streamMetadata.numValues());
    return decodeLongArray(values, streamMetadata, isSigned);
  }

  private static List<Long> decodeLongArray(
      long[] values, StreamMetadata streamMetadata, boolean isSigned) {
      switch (streamMetadata.logicalLevelTechnique1()) {
      case DELTA:
        if (streamMetadata.logicalLevelTechnique2().equals(LogicalLevelTechnique.RLE)) {
          var rleMetadata = (RleEncodedStreamMetadata) streamMetadata;
          values =
              DecodingUtils.decodeUnsignedRLE(
                  values, rleMetadata.runs(), rleMetadata.numRleValues());
          return decodeZigZagDelta(values);
        }
        return decodeZigZagDelta(values);
      case RLE:
      {
        var rleMetadata = (RleEncodedStreamMetadata) streamMetadata;
          var decodedValues = decodeRLE(values, rleMetadata.runs(), rleMetadata.numRleValues());
          return isSigned
              ? decodeZigZag(decodedValues.stream().mapToLong(i -> i).toArray())
              : decodedValues;
        }
      case NONE:
        {
          if (isSigned) {
            return decodeZigZag(values);
          }
          return Arrays.stream(values).boxed().collect(Collectors.toList());
        }
    }

    throw new IllegalArgumentException(
      "The specified logical level technique is not supported for long integers: "
          + streamMetadata.logicalLevelTechnique1());
}

  // TODO: quick and dirty -> write fast vectorized solution
  private static List<Integer> decodeRLE(int[] data, int numRuns, int numRleValues) {
    var values = new ArrayList<Integer>(numRleValues);
    for (var i = 0; i < numRuns; i++) {
      var run = data[i];
      var value = data[i + numRuns];
      for (var j = 0; j < run; j++) {
        values.add(value);
      }
    }

    return values;
  }

  // TODO: quick and dirty -> write fast vectorized solution
  private static List<Long> decodeRLE(long[] data, int numRuns, int numRleValues) {
    var values = new ArrayList<Long>(numRleValues);
    for (var i = 0; i < numRuns; i++) {
      var run = data[i];
      var value = data[i + numRuns];
      for (var j = 0; j < run; j++) {
        values.add(value);
      }
    }

    return values;
  }

  private static List<Integer> decodeDeltaRLE(int[] data, int numRuns) {
    var deltaValues = new ArrayList<Integer>();
    for (var i = 0; i < numRuns; i++) {
      var run = data[i];
      /* Only values are zig-zag encoded */
      var delta = DecodingUtils.decodeZigZag(data[i + numRuns]);
      // values.add(delta + previousValue);
      for (var j = 0; j < run; j++) {
        deltaValues.add(delta);
      }
    }

    // TODO: merge rle and delta encoding
    var values = new ArrayList<Integer>(deltaValues.size());
    var previousValue = 0;
    for (var delta : deltaValues) {
      var value = delta + previousValue;
      values.add(value);
      previousValue = value;
    }

    return values;
  }

  private static List<Long> decodeDeltaRLE(long[] data, int numRuns) {
    var deltaValues = new ArrayList<Long>();
    for (var i = 0; i < numRuns; i++) {
      var run = data[i];
      /* Only values are zig-zag encoded */
      var delta = DecodingUtils.decodeZigZag(data[i + numRuns]);
      // values.add(delta + previousValue);
      for (var j = 0; j < run; j++) {
        deltaValues.add(delta);
      }
    }

    // TODO: merge rle and delta encoding
    var values = new ArrayList<Long>(deltaValues.size());
    var previousValue = 0l;
    for (var delta : deltaValues) {
      var value = delta + previousValue;
      values.add(value);
      previousValue = value;
    }

    return values;
  }

  private static List<Integer> decodeZigZagDelta(int[] data) {
    var values = new ArrayList<Integer>(data.length);
    var previousValue = 0;
    for (var zigZagDelta : data) {
      var delta = DecodingUtils.decodeZigZag(zigZagDelta);
      var value = previousValue + delta;
      values.add(value);
      previousValue = value;
    }

    return values;
  }

  private static List<Integer> decodeDelta(int[] data) {
    var values = new ArrayList<Integer>(data.length);
    var previousValue = 0;
    for (var delta : data) {
      var value = previousValue + delta;
      values.add(value);
      previousValue = value;
    }

    return values;
  }

  private static List<Long> decodeZigZagDelta(long[] data) {
    var values = new ArrayList<Long>(data.length);
    var previousValue = 0l;
    for (var zigZagDelta : data) {
      var delta = DecodingUtils.decodeZigZag(zigZagDelta);
      var value = previousValue + delta;
      values.add(value);
      previousValue = value;
    }

    return values;
  }

  private static List<Long> decodeZigZag(long[] data) {
    var values = new ArrayList<Long>(data.length);
    for (var zigZagDelta : data) {
      var value = DecodingUtils.decodeZigZag(zigZagDelta);
      values.add(value);
    }
    return values;
  }

  private static List<Integer> decodeZigZag(int[] data) {
    var values = new ArrayList<Integer>(data.length);
    for (var zigZagDelta : data) {
      var value = DecodingUtils.decodeZigZag(zigZagDelta);
      values.add(value);
    }
    return values;
  }
}
