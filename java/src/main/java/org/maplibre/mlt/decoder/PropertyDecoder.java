package org.maplibre.mlt.decoder;

import java.io.IOException;
import java.util.*;
import javax.annotation.Nullable;
import me.lemire.integercompression.IntWrapper;
import org.maplibre.mlt.metadata.stream.StreamMetadataDecoder;
import org.maplibre.mlt.metadata.tileset.MltTilesetMetadata;

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
      MltTilesetMetadata.ScalarColumn scalarType,
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
    } else {
      presentStream = null;
      presentStreamSize = 0;
    }

    switch (scalarType.getPhysicalType()) {
      case BOOLEAN:
        {
          final var dataStreamMetadata = StreamMetadataDecoder.decode(data, offset);
          final var dataStream =
              DecodingUtils.decodeBooleanRle(
                  data, dataStreamMetadata.numValues(), dataStreamMetadata.byteLength(), offset);
          return unpack(
              dataStream, dataStreamMetadata.numValues(), presentStream, presentStreamSize);
        }
      case UINT_32:
      case INT_32:
        {
          final var dataStreamMetadata = StreamMetadataDecoder.decode(data, offset);
          final var dataStream =
              IntegerDecoder.decodeIntStream(
                  data,
                  offset,
                  dataStreamMetadata,
                  scalarType.getPhysicalType() == MltTilesetMetadata.ScalarType.INT_32);
          return unpack(dataStream, presentStream, presentStreamSize);
        }
      case FLOAT:
        {
          final var dataStreamMetadata = StreamMetadataDecoder.decode(data, offset);
          final var dataStream = FloatDecoder.decodeFloatStream(data, offset, dataStreamMetadata);
          return unpack(dataStream, presentStream, presentStreamSize);
        }
      case DOUBLE:
        {
          {
            final var dataStreamMetadata = StreamMetadataDecoder.decode(data, offset);
            final var dataStream = FloatDecoder.decodeFloatStream(data, offset, dataStreamMetadata);
            return unpack(dataStream, presentStream, presentStreamSize);
          }
        }
      case UINT_64:
      case INT_64:
        {
          final var dataStreamMetadata = StreamMetadataDecoder.decode(data, offset);
          final var dataStream =
              IntegerDecoder.decodeLongStream(
                  data,
                  offset,
                  dataStreamMetadata,
                  scalarType.getPhysicalType() == MltTilesetMetadata.ScalarType.INT_64);
          return unpack(dataStream, presentStream, presentStreamSize);
        }
      case STRING:
        {
          if (presentStream == null) {
            throw new RuntimeException("Non-nullable string columns not currently supported");
          }

          // The present stream has already been decoded
          final var strValues =
              StringDecoder.decode(data, offset, numStreams - 1, presentStream, presentStreamSize);
          return strValues.getRight();
        }
      default:
        throw new IllegalArgumentException(
            "The specified data type for the field is currently not supported: " + scalarType);
    }
  }

  public static Object decodePropertyColumn(
      byte[] data, IntWrapper offset, MltTilesetMetadata.Column column, int numStreams)
      throws IOException {

    if (column.hasScalarType()) {
      return decodeScalarPropertyColumn(
          data, offset, column.getScalarType(), column.getNullable(), numStreams);
    }

    /* Handle struct which currently only supports strings as nested fields for supporting shared dictionary encoding */
    if (numStreams > 1) {
      return StringDecoder.decodeSharedDictionary(data, offset, column).getRight();
    }

    // var presentStreamMetadata = StreamMetadata.decode(data, offset);
    // var presentStream = DecodingUtils.decodeBooleanRle(data, presentStreamMetadata.numValues(),
    // presentStreamMetadata.byteLength(), offset);
    // TODO: process present stream
    // var values = StringDecoder.decodeSharedDictionary(data, offset, fieldMetadata);
    throw new IllegalArgumentException("Present stream currently not supported for Structs.");
  }
}
