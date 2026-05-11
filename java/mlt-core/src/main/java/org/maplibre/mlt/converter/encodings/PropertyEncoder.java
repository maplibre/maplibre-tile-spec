package org.maplibre.mlt.converter.encodings;

import jakarta.annotation.Nullable;
import java.io.IOException;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.BitSet;
import java.util.Iterator;
import java.util.List;
import java.util.Map;
import java.util.Objects;
import java.util.SequencedCollection;
import java.util.TreeMap;
import java.util.stream.Collectors;
import org.jetbrains.annotations.NotNull;
import org.maplibre.mlt.converter.CollectionUtils;
import org.maplibre.mlt.converter.ColumnMapping;
import org.maplibre.mlt.converter.ConversionConfig;
import org.maplibre.mlt.converter.encodings.PropertyEncoder.MapControlValue;
import org.maplibre.mlt.data.Feature;
import org.maplibre.mlt.data.unsigned.U32;
import org.maplibre.mlt.data.unsigned.U64;
import org.maplibre.mlt.data.unsigned.U8;
import org.maplibre.mlt.metadata.stream.LogicalLevelTechnique;
import org.maplibre.mlt.metadata.stream.PhysicalLevelTechnique;
import org.maplibre.mlt.metadata.stream.PhysicalStreamType;
import org.maplibre.mlt.metadata.stream.StreamMetadata;
import org.maplibre.mlt.metadata.tileset.MltMetadata;
import org.maplibre.mlt.util.ByteArrayUtil;
import org.maplibre.mlt.util.StreamUtil;

public class PropertyEncoder {

  public static ArrayList<byte[]> encodePropertyColumns(
      SequencedCollection<MltMetadata.Column> propertyColumns,
      SequencedCollection<Feature> features,
      boolean useFastPFOR,
      boolean useFSST,
      boolean coercePropertyValues,
      @Nullable SequencedCollection<ColumnMapping> columnMappings,
      @NotNull ConversionConfig.IntegerEncodingOption integerEncodingOption)
      throws IOException {
    /*
     * TODOs: - detect if column is nullable to get rid of the present stream - test boolean rle
     * against roaring bitmaps and integer encoding for present stream and boolean values - Add
     * vector type to field metadata
     */
    final var physicalLevelTechnique =
        useFastPFOR ? PhysicalLevelTechnique.FAST_PFOR : PhysicalLevelTechnique.VARINT;
    final var estimatedBuffers = propertyColumns.size() * 5;
    final var featureScopedPropertyColumns = new ArrayList<byte[]>(estimatedBuffers);

    Iterator<ColumnMapping> columnMappingsIterator = null;
    for (var columnMetadata : propertyColumns) {
      final ArrayList<byte[]> encodedColumn;
      if (columnMetadata.isScalar()) {
        encodedColumn =
            encodeScalarPropertyColumn(
                features,
                useFSST,
                coercePropertyValues,
                columnMetadata,
                physicalLevelTechnique,
                integerEncodingOption);
      } else if (columnMetadata.is(MltMetadata.ComplexType.MAP)) {
        encodedColumn =
            encodeMapPropertyColumn(
                features, useFSST, columnMetadata, physicalLevelTechnique, integerEncodingOption);
      } else if (MltTypeMap.Tag0x01.isStruct(columnMetadata)) {
        if (columnMappingsIterator == null && columnMappings != null) {
          columnMappingsIterator = columnMappings.iterator();
        }
        if (columnMappingsIterator == null || !columnMappingsIterator.hasNext()) {
          throw new IllegalArgumentException(
              "Missing column mapping for nested property column " + columnMetadata.getName());
        }
        final var columnMapping = columnMappingsIterator.next();
        encodedColumn =
            encodeStructPropertyColumn(
                features, useFSST, columnMetadata, columnMapping, physicalLevelTechnique);
      } else {
        throw new IllegalArgumentException(
            "The specified data type for the field is currently not supported: " + columnMetadata);
      }

      featureScopedPropertyColumns.addAll(encodedColumn);
    }

    return featureScopedPropertyColumns;
  }

