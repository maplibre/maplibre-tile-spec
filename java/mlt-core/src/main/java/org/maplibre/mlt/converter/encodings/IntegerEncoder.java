package org.maplibre.mlt.converter.encodings;

import com.google.common.collect.Lists;
import com.google.common.primitives.Ints;
import com.google.common.primitives.Longs;
import jakarta.annotation.Nullable;
import java.io.IOException;
import java.util.*;
import java.util.function.BiFunction;
import org.apache.commons.lang3.ArrayUtils;
import org.jetbrains.annotations.NotNull;
import org.maplibre.mlt.converter.CollectionUtils;
import org.maplibre.mlt.converter.ConversionConfig.IntegerEncodingOption;
import org.maplibre.mlt.converter.MLTStreamObserver;
import org.maplibre.mlt.metadata.stream.*;

/*
 * TODO: Add sampling strategy for encoding selection
 *  -> Inspired by BTRBlock sampling strategy:
 *  - take about 1% of all data as samples
 *  - divide into ? blocks?
 * -> https://github.com/maxi-k/btrblocks/blob/c954ffd31f0873003dbc26bf1676ac460d7a3b05/btrblocks/scheme/double/RLE.cpp#L17
 * */
public class IntegerEncoder {

  public static class IntegerEncodingResult {
    public LogicalLevelTechnique logicalLevelTechnique1;
    public LogicalLevelTechnique logicalLevelTechnique2;
    public byte[] encodedValues;
    /* If rle or delta-rle encoding is used, otherwise can be ignored */
    public int numRuns;
    public int physicalLevelEncodedValuesLength;
    public int totalValues;
  }

  enum LogicalLevelIntegerTechnique {
    PLAIN,
    DELTA,
    RLE,
    DELTA_RLE
  }

  private IntegerEncoder() {}

  public static byte[] encodeMortonStream(
      int[] values, int numBits, int coordinateShift, PhysicalLevelTechnique physicalLevelTechnique)
      throws IOException {
    var encodedValueStream = encodeMortonCodes(values, physicalLevelTechnique);
    var valuesMetadata =
        new MortonEncodedStreamMetadata(
            PhysicalStreamType.DATA,
            new LogicalStreamType(DictionaryType.MORTON),
            encodedValueStream.logicalLevelTechnique1,
            encodedValueStream.logicalLevelTechnique2,
            physicalLevelTechnique,
            encodedValueStream.physicalLevelEncodedValuesLength,
            encodedValueStream.encodedValues.length,
            numBits,
            coordinateShift);

    return ArrayUtils.addAll(valuesMetadata.encode(), encodedValueStream.encodedValues);
  }

  // Encodes integer stream with AUTO encoding option (backward compatibility).
  public static byte[] encodeIntStream(
      List<Integer> values,
      PhysicalLevelTechnique physicalLevelTechnique,
      boolean isSigned,
      PhysicalStreamType streamType,
      LogicalStreamType logicalStreamType,
      @NotNull MLTStreamObserver streamObserver,
      @Nullable String streamName)
      throws IOException {
    return encodeIntStream(
        CollectionUtils.unboxInts(values),
        physicalLevelTechnique,
        isSigned,
        streamType,
        logicalStreamType,
        streamObserver,
        streamName);
  }

  // Encodes integer stream with AUTO encoding option (backward compatibility).
  public static byte[] encodeIntStream(
      int[] values,
      PhysicalLevelTechnique physicalLevelTechnique,
      boolean isSigned,
      PhysicalStreamType streamType,
      LogicalStreamType logicalStreamType,
      @NotNull MLTStreamObserver streamObserver,
      @Nullable String streamName)
      throws IOException {
    return encodeIntStream(
        values,
        physicalLevelTechnique,
        isSigned,
        streamType,
        logicalStreamType,
        IntegerEncodingOption.AUTO,
        streamObserver,
        streamName);
  }

