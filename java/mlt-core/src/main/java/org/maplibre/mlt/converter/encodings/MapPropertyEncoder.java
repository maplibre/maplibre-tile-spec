package org.maplibre.mlt.converter.encodings;

import jakarta.annotation.Nullable;
import java.io.IOException;
import java.math.BigDecimal;
import java.math.BigInteger;
import java.util.ArrayList;
import java.util.List;
import java.util.Map;
import java.util.SequencedCollection;
import java.util.TreeMap;
import java.util.stream.Stream;
import org.jetbrains.annotations.NotNull;
import org.jspecify.annotations.NonNull;
import org.maplibre.mlt.converter.ConversionConfig;
import org.maplibre.mlt.data.Feature;
import org.maplibre.mlt.data.unsigned.U32;
import org.maplibre.mlt.data.unsigned.U64;
import org.maplibre.mlt.data.unsigned.U8;
import org.maplibre.mlt.metadata.stream.PhysicalLevelTechnique;
import org.maplibre.mlt.metadata.stream.PhysicalStreamType;
import org.maplibre.mlt.metadata.tileset.MltMetadata;

public class MapPropertyEncoder {
  private MapPropertyEncoder() {}

  static ArrayList<byte[]> encodeMapPropertyColumn(
      @NotNull final SequencedCollection<Feature> features,
      final boolean useFSST,
      @NotNull final MltMetadata.Column columnMetadata,
      @NotNull final PhysicalLevelTechnique physicalLevelTechnique,
      @NotNull final ConversionConfig.IntegerEncodingOption encodingOption)
      throws IOException {

    final var uniqueValues = new UniqueMapValues();
    final var columnNames = getMapColumnNames(columnMetadata);

    // Recursively gather all unique keys and values, grouped
    // by type, with one list of values for each encodable type.
    for (var feature : features) {
      for (var columnName : columnNames) {
        feature
            .findProperty(columnName)
            .map(property -> property.getValue(feature.getIndex()))
            .ifPresent(value -> collectUniqueMapValues(value, uniqueValues, columnName));
      }
    }

    // Now that all the values are collected and their final order is established, assign indexes
    // for encoding.  These indexes can be inferred by the decoder and so are not encoded.
    uniqueValues.assignIndexes();

    // Flatten each property into a list of integers.
    final var flattenedMapData =
        combineFlattenedMapData(
            columnNames.stream()
                .map(columnName -> flattenMapValues(features, columnName, uniqueValues))
                .toList());

    // If all entries are null, we can skip writing data streams and just write a zero stream count
    if (flattenedMapData.allNull()) {
      return new ArrayList<>(List.of(new byte[] {0}));
    }

    // If any values are null, write a presence stream
    final var writePresenceStream = flattenedMapData.anyNull();

    // Establish the stream mask so the decoder knows which of the optional streams are present
    final var mask =
        uniqueValues.dictionaryPresenceMask() | (writePresenceStream ? MapMask.PRESENCE : 0);

    final var maxDictionaryStreams =
        11; // one for each encodable type, but strings can write 5 streams
    final var maxNumStreams = maxDictionaryStreams + 3; // length, values, presence
    final var encodedStreams = new ArrayList<byte[]>(maxNumStreams);

    encodedStreams.add(null); // placeholder for stream count
    encodedStreams.add(new byte[] {(byte) mask});

    // Encode the length stream, the only mandatory one
    var numStreams = 1;
    encodedStreams.addAll(
        IntegerEncoder.encodeIntStream(
            flattenedMapData.featureValueCounts(),
            physicalLevelTechnique,
            false,
            PhysicalStreamType.LENGTH,
            null,
            encodingOption));

    // Encode the non-empty unique dictionaries and the flattened/count streams
    if (!uniqueValues.uniqueStringValues().isEmpty()) {
      final var encoded =
          StringEncoder.encode(
              uniqueValues.uniqueStringValues().keySet(), physicalLevelTechnique, useFSST);
      numStreams += encoded.numStreams();
      encodedStreams.add(new byte[] {(byte) encoded.numStreams()});
      encodedStreams.addAll(encoded.encodedData());
    }

    if (!uniqueValues.uniqueInt32Values().isEmpty()) {
      encodedStreams.addAll(
          IntegerEncoder.encodeIntStream(
              uniqueValues.uniqueInt32Values().keySet(),
              physicalLevelTechnique,
              true,
              PhysicalStreamType.DATA,
              null,
              encodingOption));
      numStreams++;
    }

    if (!uniqueValues.uniqueUInt32Values().isEmpty()) {
      encodedStreams.addAll(
          IntegerEncoder.encodeIntStream(
              uniqueValues.uniqueUInt32Values().keySet().stream().mapToInt(U32::intValue),
              physicalLevelTechnique,
              false,
              PhysicalStreamType.DATA,
              null,
              encodingOption));
      numStreams++;
    }

    if (!uniqueValues.uniqueInt64Values().isEmpty()) {
      encodedStreams.addAll(
          IntegerEncoder.encodeLongStream(
              uniqueValues.uniqueInt64Values().keySet(),
              true,
              PhysicalStreamType.DATA,
              null,
              encodingOption));
      numStreams++;
    }

    if (!uniqueValues.uniqueUInt64Values().isEmpty()) {
      encodedStreams.addAll(
          IntegerEncoder.encodeLongStream(
              uniqueValues.uniqueUInt64Values().keySet().stream().mapToLong(U64::longValue),
              false,
              PhysicalStreamType.DATA,
              null,
              encodingOption));
      numStreams++;
    }

    if (!uniqueValues.uniqueFloatValues().isEmpty()) {
      encodedStreams.addAll(
          FloatEncoder.encodeFloatStream(uniqueValues.uniqueFloatValues().keySet()));
      numStreams++;
    }

    if (!uniqueValues.uniqueDoubleValues().isEmpty()) {
      encodedStreams.addAll(
          DoubleEncoder.encodeDoubleStream(uniqueValues.uniqueDoubleValues().keySet()));
      numStreams++;
    }

    // Encode the presence stream, if applicable
    if (writePresenceStream) {
      encodedStreams.addAll(
          BooleanEncoder.encodeBooleanStream(
              flattenedMapData.featurePresentValues(), PhysicalStreamType.PRESENT));
      numStreams++;
    }

    // Encode the values stream, if there are any values
    if (!flattenedMapData.flattenedValues().isEmpty()) {
      encodedStreams.addAll(
          IntegerEncoder.encodeIntStream(
              flattenedMapData.flattenedValues(),
              physicalLevelTechnique,
              false,
              PhysicalStreamType.OFFSET,
              null));
      numStreams++;
    }

    // Fill in the stream count
    encodedStreams.set(0, EncodingUtils.encodeVarint(numStreams, false));

    return encodedStreams;
  }