  private static ArrayList<byte[]> encodeStructPropertyColumn(
      SequencedCollection<Feature> features,
      boolean useFSST,
      MltMetadata.Column columnMetadata,
      ColumnMapping columnMapping,
      PhysicalLevelTechnique physicalLevelTechnique)
      throws IOException {
    // TODO: add present stream for struct column

    /* We limit the nesting level to one in this implementation */
    final var rootName = columnMetadata.getName();

    if (!columnMapping.getUseSharedDictionaryEncoding()) {
      throw new IllegalArgumentException(
          "Only shared dictionary encoding is currently supported for nested property columns");
    }

    /* Plan -> when there is a struct field and the useSharedDictionaryFlag is enabled
     *  share the dictionary for all string columns which are located one after
     * the other in the sequence */
    final var complexType = columnMetadata.field().type().complexType();
    final var sharedDictionary =
        new ArrayList<List<String>>(features.size() * complexType.children().size());
    for (var nestedFieldMetadata : complexType.children()) {
      if (nestedFieldMetadata.type().scalarType() == null) {
        throw new IllegalArgumentException(
            "Nested field '" + nestedFieldMetadata.name() + "' has null scalarType");
      }
      final var scalarType = nestedFieldMetadata.type().scalarType().physicalType();
      if (scalarType != MltMetadata.ScalarType.STRING) {
        throw new IllegalArgumentException(
            "Only fields of type String are currently supported as nested property columns");
      }

      final var propertyName = rootName + nestedFieldMetadata.name();
      sharedDictionary.add(
          features.stream()
              .map(
                  mvtFeature ->
                      mvtFeature
                          .findProperty(propertyName)
                          .filter(p -> p.getType().is(MltMetadata.ScalarType.STRING))
                          .map(p -> p.getValue(mvtFeature.getIndex()))
                          .flatMap(StreamUtil.optionalOfType(String.class))
                          .orElse(null))
              .collect(Collectors.toList()));
    }

    if (sharedDictionary.stream().allMatch(List::isEmpty)) {
      // Set number of streams to zero if no property values are present in this tile
      // TODO: Can we skip the column entirely in this case?
      return new ArrayList<>(List.of(new byte[] {0}));
    }
    final var nestedColumns =
        StringEncoder.encodeSharedDictionary(sharedDictionary, physicalLevelTechnique, useFSST);
    final var numStreams = nestedColumns.getLeft();
    final var encodedColumns = nestedColumns.getRight();
    assert (numStreams > 0); // encodeSharedDictionary cannot return zero streams

    final var result = new ArrayList<byte[]>(encodedColumns.size() + 1);
    result.add(EncodingUtils.encodeVarint(numStreams, false));
    result.addAll(encodedColumns);
    return result;
  }

  private static ArrayList<byte[]> encodeScalarPropertyColumn(
      SequencedCollection<Feature> features,
      boolean useFSST,
      boolean coercePropertyValues,
      MltMetadata.Column columnMetadata,
      PhysicalLevelTechnique physicalLevelTechnique,
      @NotNull ConversionConfig.IntegerEncodingOption integerEncodingOption)
      throws IOException {
    if (MltTypeMap.Tag0x01.hasStreamCount(columnMetadata)
        && features.stream().noneMatch(f -> f.findProperty(columnMetadata.getName()).isPresent())) {
      // Indicate a missing property column in the tile with a zero for the number of streams
      // TODO: Can we skip the column entirely in this case?
      return new ArrayList<>(List.of(new byte[] {0}));
    }

    return encodeScalarPropertyColumn(
        columnMetadata,
        false,
        features,
        physicalLevelTechnique,
        useFSST,
        coercePropertyValues,
        integerEncodingOption);
  }

