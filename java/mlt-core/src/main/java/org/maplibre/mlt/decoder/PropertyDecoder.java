package org.maplibre.mlt.decoder;

import jakarta.annotation.Nullable;
import java.io.IOException;
import java.math.BigInteger;
import java.util.ArrayList;
import java.util.BitSet;
import java.util.LinkedHashMap;
import java.util.List;
import me.lemire.integercompression.IntWrapper;
import org.maplibre.mlt.converter.encodings.PropertyEncoder;
import org.maplibre.mlt.data.unsigned.U32;
import org.maplibre.mlt.data.unsigned.U64;
import org.maplibre.mlt.metadata.stream.PhysicalStreamType;
import org.maplibre.mlt.metadata.stream.StreamMetadataDecoder;
import org.maplibre.mlt.metadata.tileset.MltMetadata;

public class PropertyDecoder {
  private PropertyDecoder() {}

  /// Use present bits to reconstitute the original list with null values, if appropriate
  private static <T> List<T> unpack(
      List<T> dataStream, @Nullable BitSet presentBits, int numPresentBits) {
    if (presentBits == null) {
      return dataStream;
    }
    final ArrayList<T> outValues = new ArrayList<>(presentBits.size());
    var counter = 0;
    for (var i = 0; i < numPresentBits; i++) {
      outValues.add(presentBits.get(i) ? dataStream.get(counter++) : null);
    }
    return outValues;
  }

  ///  Special case for boolean columns because `BitSet` is not compatible with `List`
  private static List<Boolean> unpack(
      BitSet dataStream, int dataStreamSize, @Nullable BitSet presentBits, int numPresentBits) {
    final var numValues = (presentBits != null) ? numPresentBits : dataStreamSize;
    final ArrayList<Boolean> booleanValues = new ArrayList<>(numValues);
    var counter = 0;
    for (var i = 0; i < numValues; i++) {
      booleanValues.add(
          (presentBits == null || presentBits.get(i)) ? dataStream.get(counter++) : null);
    }
    return booleanValues;
  }

  private static Object decodeScalarPropertyColumn(
      byte[] data,
      IntWrapper offset,
      MltMetadata.ScalarField scalarType,
      boolean nullable,
      int numStreams)
      throws IOException {
    final BitSet presentStream;
    final int presentStreamSize;
    if (nullable) {
      final var presentStreamMetadata = StreamMetadataDecoder.decode(data, offset);
      presentStream =
          DecodingUtils.decodeBooleanRle(
              data, presentStreamMetadata.numValues(), presentStreamMetadata.byteLength(), offset);
      presentStreamSize = presentStreamMetadata.numValues();
      numStreams -= 1;
    } else {
      presentStream = null;
      presentStreamSize = 0;
    }

    return switch (scalarType.physicalType()) {
      case BOOLEAN -> {
        final var dataStreamMetadata = StreamMetadataDecoder.decode(data, offset);
        final var dataStream =
            DecodingUtils.decodeBooleanRle(
                data, dataStreamMetadata.numValues(), dataStreamMetadata.byteLength(), offset);
        yield unpack(dataStream, dataStreamMetadata.numValues(), presentStream, presentStreamSize);
      }
      case UINT_32, INT_32 -> {
        final var dataStreamMetadata = StreamMetadataDecoder.decode(data, offset);
        final var signed = (scalarType.physicalType() == MltMetadata.ScalarType.INT_32);
        final var dataStream =
            IntegerDecoder.decodeIntStream(data, offset, dataStreamMetadata, signed);

        // otherwise, we have u32.MAX -> -1
        final var values =
            signed
                ? dataStream
                : dataStream.stream()
                    .map(i -> i == null ? null : Integer.toUnsignedLong(i))
                    .toList();

        yield unpack(values, presentStream, presentStreamSize);
      }
      case UINT_64, INT_64 -> {
        final var dataStreamMetadata = StreamMetadataDecoder.decode(data, offset);
        final var signed = (scalarType.physicalType() == MltMetadata.ScalarType.INT_64);
        final var dataStream =
            IntegerDecoder.decodeLongStream(data, offset, dataStreamMetadata, signed);

        // otherwise, we have u64.MAX -> -1
        final var values =
            signed
                ? dataStream
                : dataStream.stream().map(i -> i == null ? null : toU64(i)).toList();

        yield unpack(values, presentStream, presentStreamSize);
      }
      case FLOAT -> {
        final var dataStreamMetadata = StreamMetadataDecoder.decode(data, offset);
        final var dataStream = FloatDecoder.decodeFloatStream(data, offset, dataStreamMetadata);
        yield unpack(dataStream, presentStream, presentStreamSize);
      }
      case DOUBLE -> {
        final var dataStreamMetadata = StreamMetadataDecoder.decode(data, offset);
        final var dataStream = DoubleDecoder.decodeDoubleStream(data, offset, dataStreamMetadata);
        yield unpack(dataStream, presentStream, presentStreamSize);
      }
      case STRING -> {
        final var strValues =
            StringDecoder.decode(data, offset, numStreams, presentStream, presentStreamSize);
        yield strValues.strings();
      }
      case UINT_8, UNRECOGNIZED, INT_8 ->
          throw new IllegalArgumentException(
              "The specified data type for the field is currently not supported: " + scalarType);
    };
  }