  public static @NonNull List<String> getMapColumnNames(
      final MltMetadata.@NonNull Column columnMetadata) {
    // Gather column names.  If this is a single column, the name is just the type name.
    // If it's a shared column, each name is the parent name plus the child name.
    final var parentName = columnMetadata.getName();
    return columnMetadata
        .getChildren()
        .filter(children -> !children.isEmpty())
        .map(children -> children.stream().map(col -> parentName + col.name()))
        .orElse(Stream.of(parentName))
        .toList();
  }

  private static FlattenedMapData combineFlattenedMapData(
      @NotNull final List<FlattenedMapData> flattenedMapData) {
    if (flattenedMapData.isEmpty()) {
      return new FlattenedMapData(
          new ArrayList<>(), new ArrayList<>(), new ArrayList<>(), false, true);
    }

    if (flattenedMapData.size() == 1) {
      return flattenedMapData.getFirst();
    }

    final var combinedFlattenedValues = new ArrayList<Integer>();
    final var combinedFeatureValueCounts = new ArrayList<Integer>();
    final var combinedFeaturePresentValues = new ArrayList<Boolean>();
    var anyNull = false;
    var allNull = true;

    for (final var entry : flattenedMapData) {
      combinedFlattenedValues.addAll(entry.flattenedValues());
      combinedFeatureValueCounts.addAll(entry.featureValueCounts());
      combinedFeaturePresentValues.addAll(entry.featurePresentValues());
      anyNull |= entry.anyNull();
      allNull &= entry.allNull();
    }

    return new FlattenedMapData(
        combinedFlattenedValues,
        combinedFeatureValueCounts,
        combinedFeaturePresentValues,
        anyNull,
        allNull);
  }