  public static byte[] encodeIntStream(
      List<Integer> values,
      PhysicalLevelTechnique physicalLevelTechnique,
      boolean isSigned,
      PhysicalStreamType streamType,
      LogicalStreamType logicalStreamType,
      @NotNull IntegerEncodingOption encodingOption,
      @NotNull MLTStreamObserver streamObserver,
      @Nullable String streamName)
      throws IOException {
    return encodeIntStream(
        CollectionUtils.unboxInts(values),
        physicalLevelTechnique,
        isSigned,
        streamType,
        logicalStreamType,
        encodingOption,
        streamObserver,
        streamName);
  }

  public static byte[] encodeIntStream(
      int[] values,
      PhysicalLevelTechnique physicalLevelTechnique,
      boolean isSigned,
      PhysicalStreamType streamType,
      LogicalStreamType logicalStreamType,
      @NotNull IntegerEncodingOption encodingOption,
      @NotNull MLTStreamObserver streamObserver,
      @Nullable String streamName)
      throws IOException {
    var encodedValueStream =
        IntegerEncoder.encodeInt(values, physicalLevelTechnique, isSigned, encodingOption);

    // TODO: refactor -> also allow the use of none null suppression techniques
    var streamMetadata =
        (encodedValueStream.logicalLevelTechnique1 == LogicalLevelTechnique.RLE
                || encodedValueStream.logicalLevelTechnique2 == LogicalLevelTechnique.RLE)
            ? new RleEncodedStreamMetadata(
                streamType,
                logicalStreamType,
                encodedValueStream.logicalLevelTechnique1,
                encodedValueStream.logicalLevelTechnique2,
                physicalLevelTechnique,
                encodedValueStream.physicalLevelEncodedValuesLength,
                encodedValueStream.encodedValues.length,
                encodedValueStream.numRuns,
                values.length)
            : new StreamMetadata(
                streamType,
                logicalStreamType,
                encodedValueStream.logicalLevelTechnique1,
                encodedValueStream.logicalLevelTechnique2,
                physicalLevelTechnique,
                encodedValueStream.physicalLevelEncodedValuesLength,
                encodedValueStream.encodedValues.length);
    var encodedMetadata = streamMetadata.encode();
    streamObserver.observeStream(
        streamName, values, encodedMetadata, encodedValueStream.encodedValues);
    return ArrayUtils.addAll(encodedMetadata, encodedValueStream.encodedValues);
  }

  // Encodes long stream with AUTO encoding option (backward compatibility).
  public static byte[] encodeLongStream(
      long[] values,
      boolean isSigned,
      PhysicalStreamType streamType,
      LogicalStreamType logicalStreamType,
      @NotNull MLTStreamObserver streamObserver,
      @Nullable String streamName)
      throws IOException {
    return encodeLongStream(
        values,
        isSigned,
        streamType,
        logicalStreamType,
        IntegerEncodingOption.AUTO,
        streamObserver,
        streamName);
  }

  public static byte[] encodeLongStream(
      long[] values,
      boolean isSigned,
      PhysicalStreamType streamType,
      LogicalStreamType logicalStreamType,
      @NotNull IntegerEncodingOption encodingOption,
      @NotNull MLTStreamObserver streamObserver,
      @Nullable String streamName)
      throws IOException {
    var encodedValueStream = IntegerEncoder.encodeLong(values, isSigned, encodingOption);

    /* Currently FastPfor is only supported with 32 bit so for long we always have to fallback to Varint encoding */
    var streamMetadata =
        (encodedValueStream.logicalLevelTechnique1 == LogicalLevelTechnique.RLE
                || encodedValueStream.logicalLevelTechnique2 == LogicalLevelTechnique.RLE)
            ? new RleEncodedStreamMetadata(
                streamType,
                logicalStreamType,
                encodedValueStream.logicalLevelTechnique1,
                encodedValueStream.logicalLevelTechnique2,
                PhysicalLevelTechnique.VARINT,
                encodedValueStream.physicalLevelEncodedValuesLength,
                encodedValueStream.encodedValues.length,
                encodedValueStream.numRuns,
                values.length)
            : new StreamMetadata(
                streamType,
                logicalStreamType,
                encodedValueStream.logicalLevelTechnique1,
                encodedValueStream.logicalLevelTechnique2,
                PhysicalLevelTechnique.VARINT,
                encodedValueStream.physicalLevelEncodedValuesLength,
                encodedValueStream.encodedValues.length);
    var encodedMetadata = streamMetadata.encode();
    streamObserver.observeStream(
        streamName, values, encodedMetadata, encodedValueStream.encodedValues);
    return ArrayUtils.addAll(encodedMetadata, encodedValueStream.encodedValues);
  }