  private static @Nullable Boolean getBooleanPropertyValue(
      @NotNull Feature feature, @NotNull String name) {
    return feature
        .findProperty(name)
        .filter(p -> p.getType().is(MltMetadata.ScalarType.BOOLEAN))
        .map(p -> (Boolean) p.getValue(feature.getIndex()))
        .orElse(null);
  }

  private static Integer strictIntOrNull(@Nullable Long value) {
    return (value != null && value.intValue() == value.longValue()) ? value.intValue() : null;
  }

  private static Integer strictIntOrNull(@Nullable U64 value) {
    return (value != null && value.intValue().longValue() == value.longValue())
        ? value.intValue()
        : null;
  }

  private static Integer strictIntOrNull(@Nullable Float value) {
    return (value != null && value.intValue() == value) ? value.intValue() : null;
  }

  private static Integer strictIntOrNull(@Nullable Double value) {
    return (value != null && value.intValue() == value) ? value.intValue() : null;
  }

  private static Long strictLongOrNull(@Nullable Float value) {
    return (value != null && value.longValue() == value) ? value.longValue() : null;
  }

  private static Long strictLongOrNull(@Nullable Double value) {
    return (value != null && value.longValue() == value) ? value.longValue() : null;
  }

  private static Float strictFloatOrNull(@Nullable Double value) {
    return (value != null && value.floatValue() == value) ? value.floatValue() : null;
  }

  private static Integer getIntPropertyValue(@NotNull Feature feature, @NotNull String name) {
    final var index = feature.getIndex();
    return feature
        .findProperty(name)
        .map(
            p ->
                switch (p.getType().getScalarType().orElse(null)) {
                  case BOOLEAN -> ((Boolean) p.getValue(index)) ? 1 : 0;
                  case UINT_8 -> ((U8) p.getValue(index)).intValue();
                  case INT_8, INT_32 -> ((Number) p.getValue(index)).intValue();
                  case UINT_32 -> ((U32) p.getValue(index)).intValue();
                  case INT_64 -> strictIntOrNull((Long) p.getValue(index));
                  case UINT_64 -> strictIntOrNull((U64) p.getValue(index));
                  case FLOAT -> strictIntOrNull((Float) p.getValue(index));
                  case DOUBLE -> strictIntOrNull((Double) p.getValue(index));
                  default -> null;
                })
        .orElse(null);
  }

  private static Long getLongPropertyValue(@NotNull Feature feature, @NotNull String name) {
    final var index = feature.getIndex();
    return feature
        .findProperty(name)
        .map(
            p ->
                switch (p.getType().getScalarType().orElse(null)) {
                  case BOOLEAN -> ((Boolean) p.getValue(index)) ? 1L : 0L;
                  case UINT_8 -> ((U8) p.getValue(index)).longValue();
                  case UINT_32 -> ((U32) p.getValue(index)).longValue();
                  case INT_8, INT_32, INT_64 -> ((Number) p.getValue(index)).longValue();
                  case UINT_64 -> ((U64) p.getValue(index)).longValue();
                  case FLOAT -> strictLongOrNull((Float) p.getValue(index));
                  case DOUBLE -> strictLongOrNull((Double) p.getValue(index));
                  default -> null;
                })
        .orElse(null);
  }

  private static @Nullable Float getFloatPropertyValue(
      @NotNull Feature feature, @NotNull String name) {
    final var index = feature.getIndex();
    return feature
        .findProperty(name)
        .map(
            p ->
                switch (p.getType().getScalarType().orElse(null)) {
                  case BOOLEAN -> ((Boolean) p.getValue(index)) ? 1.0f : 0.0f;
                  case UINT_8 -> ((U8) p.getValue(index)).intValue().floatValue();
                  case UINT_32 -> ((U32) p.getValue(index)).longValue().floatValue();
                  case UINT_64 -> ((U64) p.getValue(index)).longValue().floatValue();
                  case INT_8, INT_32, INT_64, FLOAT -> ((Number) p.getValue(index)).floatValue();
                  case DOUBLE -> strictFloatOrNull((Double) p.getValue(index));
                  default -> null;
                })
        .orElse(null);
  }