  /// Walk the values for each feature, building a flattened list of value indexes for all features
  private static FlattenedMapData flattenMapValues(
      @NotNull final SequencedCollection<Feature> features,
      @NotNull final String columnName,
      @NotNull final UniqueMapValues uniqueValues) {
    final var estimatedValuesPerFeature = 10;
    final var flattenedValues = new ArrayList<Integer>(estimatedValuesPerFeature * features.size());
    final var featureValueCounts = new ArrayList<Integer>(features.size());
    final var featurePresentValues = new ArrayList<Boolean>(features.size());
    boolean anyNull = false;
    boolean allNull = true;

    for (final var feature : features) {
      final var startIndex = flattenedValues.size();
      final var value =
          feature.findProperty(columnName).map(property -> property.getValue(feature.getIndex()));
      value.ifPresent(v -> appendMapEntries(v, flattenedValues, uniqueValues, columnName));
      final var featureValueCount = flattenedValues.size() - startIndex;
      if (value.isPresent()) {
        featureValueCounts.add(featureValueCount);
        allNull = false;
      } else {
        anyNull = true;
      }
      featurePresentValues.add(value.isPresent());
    }

    return new FlattenedMapData(
        flattenedValues, featureValueCounts, featurePresentValues, anyNull, allNull);
  }

  private static void appendMapEntries(
      @Nullable final Object value,
      @NotNull final ArrayList<Integer> flattenedValues,
      @NotNull final UniqueMapValues uniqueValues,
      @NotNull final String columnName) {
    if (value instanceof Map<?, ?> mapValue) {
      for (final var entry : mapValue.entrySet()) {
        if (entry.getKey() == null) {
          throw new IllegalArgumentException(
              "Nested map entry has null key in column '" + columnName + "'");
        }

        flattenedValues.add(
            getScalarIndex(
                uniqueValues.uniqueStringValues(), entry.getKey().toString(), columnName));
        appendMapEntryValue(entry.getValue(), flattenedValues, uniqueValues, columnName);
      }
    } else if (value instanceof Iterable<?> iterable) {
      appendListValue(iterable, flattenedValues, uniqueValues, columnName);
    } else if (value != null) {
      // Root-level scalar map values are encoded as a single scalar/control token.
      flattenedValues.add(getScalarIndex(value, uniqueValues, columnName));
    }
  }

  private static void appendMapEntryValue(
      @Nullable Object value,
      @NotNull final ArrayList<Integer> flattenedValues,
      @NotNull final UniqueMapValues uniqueValues,
      @NotNull final String columnName) {
    switch (value) {
      case null -> {
        // Null values are elided
      }
      case Map<?, ?> nestedMap -> {
        appendMapValue(nestedMap, flattenedValues, uniqueValues, columnName);
      }
      case Iterable<?> iterable ->
          appendListValue(iterable, flattenedValues, uniqueValues, columnName);
      default -> flattenedValues.add(getScalarIndex(value, uniqueValues, columnName));
    }
  }

  private static void appendMapValue(
      @NotNull final Map<?, ?> mapValue,
      @NotNull final ArrayList<Integer> flattenedValues,
      @NotNull final UniqueMapValues uniqueValues,
      @NotNull final String columnName) {
    final var startIndex = flattenedValues.size();
    flattenedValues.add(MapControlValue.START_MAP);
    flattenedValues.add(0); // size not known yet, will be updated after payload is added
    appendMapEntries(mapValue, flattenedValues, uniqueValues, columnName);
    flattenedValues.set(startIndex + 1, flattenedValues.size() - startIndex);
  }

  private static void appendListValue(
      @NotNull final Iterable<?> iterable,
      @NotNull final ArrayList<Integer> flattenedValues,
      @NotNull final UniqueMapValues uniqueValues,
      @NotNull final String columnName) {
    final var startIndex = flattenedValues.size();
    flattenedValues.add(MapControlValue.START_LIST);
    flattenedValues.add(0); // size not known yet, will be updated after payload is added
    iterable.forEach(item -> appendListItemValue(item, flattenedValues, uniqueValues, columnName));
    flattenedValues.set(startIndex + 1, flattenedValues.size() - startIndex);
  }

