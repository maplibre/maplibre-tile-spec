package org.maplibre.mlt.converter.encodings;

import com.google.common.primitives.Bytes;
import jakarta.annotation.Nullable;
import java.io.IOException;
import java.util.*;
import java.util.stream.Collectors;
import org.apache.commons.lang3.tuple.Triple;
import org.maplibre.mlt.converter.CollectionUtils;
import org.maplibre.mlt.converter.MltConverter;
import org.maplibre.mlt.converter.Settings;
import org.maplibre.mlt.converter.mvt.ColumnMapping;
import org.maplibre.mlt.data.Feature;
import org.maplibre.mlt.metadata.stream.LogicalLevelTechnique;
import org.maplibre.mlt.metadata.stream.PhysicalLevelTechnique;
import org.maplibre.mlt.metadata.stream.PhysicalStreamType;
import org.maplibre.mlt.metadata.stream.StreamMetadata;
import org.maplibre.mlt.metadata.tileset.MltTilesetMetadata;

public class PropertyEncoder {

  public static byte[] encodePropertyColumns(
      List<MltTilesetMetadata.Column> propertyColumns,
      List<Feature> features,
      boolean useAdvancedEncodings,
      boolean coercePropertyValues,
      List<ColumnMapping> columnMappings,
      @Nullable Map<String, Triple<byte[], byte[], String>> rawStreamData)
      throws IOException {
    /*
     * TODOs: - detect if column is nullable to get rid of the present stream - test boolean rle
     * against roaring bitmaps and integer encoding for present stream and boolean values - Add
     * vector type to field metadata
     */
    var physicalLevelTechnique =
        useAdvancedEncodings ? PhysicalLevelTechnique.FAST_PFOR : PhysicalLevelTechnique.VARINT;
    var featureScopedPropertyColumns = new byte[0];

    var i = 0;
    for (var columnMetadata : propertyColumns) {
      if (columnMetadata.hasScalarType()) {
        if (MltTypeMap.Tag0x01.hasStreamCount(columnMetadata)
            && features.stream()
                .noneMatch(f -> f.properties().containsKey(columnMetadata.getName()))) {
          /* Indicate a missing property column in the tile with a zero for the number of streams */
          final var encodedFieldMetadata = EncodingUtils.encodeVarint(0, false);
          featureScopedPropertyColumns =
              CollectionUtils.concatByteArrays(featureScopedPropertyColumns, encodedFieldMetadata);
          continue;
        }

        final var encodedScalarPropertyColumn =
            encodeScalarPropertyColumn(
                columnMetadata,
                features,
                physicalLevelTechnique,
                useAdvancedEncodings,
                coercePropertyValues,
                rawStreamData);
        featureScopedPropertyColumns =
            CollectionUtils.concatByteArrays(
                featureScopedPropertyColumns, encodedScalarPropertyColumn);
      } else if (MltTypeMap.Tag0x01.isStruct(columnMetadata)) {
        if (columnMappings.isEmpty()) {
          throw new IllegalArgumentException(
              "Column mappings are required for nested property column "
                  + columnMetadata.getName());
        }

        // TODO: add present stream for struct column

        /* We limit the nesting level to one in this implementation */
        var sharedDictionary = new ArrayList<List<String>>();
        var columnMapping = columnMappings.get(i++);

        /* Plan -> when there is a struct filed and the useSharedDictionaryFlag is enabled
         *  share the dictionary for all string columns which are located one after
         * the other in the sequence */
        for (var nestedFieldMetadata : columnMetadata.getComplexType().getChildrenList()) {
          if (nestedFieldMetadata.getScalarField().getPhysicalType()
              == MltTilesetMetadata.ScalarType.STRING) {
            if (columnMapping.useSharedDictionaryEncoding()) {
              // request all string columns in row and merge
              if (nestedFieldMetadata.getName().equals("default")) {
                var propertyColumn =
                    features.stream()
                        .map(f -> (String) f.properties().get(columnMapping.mvtPropertyPrefix()))
                        .collect(Collectors.toList());
                sharedDictionary.add(propertyColumn);
              } else {
                // TODO: handle case where the nested field name is not present in the mvt layer
                // This can be the case when the Tileset Metadata document is not generated per
                // tile instead for the full tileset
                var mvtPropertyName =
                    columnMapping.mvtPropertyPrefix()
                        + Settings.MLT_CHILD_FIELD_SEPARATOR
                        + nestedFieldMetadata.getName();
                var propertyColumn =
                    features.stream()
                        .map(mvtFeature -> (String) mvtFeature.properties().get(mvtPropertyName))
                        .collect(Collectors.toList());
                sharedDictionary.add(propertyColumn);
              }
            } else {
              throw new IllegalArgumentException(
                  "Only shared dictionary encoding is currently supported for nested property columns.");
            }
          } else {
            throw new IllegalArgumentException(
                "Only fields of type String are currently supported as nested property columns.");
          }
        }

        if (sharedDictionary.stream().allMatch(List::isEmpty)) {
          /* Set number of streams to zero if no columns are present in this tile */
          final var encodedFieldMetadata = EncodingUtils.encodeVarint(0, false);
          return CollectionUtils.concatByteArrays(
              featureScopedPropertyColumns, encodedFieldMetadata);
        }

        var nestedColumns =
            StringEncoder.encodeSharedDictionary(
                sharedDictionary,
                physicalLevelTechnique,
                useAdvancedEncodings,
                rawStreamData,
                columnMetadata.getName());
        // TODO: fix -> ony quick and dirty fix
        final var numStreams = nestedColumns.getLeft() == 0 ? 0 : 1;
        /* Set number of streams to zero if no columns are present in this tile */
        final var encodedFieldMetadata = EncodingUtils.encodeVarint(numStreams, false);

        // TODO: add present stream and present stream metadata for struct column in addition
        // to the FieldMetadata to be compliant with the specification
        featureScopedPropertyColumns =
            CollectionUtils.concatByteArrays(
                featureScopedPropertyColumns, encodedFieldMetadata, nestedColumns.getRight());
      } else {
        throw new IllegalArgumentException(
            "The specified data type for the field is currently not supported: " + columnMetadata);
      }
    }

    return featureScopedPropertyColumns;
  }

