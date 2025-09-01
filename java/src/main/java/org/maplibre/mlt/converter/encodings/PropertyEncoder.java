package org.maplibre.mlt.converter.encodings;

import com.google.common.primitives.Bytes;
import org.maplibre.mlt.converter.CollectionUtils;
import org.maplibre.mlt.converter.Settings;
import org.maplibre.mlt.converter.mvt.ColumnMapping;
import org.maplibre.mlt.data.Feature;
import org.maplibre.mlt.metadata.stream.LogicalLevelTechnique;
import org.maplibre.mlt.metadata.stream.PhysicalLevelTechnique;
import org.maplibre.mlt.metadata.stream.PhysicalStreamType;
import org.maplibre.mlt.metadata.stream.StreamMetadata;
import org.maplibre.mlt.metadata.tileset.MltTilesetMetadata;
import java.io.IOException;
import java.util.*;
import java.util.stream.Collectors;

public class PropertyEncoder {
  private static String ID_COLUMN_NAME = "id";

  public static byte[] encodePropertyColumns(
      List<MltTilesetMetadata.Column> propertyColumns,
      List<Feature> features,
      boolean useAdvancedEncodings,
      Optional<List<ColumnMapping>> columnMappings)
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
        if (features.stream()
            .noneMatch(f -> f.properties().containsKey(columnMetadata.getName()))) {
          /* Indicate a missing property column in the tile with a zero for the number of streams */
          var encodedFieldMetadata = EncodingUtils.encodeVarints(new long[] {0}, false, false);
          featureScopedPropertyColumns =
              CollectionUtils.concatByteArrays(featureScopedPropertyColumns, encodedFieldMetadata);
          continue;
        }

