package org.maplibre.mlt.decoder;

import static org.maplibre.mlt.converter.encodings.PropertyEncoder.MapControlValue;

import jakarta.annotation.Nullable;
import java.io.IOException;
import java.math.BigInteger;
import java.util.ArrayList;
import java.util.BitSet;
import java.util.HashMap;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;
import java.util.Optional;
import java.util.SequencedCollection;
import me.lemire.integercompression.IntWrapper;
import org.jetbrains.annotations.NotNull;
import org.maplibre.mlt.converter.encodings.PropertyEncoder;
import org.maplibre.mlt.data.unsigned.U32;
import org.maplibre.mlt.data.unsigned.U64;
import org.maplibre.mlt.metadata.stream.PhysicalStreamType;
import org.maplibre.mlt.metadata.stream.StreamMetadataDecoder;
import org.maplibre.mlt.metadata.tileset.MltMetadata;

public class PropertyDecoder {
  private PropertyDecoder() {}

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
      final byte[] data,
      @NotNull final IntWrapper offset,
      @NotNull final MltMetadata.ScalarField scalarType,
      final boolean nullable,
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
      case null -> throw new IllegalArgumentException("Invalid scalar type metadata for column");
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

  private static @NotNull Object decodeMapPropertyColumn(
      final byte[] data,
      @NotNull final IntWrapper offset,
      @NotNull final MltMetadata.Column column,
      int numStreams)
      throws IOException {
    if (!column.is(MltMetadata.ComplexType.MAP)) {
      throw new IllegalArgumentException("Expected MAP column but found: " + column);
    }
    if (numStreams == 0) {
      return new ArrayList<>();
    }

    // Get the mask indicating which optional dictionary streams are present
    final var dictionaryMask = data[offset.get()] & 0xFF;
    offset.increment();

    // Decode the mandatory lengths stream
    final var lengthStreamMetadata = StreamMetadataDecoder.decode(data, offset);
    final var lengthStream =
        IntegerDecoder.decodeIntStream(data, offset, lengthStreamMetadata, false);
    numStreams--;

    // Decode the optional dictionary streams, based on the mask
    final SequencedCollection<String> stringValues;
    final SequencedCollection<Integer> int32Values;
    final SequencedCollection<U32> uint32Values;
    final SequencedCollection<Long> int64Values;
    final SequencedCollection<U64> uint64Values;
    final SequencedCollection<Float> floatValues;
    final SequencedCollection<Double> doubleValues;

    if ((dictionaryMask & PropertyEncoder.MapMask.STRING) != 0) {
      final var stringStreamCount = data[offset.get()];
      offset.increment();

      final var decodedStrings = StringDecoder.decode(data, offset, stringStreamCount, null, 0);
      stringValues = decodedStrings.strings();
      numStreams -= stringStreamCount;
    } else {
      stringValues = List.of();
    }

    if ((dictionaryMask & PropertyEncoder.MapMask.INT32) != 0) {
      final var streamMetadata = StreamMetadataDecoder.decode(data, offset);
      int32Values = IntegerDecoder.decodeIntStream(data, offset, streamMetadata, true);
      numStreams--;
    } else {
      int32Values = List.of();
    }

    if ((dictionaryMask & PropertyEncoder.MapMask.UINT32) != 0) {
      final var streamMetadata = StreamMetadataDecoder.decode(data, offset);
      uint32Values =
          IntegerDecoder.decodeIntStream(data, offset, streamMetadata, false).stream()
              .map(Integer::toUnsignedLong)
              .map(U32::of)
              .toList();
      numStreams--;
    } else {
      uint32Values = List.of();
    }

    if ((dictionaryMask & PropertyEncoder.MapMask.INT64) != 0) {
      final var streamMetadata = StreamMetadataDecoder.decode(data, offset);
      int64Values = IntegerDecoder.decodeLongStream(data, offset, streamMetadata, true);
      numStreams--;
    } else {
      int64Values = List.of();
    }

    if ((dictionaryMask & PropertyEncoder.MapMask.UINT64) != 0) {
      final var streamMetadata = StreamMetadataDecoder.decode(data, offset);
      uint64Values =
          IntegerDecoder.decodeLongStream(data, offset, streamMetadata, false).stream()
              .map(PropertyDecoder::toU64)
              .toList();
      numStreams--;
    } else {
      uint64Values = List.of();
    }

    if ((dictionaryMask & PropertyEncoder.MapMask.FLOAT) != 0) {
      final var streamMetadata = StreamMetadataDecoder.decode(data, offset);
      floatValues = FloatDecoder.decodeFloatStream(data, offset, streamMetadata);
      numStreams--;
    } else {
      floatValues = List.of();
    }

    if ((dictionaryMask & PropertyEncoder.MapMask.DOUBLE) != 0) {
      final var streamMetadata = StreamMetadataDecoder.decode(data, offset);
      doubleValues = DoubleDecoder.decodeDoubleStream(data, offset, streamMetadata);
      numStreams--;
    } else {
      doubleValues = List.of();
    }

    BitSet presentStream = null;
    int presentCount = 0;
    if ((dictionaryMask & PropertyEncoder.MapMask.PRESENCE) != 0) {
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

    final List<Integer> mergedFlattenedValues;
    if (numStreams > 0) {
      final var dataStreamMetadata = StreamMetadataDecoder.decode(data, offset);
      mergedFlattenedValues =
          IntegerDecoder.decodeIntStream(data, offset, dataStreamMetadata, false);
      numStreams--;
    } else {
      mergedFlattenedValues = List.of();
    }

    if (numStreams != 0) {
      throw new IllegalArgumentException(
          "Unexpected number of remaining streams while decoding map column: " + numStreams);
    }

    final var dictionaries =
        new MapValueDictionary(
            stringValues,
            int32Values,
            uint32Values,
            int64Values,
            uint64Values,
            floatValues,
            doubleValues);

    final var columnNames = PropertyEncoder.getMapColumnNames(column);
    final var featureCount =
        ((presentStream != null) ? presentCount : lengthStream.size()) / columnNames.size();

    final var decodedColumns = new HashMap<String, Object>(columnNames.size());
    var countsCursor = 0;
    var valuesCursor = 0;
    for (var childIndex = 0; childIndex < columnNames.size(); childIndex++) {
      final var columnName = columnNames.get(childIndex);
      final Optional<BitSet> childFeaturePresentValues;
      var childPresentCount = 0;
      if (presentStream != null) {
        final var childBits = new BitSet(featureCount);
        final var childPresentOffset = childIndex * featureCount;
        for (var featureIndex = 0; featureIndex < featureCount; featureIndex++) {
          final var present = presentStream.get(childPresentOffset + featureIndex);
          childBits.set(featureIndex, present);
          if (present) {
            childPresentCount++;
          }
        }
        childFeaturePresentValues = Optional.of(childBits);
      } else {
        childFeaturePresentValues = Optional.empty();
        childPresentCount = featureCount;
      }

      final var childCountsEnd = countsCursor + childPresentCount;
      if (childCountsEnd > lengthStream.size()) {
        throw new IllegalArgumentException(
            "Merged map counts underflow while decoding child streams");
      }

      final var decodedProperties = new ArrayList<>(featureCount);
      decodedColumns.put(columnName, decodedProperties);

      var flattenedIndex = valuesCursor;
      var countCursor = countsCursor;
      for (var featureIndex = 0; featureIndex < featureCount; featureIndex++) {
        final var fi = featureIndex;
        final var present = childFeaturePresentValues.map(bs -> bs.get(fi)).orElse(true);
        if (!present) {
          decodedProperties.add(null);
          continue;
        }

        if (countCursor >= childCountsEnd) {
          throw new IllegalArgumentException(
              "Map count stream underflow while decoding feature values");
        }

        final var featureValueCount = lengthStream.get(countCursor++);
        final var endIndex = flattenedIndex + featureValueCount;
        if (endIndex > mergedFlattenedValues.size()) {
          throw new IllegalArgumentException(
              "Map value stream underflow while decoding feature payload");
        }

        if (featureValueCount == 1) {
          // Special case: root-level scalar value encoded as a single scalar/control token.
          final var decodedValue =
              decodeValue(mergedFlattenedValues, flattenedIndex, endIndex, dictionaries);
          if (decodedValue.nextIndex() != endIndex) {
            throw new IllegalArgumentException(
                "Root scalar payload did not consume exactly one value");
          }
          decodedProperties.add(decodedValue.value());
          flattenedIndex = decodedValue.nextIndex();
        } else if (flattenedIndex < endIndex
            && mergedFlattenedValues.get(flattenedIndex) == MapControlValue.START_LIST) {
          // Decode as a list
          final var decodedValue =
              decodeValue(mergedFlattenedValues, flattenedIndex, endIndex, dictionaries);
          decodedProperties.add(decodedValue.value());
          flattenedIndex = decodedValue.nextIndex();
        } else {
          // Decode as map entries
          final var decodedMap =
              decodeMapEntries(mergedFlattenedValues, flattenedIndex, endIndex, dictionaries);
          decodedProperties.add(decodedMap.value());
          flattenedIndex = decodedMap.nextIndex();
        }
      }
      if (countCursor != childCountsEnd) {
        throw new IllegalArgumentException("Unused map feature counts remain after decode");
      }
      final var childValueCount =
          lengthStream.subList(countsCursor, childCountsEnd).stream()
              .mapToInt(Integer::intValue)
              .sum();
      final var childValuesEnd = valuesCursor + childValueCount;
      if (flattenedIndex != childValuesEnd) {
        throw new IllegalArgumentException("Unused flattened map values remain after decode");
      }
      valuesCursor = childValuesEnd;
      countsCursor = childCountsEnd;
    }

    return decodedColumns;
  }

  private static U64 toU64(final long value) {
    return (value >= 0)
        ? U64.of(BigInteger.valueOf(value))
        : U64.of(BigInteger.valueOf(value).add(BigInteger.ONE.shiftLeft(64)));
  }

  private static DecodedMap decodeMapEntries(
      @NotNull final List<Integer> flattenedValues,
      final int startIndex,
      final int endIndex,
      @NotNull final MapValueDictionary dictionaries) {
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
      @NotNull final List<Integer> flattenedValues,
      final int startIndex,
      final int endIndex,
      @NotNull final PropertyDecoder.MapValueDictionary dictionaries) {
    if (startIndex >= endIndex) {
      throw new IllegalArgumentException("Unexpected end of map value stream");
    }

    final var token = flattenedValues.get(startIndex);
    return switch (token) {
      case MapControlValue.FALSE -> new DecodedValue(false, startIndex + 1);
      case MapControlValue.TRUE -> new DecodedValue(true, startIndex + 1);
      case MapControlValue.START_MAP, MapControlValue.START_LIST -> {
        final var valueEndIndex = getValueEndIndex(flattenedValues, startIndex, endIndex);
        final var payloadStart = startIndex + 2;

        if (token == MapControlValue.START_MAP) {
          final var nestedMap =
              decodeMapEntries(flattenedValues, payloadStart, valueEndIndex, dictionaries);
          if (nestedMap.nextIndex() != valueEndIndex) {
            throw new IllegalArgumentException(
                "Nested map payload did not end on a value boundary");
          }
          yield new DecodedValue(nestedMap.value(), valueEndIndex);
        }

        final var listValues = new ArrayList<>(valueEndIndex - payloadStart);
        var index = payloadStart;
        while (index < valueEndIndex) {
          final var nestedValue = decodeValue(flattenedValues, index, valueEndIndex, dictionaries);
          listValues.add(nestedValue.value());
          index = nestedValue.nextIndex();
        }
        if (index != valueEndIndex) {
          throw new IllegalArgumentException("List payload did not end on a value boundary");
        }
        yield new DecodedValue(listValues, valueEndIndex);
      }
      default -> new DecodedValue(decodeScalarByIndex(token, dictionaries), startIndex + 1);
    };
  }

  private static int getValueEndIndex(List<Integer> flattenedValues, int startIndex, int endIndex) {
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
    return valueEndIndex;
  }

  private static String decodeMapKey(
      final int dictionaryIndex, @NotNull final MapValueDictionary dictionaries) {
    final var value = decodeScalarByIndex(dictionaryIndex, dictionaries);
    if (value instanceof String s) {
      return s;
    }
    throw new IllegalArgumentException(
        "Map key dictionary index does not resolve to a string: " + dictionaryIndex);
  }

  private static Object decodeScalarByIndex(
      final int dictionaryIndex, @NotNull final MapValueDictionary dictionaries) {
    final var scalarBase = MapControlValue.COUNT;
    if (dictionaryIndex < scalarBase) {
      throw new IllegalArgumentException("Invalid scalar dictionary index: " + dictionaryIndex);
    }

    final var value = dictionaries.valueByIndex(dictionaryIndex);
    if (value != null) {
      return value;
    }

    throw new IllegalArgumentException("Scalar dictionary index out of range: " + dictionaryIndex);
  }

  private static final class MapValueDictionary {
    private final List<Object> values;

    private MapValueDictionary(
        @NotNull final SequencedCollection<String> strings,
        @NotNull final SequencedCollection<Integer> int32s,
        @NotNull final SequencedCollection<U32> uint32s,
        @NotNull final SequencedCollection<Long> int64s,
        @NotNull final SequencedCollection<U64> uint64s,
        @NotNull final SequencedCollection<Float> floats,
        @NotNull final SequencedCollection<Double> doubles) {
      final var totalValues =
          strings.size()
              + int32s.size()
              + uint32s.size()
              + int64s.size()
              + uint64s.size()
              + floats.size()
              + doubles.size();
      values = new ArrayList<>(totalValues);
      values.addAll(strings);
      values.addAll(int32s);
      values.addAll(uint32s);
      values.addAll(int64s);
      values.addAll(uint64s);
      values.addAll(floats);
      values.addAll(doubles);
    }

    @Nullable
    private Object valueByIndex(final int dictionaryIndex) {
      final var offset = dictionaryIndex - MapControlValue.COUNT;
      return (offset >= 0 && offset < values.size()) ? values.get(offset) : null;
    }
  }

  private record DecodedMap(@NotNull Map<String, Object> value, int nextIndex) {}

  private record DecodedValue(@Nullable Object value, int nextIndex) {}
}
