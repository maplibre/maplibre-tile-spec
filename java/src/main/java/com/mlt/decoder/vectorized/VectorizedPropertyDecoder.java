package com.mlt.decoder.vectorized;

import com.mlt.metadata.stream.StreamMetadata;
import com.mlt.metadata.stream.StreamMetadataDecoder;
import com.mlt.metadata.tileset.MltTilesetMetadata;
import com.mlt.vector.BitVector;
import com.mlt.vector.Vector;
import com.mlt.vector.VectorType;
import com.mlt.vector.constant.IntConstVector;
import com.mlt.vector.constant.LongConstVector;
import com.mlt.vector.flat.BooleanFlatVector;
import com.mlt.vector.flat.DoubleFlatVector;
import com.mlt.vector.flat.FloatFlatVector;
import com.mlt.vector.flat.IntFlatVector;
import com.mlt.vector.flat.LongFlatVector;
import java.io.IOException;
import me.lemire.integercompression.IntWrapper;

public class VectorizedPropertyDecoder {
  private VectorizedPropertyDecoder() {}

  public static Vector decodePropertyColumn(
      byte[] data, IntWrapper offset, MltTilesetMetadata.Column column, int numStreams)
      throws IOException {
    StreamMetadata presentStreamMetadata;
    if (column.hasScalarType()) {
      BitVector presentStream = null;
      var numValues = 0;
      if (numStreams > 1) {
        presentStreamMetadata = StreamMetadataDecoder.decode(data, offset);
        numValues = presentStreamMetadata.numValues();
        var presentVector = VectorizedDecodingUtils.decodeBooleanRle(data, numValues, offset);
        presentStream = new BitVector(presentVector, presentStreamMetadata.numValues());
      }

      var scalarType = column.getScalarType();
      switch (scalarType.getPhysicalType()) {
        case BOOLEAN:
          {
            var dataStreamMetadata = StreamMetadataDecoder.decode(data, offset);
            var dataStream =
                VectorizedDecodingUtils.decodeBooleanRle(
                    data, dataStreamMetadata.numValues(), offset);
            var dataVector = new BitVector(dataStream, dataStreamMetadata.numValues());
            return presentStream != null
                ? new BooleanFlatVector(column.getName(), presentStream, dataVector)
                : new BooleanFlatVector(column.getName(), dataVector);
          }
        case UINT_32:
        case INT_32:
          {
            var dataStreamMetadata = StreamMetadataDecoder.decode(data, offset);
            var dataStream =
                VectorizedIntegerDecoder.decodeIntStream(
                    data,
                    offset,
                    dataStreamMetadata,
                    scalarType.getPhysicalType() == MltTilesetMetadata.ScalarType.INT_32);
            return presentStream != null
                ? new IntFlatVector(column.getName(), presentStream, dataStream)
                : new IntFlatVector(column.getName(), dataStream);
          }
        case UINT_64:
        case INT_64:
          {
            var dataStreamMetadata = StreamMetadataDecoder.decode(data, offset);
            var dataStream =
                VectorizedIntegerDecoder.decodeLongStream(
                    data,
                    offset,
                    dataStreamMetadata,
                    scalarType.getPhysicalType() == MltTilesetMetadata.ScalarType.INT_64);
            return presentStream != null
                ? new LongFlatVector(column.getName(), presentStream, dataStream)
                : new LongFlatVector(column.getName(), dataStream);
          }
        case FLOAT:
          {
            var dataStreamMetadata = StreamMetadataDecoder.decode(data, offset);
            var dataStream =
                VectorizedFloatDecoder.decodeFloatStream(data, offset, dataStreamMetadata);
            return presentStream != null
                ? new FloatFlatVector(column.getName(), presentStream, dataStream)
                : new FloatFlatVector(column.getName(), dataStream);
          }
          /*case DOUBLE:{
              break;
          }*/
        case STRING:
          {
            return VectorizedStringDecoder.decode(
                column.getName(), data, offset, numStreams - 1, presentStream);
          }
        default:
          throw new IllegalArgumentException(
              "The specified data type for the field is currently not supported: " + scalarType);
      }
    }

    /* Handle struct which currently only supports strings as nested fields for supporting shared dictionary encoding */
    if (numStreams == 1) {
      throw new IllegalArgumentException("Present stream currently not supported for Structs.");
    }

    return VectorizedStringDecoder.decodeSharedDictionary(data, offset, column);
  }