  private static Boolean getBooleanPropertyValue(
      Feature feature, MltTilesetMetadata.Column columnMetadata) {
    final var rawValue = feature.properties().get(columnMetadata.getName());
    if (rawValue instanceof Boolean) {
      return (Boolean) rawValue;
    }
    return null;
  }

  private static Integer getIntPropertyValue(
      Feature feature, MltTilesetMetadata.Column columnMetadata) {
    final var rawValue = feature.properties().get(columnMetadata.getName());
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

  private static Long getLongPropertyValue(
      Feature feature, MltTilesetMetadata.Column columnMetadata) {
    final var rawValue = feature.properties().get(columnMetadata.getName());
    if (rawValue instanceof Long) {
      return (Long) rawValue;
    } else if (rawValue instanceof Integer) {
      return (long) (int) rawValue;
    }
    return null;
  }

  private static Float getFloatPropertyValue(
      Feature feature, MltTilesetMetadata.Column columnMetadata) {
    final var rawValue = feature.properties().get(columnMetadata.getName());
    if (rawValue instanceof Float) {
      return (Float) rawValue;
    } else if (rawValue instanceof Double) {
      return (float) (double) rawValue;
    }
    return null;
  }

  private static String getStringPropertyValue(
      Feature feature, MltTilesetMetadata.Column columnMetadata, boolean coercePropertyValues) {
    final var rawValue = feature.properties().get(columnMetadata.getName());
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
      MltTilesetMetadata.Column columnMetadata,
      List<Feature> features,
      PhysicalLevelTechnique physicalLevelTechnique,
      boolean useAdvancedEncodings,
      boolean coercePropertyValues,
      @Nullable Map<String, Triple<byte[], byte[], String>> rawStreamData)
      throws IOException {
    final var scalarType = columnMetadata.getScalarType().getPhysicalType();
    switch (scalarType) {
      case BOOLEAN:
        {
          // no stream count
          return encodeBooleanColumn(features, columnMetadata, rawStreamData);
        }
      case INT_32:
      case UINT_32:
        {
          final var signed = (scalarType == MltTilesetMetadata.ScalarType.INT_32);
          // no stream count
          return encodeInt32Column(
              features, columnMetadata, physicalLevelTechnique, signed, rawStreamData);
        }
      case INT_64:
      case UINT_64:
        {
          final var signed = (scalarType == MltTilesetMetadata.ScalarType.INT_64);
          // no stream count
          return encodeInt64Column(features, columnMetadata, signed, rawStreamData);
        }
      case FLOAT:
      case DOUBLE:
        {
          // no stream count
          return encodeFloatColumn(features, columnMetadata, rawStreamData);
        }
      case STRING:
        {
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

          final byte[] presentStream;
          if (columnMetadata.getNullable()) {
            final var presentValues = rawStringValues.stream().map(Objects::nonNull).toList();
            presentStream =
                BooleanEncoder.encodeBooleanStream(
                    presentValues,
                    PhysicalStreamType.PRESENT,
                    rawStreamData,
                    "prop_" + columnMetadata.getName() + "_present");
          } else {
            presentStream = new byte[0];
          }

          var stringColumn =
              StringEncoder.encode(
                  stringValues,
                  physicalLevelTechnique,
                  useAdvancedEncodings,
                  rawStreamData,
                  "prop_" + columnMetadata.getName());

          /* Plus 1 for present stream */
          final var streamCount = stringColumn.getLeft() + ((presentStream.length > 0) ? 1 : 0);
          final var encodedFieldMetadata = EncodingUtils.encodeVarint(streamCount, false);
          return CollectionUtils.concatByteArrays(
              encodedFieldMetadata, presentStream, stringColumn.getRight());
        }
      default:
        throw new IllegalArgumentException(
            "The specified scalar data type is currently not supported: " + scalarType);
    }
  }

  private static byte[] encodeBooleanColumn(
      List<Feature> features,
      MltTilesetMetadata.Column metadata,
      @Nullable Map<String, Triple<byte[], byte[], String>> rawStreamData)
      throws IOException {
    final var fieldName = metadata.getName();
    final var presentStream = metadata.getNullable() ? new BitSet(features.size()) : null;
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

    if (rawStreamData != null) {
      if (presentStream != null) {
        GeometryEncoder.recordStream(
            "prop_" + fieldName + "_present",
            bitValues(presentStream),
            encodedPresentStreamMetadata,
            encodedPresentStream,
            rawStreamData);
      }
      GeometryEncoder.recordStream(
          "prop_" + fieldName,
          bitValues(dataStream),
          encodedDataStreamMetadata,
          encodedDataStream,
          rawStreamData);
    }

    return Bytes.concat(
        encodedPresentStreamMetadata,
        encodedPresentStream,
        encodedDataStreamMetadata,
        encodedDataStream);
  }

  private static byte[] encodeFloatColumn(
      List<Feature> features,
      MltTilesetMetadata.Column metadata,
      @Nullable Map<String, Triple<byte[], byte[], String>> rawStreamData)
      throws IOException {
    final var fieldName = metadata.getName();
    final var values = new ArrayList<Float>();
    final var presentValues =
        metadata.getNullable() ? new ArrayList<Boolean>(features.size()) : null;
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
                rawStreamData,
                "prop_" + fieldName + "_present")
            : new byte[0];
    final var encodedDataStream =
        FloatEncoder.encodeFloatStream(values, rawStreamData, "prop_" + fieldName);
    return Bytes.concat(encodedPresentStream, encodedDataStream);
  }