  private static @Nullable Double getDoublePropertyValue(
      @NotNull Feature feature, @NotNull String name) {
    final var index = feature.getIndex();
    return feature
        .findProperty(name)
        .map(
            p ->
                switch (p.getType().getScalarType().orElse(null)) {
                  case BOOLEAN -> ((Boolean) p.getValue(index)) ? 1.0 : 0.0;
                  case UINT_8 -> ((U8) p.getValue(index)).intValue().doubleValue();
                  case UINT_32 -> ((U32) p.getValue(index)).longValue().doubleValue();
                  case UINT_64 -> ((U64) p.getValue(index)).longValue().doubleValue();
                  case INT_8, INT_32, INT_64, FLOAT, DOUBLE ->
                      ((Number) p.getValue(index)).doubleValue();
                  default -> null;
                })
        .orElse(null);
  }

  private static String getStringPropertyValue(
      @NotNull Feature feature, @NotNull String name, boolean coercePropertyValues) {
    final var index = feature.getIndex();
    return feature
        .findProperty(name)
        .map(
            p ->
                switch (p.getType().getScalarType().orElse(null)) {
                  case STRING -> (String) p.getValue(index);
                  default -> coercePropertyValues ? p.getValue(index).toString() : null;
                })
        .orElse(null);
  }

  public static ArrayList<byte[]> encodeScalarPropertyColumn(
      MltMetadata.Column columnMetadata,
      boolean isID,
      SequencedCollection<Feature> features,
      PhysicalLevelTechnique physicalLevelTechnique,
      boolean useFSST,
      boolean coercePropertyValues,
      @NotNull ConversionConfig.IntegerEncodingOption integerEncodingOption)
      throws IOException {
    final var scalarType =
        columnMetadata
            .getScalarType()
            .orElseThrow(() -> new IllegalArgumentException("scalarType must not be null"));
    return switch (scalarType) {
      case BOOLEAN ->
          // no stream count
          encodeBooleanColumn(features, columnMetadata);
      case INT_32, UINT_32 -> {
        final var signed = (scalarType == MltMetadata.ScalarType.INT_32);
        // no stream count
        yield encodeInt32Column(
            features, columnMetadata, isID, physicalLevelTechnique, signed, integerEncodingOption);
      }
      case INT_64, UINT_64 -> {
        final var signed = (scalarType == MltMetadata.ScalarType.INT_64);
        // no stream count
        yield encodeInt64Column(features, columnMetadata, isID, signed, integerEncodingOption);
      }
      case FLOAT -> {
        // no stream count
        yield encodeFloatColumn(features, columnMetadata);
      }
      case DOUBLE -> {
        // no stream count
        yield encodeDoubleColumn(features, columnMetadata);
      }
      case STRING ->
          encodeStringColumn(
              columnMetadata, features, physicalLevelTechnique, useFSST, coercePropertyValues);
      default ->
          throw new IllegalArgumentException(
              "The specified scalar data type is currently not supported: " + scalarType);
    };
  }