  private static Object decodeMapPropertyColumn(
      byte[] data, IntWrapper offset, MltMetadata.Column column, int numStreams)
      throws IOException {
    if (!column.is(MltMetadata.ComplexType.MAP)) {
      throw new IllegalArgumentException("Expected MAP column but found: " + column);
    }
    if (numStreams == 0) {
      return new ArrayList<>();
    }

    final var dictionaryMask = data[offset.get()] & 0xFF;
    offset.increment();

    // Decode the mandatory lengths stream
    final var lengthStreamMetadata = StreamMetadataDecoder.decode(data, offset);
    final var nonEmptyFeatureValueCounts =
        IntegerDecoder.decodeIntStream(data, offset, lengthStreamMetadata, false);
    numStreams--;

    // Decode the optional dictionary streams, based on the mask
    List<String> stringValues = List.of();
    List<Integer> int32Values = List.of();
    List<U32> uint32Values = List.of();
    List<Long> int64Values = List.of();
    List<U64> uint64Values = List.of();
    List<Float> floatValues = List.of();
    List<Double> doubleValues = List.of();

    if ((dictionaryMask & PropertyEncoder.MASK_STRING) != 0) {
      final var stringStreamCount = data[offset.get()];
      offset.increment();

      final var decodedStrings = StringDecoder.decode(data, offset, stringStreamCount, null, 0);
      stringValues = decodedStrings.strings();
      numStreams -= stringStreamCount;
    }
    if ((dictionaryMask & PropertyEncoder.MASK_INT32) != 0) {
      final var streamMetadata = StreamMetadataDecoder.decode(data, offset);
      int32Values = IntegerDecoder.decodeIntStream(data, offset, streamMetadata, true);
      numStreams--;
    }
    if ((dictionaryMask & PropertyEncoder.MASK_UINT32) != 0) {
      final var streamMetadata = StreamMetadataDecoder.decode(data, offset);
      uint32Values =
          IntegerDecoder.decodeIntStream(data, offset, streamMetadata, false).stream()
              .map(Integer::toUnsignedLong)
              .map(U32::of)
              .toList();
      numStreams--;
    }
    if ((dictionaryMask & PropertyEncoder.MASK_INT64) != 0) {
      final var streamMetadata = StreamMetadataDecoder.decode(data, offset);
      int64Values = IntegerDecoder.decodeLongStream(data, offset, streamMetadata, true);
      numStreams--;
    }
    if ((dictionaryMask & PropertyEncoder.MASK_UINT64) != 0) {
      final var streamMetadata = StreamMetadataDecoder.decode(data, offset);
      uint64Values =
          IntegerDecoder.decodeLongStream(data, offset, streamMetadata, false).stream()
              .map(PropertyDecoder::toU64)
              .toList();
      numStreams--;
    }
    if ((dictionaryMask & PropertyEncoder.MASK_FLOAT) != 0) {
      final var streamMetadata = StreamMetadataDecoder.decode(data, offset);
      floatValues = FloatDecoder.decodeFloatStream(data, offset, streamMetadata);
      numStreams--;
    }
    if ((dictionaryMask & PropertyEncoder.MASK_DOUBLE) != 0) {
      final var streamMetadata = StreamMetadataDecoder.decode(data, offset);
      doubleValues = DoubleDecoder.decodeDoubleStream(data, offset, streamMetadata);
      numStreams--;
    }

    BitSet presentStream = null;
    int presentCount = 0;
    if ((dictionaryMask & PropertyEncoder.MASK_PRESENCE) != 0) {
      final var metadata = StreamMetadataDecoder.decode(data, offset);
      if (metadata.physicalStreamType() != PhysicalStreamType.PRESENT) {
        throw new IllegalArgumentException(
            "Expected PRESENT stream for map column but found: " + metadata.physicalStreamType());
      }
      presentCount = metadata.numValues();
      presentStream =
          DecodingUtils.decodeBooleanRle(data, presentCount, metadata.byteLength(), offset);
      numStreams--;
    }

    final List<Integer> flattenedValues;
    if (numStreams > 0) {
      final var dataStreamMetadata = StreamMetadataDecoder.decode(data, offset);
      flattenedValues = IntegerDecoder.decodeIntStream(data, offset, dataStreamMetadata, false);
      numStreams--;
    } else {
      flattenedValues = List.of();
    }

    if (numStreams != 0) {
      throw new IllegalArgumentException(
          "Unexpected number of remaining streams while decoding map column: " + numStreams);
    }

    final var dictionaries =
        new MapDictionaries(
            stringValues,
            int32Values,
            uint32Values,
            int64Values,
            uint64Values,
            floatValues,
            doubleValues);

    final int featureCount =
        (presentStream != null) ? presentCount : nonEmptyFeatureValueCounts.size();
    final var decodedMaps = new ArrayList<Object>(featureCount);
    var countIndex = 0;
    var flattenedIndex = 0;
    for (var featureIndex = 0; featureIndex < featureCount; featureIndex++) {
      final var present = (presentStream == null) || presentStream.get(featureIndex);
      if (!present) {
        decodedMaps.add(null);
        continue;
      }

      if (countIndex >= nonEmptyFeatureValueCounts.size()) {
        throw new IllegalArgumentException(
            "Map count stream underflow while decoding feature values");
      }

      final var featureValueCount = nonEmptyFeatureValueCounts.get(countIndex++);
      final var endIndex = flattenedIndex + featureValueCount;
      if (endIndex > flattenedValues.size()) {
        throw new IllegalArgumentException(
            "Map value stream underflow while decoding feature payload");
      }

      // Check if this is a list value (for properties that are lists of maps)
      if (flattenedIndex < endIndex && flattenedValues.get(flattenedIndex) == PropertyEncoder.MapControlValue.START_LIST.value) {
        // Decode as a list value instead of map entries
        final var decodedValue = decodeValue(flattenedValues, flattenedIndex, endIndex, dictionaries);
        decodedMaps.add(decodedValue.value());
        flattenedIndex = decodedValue.nextIndex();
      } else {
        // Decode as map entries
        final var decodedMap =
            decodeMapEntries(flattenedValues, flattenedIndex, endIndex, dictionaries);
        decodedMaps.add(decodedMap.value());
        flattenedIndex = decodedMap.nextIndex();
      }
    }

    if (countIndex != nonEmptyFeatureValueCounts.size()) {
      throw new IllegalArgumentException("Unused map feature counts remain after decode");
    }
    if (flattenedIndex != flattenedValues.size()) {
      throw new IllegalArgumentException("Unused flattened map values remain after decode");
    }

    return decodedMaps;
  }

