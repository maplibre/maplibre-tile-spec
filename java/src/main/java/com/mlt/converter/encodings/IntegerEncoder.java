package com.mlt.converter.encodings;

import com.google.common.collect.Lists;
import com.mlt.metadata.stream.*;
import java.util.*;
import java.util.function.BiFunction;
import java.util.stream.Collectors;
import java.util.stream.Stream;
import org.apache.commons.lang3.ArrayUtils;

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
      List<Integer> values,
      int numBits,
      int coordinateShift,
      PhysicalLevelTechnique physicalLevelTechnique) {
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

  public static byte[] encodeIntStream(
      List<Integer> values,
      PhysicalLevelTechnique physicalLevelTechnique,
      boolean isSigned,
      PhysicalStreamType streamType,
      LogicalStreamType logicalStreamType) {
    var encodedValueStream = IntegerEncoder.encodeInt(values, physicalLevelTechnique, isSigned);

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
                values.size())
            : new StreamMetadata(
                streamType,
                logicalStreamType,
                encodedValueStream.logicalLevelTechnique1,
                encodedValueStream.logicalLevelTechnique2,
                physicalLevelTechnique,
                encodedValueStream.physicalLevelEncodedValuesLength,
                encodedValueStream.encodedValues.length);

    return ArrayUtils.addAll(streamMetadata.encode(), encodedValueStream.encodedValues);
  }

  public static byte[] encodeLongStream(
      List<Long> values,
      boolean isSigned,
      PhysicalStreamType streamType,
      LogicalStreamType logicalStreamType) {
    var encodedValueStream = IntegerEncoder.encodeLong(values, isSigned);

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
                values.size())
            : new StreamMetadata(
                streamType,
                logicalStreamType,
                encodedValueStream.logicalLevelTechnique1,
                encodedValueStream.logicalLevelTechnique2,
                PhysicalLevelTechnique.VARINT,
                encodedValueStream.physicalLevelEncodedValuesLength,
                encodedValueStream.encodedValues.length);

    return ArrayUtils.addAll(streamMetadata.encode(), encodedValueStream.encodedValues);
  }

  // TODO: make dependent on specified LogicalLevelTechnique
  public static IntegerEncodingResult encodeMortonCodes(
      List<Integer> values, PhysicalLevelTechnique physicalLevelTechnique) {
    var previousValue = 0;
    var deltaValues = new ArrayList<Integer>();
    for (var i = 0; i < values.size(); i++) {
      var value = values.get(i);
      var delta = value - previousValue;
      deltaValues.add(delta);
      previousValue = value;
    }

    var encodedValues =
        physicalLevelTechnique == PhysicalLevelTechnique.FAST_PFOR
            ? encodeFastPfor(deltaValues, false)
            : encodeVarint(
                deltaValues.stream().mapToLong(i -> i).boxed().collect(Collectors.toList()), false);

    var result = new IntegerEncodingResult();
    result.logicalLevelTechnique1 = LogicalLevelTechnique.MORTON;
    result.logicalLevelTechnique2 = LogicalLevelTechnique.DELTA;
    result.physicalLevelEncodedValuesLength = values.size();
    result.numRuns = 0;
    result.encodedValues = encodedValues;
    return result;
  }

  /*
   * Integers are encoded based on the two lightweight compression techniques delta and rle as well
   * as a combination of both schemes called delta-rle.
   * */
  public static IntegerEncodingResult encodeInt(
      List<Integer> values, PhysicalLevelTechnique physicalLevelTechnique, boolean isSigned) {
    var previousValue = 0;
    var previousDelta = 0;
    var runs = 1;
    var deltaRuns = 1;
    var deltaValues = new ArrayList<Integer>();
    for (var i = 0; i < values.size(); i++) {
      var value = values.get(i);
      var delta = value - previousValue;
      deltaValues.add(delta);

      if (value != previousValue && i != 0) {
        runs++;
      }

      if (delta != previousDelta && i != 0) {
        deltaRuns++;
      }

      previousValue = value;
      previousDelta = delta;
    }

    BiFunction<List<Integer>, Boolean, byte[]> encoder =
        physicalLevelTechnique == PhysicalLevelTechnique.FAST_PFOR
            ? (v, s) -> encodeFastPfor(v, s)
            : (v, s) ->
                encodeVarint(v.stream().mapToLong(i -> i).boxed().collect(Collectors.toList()), s);

    var plainEncodedValues = encoder.apply(values, isSigned);
    var deltaEncodedValues = encoder.apply(deltaValues, true);
    var encodedValues = Lists.newArrayList(plainEncodedValues, deltaEncodedValues);
    byte[] rleEncodedValues = null;
    byte[] deltaRleEncodedValues = null;
    var rlePhysicalLevelEncodedValuesLength = 0;
    var deltaRlePhysicalLevelEncodedValuesLength = 0;

    /* Use selection logic from BTR Blocks -> https://github.com/maxi-k/btrblocks/blob/c954ffd31f0873003dbc26bf1676ac460d7a3b05/btrblocks/scheme/double/RLE.cpp#L17 */
    /**
     * if there are ony a view values (e.g. 4 times 1) rle only produces the same size then other
     * encodings. Since we want to force that all const streams use RLE encoding we use this current
     * workaround
     */
    var isConstStream = false;
    if (values.size() / runs >= 2) {
      // TODO: get rid of conversion
      var rleValues = EncodingUtils.encodeRle(values.stream().mapToInt(i -> i).toArray());
      rlePhysicalLevelEncodedValuesLength =
          rleValues.getLeft().size() + rleValues.getRight().size();
      rleEncodedValues =
          encoder.apply(
              Stream.concat(
                      rleValues.getLeft().stream(),
                      isSigned
                          ? Arrays.stream(
                                  EncodingUtils.encodeZigZag(
                                      rleValues.getRight().stream().mapToInt(i -> i).toArray()))
                              .boxed()
                          : rleValues.getRight().stream())
                  .toList(),
              false);
      isConstStream = rleValues.getLeft().size() == 1;
    }

    if (deltaValues.size() / deltaRuns >= 2) {
      // TODO: get rid of conversion
      var deltaRleValues = EncodingUtils.encodeRle(deltaValues.stream().mapToInt(i -> i).toArray());
      deltaRlePhysicalLevelEncodedValuesLength =
          deltaRleValues.getLeft().size() + deltaRleValues.getRight().size();
      var zigZagDelta =
          EncodingUtils.encodeZigZag(deltaRleValues.getRight().stream().mapToInt(i -> i).toArray());
      // TODO: encode runs and length separate?
      deltaRleEncodedValues =
          encoder.apply(
              Stream.concat(deltaRleValues.getLeft().stream(), Arrays.stream(zigZagDelta).boxed())
                  .toList(),
              false);
    }

    encodedValues.add(rleEncodedValues);
    encodedValues.add(deltaRleEncodedValues);

    // TODO: refactor -> find proper solution
    var encodedValuesSizes =
        encodedValues.stream()
            .map(v -> v == null ? Integer.MAX_VALUE : v.length)
            .collect(Collectors.toList());
    var index =
        isConstStream
            ? LogicalLevelIntegerTechnique.RLE.ordinal()
            : encodedValuesSizes.indexOf(Collections.min(encodedValuesSizes));
    var encoding = LogicalLevelIntegerTechnique.values()[index];

    var result = new IntegerEncodingResult();
    result.encodedValues = encodedValues.get(index);
    result.physicalLevelEncodedValuesLength = values.size();
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

  // TODO: make generic to merge with encodeInt
  public static IntegerEncodingResult encodeLong(List<Long> values, boolean isSigned) {
    var previousValue = 0l;
    var previousDelta = 0l;
    var runs = 1;
    var deltaRuns = 1;
    var deltaValues = new ArrayList<Long>();
    for (var i = 0; i < values.size(); i++) {
      var value = values.get(i);
      var delta = value - previousValue;
      deltaValues.add(delta);

      if (value != previousValue && i != 0) {
        runs++;
      }

      if (delta != previousDelta && i != 0) {
        deltaRuns++;
      }

      previousValue = value;
      previousDelta = delta;
    }

    BiFunction<List<Long>, Boolean, byte[]> encoder =
        (v, s) ->
            encodeVarint(v.stream().mapToLong(i -> i).boxed().collect(Collectors.toList()), s);

    var plainEncodedValues = encoder.apply(values, isSigned);
    var deltaEncodedValues = encoder.apply(deltaValues, true);
    var encodedValues = Lists.newArrayList(plainEncodedValues, deltaEncodedValues);

    byte[] rleEncodedValues = null;
    byte[] deltaRleEncodedValues = null;
    var rlePhysicalLevelEncodedValuesLength = 0;
    var deltaRlePhysicalLevelEncodedValuesLength = 0;

    /* Use selection logic from BTR Blocks -> https://github.com/maxi-k/btrblocks/blob/c954ffd31f0873003dbc26bf1676ac460d7a3b05/btrblocks/scheme/double/RLE.cpp#L17 */
    if (values.size() / runs >= 2) {
      // TODO: get rid of conversion
      var rleValues = EncodingUtils.encodeRle(values.stream().mapToLong(i -> i).toArray());
      rlePhysicalLevelEncodedValuesLength =
          rleValues.getLeft().size() + rleValues.getRight().size();
      rleEncodedValues =
          encoder.apply(
              Stream.concat(
                      rleValues.getLeft().stream().mapToLong(i -> i).boxed(),
                      isSigned
                          ? Arrays.stream(
                                  EncodingUtils.encodeZigZag(
                                      rleValues.getRight().stream().mapToLong(i -> i).toArray()))
                              .boxed()
                          : rleValues.getRight().stream())
                  .toList(),
              false);
    }

    if (deltaValues.size() / deltaRuns >= 2) {
      // TODO: get rid of conversion
      var deltaRleValues =
          EncodingUtils.encodeRle(deltaValues.stream().mapToLong(i -> i).toArray());
      deltaRlePhysicalLevelEncodedValuesLength =
          deltaRleValues.getLeft().size() + deltaRleValues.getRight().size();
      var zigZagDelta =
          EncodingUtils.encodeZigZag(
              deltaRleValues.getRight().stream().mapToLong(i -> i).toArray());
      // TODO: encode runs and length separate?
      deltaRleEncodedValues =
          encoder.apply(
              Stream.concat(
                      deltaRleValues.getLeft().stream().mapToLong(i -> i).boxed(),
                      Arrays.stream(zigZagDelta).boxed())
                  .toList(),
              false);
    }

    encodedValues.add(rleEncodedValues);
    encodedValues.add(deltaRleEncodedValues);

    // TODO: refactor -> find proper solution
    var encodedValuesSizes =
        encodedValues.stream()
            .map(v -> v == null ? Integer.MAX_VALUE : v.length)
            .collect(Collectors.toList());
    var index = encodedValuesSizes.indexOf(Collections.min(encodedValuesSizes));
    var encoding = LogicalLevelIntegerTechnique.values()[index];

    var result = new IntegerEncodingResult();
    result.encodedValues = encodedValues.get(index);
    result.physicalLevelEncodedValuesLength = values.size();
    if (encoding == LogicalLevelIntegerTechnique.RLE) {
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

  public static byte[] encodeFastPfor(List<Integer> values, boolean signed) {
    return EncodingUtils.encodeFastPfor128(
        values.stream().mapToInt(i -> i).toArray(), signed, false);
  }

  public static byte[] encodeVarint(List<Long> values, boolean signed) {
    return EncodingUtils.encodeVarints(values.stream().mapToLong(i -> i).toArray(), signed, false);
  }
}