  // TODO: make dependent on specified LogicalLevelTechnique
  public static IntegerEncodingResult encodeMortonCodes(
      int[] values, PhysicalLevelTechnique physicalLevelTechnique) throws IOException {
    var previousValue = 0;
    int[] deltaValues = new int[values.length];
    for (var i = 0; i < values.length; i++) {
      var value = values[i];
      var delta = value - previousValue;
      deltaValues[i] = delta;
      previousValue = value;
    }

    var encodedValues =
        physicalLevelTechnique == PhysicalLevelTechnique.FAST_PFOR
            ? encodeFastPfor(deltaValues, false)
            : EncodingUtils.encodeVarints(deltaValues, false, false);

    var result = new IntegerEncodingResult();
    result.logicalLevelTechnique1 = LogicalLevelTechnique.MORTON;
    result.logicalLevelTechnique2 = LogicalLevelTechnique.DELTA;
    result.physicalLevelEncodedValuesLength = values.length;
    result.numRuns = 0;
    result.encodedValues = encodedValues;
    return result;
  }

  // Encodes integers with AUTO encoding option (backward compatibility).
  public static IntegerEncodingResult encodeInt(
      int[] values, PhysicalLevelTechnique physicalLevelTechnique, boolean isSigned) {
    return encodeInt(values, physicalLevelTechnique, isSigned, IntegerEncodingOption.AUTO);
  }