  private static U64 toU64(Long value) {
    return (value >= 0)
        ? U64.of(BigInteger.valueOf(value))
        : U64.of(BigInteger.valueOf(value).add(BigInteger.ONE.shiftLeft(64)));
  }

  private static DecodedMap decodeMapEntries(
      List<Integer> flattenedValues, int startIndex, int endIndex, MapDictionaries dictionaries) {
    final var result = new LinkedHashMap<String, Object>();
    var index = startIndex;
    while (index < endIndex) {
      final var key = decodeMapKey(flattenedValues.get(index++), dictionaries);
      final var valueResult = decodeValue(flattenedValues, index, endIndex, dictionaries);
      result.put(key, valueResult.value());
      index = valueResult.nextIndex();
    }

    if (index != endIndex) {
      throw new IllegalArgumentException("Map payload did not end on a value boundary");
    }
    return new DecodedMap(result, index);
  }

  private static DecodedValue decodeValue(
      List<Integer> flattenedValues, int startIndex, int endIndex, MapDictionaries dictionaries) {
    if (startIndex >= endIndex) {
      throw new IllegalArgumentException("Unexpected end of map value stream");
    }

    final var token = flattenedValues.get(startIndex);
    if (token == PropertyEncoder.MapControlValue.NULL.value) {
      return new DecodedValue(null, startIndex + 1);
    }
    if (token == PropertyEncoder.MapControlValue.FALSE.value) {
      return new DecodedValue(false, startIndex + 1);
    }
    if (token == PropertyEncoder.MapControlValue.TRUE.value) {
      return new DecodedValue(true, startIndex + 1);
    }

    if (token == PropertyEncoder.MapControlValue.START_MAP.value
        || token == PropertyEncoder.MapControlValue.START_LIST.value) {
      if (startIndex + 1 >= endIndex) {
        throw new IllegalArgumentException("Missing length for nested map/list payload");
      }

      final var encodedLength = flattenedValues.get(startIndex + 1);
      if (encodedLength < 2) {
        throw new IllegalArgumentException("Invalid nested payload length: " + encodedLength);
      }
      final var valueEndIndex = startIndex + encodedLength;
      if (valueEndIndex > endIndex) {
        throw new IllegalArgumentException("Nested payload exceeds containing payload bounds");
      }

      final var payloadStart = startIndex + 2;
      if (token == PropertyEncoder.MapControlValue.START_MAP.value) {
        final var nestedMap =
            decodeMapEntries(flattenedValues, payloadStart, valueEndIndex, dictionaries);
        return new DecodedValue(nestedMap.value(), valueEndIndex);
      }

      final var listValues = new ArrayList<Object>();
      var index = payloadStart;
      while (index < valueEndIndex) {
        final var nestedValue = decodeValue(flattenedValues, index, valueEndIndex, dictionaries);
        listValues.add(nestedValue.value());
        index = nestedValue.nextIndex();
      }
      if (index != valueEndIndex) {
        throw new IllegalArgumentException("List payload did not end on a value boundary");
      }
      return new DecodedValue(listValues, valueEndIndex);
    }

    return new DecodedValue(decodeScalarByIndex(token, dictionaries), startIndex + 1);
  }