  private static ArrayList<byte[]> encodeStringColumn(
      MltMetadata.Column columnMetadata,
      SequencedCollection<Feature> features,
      PhysicalLevelTechnique physicalLevelTechnique,
      boolean useFSST,
      boolean coercePropertyValues)
      throws IOException {
    /*
     * -> Single Column
     *   -> Plain Encoding Stream -> present, length, data
     *   -> Dictionary Encoding Streams -> present, length, data, dictionary
     * -> N Columns Dictionary
     *   -> SharedDictionaryLength, SharedDictionary, present1, data1, present2, data2
     * -> N Columns FsstDictionary
     * */
    final var rawStringValues =
        features.stream()
            .map(f -> getStringPropertyValue(f, columnMetadata.getName(), coercePropertyValues))
            .toArray(String[]::new);
    final var stringValues = Arrays.stream(rawStringValues).filter(Objects::nonNull).toList();

    final ArrayList<byte[]> presentStream;
    if (columnMetadata.isNullable()) {
      final var presentValues = Arrays.stream(rawStringValues).map(Objects::nonNull);
      presentStream =
          BooleanEncoder.encodeBooleanStream(
              rawStringValues.length, presentValues::iterator, PhysicalStreamType.PRESENT);
    } else {
      presentStream = new ArrayList<>();
    }

    final var stringColumn = StringEncoder.encode(stringValues, physicalLevelTechnique, useFSST);

    /* Plus 1 for present stream */
    final var hasPresentStream = ByteArrayUtil.totalLength(presentStream) > 0;
    final var streamCount = stringColumn.numStreams() + (hasPresentStream ? 1 : 0);
    final var encodedFieldMetadata = EncodingUtils.encodeVarint(streamCount, false);

    final var result =
        new ArrayList<byte[]>(presentStream.size() + stringColumn.encodedData().size() + 1);
    result.add(encodedFieldMetadata);
    result.addAll(presentStream);
    result.addAll(stringColumn.encodedData());
    return result;
  }

  private static ArrayList<byte[]> encodeBooleanColumn(
      SequencedCollection<Feature> features, MltMetadata.Column metadata) throws IOException {
    final var presentStream = metadata.isNullable() ? new BitSet(features.size()) : null;
    final var dataStream = new BitSet();
    var dataStreamIndex = 0;
    var presentStreamIndex = 0;
    for (var feature : features) {
      final var propertyValue = getBooleanPropertyValue(feature, metadata.getName());
      final var present = (propertyValue != null);
      if (present) {
        dataStream.set(dataStreamIndex++, (boolean) propertyValue);
      }
      if (presentStream != null) {
        presentStream.set(presentStreamIndex++, present);
      }
    }

    final var encodedPresentStream =
        (presentStream != null)
            ? EncodingUtils.encodeBooleanRle(presentStream, presentStreamIndex)
            : new byte[0];
    final var encodedDataStream = EncodingUtils.encodeBooleanRle(dataStream, dataStreamIndex);

    final var result =
        (presentStream != null)
            ? new StreamMetadata(
                    PhysicalStreamType.PRESENT,
                    null,
                    LogicalLevelTechnique.RLE,
                    LogicalLevelTechnique.NONE,
                    PhysicalLevelTechnique.NONE,
                    presentStreamIndex,
                    encodedPresentStream.length)
                .encode()
            : new ArrayList<byte[]>();
    final var encodedDataStreamMetadata =
        new StreamMetadata(
                PhysicalStreamType.DATA,
                null,
                LogicalLevelTechnique.RLE,
                LogicalLevelTechnique.NONE,
                PhysicalLevelTechnique.NONE,
                dataStreamIndex,
                encodedDataStream.length)
            .encode();

    result.add(encodedPresentStream);
    result.addAll(encodedDataStreamMetadata);
    result.add(encodedDataStream);
    return result;
  }

  private static ArrayList<byte[]> encodeFloatColumn(
      SequencedCollection<Feature> features, MltMetadata.Column metadata) throws IOException {
    final var values = new ArrayList<Float>(features.size());
    final var presentValues = metadata.isNullable() ? new boolean[features.size()] : null;
    var presentIndex = 0;
    for (var feature : features) {
      final var propertyValue = getFloatPropertyValue(feature, metadata.getName());
      final var present = (propertyValue != null);
      if (present) {
        values.add(propertyValue);
      }
      if (presentValues != null) {
        presentValues[presentIndex++] = present;
      }
    }

    final var encodedPresentStream =
        (presentValues != null)
            ? BooleanEncoder.encodeBooleanStream(presentValues, PhysicalStreamType.PRESENT)
            : null;
    final var result = FloatEncoder.encodeFloatStream(values);
    if (encodedPresentStream != null) {
      result.addAll(0, encodedPresentStream);
    }
    return result;
  }