  private static void appendListItemValue(
      @Nullable final Object item,
      @NotNull final ArrayList<Integer> listValueIndexes,
      @NotNull final UniqueMapValues uniqueValues,
      @NotNull final String columnName) {
    switch (item) {
      case null -> {
        // Null entries are elided
      }
      case Map<?, ?> nestedMap ->
          appendMapValue(nestedMap, listValueIndexes, uniqueValues, columnName);
      case Iterable<?> nestedList ->
          appendListValue(nestedList, listValueIndexes, uniqueValues, columnName);
      default -> listValueIndexes.add(getScalarIndex(item, uniqueValues, columnName));
    }
  }

  /// Find the index of a scalar value within the combined set of unique values
  private static int getScalarIndex(
      @NotNull final Object value,
      @NotNull final UniqueMapValues uniqueValues,
      @NotNull final String columnName) {
    return switch (value) {
      case Boolean boolValue -> boolValue ? MapControlValue.TRUE : MapControlValue.FALSE;
      case U8 u8Value ->
          getScalarIndex(uniqueValues.uniqueUInt32Values(), U32.of(u8Value.intValue()), columnName);
      case Integer intValue ->
          getScalarIndex(uniqueValues.uniqueInt32Values(), intValue, columnName);
      case U32 u32Value -> getScalarIndex(uniqueValues.uniqueUInt32Values(), u32Value, columnName);
      case Long longValue ->
          getScalarIndex(uniqueValues.uniqueInt64Values(), longValue, columnName);
      case U64 u64Value -> getScalarIndex(uniqueValues.uniqueUInt64Values(), u64Value, columnName);
      case Float floatValue ->
          getScalarIndex(uniqueValues.uniqueFloatValues(), floatValue, columnName);
      case Double doubleValue ->
          getScalarIndex(uniqueValues.uniqueDoubleValues(), doubleValue, columnName);
      case String stringValue ->
          getScalarIndex(uniqueValues.uniqueStringValues(), stringValue, columnName);
      case BigInteger bigIntValue -> {
        if (bigIntValue.bitLength() < 32) {
          yield getScalarIndex(
              uniqueValues.uniqueInt32Values(), bigIntValue.intValueExact(), columnName);
        } else if (bigIntValue.signum() >= 0 && bigIntValue.bitLength() <= 32) {
          yield getScalarIndex(
              uniqueValues.uniqueUInt32Values(), U32.of(bigIntValue.longValueExact()), columnName);
        } else if (bigIntValue.bitLength() < 64) {
          yield getScalarIndex(
              uniqueValues.uniqueInt64Values(), bigIntValue.longValueExact(), columnName);
        } else if (bigIntValue.signum() >= 0 && bigIntValue.bitLength() <= 64) {
          yield getScalarIndex(uniqueValues.uniqueUInt64Values(), U64.of(bigIntValue), columnName);
        } else {
          throw new IllegalArgumentException(
              "BigInteger value out of uint64 range in column '"
                  + columnName
                  + "': "
                  + bigIntValue);
        }
      }
      case BigDecimal bigDecValue -> {
        final float f = bigDecValue.floatValue();
        if (!Float.isInfinite(f)
            && !Float.isNaN(f)
            && BigDecimal.valueOf(f).compareTo(bigDecValue) == 0) {
          yield getScalarIndex(uniqueValues.uniqueFloatValues(), f, columnName);
        }
        final double d = bigDecValue.doubleValue();
        if (Double.isInfinite(d)
            || Double.isNaN(d)
            || BigDecimal.valueOf(d).compareTo(bigDecValue) == 0) {
          yield getScalarIndex(uniqueValues.uniqueDoubleValues(), d, columnName);
        }
        throw new IllegalArgumentException(
            "BigDecimal not exactly representable as float or double in column '"
                + columnName
                + "': "
                + bigDecValue);
      }
      default ->
          throw new IllegalArgumentException(
              "Unsupported nested map property value type in column '"
                  + columnName
                  + "': "
                  + value.getClass().getName());
    };
  }

