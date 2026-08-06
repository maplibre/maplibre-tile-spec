package org.maplibre.mlt.decoder;

import jakarta.annotation.Nullable;
import java.io.IOException;
import java.util.ArrayList;
import java.util.BitSet;
import java.util.List;
import me.lemire.integercompression.IntWrapper;
import org.jetbrains.annotations.NotNull;
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
      return MapPropertyDecoder.decodeMapPropertyColumn(data, offset, column, numStreams);
    }

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
                : dataStream.stream()
                    .map(i -> i == null ? null : MapPropertyDecoder.toU64(i))
                    .toList();

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
}
