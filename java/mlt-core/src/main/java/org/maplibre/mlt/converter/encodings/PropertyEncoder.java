package org.maplibre.mlt.converter.encodings;

import com.google.common.primitives.Bytes;
import java.io.IOException;
import java.util.*;
import java.util.stream.Collectors;
import org.jetbrains.annotations.NotNull;
import org.maplibre.mlt.converter.CollectionUtils;
import org.maplibre.mlt.converter.ConversionConfig;
import org.maplibre.mlt.converter.MLTStreamObserver;
import org.maplibre.mlt.converter.mvt.ColumnMapping;
import org.maplibre.mlt.data.Feature;
import org.maplibre.mlt.metadata.stream.LogicalLevelTechnique;
import org.maplibre.mlt.metadata.stream.PhysicalLevelTechnique;
import org.maplibre.mlt.metadata.stream.PhysicalStreamType;
import org.maplibre.mlt.metadata.stream.StreamMetadata;
import org.maplibre.mlt.metadata.tileset.MltMetadata;

public class PropertyEncoder {

  public static byte[] encodePropertyColumns(
      List<MltMetadata.Column> propertyColumns,
      List<Feature> features,
      boolean useFastPFOR,
      boolean useFSST,
      boolean coercePropertyValues,
      List<ColumnMapping> columnMappings,
      @NotNull ConversionConfig.IntegerEncodingOption integerEncodingOption,
      @NotNull MLTStreamObserver streamObserver)
      throws IOException {
    /*
     * TODOs: - detect if column is nullable to get rid of the present stream - test boolean rle
     * against roaring bitmaps and integer encoding for present stream and boolean values - Add
     * vector type to field metadata
     */
    var physicalLevelTechnique =
        useFastPFOR ? PhysicalLevelTechnique.FAST_PFOR : PhysicalLevelTechnique.VARINT;
    var featureScopedPropertyColumns = new byte[0];

    var i = 0;
    for (var columnMetadata : propertyColumns) {
      final byte[] encodedColumn;
      if (columnMetadata.scalarType != null) {
        encodedColumn =
            encodeScalarPropertyColumn(
                features,
                useFSST,
                coercePropertyValues,
                streamObserver,
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
                features,
                useFSST,
                streamObserver,
                columnMetadata,
                columnMapping,
                physicalLevelTechnique);
      } else {
        throw new IllegalArgumentException(
            "The specified data type for the field is currently not supported: " + columnMetadata);
      }

      featureScopedPropertyColumns =
          CollectionUtils.concatByteArrays(featureScopedPropertyColumns, encodedColumn);
    }

    return featureScopedPropertyColumns;
  }