  private static String decodeMapKey(int dictionaryIndex, MapDictionaries dictionaries) {
    final var value = decodeScalarByIndex(dictionaryIndex, dictionaries);
    if (value instanceof String s) {
      return s;
    }
    throw new IllegalArgumentException(
        "Map key dictionary index does not resolve to a string: " + dictionaryIndex);
  }

  private static Object decodeScalarByIndex(int dictionaryIndex, MapDictionaries dictionaries) {
    final var scalarBase = PropertyEncoder.MapControlValue.COUNT.value;
    if (dictionaryIndex < scalarBase) {
      throw new IllegalArgumentException("Invalid scalar dictionary index: " + dictionaryIndex);
    }

    var index = dictionaryIndex - scalarBase;

    if (index < dictionaries.strings().size()) {
      return dictionaries.strings().get(index);
    }
    index -= dictionaries.strings().size();

    if (index < dictionaries.int32s().size()) {
      return dictionaries.int32s().get(index);
    }
    index -= dictionaries.int32s().size();

    if (index < dictionaries.uint32s().size()) {
      return dictionaries.uint32s().get(index);
    }
    index -= dictionaries.uint32s().size();

    if (index < dictionaries.int64s().size()) {
      return dictionaries.int64s().get(index);
    }
    index -= dictionaries.int64s().size();

    if (index < dictionaries.uint64s().size()) {
      return dictionaries.uint64s().get(index);
    }
    index -= dictionaries.uint64s().size();

    if (index < dictionaries.floats().size()) {
      return dictionaries.floats().get(index);
    }
    index -= dictionaries.floats().size();

    if (index < dictionaries.doubles().size()) {
      return dictionaries.doubles().get(index);
    }

    throw new IllegalArgumentException("Scalar dictionary index out of range: " + dictionaryIndex);
  }

  private record MapDictionaries(
      List<String> strings,
      List<Integer> int32s,
      List<U32> uint32s,
      List<Long> int64s,
      List<U64> uint64s,
      List<Float> floats,
      List<Double> doubles) {}

  private record DecodedMap(LinkedHashMap<String, Object> value, int nextIndex) {}

  private record DecodedValue(Object value, int nextIndex) {}

  public static Object decodePropertyColumn(
      byte[] data, IntWrapper offset, MltMetadata.Column column, int numStreams)
      throws IOException {
    if (column.isScalar()) {
      return decodeScalarPropertyColumn(
          data, offset, column.field().type().scalarType(), column.isNullable(), numStreams);
    }

    /* Handle struct which currently only supports strings as nested fields for supporting shared dictionary encoding */
    if (column.is(MltMetadata.ComplexType.STRUCT)) {
      if (numStreams > 1) {
        return StringDecoder.decodeSharedDictionary(data, offset, column).getRight();
      }
    } else if (column.is(MltMetadata.ComplexType.MAP)) {
      return PropertyDecoder.decodeMapPropertyColumn(data, offset, column, numStreams);
    }

    // var presentStreamMetadata = StreamMetadata.decode(data, offset);
    // var presentStream = DecodingUtils.decodeBooleanRle(data, presentStreamMetadata.numValues(),
    // presentStreamMetadata.byteLength(), offset);
    // TODO: process present stream
    // var values = StringDecoder.decodeSharedDictionary(data, offset, fieldMetadata);
    throw new IllegalArgumentException("Present stream currently not supported for Structs.");
  }
}