  private static byte[] encodeInt32Column(
      List<Feature> features,
      MltTilesetMetadata.Column metadata,
      PhysicalLevelTechnique physicalLevelTechnique,
      boolean isSigned,
      @Nullable Map<String, Triple<byte[], byte[], String>> rawStreamData)
      throws IOException {
    final var fieldName = metadata.getName();
    final var values = new ArrayList<Integer>();
    final var presentValues =
        metadata.getNullable() ? new ArrayList<Boolean>(features.size()) : null;
    for (var feature : features) {
      // TODO: refactor -> handle long values for ids differently
      final var propertyValue =
          MltConverter.isID(metadata)
              ? Integer.valueOf((int) feature.id())
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
                rawStreamData,
                "prop_" + fieldName + "_present")
            : new byte[0];
    var encodedDataStream =
        IntegerEncoder.encodeIntStream(
            values,
            physicalLevelTechnique,
            isSigned,
            PhysicalStreamType.DATA,
            null,
            rawStreamData,
            "prop_" + fieldName);

    return Bytes.concat(encodedPresentStream, encodedDataStream);
  }

  private static byte[] encodeInt64Column(
      List<Feature> features,
      MltTilesetMetadata.Column metadata,
      boolean isSigned,
      @Nullable Map<String, Triple<byte[], byte[], String>> rawStreamData)
      throws IOException {
    final var fieldName = metadata.getName();
    final var values = new ArrayList<Long>();
    final var presentValues =
        metadata.getNullable() ? new ArrayList<Boolean>(features.size()) : null;
    for (var feature : features) {
      final var propertyValue =
              MltConverter.isID(metadata)
              ? Long.valueOf(feature.id())
              : getLongPropertyValue(feature, metadata);
      final var present = (propertyValue != null);
      if (present) {
        values.add((long) propertyValue);
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
                rawStreamData,
                "prop_" + fieldName + "_present")
            : new byte[0];
    var encodedDataStream =
        IntegerEncoder.encodeLongStream(
            values, isSigned, PhysicalStreamType.DATA, null, rawStreamData, "prop_" + fieldName);
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