  /*
   * Integers are encoded based on the two lightweight compression techniques delta and rle as well
   * as a combination of both schemes called delta-rle.
   * */
  public static IntegerEncodingResult encodeInt(
      int[] values,
      PhysicalLevelTechnique physicalLevelTechnique,
      boolean isSigned,
      @NotNull IntegerEncodingOption encodingOption) {
    var previousValue = 0;
    var previousDelta = 0;
    var runs = 1;
    var deltaRuns = 1;
    var deltaValues = new int[values.length];
    for (int i = 0; i < values.length; i++) {
      int value = values[i];
      var delta = value - previousValue;
      deltaValues[i] = delta;

      if (value != previousValue && i != 0) {
        runs++;
      }

      if (delta != previousDelta && i != 0) {
        deltaRuns++;
      }

      previousValue = value;
      previousDelta = delta;
    }

    BiFunction<int[], Boolean, byte[]> encoder =
        physicalLevelTechnique == PhysicalLevelTechnique.FAST_PFOR
            ? IntegerEncoder::encodeFastPfor
            : (v, s) -> {
              try {
                return EncodingUtils.encodeVarints(v, s, false);
              } catch (IOException e) {
                throw new RuntimeException(e);
              }
            };

    // Early return for forced PLAIN encoding
    if (encodingOption == IntegerEncodingOption.PLAIN) {
      var result = new IntegerEncodingResult();
      result.encodedValues = encoder.apply(values, isSigned);
      result.physicalLevelEncodedValuesLength = values.length;
      result.logicalLevelTechnique1 = LogicalLevelTechnique.NONE;
      result.logicalLevelTechnique2 = LogicalLevelTechnique.NONE;
      return result;
    }

    // Early return for forced DELTA encoding
    if (encodingOption == IntegerEncodingOption.DELTA) {
      var result = new IntegerEncodingResult();
      result.encodedValues = encoder.apply(deltaValues, true);
      result.physicalLevelEncodedValuesLength = values.length;
      result.logicalLevelTechnique1 = LogicalLevelTechnique.DELTA;
      result.logicalLevelTechnique2 = LogicalLevelTechnique.NONE;
      return result;
    }

    var plainEncodedValues = encoder.apply(values, isSigned);
    var deltaEncodedValues = encoder.apply(deltaValues, true);
    var encodedValues = Lists.newArrayList(plainEncodedValues, deltaEncodedValues);
    byte[] rleEncodedValues = null;
    byte[] deltaRleEncodedValues = null;
    var rlePhysicalLevelEncodedValuesLength = 0;
    var deltaRlePhysicalLevelEncodedValuesLength = 0;

    /* Use selection logic from BTR Blocks -> https://github.com/maxi-k/btrblocks/blob/c954ffd31f0873003dbc26bf1676ac460d7a3b05/btrblocks/scheme/double/RLE.cpp#L17 */
    /*
     * if there are ony a view values (e.g. 4 times 1) rle only produces the same size then other
     * encodings. Since we want to force that all const streams use RLE encoding we use this current
     * workaround
     */
    var isConstStream = false;
    if (values.length / runs >= 2
        && (encodingOption == IntegerEncodingOption.AUTO
            || encodingOption == IntegerEncodingOption.RLE)) {
      var rleValues = EncodingUtils.encodeRle(values);
      rlePhysicalLevelEncodedValuesLength =
          rleValues.getLeft().length + rleValues.getRight().length;
      rleEncodedValues =
          encoder.apply(
              Ints.concat(
                  rleValues.getLeft(),
                  isSigned
                      ? EncodingUtils.encodeZigZag(rleValues.getRight())
                      : rleValues.getRight()),
              false);
      isConstStream = rleValues.getLeft().length == 1;

      // Early return for forced RLE encoding
      if (encodingOption == IntegerEncodingOption.RLE) {
        var result = new IntegerEncodingResult();
        result.encodedValues = rleEncodedValues;
        result.physicalLevelEncodedValuesLength = rlePhysicalLevelEncodedValuesLength;
        result.numRuns = runs;
        result.logicalLevelTechnique1 = LogicalLevelTechnique.RLE;
        result.logicalLevelTechnique2 = LogicalLevelTechnique.NONE;
        return result;
      }
    }

    if (deltaValues.length / deltaRuns >= 2) {
      // TODO: get rid of conversion
      var deltaRleValues = EncodingUtils.encodeRle(deltaValues);
      deltaRlePhysicalLevelEncodedValuesLength =
          deltaRleValues.getLeft().length + deltaRleValues.getRight().length;
      var zigZagDelta = EncodingUtils.encodeZigZag(deltaRleValues.getRight());
      // TODO: encode runs and length separate?
      deltaRleEncodedValues =
          encoder.apply(Ints.concat(deltaRleValues.getLeft(), zigZagDelta), false);

      // Early return for forced DELTA_RLE encoding
      if (encodingOption == IntegerEncodingOption.DELTA_RLE) {
        var result = new IntegerEncodingResult();
        result.encodedValues = deltaRleEncodedValues;
        result.physicalLevelEncodedValuesLength = deltaRlePhysicalLevelEncodedValuesLength;
        result.numRuns = deltaRuns;
        result.logicalLevelTechnique1 = LogicalLevelTechnique.DELTA;
        result.logicalLevelTechnique2 = LogicalLevelTechnique.RLE;
        return result;
      }
    }

    encodedValues.add(rleEncodedValues);
    encodedValues.add(deltaRleEncodedValues);

    // TODO: refactor -> find proper solution
    var encodedValuesSizes =
        encodedValues.stream().map(v -> v == null ? Integer.MAX_VALUE : v.length).toList();
    var index =
        isConstStream
            ? LogicalLevelIntegerTechnique.RLE.ordinal()
            : encodedValuesSizes.indexOf(Collections.min(encodedValuesSizes));
    var encoding = LogicalLevelIntegerTechnique.values()[index];

    var result = new IntegerEncodingResult();
    result.encodedValues = encodedValues.get(index);
    result.physicalLevelEncodedValuesLength = values.length;
    if (encoding == LogicalLevelIntegerTechnique.RLE || isConstStream) {
      result.numRuns = runs;
      result.physicalLevelEncodedValuesLength = rlePhysicalLevelEncodedValuesLength;
      result.logicalLevelTechnique1 = LogicalLevelTechnique.RLE;
      result.logicalLevelTechnique2 = LogicalLevelTechnique.NONE;
    } else if (encoding == LogicalLevelIntegerTechnique.DELTA_RLE) {
      result.numRuns = deltaRuns;
      result.physicalLevelEncodedValuesLength = deltaRlePhysicalLevelEncodedValuesLength;
      result.logicalLevelTechnique1 = LogicalLevelTechnique.DELTA;
      result.logicalLevelTechnique2 = LogicalLevelTechnique.RLE;
    } else if (encoding == LogicalLevelIntegerTechnique.DELTA) {
      result.logicalLevelTechnique1 = LogicalLevelTechnique.DELTA;
      result.logicalLevelTechnique2 = LogicalLevelTechnique.NONE;
    } else {
      result.logicalLevelTechnique1 = LogicalLevelTechnique.NONE;
      result.logicalLevelTechnique2 = LogicalLevelTechnique.NONE;
    }

    return result;
  }

