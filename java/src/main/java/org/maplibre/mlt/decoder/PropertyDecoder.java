package org.maplibre.mlt.decoder;

import org.maplibre.mlt.metadata.stream.StreamMetadata;
import org.maplibre.mlt.metadata.stream.StreamMetadataDecoder;
import org.maplibre.mlt.metadata.tileset.MltTilesetMetadata;
import java.io.IOException;
import java.util.*;
import me.lemire.integercompression.IntWrapper;

public class PropertyDecoder {

  private PropertyDecoder() {}

  public static Object decodePropertyColumn(
      byte[] data, IntWrapper offset, MltTilesetMetadata.Column column, int numStreams)
      throws IOException {
    StreamMetadata presentStreamMetadata = null;

    if (column.hasScalarType()) {
      BitSet presentStream = null;
      var numValues = 0;
      if (numStreams > 1) {
        presentStreamMetadata = StreamMetadataDecoder.decode(data, offset);
        numValues = presentStreamMetadata.numValues();
        presentStream =
            DecodingUtils.decodeBooleanRle(
                data,
                presentStreamMetadata.numValues(),
                presentStreamMetadata.byteLength(),
                offset);
      }

      var scalarType = column.getScalarType();
      switch (scalarType.getPhysicalType()) {
        case BOOLEAN:
          {
            var dataStreamMetadata = StreamMetadataDecoder.decode(data, offset);
            var dataStream =
                DecodingUtils.decodeBooleanRle(
                    data, dataStreamMetadata.numValues(), dataStreamMetadata.byteLength(), offset);
            var booleanValues = new ArrayList<Boolean>(presentStreamMetadata.numValues());
            var counter = 0;
            for (var i = 0; i < presentStreamMetadata.numValues(); i++) {
              var value = presentStream.get(i) ? dataStream.get(counter++) : null;
              booleanValues.add(value);
            }
            return booleanValues;
          }
        case UINT_32:
        case INT_32:
          {
            var dataStreamMetadata = StreamMetadataDecoder.decode(data, offset);
            var dataStream =
                IntegerDecoder.decodeIntStream(
                    data,
                    offset,
                    dataStreamMetadata,
                    scalarType.getPhysicalType() == MltTilesetMetadata.ScalarType.INT_32);
            var counter = 0;
            var values = new ArrayList<Integer>();
            for (var i = 0; i < presentStreamMetadata.numValues(); i++) {
              var value = presentStream.get(i) ? dataStream.get(counter++) : null;
              values.add(value);
            }
            return values;
          }
        case FLOAT:
          {
            var dataStreamMetadata = StreamMetadataDecoder.decode(data, offset);
            var dataStream = FloatDecoder.decodeFloatStream(data, offset, dataStreamMetadata);
            var values = new ArrayList<Float>();
            var counter = 0;
            for (var i = 0; i < presentStreamMetadata.numValues(); i++) {
              var value = presentStream.get(i) ? dataStream.get(counter++) : null;
              values.add(value);
            }
            return values;
          }
        case DOUBLE:
          {
            {
              var dataStreamMetadata = StreamMetadataDecoder.decode(data, offset);
              var dataStream = FloatDecoder.decodeFloatStream(data, offset, dataStreamMetadata);
              var values = new ArrayList<Float>();
              var counter = 0;
              for (var i = 0; i < presentStreamMetadata.numValues(); i++) {
                var value = presentStream.get(i) ? dataStream.get(counter++) : null;
                values.add(value);
              }
              return values;
            }
          }
        case UINT_64:
        case INT_64:
          {
            var dataStreamMetadata = StreamMetadataDecoder.decode(data, offset);
            var dataStream =
                IntegerDecoder.decodeLongStream(
                    data,
                    offset,
                    dataStreamMetadata,
                    scalarType.getPhysicalType() == MltTilesetMetadata.ScalarType.INT_64);
            var values = new ArrayList<Long>();
            var counter = 0;
            for (var i = 0; i < presentStreamMetadata.numValues(); i++) {
              var value = presentStream.get(i) ? dataStream.get(counter++) : null;
              values.add(value);
            }
            return values;
          }
        case STRING:
          {
            var strValues =
                StringDecoder.decode(data, offset, numStreams - 1, presentStream, numValues);
            return strValues.getRight();
          }
        default:
          throw new IllegalArgumentException(
              "The specified data type for the field is currently not supported: " + scalarType);
      }
    }

    /* Handle struct which currently only supports strings as nested fields for supporting shared dictionary encoding */
    if (numStreams == 1) {
      // var presentStreamMetadata = StreamMetadata.decode(data, offset);
      // var presentStream = DecodingUtils.decodeBooleanRle(data, presentStreamMetadata.numValues(),
      // presentStreamMetadata.byteLength(), offset);
      // TODO: process present stream
      // var values = StringDecoder.decodeSharedDictionary(data, offset, fieldMetadata);
      throw new IllegalArgumentException("Present stream currently not supported for Structs.");
    } else {
      var result = StringDecoder.decodeSharedDictionary(data, offset, column);
      return result.getRight();
    }
  }
}