  private static ArrayList<byte[]> encodeDoubleColumn(
      SequencedCollection<Feature> features, MltMetadata.Column metadata) throws IOException {
    final var values = new ArrayList<Double>(features.size());
    final var presentValues = metadata.isNullable() ? new boolean[features.size()] : null;
    var presentIndex = 0;
    for (var feature : features) {
      final var propertyValue = getDoublePropertyValue(feature, metadata.getName());
      final var present = (propertyValue != null);
      if (present) {
        values.add(propertyValue);
      }
      if (presentValues != null) {
        presentValues[presentIndex++] = present;
      }
    }

    final var encodedPresentStream =
        (presentValues != null)
            ? BooleanEncoder.encodeBooleanStream(presentValues, PhysicalStreamType.PRESENT)
            : null;
    final var result = DoubleEncoder.encodeDoubleStream(values);
    if (encodedPresentStream != null) {
      result.addAll(0, encodedPresentStream);
    }
    return result;
  }

  private static ArrayList<byte[]> encodeInt32Column(
      SequencedCollection<Feature> features,
      MltMetadata.Column metadata,
      boolean isID,
      PhysicalLevelTechnique physicalLevelTechnique,
      boolean isSigned,
      @NotNull ConversionConfig.IntegerEncodingOption integerEncodingOption)
      throws IOException {
    final var values = new ArrayList<Integer>(features.size());
    final var presentValues = metadata.isNullable() ? new boolean[features.size()] : null;
    var presentIndex = 0;
    for (var feature : features) {
      // Force ID values to integer for this column.
      // If long were required, `encodeInt64Column` would have been called instead.
      final var propertyValue =
          isID
              ? (feature.hasId() ? Math.toIntExact(feature.getId()) : null)
              : getIntPropertyValue(feature, metadata.getName());
      final var present = (propertyValue != null);
      if (present) {
        values.add(propertyValue);
      }
      if (presentValues != null) {
        presentValues[presentIndex++] = present;
      }
      // If the column is not nullable, all values must be present.
      // Failure of this assertion indicates a problem with metadata creation,
      // or use of the metadata to encode data other than what it describes.
      assert (present || metadata.isNullable());
    }

    final var encodedPresentStream =
        (presentValues != null)
            ? BooleanEncoder.encodeBooleanStream(presentValues, PhysicalStreamType.PRESENT)
            : null;
    final var result =
        IntegerEncoder.encodeIntStream(
            CollectionUtils.unboxInts(values),
            physicalLevelTechnique,
            isSigned,
            PhysicalStreamType.DATA,
            null,
            integerEncodingOption);

    if (encodedPresentStream != null) {
      result.addAll(0, encodedPresentStream);
    }
    return result;
  }

  private static ArrayList<byte[]> encodeInt64Column(
      SequencedCollection<Feature> features,
      MltMetadata.Column metadata,
      boolean isID,
      boolean isSigned,
      @NotNull ConversionConfig.IntegerEncodingOption integerEncodingOption)
      throws IOException {
    final var values = new ArrayList<Long>(features.size());
    final var presentValues = metadata.isNullable() ? new boolean[features.size()] : null;
    var presentIndex = 0;
    for (var feature : features) {
      final var propertyValue =
          isID ? feature.idOrNull() : getLongPropertyValue(feature, metadata.getName());
      final var present = (propertyValue != null);
      if (present) {
        values.add(propertyValue);
      }
      if (presentValues != null) {
        presentValues[presentIndex++] = present;
      }
    }

    final var encodedPresentStream =
        (presentValues != null)
            ? BooleanEncoder.encodeBooleanStream(presentValues, PhysicalStreamType.PRESENT)
            : null;
    final var result =
        IntegerEncoder.encodeLongStream(
            CollectionUtils.unboxLongs(values),
            isSigned,
            PhysicalStreamType.DATA,
            null,
            integerEncodingOption);

    if (encodedPresentStream != null) {
      result.addAll(0, encodedPresentStream);
    }
    return result;
  }