  private static <T> int getScalarIndex(
      @NotNull final Map<T, Integer> indexes,
      @NotNull final T value,
      @NotNull final String columnName) {
    final var index = indexes.get(value);
    if (index == null) {
      throw new IllegalArgumentException(
          "Value missing from nested map dictionary in column '" + columnName + "': " + value);
    }
    return index;
  }

  /// Recursively walk a nested map property value, collecting all the unique scalars used as key
  /// and leaf values.
  /// Indexes cannot be established yet, so all are set to zero.
  private static void collectUniqueMapValues(
      @Nullable final Object value,
      @NotNull final UniqueMapValues uniqueValues,
      @NotNull final String columnName) {
    switch (value) {
      case null -> {
        // nulls values are elided
      }
      case Map<?, ?> map -> {
        for (final var entry : map.entrySet()) {
          if (entry.getKey() != null) {
            uniqueValues.uniqueStringValues().putIfAbsent(entry.getKey().toString(), 0);
          }
          collectUniqueMapValues(entry.getValue(), uniqueValues, columnName);
        }
      }
      case Iterable<?> iterable ->
          iterable.forEach(i -> collectUniqueMapValues(i, uniqueValues, columnName));
      case Boolean ignored -> {
        // booleans are encoded as control values, not dictionary entries
      }
      case U8 u8Value ->
          uniqueValues.uniqueUInt32Values().putIfAbsent(U32.of(u8Value.intValue()), 0);
      case Integer intValue -> uniqueValues.uniqueInt32Values().putIfAbsent(intValue, 0);
      case U32 u32Value -> uniqueValues.uniqueUInt32Values().putIfAbsent(u32Value, 0);
      case Long longValue -> uniqueValues.uniqueInt64Values().putIfAbsent(longValue, 0);
      case U64 u64Value -> uniqueValues.uniqueUInt64Values().putIfAbsent(u64Value, 0);
      case Float floatValue -> uniqueValues.uniqueFloatValues().putIfAbsent(floatValue, 0);
      case Double doubleValue -> uniqueValues.uniqueDoubleValues().putIfAbsent(doubleValue, 0);
      case String stringValue -> uniqueValues.uniqueStringValues().putIfAbsent(stringValue, 0);
      case BigInteger bigIntValue -> {
        if (bigIntValue.bitLength() < 32) {
          uniqueValues.uniqueInt32Values().putIfAbsent(bigIntValue.intValueExact(), 0);
        } else if (bigIntValue.signum() >= 0 && bigIntValue.bitLength() <= 32) {
          uniqueValues.uniqueUInt32Values().putIfAbsent(U32.of(bigIntValue.longValueExact()), 0);
        } else if (bigIntValue.bitLength() < 64) {
          uniqueValues.uniqueInt64Values().putIfAbsent(bigIntValue.longValueExact(), 0);
        } else if (bigIntValue.signum() >= 0 && bigIntValue.bitLength() <= 64) {
          uniqueValues.uniqueUInt64Values().putIfAbsent(U64.of(bigIntValue), 0);
        } else {
          throw new IllegalArgumentException(
              "BigInteger value out of uint64 range in column '"
                  + columnName
                  + "': "
                  + bigIntValue);
        }
      }
      case BigDecimal bigDecValue -> {
        final float f = bigDecValue.floatValue();
        if (!Float.isInfinite(f)
            && !Float.isNaN(f)
            && BigDecimal.valueOf(f).compareTo(bigDecValue) == 0) {
          uniqueValues.uniqueFloatValues().putIfAbsent(f, 0);
        } else {
          final double d = bigDecValue.doubleValue();
          if (Double.isInfinite(d)
              || Double.isNaN(d)
              || BigDecimal.valueOf(d).compareTo(bigDecValue) == 0) {
            uniqueValues.uniqueDoubleValues().putIfAbsent(d, 0);
          } else {
            throw new IllegalArgumentException(
                "BigDecimal not exactly representable as float or double in column '"
                    + columnName
                    + "': "
                    + bigDecValue);
          }
        }
      }
      default ->
          throw new IllegalArgumentException(
              "Unsupported nested map property value type in column '"
                  + columnName
                  + "': "
                  + value.getClass().getName());
    }
  }

