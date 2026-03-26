package org.maplibre.mlt.converter.encodings;

import java.io.IOException;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.BitSet;
import java.util.List;
import java.util.Objects;
import java.util.stream.Collectors;
import org.jetbrains.annotations.NotNull;
import org.maplibre.mlt.converter.CollectionUtils;
import org.maplibre.mlt.converter.ConversionConfig;
import org.maplibre.mlt.converter.mvt.ColumnMapping;
import org.maplibre.mlt.data.Feature;
import org.maplibre.mlt.data.unsigned.Unsigned;
import org.maplibre.mlt.metadata.stream.LogicalLevelTechnique;
import org.maplibre.mlt.metadata.stream.PhysicalLevelTechnique;
import org.maplibre.mlt.metadata.stream.PhysicalStreamType;
import org.maplibre.mlt.metadata.stream.StreamMetadata;
import org.maplibre.mlt.metadata.tileset.MltMetadata;
import org.maplibre.mlt.util.ByteArrayUtil;

public class PropertyEncoder {

  public static ArrayList<byte[]> encodePropertyColumns(
      List<MltMetadata.Column> propertyColumns,
      List<Feature> features,
      boolean useFastPFOR,
      boolean useFSST,
      boolean coercePropertyValues,
      List<ColumnMapping> columnMappings,
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

    var i = 0;
    for (var columnMetadata : propertyColumns) {
      final ArrayList<byte[]> encodedColumn;
      if (columnMetadata.scalarType != null) {
        encodedColumn =
            encodeScalarPropertyColumn(
                features,
                useFSST,
                coercePropertyValues,
                columnMetadata,
                physicalLevelTechnique,
                integerEncodingOption);
      } else if (MltTypeMap.Tag0x01.isStruct(columnMetadata)) {
        if (columnMappings.size() <= i) {
          throw new IllegalArgumentException(
              "Missing column mapping nested property column " + columnMetadata.name);
        }
        final var columnMapping = columnMappings.get(i++);
        encodedColumn =
            encodeStructPropertyColumn(
                features, useFSST, columnMetadata, columnMapping, physicalLevelTechnique);
      } else {
        throw new IllegalArgumentException(
            "The specified data type for the field is currently not supported: " + columnMetadata);
      }

      var a = ByteArrayUtil.totalLength(encodedColumn);
      featureScopedPropertyColumns.addAll(encodedColumn);
    }

    return featureScopedPropertyColumns;
  }