        var encodedScalarPropertyColumn =
            encodeScalarPropertyColumn(
                columnMetadata, features, physicalLevelTechnique, useAdvancedEncodings);
        featureScopedPropertyColumns =
            CollectionUtils.concatByteArrays(
                featureScopedPropertyColumns, encodedScalarPropertyColumn);
      } else if (columnMetadata.hasComplexType()
          && columnMetadata.getComplexType().getPhysicalType()
              == MltTilesetMetadata.ComplexType.STRUCT) {
        if (columnMappings.isEmpty()) {
          throw new IllegalArgumentException(
              "Column mappings are required for nested property column "
                  + columnMetadata.getName());
        }

        // TODO: add present stream for struct column

        /* We limit the nesting level to one in this implementation */
        var sharedDictionary = new ArrayList<List<String>>();
        var columnMapping = columnMappings.get().get(i++);

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
          var encodedFieldMetadata = EncodingUtils.encodeVarints(new long[] {0}, false, false);
          return CollectionUtils.concatByteArrays(
              featureScopedPropertyColumns, encodedFieldMetadata);
        }

        var nestedColumns =
            StringEncoder.encodeSharedDictionary(
                sharedDictionary, physicalLevelTechnique, useAdvancedEncodings);
        // TODO: fix -> ony quick and dirty fix
        var numStreams = nestedColumns.getLeft() == 0 ? 0 : 1;
        /* Set number of streams to zero if no columns are present in this tile */
        var encodedFieldMetadata =
            EncodingUtils.encodeVarints(new long[] {numStreams}, false, false);

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

  public static byte[] encodeScalarPropertyColumn(
      MltTilesetMetadata.Column columnMetadata,
      List<Feature> features,
      PhysicalLevelTechnique physicalLevelTechnique,
      boolean useAdvancedEncodings)
      throws IOException {
    var scalarType = columnMetadata.getScalarType().getPhysicalType();
    switch (scalarType) {
      case BOOLEAN:
        {
          var booleanColumn = encodeBooleanColumn(features, columnMetadata.getName());
          var encodedFieldMetadata = EncodingUtils.encodeVarints(new long[] {2}, false, false);
          return CollectionUtils.concatByteArrays(encodedFieldMetadata, booleanColumn);
        }
      case UINT_32:
        {
          var intColumn =
              encodeInt32Column(features, columnMetadata.getName(), physicalLevelTechnique, false);
          var encodedFieldMetadata = EncodingUtils.encodeVarints(new long[] {2}, false, false);
          return CollectionUtils.concatByteArrays(encodedFieldMetadata, intColumn);
        }
      case INT_32:
        {
          var intColumn =
              encodeInt32Column(features, columnMetadata.getName(), physicalLevelTechnique, true);
          var encodedFieldMetadata = EncodingUtils.encodeVarints(new long[] {2}, false, false);
          return CollectionUtils.concatByteArrays(encodedFieldMetadata, intColumn);
        }
      case UINT_64:
        {
          var intColumn = encodeInt64Column(features, columnMetadata.getName(), false);
          var encodedFieldMetadata = EncodingUtils.encodeVarints(new long[] {2}, false, false);
          return CollectionUtils.concatByteArrays(encodedFieldMetadata, intColumn);
        }
      case INT_64:
        {
          var intColumn = encodeInt64Column(features, columnMetadata.getName(), true);
          var encodedFieldMetadata = EncodingUtils.encodeVarints(new long[] {2}, false, false);
          return CollectionUtils.concatByteArrays(encodedFieldMetadata, intColumn);
        }
      case FLOAT:
      case DOUBLE:
        {
          var floatColumn = encodeFloatColumn(features, columnMetadata.getName());
          var encodedFieldMetadata = EncodingUtils.encodeVarints(new long[] {2}, false, false);
          return CollectionUtils.concatByteArrays(encodedFieldMetadata, floatColumn);
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
          var present =
              features.stream()
                  .map(f -> f.properties().get(columnMetadata.getName()) != null)
                  .collect(Collectors.toList());
          var presentStream =
              BooleanEncoder.encodeBooleanStream(present, PhysicalStreamType.PRESENT);

          var values =
              features.stream()
                  .map(f -> (String) f.properties().get(columnMetadata.getName()))
                  .filter(v -> v != null)
                  .collect(Collectors.toList());
          var stringColumn =
              StringEncoder.encode(values, physicalLevelTechnique, useAdvancedEncodings);
          /* Plus 1 for present stream */
          var encodedFieldMetadata =
              EncodingUtils.encodeVarints(new long[] {stringColumn.getLeft() + 1}, false, false);
          return CollectionUtils.concatByteArrays(
              encodedFieldMetadata, presentStream, stringColumn.getRight());
        }
      default:
        throw new IllegalArgumentException(
            "The specified scalar data type is currently not supported: " + scalarType);
    }
  }

  private static byte[] encodeBooleanColumn(List<Feature> features, String fieldName)
      throws IOException {
    var presentStream = new BitSet(features.size());
    var dataStream = new BitSet();
    var dataStreamIndex = 0;
    var presentStreamIndex = 0;
    for (var feature : features) {
      var propertyValue = feature.properties().get(fieldName);
      if (propertyValue != null) {
        dataStream.set(dataStreamIndex++, (boolean) propertyValue);
        presentStream.set(presentStreamIndex++, true);
      } else {
        presentStream.set(presentStreamIndex++, false);
      }
    }

    var encodedPresentStream = EncodingUtils.encodeBooleanRle(presentStream, presentStreamIndex);
    var encodedDataStream = EncodingUtils.encodeBooleanRle(dataStream, dataStreamIndex);

    var encodedPresentStreamMetadata =
        new StreamMetadata(
                PhysicalStreamType.PRESENT,
                null,
                LogicalLevelTechnique.RLE,
                LogicalLevelTechnique.NONE,
                PhysicalLevelTechnique.NONE,
                presentStreamIndex,
                encodedPresentStream.length)
            .encode();
    var encodedDataStreamMetadata =
        new StreamMetadata(
                PhysicalStreamType.DATA,
                null,
                LogicalLevelTechnique.RLE,
                LogicalLevelTechnique.NONE,
                PhysicalLevelTechnique.NONE,
                dataStreamIndex,
                encodedDataStream.length)
            .encode();
    return Bytes.concat(
        encodedPresentStreamMetadata,
        encodedPresentStream,
        encodedDataStreamMetadata,
        encodedDataStream);
  }

  private static byte[] encodeFloatColumn(List<Feature> features, String fieldName)
      throws IOException {
    var values = new ArrayList<Float>();
    var present = new ArrayList<Boolean>(features.size());
    for (var feature : features) {
      var propertyValue = feature.properties().get(fieldName);
      if (propertyValue != null) {
        switch (propertyValue.getClass().getSimpleName()) {
          case "Double":
            var doubleValue = (Double) propertyValue;
            values.add(doubleValue.floatValue());
            break;
          default:
            values.add((float) propertyValue);
            break;
        }
        present.add(true);
      } else {
        present.add(false);
      }
    }

    var encodedPresentStream =
        BooleanEncoder.encodeBooleanStream(present, PhysicalStreamType.PRESENT);
    var encodedDataStream = FloatEncoder.encodeFloatStream(values);
    return Bytes.concat(encodedPresentStream, encodedDataStream);
  }

  private static byte[] encodeInt32Column(
      List<Feature> features,
      String fieldName,
      PhysicalLevelTechnique physicalLevelTechnique,
      boolean isSigned)
      throws IOException {
    var values = new ArrayList<Integer>();
    var present = new ArrayList<Boolean>(features.size());
    for (var feature : features) {
      var propertyValue =
          fieldName.equals(ID_COLUMN_NAME) ? feature.id() : feature.properties().get(fieldName);
      if (propertyValue != null) {
        // TODO: refactor -> handle long values for ids differently
        var intValue =
            propertyValue instanceof Long ? ((Long) propertyValue).intValue() : (int) propertyValue;
        values.add(intValue);
        present.add(true);
      } else {
        present.add(false);
      }
    }

    var encodedPresentStream =
        BooleanEncoder.encodeBooleanStream(present, PhysicalStreamType.PRESENT);
    var encodedDataStream =
        IntegerEncoder.encodeIntStream(
            values, physicalLevelTechnique, isSigned, PhysicalStreamType.DATA, null);

    return Bytes.concat(encodedPresentStream, encodedDataStream);
  }

  private static byte[] encodeInt64Column(
      List<Feature> features, String fieldName, boolean isSigned) throws IOException {
    var values = new ArrayList<Long>();
    var present = new ArrayList<Boolean>(features.size());
    for (var feature : features) {
      var propertyValue =
          fieldName.equals(ID_COLUMN_NAME) ? feature.id() : feature.properties().get(fieldName);
      if (propertyValue != null) {
        values.add((long) propertyValue);
        present.add(true);
      } else {
        present.add(false);
      }
    }

    var encodedPresentStream =
        BooleanEncoder.encodeBooleanStream(present, PhysicalStreamType.PRESENT);
    var encodedDataStream =
        IntegerEncoder.encodeLongStream(values, isSigned, PhysicalStreamType.DATA, null);
    return Bytes.concat(encodedPresentStream, encodedDataStream);
  }
}