  private static byte[] encodeStructPropertyColumn(
      List<Feature> features,
      boolean useFSST,
      MLTStreamObserver streamObserver,
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
      return EncodingUtils.encodeVarint(0, false);
    }
    final var nestedColumns =
        StringEncoder.encodeSharedDictionary(
            sharedDictionary,
            physicalLevelTechnique,
            useFSST,
            streamObserver,
            "prop_" + columnMetadata.name);
    final var numStreams = nestedColumns.getLeft();
    final var encodedColumns = nestedColumns.getRight();
    assert (numStreams > 0); // encodeSharedDictionary cannot return zero streams
    final var encodedNumStreams = EncodingUtils.encodeVarint(numStreams, false);
    return CollectionUtils.concatByteArrays(encodedNumStreams, encodedColumns);
  }

  private static byte[] encodeScalarPropertyColumn(
      List<Feature> features,
      boolean useFSST,
      boolean coercePropertyValues,
      MLTStreamObserver streamObserver,
      MltMetadata.Column columnMetadata,
      PhysicalLevelTechnique physicalLevelTechnique,
      @NotNull ConversionConfig.IntegerEncodingOption integerEncodingOption)
      throws IOException {
    if (MltTypeMap.Tag0x01.hasStreamCount(columnMetadata)
        && features.stream().noneMatch(f -> f.properties().containsKey(columnMetadata.name))) {
      // Indicate a missing property column in the tile with a zero for the number of streams
      // TODO: Can we skip the column entirely in this case?
      return EncodingUtils.encodeVarint(0, false);
    }

    return encodeScalarPropertyColumn(
        columnMetadata,
        false,
        features,
        physicalLevelTechnique,
        useFSST,
        coercePropertyValues,
        integerEncodingOption,
        streamObserver);
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
    if (rawValue instanceof Integer) {
      return (Integer) rawValue;
    } else if (rawValue instanceof Long) {
      final var v = (long) rawValue;
      if ((int) v == v) {
        return (int) v;
      }
    }
    return null;
  }

  private static Long getLongPropertyValue(Feature feature, MltMetadata.Column columnMetadata) {
    final var rawValue = feature.properties().get(columnMetadata.name);
    if (rawValue instanceof Long) {
      return (Long) rawValue;
    } else if (rawValue instanceof Integer) {
      return (long) (int) rawValue;
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

  public static byte[] encodeScalarPropertyColumn(
      MltMetadata.Column columnMetadata,
      boolean isID,
      List<Feature> features,
      PhysicalLevelTechnique physicalLevelTechnique,
      boolean useFSST,
      boolean coercePropertyValues,
      @NotNull ConversionConfig.IntegerEncodingOption integerEncodingOption,
      @NotNull MLTStreamObserver streamObserver)
      throws IOException {
    if (columnMetadata.scalarType == null) {
      throw new IllegalArgumentException("scalarType must not be null");
    }
    final var scalarType = columnMetadata.scalarType.physicalType;
    return switch (scalarType) {
      case BOOLEAN ->
          // no stream count
          encodeBooleanColumn(features, columnMetadata, streamObserver);
      case INT_32, UINT_32 -> {
        final var signed = (scalarType == MltMetadata.ScalarType.INT_32);
        // no stream count
        yield encodeInt32Column(
            features,
            columnMetadata,
            isID,
            physicalLevelTechnique,
            signed,
            integerEncodingOption,
            streamObserver);
      }
      case INT_64, UINT_64 -> {
        final var signed = (scalarType == MltMetadata.ScalarType.INT_64);
        // no stream count
        yield encodeInt64Column(
            features, columnMetadata, isID, signed, integerEncodingOption, streamObserver);
      }
      case FLOAT, DOUBLE ->
          // no stream count
          encodeFloatColumn(features, columnMetadata, streamObserver);
      case STRING ->
          encodeStringColumn(
              columnMetadata,
              features,
              physicalLevelTechnique,
              useFSST,
              coercePropertyValues,
              streamObserver);
      default ->
          throw new IllegalArgumentException(
              "The specified scalar data type is currently not supported: " + scalarType);
    };
  }

  private static byte[] encodeStringColumn(
      MltMetadata.Column columnMetadata,
      List<Feature> features,
      PhysicalLevelTechnique physicalLevelTechnique,
      boolean useFSST,
      boolean coercePropertyValues,
      MLTStreamObserver streamObserver)
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
            .toList();
    final var stringValues =
        rawStringValues.stream().filter(Objects::nonNull).collect(Collectors.toList());

    if (streamObserver.isActive()) {
      streamObserver.observeStream(
          "prop_" + columnMetadata.name + "_strings", rawStringValues, null, null);
    }

    final byte[] presentStream;
    if (columnMetadata.isNullable) {
      final var presentValues = rawStringValues.stream().map(Objects::nonNull).toList();
      presentStream =
          BooleanEncoder.encodeBooleanStream(
              presentValues,
              PhysicalStreamType.PRESENT,
              streamObserver,
              "prop_" + columnMetadata.name + "_present");
    } else {
      presentStream = new byte[0];
    }

    var stringColumn =
        StringEncoder.encode(
            stringValues,
            physicalLevelTechnique,
            useFSST,
            streamObserver,
            "prop_" + columnMetadata.name);

    /* Plus 1 for present stream */
    final var streamCount = stringColumn.getLeft() + ((presentStream.length > 0) ? 1 : 0);
    final var encodedFieldMetadata = EncodingUtils.encodeVarint(streamCount, false);
    return CollectionUtils.concatByteArrays(
        encodedFieldMetadata, presentStream, stringColumn.getRight());
  }

  private static byte[] encodeBooleanColumn(
      List<Feature> features,
      MltMetadata.Column metadata,
      @NotNull MLTStreamObserver streamObserver)
      throws IOException {
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

    final var encodedPresentStreamMetadata =
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
            : new byte[0];
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

    if (presentStream != null) {
      streamObserver.observeStream(
          "prop_" + fieldName + "_present",
          bitValues(presentStream),
          encodedPresentStreamMetadata,
          encodedPresentStream);
    }
    streamObserver.observeStream(
        "prop_" + fieldName, bitValues(dataStream), encodedDataStreamMetadata, encodedDataStream);

    return Bytes.concat(
        encodedPresentStreamMetadata,
        encodedPresentStream,
        encodedDataStreamMetadata,
        encodedDataStream);
  }

  private static byte[] encodeFloatColumn(
      List<Feature> features,
      MltMetadata.Column metadata,
      @NotNull MLTStreamObserver streamObserver)
      throws IOException {
    final var fieldName = metadata.name;
    final var values = new ArrayList<Float>();
    final var presentValues = metadata.isNullable ? new ArrayList<Boolean>(features.size()) : null;
    for (var feature : features) {
      final var propertyValue = getFloatPropertyValue(feature, metadata);
      final var present = (propertyValue != null);
      if (present) {
        values.add(propertyValue);
      }
      if (presentValues != null) {
        presentValues.add(present);
      }
    }

    final var encodedPresentStream =
        (presentValues != null)
            ? BooleanEncoder.encodeBooleanStream(
                presentValues,
                PhysicalStreamType.PRESENT,
                streamObserver,
                "prop_" + fieldName + "_present")
            : new byte[0];
    final var encodedDataStream =
        FloatEncoder.encodeFloatStream(values, streamObserver, "prop_" + fieldName);
    return Bytes.concat(encodedPresentStream, encodedDataStream);
  }

  private static byte[] encodeInt32Column(
      List<Feature> features,
      MltMetadata.Column metadata,
      boolean isID,
      PhysicalLevelTechnique physicalLevelTechnique,
      boolean isSigned,
      @NotNull ConversionConfig.IntegerEncodingOption integerEncodingOption,
      @NotNull MLTStreamObserver streamObserver)
      throws IOException {
    final var fieldName = metadata.name;
    final var values = new ArrayList<Integer>();
    final var presentValues = metadata.isNullable ? new ArrayList<Boolean>(features.size()) : null;
    for (var feature : features) {
      // TODO: refactor -> handle long values for ids differently
      final var propertyValue =
          isID
              ? (feature.hasId() ? Integer.valueOf((int) feature.id()) : null)
              : getIntPropertyValue(feature, metadata);
      final var present = (propertyValue != null);
      if (present) {
        values.add(propertyValue);
      }
      if (presentValues != null) {
        presentValues.add(present);
      }
    }

    var encodedPresentStream =
        (presentValues != null)
            ? BooleanEncoder.encodeBooleanStream(
                presentValues,
                PhysicalStreamType.PRESENT,
                streamObserver,
                "prop_" + fieldName + "_present")
            : new byte[0];
    var encodedDataStream =
        IntegerEncoder.encodeIntStream(
            CollectionUtils.unboxInts(values),
            physicalLevelTechnique,
            isSigned,
            PhysicalStreamType.DATA,
            null,
            integerEncodingOption,
            streamObserver,
            "prop_" + fieldName);

    return Bytes.concat(encodedPresentStream, encodedDataStream);
  }

  private static byte[] encodeInt64Column(
      List<Feature> features,
      MltMetadata.Column metadata,
      boolean isID,
      boolean isSigned,
      @NotNull ConversionConfig.IntegerEncodingOption integerEncodingOption,
      @NotNull MLTStreamObserver streamObserver)
      throws IOException {
    final var fieldName = metadata.name;
    final var values = new ArrayList<Long>();
    final var presentValues = metadata.isNullable ? new ArrayList<Boolean>(features.size()) : null;
    for (var feature : features) {
      final var propertyValue = isID ? feature.idOrNull() : getLongPropertyValue(feature, metadata);
      final var present = (propertyValue != null);
      if (present) {
        values.add(propertyValue);
      }
      if (presentValues != null) {
        presentValues.add(present);
      }
    }

    var encodedPresentStream =
        (presentValues != null)
            ? BooleanEncoder.encodeBooleanStream(
                presentValues,
                PhysicalStreamType.PRESENT,
                streamObserver,
                "prop_" + fieldName + "_present")
            : new byte[0];
    var encodedDataStream =
        IntegerEncoder.encodeLongStream(
            CollectionUtils.unboxLongs(values),
            isSigned,
            PhysicalStreamType.DATA,
            null,
            integerEncodingOption,
            streamObserver,
            "prop_" + fieldName);
    return Bytes.concat(encodedPresentStream, encodedDataStream);
  }

  private static ArrayList<Boolean> bitValues(BitSet bits) {
    var values = new ArrayList<Boolean>(bits.length());
    for (int i = 0; i < bits.length(); i++) {
      values.add(bits.get(i));
    }
    return values;
  }
}