  /// Special data values indicating structure in the flattened map value stream
  public static final class MapControlValue {
    private MapControlValue() {}

    /// Boolean values are not stored in a dictionary, but encoded directly
    public static final int FALSE = 0;
    public static final int TRUE = 1;
    /// Indicates the value is a nested map rather than a scalar,
    /// followed by nested payload length and payload values.
    public static final int START_MAP = 2;
    /// Indicates the value is a list rather than a scalar,
    /// followed by payload length and payload values.
    public static final int START_LIST = 3;
    /// Number of reserved control values, i.e. the starting index for dictionary values
    public static final int COUNT = 4;
  }

  /// Bitmask values used to indicate which dictionary streams follow
  public static final class MapMask {
    private MapMask() {}

    public static final int STRING = 1;
    public static final int INT32 = 1 << 1;
    public static final int UINT32 = 1 << 2;
    public static final int INT64 = 1 << 3;
    public static final int UINT64 = 1 << 4;
    public static final int FLOAT = 1 << 5;
    public static final int DOUBLE = 1 << 6;
    public static final int PRESENCE = 1 << 7;
  }

  /// Holds the sorted unique values for each encodable type for a map property column and their
  /// corresponding indexes
  // TODO: Better to combine [u]int32/64 values and encode everything as 64-bit if any need it?
  record UniqueMapValues(
      @NotNull TreeMap<String, Integer> uniqueStringValues,
      @NotNull TreeMap<Integer, Integer> uniqueInt32Values,
      @NotNull TreeMap<U32, Integer> uniqueUInt32Values,
      @NotNull TreeMap<Long, Integer> uniqueInt64Values,
      @NotNull TreeMap<U64, Integer> uniqueUInt64Values,
      @NotNull TreeMap<Float, Integer> uniqueFloatValues,
      @NotNull TreeMap<Double, Integer> uniqueDoubleValues) {

    UniqueMapValues() {
      this(
          new TreeMap<>(),
          new TreeMap<>(),
          new TreeMap<>(),
          new TreeMap<>(),
          new TreeMap<>(),
          new TreeMap<>(),
          new TreeMap<>());
    }

    /// Once all values are collected, assign an index to each.
    void assignIndexes() {
      var start = MapControlValue.COUNT;
      start = assignIndexes(uniqueStringValues, start);
      start = assignIndexes(uniqueInt32Values, start);
      start = assignIndexes(uniqueUInt32Values, start);
      start = assignIndexes(uniqueInt64Values, start);
      start = assignIndexes(uniqueUInt64Values, start);
      start = assignIndexes(uniqueFloatValues, start);
      assignIndexes(uniqueDoubleValues, start);
    }

    private static <T> int assignIndexes(@NotNull TreeMap<T, Integer> values, int startIndex) {
      for (final var entry : values.entrySet()) {
        entry.setValue(startIndex++);
      }
      return startIndex;
    }

    /// Calculate the mask indicating which value streams are written
    int dictionaryPresenceMask() {
      return ((!uniqueStringValues.isEmpty() ? MapMask.STRING : 0)
          | (!uniqueInt32Values.isEmpty() ? MapMask.INT32 : 0)
          | (!uniqueUInt32Values.isEmpty() ? MapMask.UINT32 : 0)
          | (!uniqueInt64Values.isEmpty() ? MapMask.INT64 : 0)
          | (!uniqueUInt64Values.isEmpty() ? MapMask.UINT64 : 0)
          | (!uniqueFloatValues.isEmpty() ? MapMask.FLOAT : 0)
          | (!uniqueDoubleValues.isEmpty() ? MapMask.DOUBLE : 0));
    }
  }

  private record FlattenedMapData(
      @NotNull ArrayList<Integer> flattenedValues,
      @NotNull ArrayList<Integer> featureValueCounts,
      @NotNull ArrayList<Boolean> featurePresentValues,
      boolean anyNull,
      boolean allNull) {}
}