  public static Vector decodeToRandomAccessFormat(
      byte[] data,
      IntWrapper offset,
      MltTilesetMetadata.Column column,
      int numStreams,
      int numFeatures) {
    StreamMetadata presentStreamMetadata;
    if (column.hasScalarType()) {
      BitVector nullabilityBuffer = null;
      var numValues = 0;
      if (numStreams == 0) {
        /*
         * The absence of an entire column can be identified by a zero value for the number of
         * streams.
         */
        return null;
      } else if (numStreams > 1) {
        presentStreamMetadata = StreamMetadataDecoder.decode(data, offset);
        // TODO: get rid of that check by not including the present stream if not nullable
        var vectorType =
            VectorizedDecodingUtils.getVectorTypeBooleanStream(
                numFeatures, presentStreamMetadata.byteLength(), data, offset);
        /*
         * If vector type equals const create vector without a nullabilityBuffer which specifies
         * that the column is not nullable.The absence of a column can be specified by a zero value
         * for numValues
         */
        if (vectorType == VectorType.FLAT) {
          numValues = presentStreamMetadata.numValues();
          var presentVector = VectorizedDecodingUtils.decodeBooleanRle(data, numValues, offset);
          nullabilityBuffer = new BitVector(presentVector, presentStreamMetadata.numValues());
        } else {
          /**
           * Const vector -> all values are present so this is a not nullable column, since if all
           * values are not present, the absence of the full column is specified with a zero value
           * for the number of streams.
           */
          offset.add(presentStreamMetadata.byteLength());
        }
      }

      var scalarType = column.getScalarType();
      switch (scalarType.getPhysicalType()) {
        case BOOLEAN:
          {
            var dataStreamMetadata = StreamMetadataDecoder.decode(data, offset);
            var vectorType =
                VectorizedDecodingUtils.getVectorTypeBooleanStream(
                    numFeatures, dataStreamMetadata.byteLength(), data, offset);
            boolean isNullable = nullabilityBuffer != null;
            if (vectorType.equals(VectorType.FLAT)) {
              var dataStream =
                  isNullable
                      ? VectorizedDecodingUtils.decodeNullableBooleanRle(
                          data, dataStreamMetadata.numValues(), offset, nullabilityBuffer)
                      : VectorizedDecodingUtils.decodeBooleanRle(
                          data, dataStreamMetadata.numValues(), offset);
              var dataVector = new BitVector(dataStream, dataStreamMetadata.numValues());
              return new BooleanFlatVector(column.getName(), nullabilityBuffer, dataVector);
            } else {
              // TODO: handle const
              throw new IllegalArgumentException("ConstBooleanVector ist not supported yet.");
            }
          }
        case UINT_32:
        case INT_32:
          {
            var dataStreamMetadata = StreamMetadataDecoder.decode(data, offset);
            var vectorType = VectorizedDecodingUtils.getVectorTypeIntStream(dataStreamMetadata);
            var isSigned = scalarType.getPhysicalType() == MltTilesetMetadata.ScalarType.INT_32;
            var isNullable = nullabilityBuffer != null;
            if (vectorType.equals(VectorType.FLAT)) {
              var dataStream =
                  isNullable
                      ? VectorizedIntegerDecoder.decodeNullableIntStream(
                          data, offset, dataStreamMetadata, isSigned, nullabilityBuffer)
                      : VectorizedIntegerDecoder.decodeIntStream(
                          data, offset, dataStreamMetadata, isSigned);
              return new IntFlatVector(column.getName(), nullabilityBuffer, dataStream);
            } else {
              /** handle ConstVector */
              var constValue =
                  VectorizedIntegerDecoder.decodeConstIntStream(
                      data, offset, dataStreamMetadata, isSigned);
              return new IntConstVector(column.getName(), nullabilityBuffer, constValue);
            }
          }
        case UINT_64:
        case INT_64:
          {
            var dataStreamMetadata = StreamMetadataDecoder.decode(data, offset);
            var vectorType = VectorizedDecodingUtils.getVectorTypeIntStream(dataStreamMetadata);
            var isNullable = nullabilityBuffer != null;
            var isSigned = scalarType.getPhysicalType() == MltTilesetMetadata.ScalarType.INT_64;
            if (vectorType.equals(VectorType.FLAT)) {
              var dataStream =
                  isNullable
                      ? VectorizedIntegerDecoder.decodeNullableLongStream(
                          data, offset, dataStreamMetadata, isSigned, nullabilityBuffer)
                      : VectorizedIntegerDecoder.decodeLongStream(
                          data, offset, dataStreamMetadata, isSigned);
              return new LongFlatVector(column.getName(), nullabilityBuffer, dataStream);
            } else {
              /** handle ConstVector */
              var constValue =
                  VectorizedIntegerDecoder.decodeConstLongStream(
                      data, offset, dataStreamMetadata, isSigned);
              return new LongConstVector(column.getName(), nullabilityBuffer, constValue);
            }
          }
        case FLOAT:
          {
            // TODO: add rle encoding and ConstVector
            var dataStreamMetadata = StreamMetadataDecoder.decode(data, offset);
            var dataStream =
                nullabilityBuffer != null
                    ? VectorizedFloatDecoder.decodeNullableFloatStream(
                        data, offset, dataStreamMetadata, nullabilityBuffer)
                    : VectorizedFloatDecoder.decodeFloatStream(data, offset, dataStreamMetadata);
            return new FloatFlatVector(column.getName(), nullabilityBuffer, dataStream);
          }
        case DOUBLE:
          {
            // TODO: add rle encoding and ConstVector
            var dataStreamMetadata = StreamMetadataDecoder.decode(data, offset);
            var dataStream =
                nullabilityBuffer != null
                    ? VectorizedDoubleDecoder.decodeNullableDoubleStream(
                        data, offset, dataStreamMetadata, nullabilityBuffer)
                    : VectorizedDoubleDecoder.decodeDoubleStream(data, offset, dataStreamMetadata);
            return new DoubleFlatVector(column.getName(), nullabilityBuffer, dataStream);
          }
        case STRING:
          {
            return VectorizedStringDecoder.decodeToRandomAccessFormat(
                column.getName(), data, offset, numStreams - 1, nullabilityBuffer);
          }
        default:
          throw new IllegalArgumentException(
              "The specified data type for the field is currently not supported: " + scalarType);
      }
    }

    /* Handle struct which currently only supports strings as nested fields for supporting shared dictionary encoding */
    if (numStreams == 1) {
      throw new IllegalArgumentException("Present stream currently not supported for Structs.");
    }

    return VectorizedStringDecoder.decodeSharedDictionaryToRandomAccessFormat(
        data, offset, column, numFeatures);
  }
}