  private static ArrayList<byte[]> encodeStructPropertyColumn(
      List<Feature> features,
      boolean useFSST,
      MltMetadata.Column columnMetadata,
      ColumnMapping columnMapping,
      PhysicalLevelTechnique physicalLevelTechnique)
      throws IOException {
    // TODO: add present stream for struct column

    /* We limit the nesting level to one in this implementation */
    final var rootName = columnMetadata.name;
    final var sharedDictionary = new ArrayList<List<String>>();

    if (!columnMapping.getUseSharedDictionaryEncoding()) {
      throw new IllegalArgumentException(
          "Only shared dictionary encoding is currently supported for nested property columns");
    }

    /* Plan -> when there is a struct filed and the useSharedDictionaryFlag is enabled
     *  share the dictionary for all string columns which are located one after
     * the other in the sequence */
    final var complexType = columnMetadata.complexType;
    for (var nestedFieldMetadata : complexType.children) {
      if (nestedFieldMetadata.scalarType == null) {
        throw new IllegalArgumentException(
            "Nested field '" + nestedFieldMetadata.name + "' has null scalarType");
      }
      final var scalarType = nestedFieldMetadata.scalarType.physicalType;
      if (scalarType != MltMetadata.ScalarType.STRING) {
        throw new IllegalArgumentException(
            "Only fields of type String are currently supported as nested property columns");
      }

      final var propertyName = rootName + nestedFieldMetadata.name;
      sharedDictionary.add(
          features.stream()
              .map(mvtFeature -> (String) mvtFeature.properties().get(propertyName))
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

    final var result = new ArrayList<byte[]>();
    result.add(EncodingUtils.encodeVarint(numStreams, false));
    result.addAll(encodedColumns);
    return result;
  }

  private static ArrayList<byte[]> encodeScalarPropertyColumn(
      List<Feature> features,
      boolean useFSST,
      boolean coercePropertyValues,
      MltMetadata.Column columnMetadata,
      PhysicalLevelTechnique physicalLevelTechnique,
      @NotNull ConversionConfig.IntegerEncodingOption integerEncodingOption)
      throws IOException {
    if (MltTypeMap.Tag0x01.hasStreamCount(columnMetadata)
        && features.stream().noneMatch(f -> f.properties().containsKey(columnMetadata.name))) {
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

  private static Boolean getBooleanPropertyValue(
      Feature feature, MltMetadata.Column columnMetadata) {
    final var rawValue = feature.properties().get(columnMetadata.name);
    if (rawValue instanceof Boolean) {
      return (Boolean) rawValue;
    }
    return null;
  }

  private static Integer getIntPropertyValue(Feature feature, MltMetadata.Column columnMetadata) {
    final var rawValue = feature.properties().get(columnMetadata.name);
    if (rawValue instanceof Integer i) {
      return i;
    } else if (rawValue instanceof Long l) {
      final var v = l.longValue();
      if ((int) v == v) {
        return (int) v;
      }
    } else if (rawValue instanceof Unsigned u) {
      return u.intValue();
    }
    return null;
  }

  private static Long getLongPropertyValue(Feature feature, MltMetadata.Column columnMetadata) {
    final var rawValue = feature.properties().get(columnMetadata.name);
    if (rawValue instanceof Long l) {
      return l;
    } else if (rawValue instanceof Integer i) {
      return (long) i.intValue();
    } else if (rawValue instanceof Unsigned u) {
      return u.longValue();
    }
    return null;
  }

  private static Float getFloatPropertyValue(Feature feature, MltMetadata.Column columnMetadata) {
    final var rawValue = feature.properties().get(columnMetadata.name);
    if (rawValue instanceof Float) {
      return (Float) rawValue;
    } else if (rawValue instanceof Double) {
      return (float) (double) rawValue;
    }
    return null;
  }

  private static Double getDoublePropertyValue(Feature feature, MltMetadata.Column columnMetadata) {
    final var rawValue = feature.properties().get(columnMetadata.name);
    if (rawValue instanceof Double) {
      return (Double) rawValue;
    } else if (rawValue instanceof Float) {
      return (double) rawValue;
    }
    return null;
  }

  private static String getStringPropertyValue(
      Feature feature, MltMetadata.Column columnMetadata, boolean coercePropertyValues) {
    final var rawValue = feature.properties().get(columnMetadata.name);
    if (rawValue != null) {
      if (rawValue instanceof String) {
        return (String) rawValue;
      }
      if (coercePropertyValues) {
        return rawValue.toString();
      }
    }
    return null;
  }

  public static ArrayList<byte[]> encodeScalarPropertyColumn(
      MltMetadata.Column columnMetadata,
      boolean isID,
      List<Feature> features,
      PhysicalLevelTechnique physicalLevelTechnique,
      boolean useFSST,
      boolean coercePropertyValues,
      @NotNull ConversionConfig.IntegerEncodingOption integerEncodingOption)
      throws IOException {
    if (columnMetadata.scalarType == null) {
      throw new IllegalArgumentException("scalarType must not be null");
    }
    final var scalarType = columnMetadata.scalarType.physicalType;
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
      List<Feature> features,
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
            .map(f -> getStringPropertyValue(f, columnMetadata, coercePropertyValues))
            .toArray(String[]::new);
    final var stringValues = Arrays.stream(rawStringValues).filter(Objects::nonNull).toList();

    final ArrayList<byte[]> presentStream;
    if (columnMetadata.isNullable) {
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

    final var result = new ArrayList<byte[]>(stringColumn.getRight().size() + 2);
    result.add(encodedFieldMetadata);
    result.addAll(presentStream);
    result.addAll(stringColumn.getRight());
    return result;
  }

  private static ArrayList<byte[]> encodeBooleanColumn(
      List<Feature> features, MltMetadata.Column metadata) throws IOException {
    final var fieldName = metadata.name;
    final var presentStream = metadata.isNullable ? new BitSet(features.size()) : null;
    final var dataStream = new BitSet();
    var dataStreamIndex = 0;
    var presentStreamIndex = 0;
    for (var feature : features) {
      final var propertyValue = getBooleanPropertyValue(feature, metadata);
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
      List<Feature> features, MltMetadata.Column metadata) throws IOException {
    final var values = new ArrayList<Float>();
    final var presentValues = metadata.isNullable ? new Boolean[features.size()] : null;
    var presentIndex = 0;
    for (var feature : features) {
      final var propertyValue = getFloatPropertyValue(feature, metadata);
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
      List<Feature> features, MltMetadata.Column metadata) throws IOException {
    final var fieldName = metadata.name;
    final var values = new ArrayList<Double>();
    final var presentValues = metadata.isNullable ? new Boolean[features.size()] : null;
    var presentIndex = 0;
    for (var feature : features) {
      final var propertyValue = getDoublePropertyValue(feature, metadata);
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
      List<Feature> features,
      MltMetadata.Column metadata,
      boolean isID,
      PhysicalLevelTechnique physicalLevelTechnique,
      boolean isSigned,
      @NotNull ConversionConfig.IntegerEncodingOption integerEncodingOption)
      throws IOException {
    final var fieldName = metadata.name;
    final var values = new ArrayList<Integer>();
    final var presentValues = metadata.isNullable ? new Boolean[features.size()] : null;
    var presentIndex = 0;
    for (var feature : features) {
      // Force ID values to integer for this column.
      // If long were required, `encodeInt64Column` would have been called instead.
      final var propertyValue =
          isID
              // Cast to int to preserve the unsigned bit pattern (e.g. u32::MAX = 0xFFFFFFFF
              // is -1 as int, which VarInt encodes correctly as unsigned 4294967295).
              ? (feature.hasId() ? Integer.valueOf((int) feature.id()) : null)
              : getIntPropertyValue(feature, metadata);
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
      assert (present || metadata.isNullable);
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
      List<Feature> features,
      MltMetadata.Column metadata,
      boolean isID,
      boolean isSigned,
      @NotNull ConversionConfig.IntegerEncodingOption integerEncodingOption)
      throws IOException {
    final var fieldName = metadata.name;
    final var values = new ArrayList<Long>();
    final var presentValues = metadata.isNullable ? new Boolean[features.size()] : null;
    var presentIndex = 0;
    for (var feature : features) {
      final var propertyValue = isID ? feature.idOrNull() : getLongPropertyValue(feature, metadata);
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

  private static Boolean[] bitValues(BitSet bits) {
    var values = new Boolean[bits.length()];
    for (int i = 0; i < bits.length(); i++) {
      values[i] = bits.get(i);
    }
    return values;
  }
}