  // Encodes long values with AUTO encoding option (backward compatibility).
  public static IntegerEncodingResult encodeLong(long[] values, boolean isSigned) {
    return encodeLong(values, isSigned, IntegerEncodingOption.AUTO);
  }

  public static IntegerEncodingResult encodeLong(
      long[] values, boolean isSigned, @NotNull IntegerEncodingOption encodingOption) {
    var previousValue = 0L;
    var previousDelta = 0L;
    var runs = 1;
    var deltaRuns = 1;
    var deltaValues = new long[values.length];
    for (var i = 0; i < values.length; i++) {
      var value = values[i];
      var delta = value - previousValue;
      deltaValues[i] = delta;

      if (value != previousValue && i != 0) {
        runs++;
      }

      if (delta != previousDelta && i != 0) {
        deltaRuns++;
      }

      previousValue = value;
      previousDelta = delta;
    }

    BiFunction<long[], Boolean, byte[]> encoder =
        (v, s) -> {
          try {
            return EncodingUtils.encodeLongVarints(v, s, false);
          } catch (IOException e) {
            throw new RuntimeException(e);
          }
        };

    if (encodingOption == IntegerEncodingOption.PLAIN) {
      var result = new IntegerEncodingResult();
      result.encodedValues = encoder.apply(values, isSigned);
      result.physicalLevelEncodedValuesLength = values.length;
      result.logicalLevelTechnique1 = LogicalLevelTechnique.NONE;
      result.logicalLevelTechnique2 = LogicalLevelTechnique.NONE;
      return result;
    }

    if (encodingOption == IntegerEncodingOption.DELTA) {
      var result = new IntegerEncodingResult();
      result.encodedValues = encoder.apply(deltaValues, true);
      result.physicalLevelEncodedValuesLength = values.length;
      result.logicalLevelTechnique1 = LogicalLevelTechnique.DELTA;
      result.logicalLevelTechnique2 = LogicalLevelTechnique.NONE;
      return result;
    }

    var plainEncodedValues = encoder.apply(values, isSigned);
    var deltaEncodedValues = encoder.apply(deltaValues, true);
    var encodedValues = Lists.newArrayList(plainEncodedValues, deltaEncodedValues);

    byte[] rleEncodedValues = null;
    byte[] deltaRleEncodedValues = null;
    var rlePhysicalLevelEncodedValuesLength = 0;
    var deltaRlePhysicalLevelEncodedValuesLength = 0;
    var isConstStream = false;

    /* Use selection logic from BTR Blocks -> https://github.com/maxi-k/btrblocks/blob/c954ffd31f0873003dbc26bf1676ac460d7a3b05/btrblocks/scheme/double/RLE.cpp#L17 */
    if (values.length / runs >= 2
        && (encodingOption == IntegerEncodingOption.AUTO
            || encodingOption == IntegerEncodingOption.RLE)) {
      // TODO: get rid of conversion
      var rleValues = EncodingUtils.encodeRle(values);
      rlePhysicalLevelEncodedValuesLength =
          rleValues.getLeft().length + rleValues.getRight().length;
      rleEncodedValues =
          encoder.apply(
              Longs.concat(
                  rleValues.getLeft(),
                  isSigned
                      ? EncodingUtils.encodeZigZag(rleValues.getRight())
                      : rleValues.getRight()),
              false);
      isConstStream = rleValues.getLeft().length == 1;

      if (encodingOption == IntegerEncodingOption.RLE) {
        var result = new IntegerEncodingResult();
        result.encodedValues = rleEncodedValues;
        result.physicalLevelEncodedValuesLength = rlePhysicalLevelEncodedValuesLength;
        result.numRuns = runs;
        result.logicalLevelTechnique1 = LogicalLevelTechnique.RLE;
        result.logicalLevelTechnique2 = LogicalLevelTechnique.NONE;
        return result;
      }
    }

    if (deltaValues.length / deltaRuns >= 2) {
      // TODO: get rid of conversion
      var deltaRleValues = EncodingUtils.encodeRle(deltaValues);
      deltaRlePhysicalLevelEncodedValuesLength =
          deltaRleValues.getLeft().length + deltaRleValues.getRight().length;
      var zigZagDelta = EncodingUtils.encodeZigZag(deltaRleValues.getRight());
      // TODO: encode runs and length separate?
      deltaRleEncodedValues =
          encoder.apply(Longs.concat(deltaRleValues.getLeft(), zigZagDelta), false);

      if (encodingOption == IntegerEncodingOption.DELTA_RLE) {
        var result = new IntegerEncodingResult();
        result.encodedValues = deltaRleEncodedValues;
        result.physicalLevelEncodedValuesLength = deltaRlePhysicalLevelEncodedValuesLength;
        result.numRuns = deltaRuns;
        result.logicalLevelTechnique1 = LogicalLevelTechnique.DELTA;
        result.logicalLevelTechnique2 = LogicalLevelTechnique.RLE;
        return result;
      }
    }

    encodedValues.add(rleEncodedValues);
    encodedValues.add(deltaRleEncodedValues);

    // TODO: refactor -> find proper solution
    var encodedValuesSizes =
        encodedValues.stream().map(v -> v == null ? Integer.MAX_VALUE : v.length).toList();
    var index =
        isConstStream
            ? LogicalLevelIntegerTechnique.RLE.ordinal()
            : encodedValuesSizes.indexOf(Collections.min(encodedValuesSizes));
    var encoding = LogicalLevelIntegerTechnique.values()[index];

    var result = new IntegerEncodingResult();
    result.encodedValues = encodedValues.get(index);
    result.physicalLevelEncodedValuesLength = values.length;
    if (encoding == LogicalLevelIntegerTechnique.RLE || isConstStream) {
      result.numRuns = runs;
      result.physicalLevelEncodedValuesLength = rlePhysicalLevelEncodedValuesLength;
      result.logicalLevelTechnique1 = LogicalLevelTechnique.RLE;
      result.logicalLevelTechnique2 = LogicalLevelTechnique.NONE;
    } else if (encoding == LogicalLevelIntegerTechnique.DELTA_RLE) {
      result.numRuns = deltaRuns;
      result.physicalLevelEncodedValuesLength = deltaRlePhysicalLevelEncodedValuesLength;
      result.logicalLevelTechnique1 = LogicalLevelTechnique.DELTA;
      result.logicalLevelTechnique2 = LogicalLevelTechnique.RLE;
    } else if (encoding == LogicalLevelIntegerTechnique.DELTA) {
      result.logicalLevelTechnique1 = LogicalLevelTechnique.DELTA;
      result.logicalLevelTechnique2 = LogicalLevelTechnique.NONE;
    } else {
      result.logicalLevelTechnique1 = LogicalLevelTechnique.NONE;
      result.logicalLevelTechnique2 = LogicalLevelTechnique.NONE;
    }

    return result;
  }

  public static byte[] encodeFastPfor(int[] values, boolean signed) {
    return EncodingUtils.encodeFastPfor128(values, signed, false);
  }

  public static byte[] encodeVarint(int[] values, boolean signed) throws IOException {
    return EncodingUtils.encodeVarints(values, signed, false);
  }

  public static byte[] encodeLongVarint(long[] values, boolean signed) throws IOException {
    return EncodingUtils.encodeLongVarints(values, signed, false);
  }
}