  private static ArrayList<byte[]> encodeMapPropertyColumn(
      @NotNull final SequencedCollection<Feature> features,
      final boolean useFSST,
      @NotNull final MltMetadata.Column columnMetadata,
      @NotNull final PhysicalLevelTechnique physicalLevelTechnique,
      @NotNull final ConversionConfig.IntegerEncodingOption encodingOption)
      throws IOException {

    final var columnName = columnMetadata.getName();
    final var uniqueValues = new UniqueMapValues();

    // Recursively gather all unique keys and values from nested fields,
    // grouped by type, with one list of values for each encodable type.
    for (var feature : features) {
      feature
          .findProperty(columnName)
          .map(property -> property.getValue(feature.getIndex()))
          .ifPresent(value -> collectUniqueMapValues(value, uniqueValues, columnName));
    }

    // Now that all the values are collected and their final
    // order is established, assign indexes for encoding
    uniqueValues.assignIndexes();

    // Flatten each property into a list of integers
    final var flattenedMapData = flattenMapValues(features, columnName, uniqueValues);
    assert flattenedMapData.featureValueCounts().size() == features.size();
    assert flattenedMapData.flattenedValues().stream().allMatch(Objects::nonNull);

    // If all maps are empty, we can skip writing data streams and just write a zero stream count
    if (flattenedMapData.allEmpty()) {
      return new ArrayList<>(List.of(new byte[] {0}));
    }

    // If any values are empty/null (which we conflate), write a presence stream
    final var writePresenceStream = flattenedMapData.anyEmpty();

    // Establish the stream mask so the decoder knows which of the optional streams are present
    final var mask =
        uniqueValues.dictionaryPresenceMask() | (writePresenceStream ? MASK_PRESENCE : 0);

    final var maxNumStreams = 15;
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
              uniqueValues.uniqueStringValues().keySet(),
              physicalLevelTechnique,
              useFSST);
      numStreams += encoded.numStreams();
      encodedStreams.add(new byte[] { (byte)encoded.numStreams() });
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
              PhysicalStreamType.DATA,
              null));
      numStreams++;
    }

    // Fill in the stream count
    encodedStreams.set(0, EncodingUtils.encodeVarint(numStreams, false));

    return encodedStreams;
  }

  public enum MapControlValue {
    NULL(0),
    FALSE(1),
    TRUE(2),
    START_MAP(
        3), /// indicates the value is a nested map rather than a scalar, followed by nested payload
    // length and payload values
    START_LIST(
        4), /// indicates the value is a list rather than a scalar, followed by payload length and
    // payload values
    COUNT(5);

    MapControlValue(int value) {
      this.value = value;
    }

    public final int value;
  }

  public static final int MASK_STRING = 1;
  public static final int MASK_INT32 = 1 << 1;
  public static final int MASK_UINT32 = 1 << 2;
  public static final int MASK_INT64 = 1 << 3;
  public static final int MASK_UINT64 = 1 << 4;
  public static final int MASK_FLOAT = 1 << 5;
  public static final int MASK_DOUBLE = 1 << 6;
  public static final int MASK_PRESENCE = 1 << 7;

  /// Holds the sorted unique values for each encodable type for a map property column and their
  // corresponding indexes
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
      var start = MapControlValue.COUNT.value;
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
      return ((!uniqueStringValues.isEmpty() ? MASK_STRING : 0)
          | (!uniqueInt32Values.isEmpty() ? MASK_INT32 : 0)
          | (!uniqueUInt32Values.isEmpty() ? MASK_UINT32 : 0)
          | (!uniqueInt64Values.isEmpty() ? MASK_INT64 : 0)
          | (!uniqueUInt64Values.isEmpty() ? MASK_UINT64 : 0)
          | (!uniqueFloatValues.isEmpty() ? MASK_FLOAT : 0)
          | (!uniqueDoubleValues.isEmpty() ? MASK_DOUBLE : 0));
    }
  }

  private record FlattenedMapData(
      @NotNull ArrayList<Integer> flattenedValues,
      @NotNull ArrayList<Integer> featureValueCounts,
      @NotNull ArrayList<Boolean> featurePresentValues,
      boolean anyEmpty,
      boolean allEmpty) {}

  /// Walk the values for each feature, building a flattened list of value indexes for all features
  private static FlattenedMapData flattenMapValues(
      @NotNull final SequencedCollection<Feature> features,
      @NotNull final String columnName,
      @NotNull final UniqueMapValues uniqueValues) {
    final var estimatedValuesPerFeature = 10;
    final var flattenedValues = new ArrayList<Integer>(estimatedValuesPerFeature * features.size());
    final var featureValueCounts = new ArrayList<Integer>(features.size());
    final var featurePresentValues = new ArrayList<Boolean>(features.size());
    boolean anyEmpty = false;
    boolean allEmpty = true;

    for (final var feature : features) {
      final var startIndex = flattenedValues.size();
      feature
          .findProperty(columnName)
          .map(property -> property.getValue(feature.getIndex()))
          .ifPresent(value -> appendMapEntries(value, flattenedValues, uniqueValues, columnName));
      final var featureValueCount = flattenedValues.size() - startIndex;
      if (featureValueCount > 0) {
        featureValueCounts.add(featureValueCount);
        allEmpty = false;
      } else {
        anyEmpty = true;
      }
      featurePresentValues.add(featureValueCount > 0);
    }

    return new FlattenedMapData(
        flattenedValues, featureValueCounts, featurePresentValues, anyEmpty, allEmpty);
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
                getScalarIndex(uniqueValues.uniqueStringValues(), entry.getKey().toString(), columnName));
        appendMapEntryValue(entry.getValue(), flattenedValues, uniqueValues, columnName);
      }
    } else if (value instanceof Iterable<?> iterable) {
      appendListValue(iterable, flattenedValues, uniqueValues, columnName);
    } else if (value != null) {
      throw new IllegalArgumentException(
          "Expected top-level map property value in column '"
              + columnName
              + "' but found "
              + value.getClass().getName());
    }
  }

  private static void appendMapEntryValue(
      @Nullable Object value,
      @NotNull final ArrayList<Integer> flattenedValues,
      @NotNull final UniqueMapValues uniqueValues,
      @NotNull final String columnName) {
    switch (value) {
      case null -> flattenedValues.add(MapControlValue.NULL.value);
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
    flattenedValues.add(MapControlValue.START_MAP.value);
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
    flattenedValues.add(MapControlValue.START_LIST.value);
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
      case null -> listValueIndexes.add(MapControlValue.NULL.value);
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
      case Boolean boolValue ->
          boolValue ? MapControlValue.TRUE.value : MapControlValue.FALSE.value;
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
  // and leaf values.
  /// Indexes cannot be established yet, so all are set to zero.
  private static void collectUniqueMapValues(
      @Nullable final Object value,
      @NotNull final UniqueMapValues uniqueValues,
      @NotNull final String columnName) {
    switch (value) {
      case null -> {
        // nulls are encoded as control markers, not dictionary values
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
      default ->
          throw new IllegalArgumentException(
              "Unsupported nested map property value type in column '"
                  + columnName
                  + "': "
                  + value.getClass().getName());
    }
  }
}
