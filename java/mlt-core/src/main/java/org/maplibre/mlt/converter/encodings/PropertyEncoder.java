package org.maplibre.mlt.converter.encodings;

import jakarta.annotation.Nullable;
import java.io.IOException;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.BitSet;
import java.util.List;
import java.util.Objects;
import java.util.SequencedCollection;
import java.util.stream.Collectors;
import org.jetbrains.annotations.NotNull;
import org.maplibre.mlt.converter.CollectionUtils;
import org.maplibre.mlt.converter.ColumnMapping;
import org.maplibre.mlt.converter.ConversionConfig;
import org.maplibre.mlt.data.Feature;
import org.maplibre.mlt.data.Property;
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
      SequencedCollection<ColumnMapping> columnMappings,
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

    final var columnMappingsIterator = columnMappings.iterator();
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
      } else if (MltTypeMap.Tag0x01.isStruct(columnMetadata)) {
        if (!columnMappingsIterator.hasNext()) {
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
    final var complexType = columnMetadata.field.type.complexType;
    final var sharedDictionary =
        new ArrayList<List<String>>(features.size() * complexType.children.size());
    for (var nestedFieldMetadata : complexType.children) {
      if (nestedFieldMetadata.type.scalarType == null) {
        throw new IllegalArgumentException(
            "Nested field '" + nestedFieldMetadata.name + "' has null scalarType");
      }
      final var scalarType = nestedFieldMetadata.type.scalarType.physicalType;
      if (scalarType != MltMetadata.ScalarType.STRING) {
        throw new IllegalArgumentException(
            "Only fields of type String are currently supported as nested property columns");
      }

      final var propertyName = rootName + nestedFieldMetadata.name;
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
      return new ArrayList<>(Arrays.asList(new byte[] {0}));
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
      return new ArrayList<>(Arrays.asList(new byte[] {0}));
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

  private static Boolean getBooleanPropertyValue(@NotNull Feature feature, @NotNull String name) {
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

  private static @Nullable MltMetadata.ScalarType getScalarType(@NotNull Property property) {
    return property.getType().scalarType != null
        ? property.getType().scalarType.physicalType
        : null;
  }

  private static Integer getIntPropertyValue(@NotNull Feature feature, @NotNull String name) {
    final var index = feature.getIndex();
    return feature
        .findProperty(name)
        .map(
            p ->
                switch (getScalarType(p)) {
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
                switch (getScalarType(p)) {
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

  private static Float getFloatPropertyValue(@NotNull Feature feature, @NotNull String name) {
    final var index = feature.getIndex();
    return feature
        .findProperty(name)
        .map(
            p ->
                switch (getScalarType(p)) {
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

  private static Double getDoublePropertyValue(@NotNull Feature feature, @NotNull String name) {
    final var index = feature.getIndex();
    return feature
        .findProperty(name)
        .map(
            p ->
                switch (getScalarType(p)) {
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
                switch (getScalarType(p)) {
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
      final var presentValues =
          Arrays.stream(rawStringValues).map(Objects::nonNull).toArray(Boolean[]::new);
      presentStream = BooleanEncoder.encodeBooleanStream(presentValues, PhysicalStreamType.PRESENT);
    } else {
      presentStream = new ArrayList<>();
    }

    final var stringColumn = StringEncoder.encode(stringValues, physicalLevelTechnique, useFSST);

    /* Plus 1 for present stream */
    final var hasPresentStream = ByteArrayUtil.totalLength(presentStream) > 0;
    final var streamCount = stringColumn.getLeft() + (hasPresentStream ? 1 : 0);
    final var encodedFieldMetadata = EncodingUtils.encodeVarint(streamCount, false);

    final var result =
        new ArrayList<byte[]>(presentStream.size() + stringColumn.getRight().size() + 1);
    result.add(encodedFieldMetadata);
    result.addAll(presentStream);
    result.addAll(stringColumn.getRight());
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
    final var presentValues = metadata.isNullable() ? new Boolean[features.size()] : null;
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
    final var presentValues = metadata.isNullable() ? new Boolean[features.size()] : null;
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
    final var presentValues = metadata.isNullable() ? new Boolean[features.size()] : null;
    var presentIndex = 0;
    for (var feature : features) {
      // Force ID values to integer for this column.
      // If long were required, `encodeInt64Column` would have been called instead.
      final var propertyValue =
          isID
              ? (feature.hasId() ? Integer.valueOf(Math.toIntExact(feature.getId())) : null)
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
    final var presentValues = metadata.isNullable() ? new Boolean[features.size()] : null;
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
}
