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
import com.mlt.vector.flat.FloatFlatVector;
import com.mlt.vector.flat.IntFlatVector;
import com.mlt.vector.flat.LongFlatVector;
import me.lemire.integercompression.IntWrapper;

import java.io.IOException;

enum BufferType{
    CONST,
    SEQUENCE,
    FLAT
}

public class VectorizedPropertyDecoder {
    private VectorizedPropertyDecoder(){}

    public static Vector decodePropertyColumn(byte[] data, IntWrapper offset, MltTilesetMetadata.Column column, int numStreams) throws IOException {
        StreamMetadata presentStreamMetadata;
        if(column.hasScalarType()){
            BitVector presentStream = null;
            var numValues = 0;
            if(numStreams > 1){
                presentStreamMetadata = StreamMetadataDecoder.decode(data, offset);
                numValues = presentStreamMetadata.numValues();
                var presentVector = VectorizedDecodingUtils.decodeBooleanRle(data, numValues, offset);
                presentStream = new BitVector(presentVector, presentStreamMetadata.numValues());
            }

            var scalarType = column.getScalarType();
            switch (scalarType.getPhysicalType()){
                case BOOLEAN: {
                    var dataStreamMetadata = StreamMetadataDecoder.decode(data, offset);
                    var dataStream = VectorizedDecodingUtils.decodeBooleanRle(data, dataStreamMetadata.numValues(), offset);
                    var dataVector = new BitVector(dataStream, dataStreamMetadata.numValues());
                    return presentStream != null ? new BooleanFlatVector(column.getName(), presentStream, dataVector) :
                            new BooleanFlatVector(column.getName(), dataVector);
                }
                case UINT_32:
                case INT_32: {
                        var dataStreamMetadata = StreamMetadataDecoder.decode(data, offset);
                        var dataStream = VectorizedIntegerDecoder.decodeIntStream(data, offset, dataStreamMetadata,
                                scalarType.getPhysicalType() == MltTilesetMetadata.ScalarType.INT_32);
                        return presentStream != null ? new IntFlatVector(column.getName(), presentStream, dataStream) :
                                new IntFlatVector(column.getName(), dataStream);
                }
                case UINT_64:
                case INT_64: {
                    var dataStreamMetadata = StreamMetadataDecoder.decode(data, offset);
                    var dataStream = VectorizedIntegerDecoder.decodeLongStream(data, offset, dataStreamMetadata,
                            scalarType.getPhysicalType() == MltTilesetMetadata.ScalarType.INT_32);
                    return presentStream != null ? new LongFlatVector(column.getName(), presentStream, dataStream) :
                            new LongFlatVector(column.getName(), dataStream);
                }
                case FLOAT:{
                    var dataStreamMetadata = StreamMetadataDecoder.decode(data, offset);
                    var dataStream = VectorizedFloatDecoder.decodeFloatStream(data, offset, dataStreamMetadata);
                    return presentStream != null ? new FloatFlatVector(column.getName(), presentStream, dataStream) :
                            new FloatFlatVector(column.getName(), dataStream);
                }
                /*case DOUBLE:{
                    break;
                }*/
                case STRING: {
                    return VectorizedStringDecoder.decode(column.getName(), data, offset, numStreams - 1, presentStream);
                }
                default:
                    throw new IllegalArgumentException("The specified data type for the field is currently not supported.");
            }
        }

        /* Handle struct which currently only supports strings as nested fields for supporting shared dictionary encoding */
        if (numStreams == 1) {
            throw new IllegalArgumentException("Present stream currently not supported for Structs.");
        }

        return VectorizedStringDecoder.decodeSharedDictionary(data, offset, column);
    }

    public static Vector decodeToRandomAccessFormat(byte[] data, IntWrapper offset, MltTilesetMetadata.Column column,
                                                          int numStreams, int numFeatures) throws IOException {
        StreamMetadata presentStreamMetadata;
        if(column.hasScalarType()){
            BitVector nullabilityBuffer = null;
            var numValues = 0;
            if(numStreams > 1){
                presentStreamMetadata = StreamMetadataDecoder.decode(data, offset);
                //TODO: get rid of that check by not including the present stream if not nullable
                var vectorType = VectorizedDecodingUtils.getVectorTypeBooleanStream(numFeatures, presentStreamMetadata.byteLength());
                /** If vector type equals const create vector without a nullabilityBuffer which specifies
                 *  that the column is not nullable */
                if(vectorType == VectorType.FLAT){
                    numValues = presentStreamMetadata.numValues();
                    var presentVector = VectorizedDecodingUtils.decodeBooleanRle(data, numValues, offset);
                    nullabilityBuffer = new BitVector(presentVector, presentStreamMetadata.numValues());
                }
            }

            var scalarType = column.getScalarType();
            switch (scalarType.getPhysicalType()){
                case BOOLEAN: {
                    var dataStreamMetadata = StreamMetadataDecoder.decode(data, offset);
                    var vectorType = VectorizedDecodingUtils.getVectorTypeBooleanStream(numFeatures, dataStreamMetadata.byteLength());
                    boolean isNullable = dataStreamMetadata.numValues() != numFeatures;
                    if(vectorType.equals(VectorType.FLAT)){
                        if(!isNullable){
                            var dataStream = VectorizedDecodingUtils.decodeBooleanRle(data, dataStreamMetadata.numValues(), offset);
                            var dataVector = new BitVector(dataStream, dataStreamMetadata.numValues());
                            return new BooleanFlatVector(column.getName(), nullabilityBuffer, dataVector);
                        }

                        throw new IllegalArgumentException("Nullable boolean RLE ist not supported yet.");
                    }
                    else{
                        //handle const
                        throw new IllegalArgumentException("ConstBooleanVector ist not supported yet.");
                    }

                }
                case UINT_32:
                case INT_32: {
                    var dataStreamMetadata = StreamMetadataDecoder.decode(data, offset);
                    var vectorType = VectorizedDecodingUtils.getVectorTypeIntStream(dataStreamMetadata);
                    boolean isNullable = dataStreamMetadata.numValues() != numFeatures;
                    var isSigned = scalarType.getPhysicalType() == MltTilesetMetadata.ScalarType.INT_32;
                    if(vectorType.equals(VectorType.FLAT)){
                        if(!isNullable){
                            var dataStream = VectorizedIntegerDecoder.decodeIntStream(data, offset, dataStreamMetadata,
                                    isSigned);
                            return new IntFlatVector(column.getName(), dataStream);
                        }

                        var dataStream = VectorizedIntegerDecoder.decodeNullableIntStream(data, offset, dataStreamMetadata,
                                isSigned, nullabilityBuffer);
                        return new IntFlatVector(column.getName(), nullabilityBuffer, dataStream);
                    }
                    else{
                        /** handle ConstVector */
                        var constValue = VectorizedIntegerDecoder.decodeConstIntStream(data, offset, dataStreamMetadata,
                                isSigned);
                        return isNullable? new IntConstVector(column.getName(), nullabilityBuffer, constValue) :
                                new IntConstVector(column.getName(), constValue);
                    }
                }
                case UINT_64:
                case INT_64: {
                    var dataStreamMetadata = StreamMetadataDecoder.decode(data, offset);
                    var vectorType = VectorizedDecodingUtils.getVectorTypeIntStream(dataStreamMetadata);
                    boolean isNullable = dataStreamMetadata.numValues() != numFeatures;
                    var isSigned = scalarType.getPhysicalType() == MltTilesetMetadata.ScalarType.INT_32;
                    if(vectorType.equals(VectorType.FLAT)){
                        if(!isNullable){
                            var dataStream = VectorizedIntegerDecoder.decodeLongStream(data, offset, dataStreamMetadata,
                                    isSigned);
                            return new LongFlatVector(column.getName(), dataStream);
                        }

                        var dataStream = VectorizedIntegerDecoder.decodeNullableLongStream(data, offset, dataStreamMetadata,
                                isSigned, nullabilityBuffer);
                        return new LongFlatVector(column.getName(), nullabilityBuffer, dataStream);
                    }
                    else{
                        /** handle ConstVector */
                        var constValue = VectorizedIntegerDecoder.decodeConstLongStream(data, offset, dataStreamMetadata,
                                isSigned);
                        return isNullable? new LongConstVector(column.getName(), nullabilityBuffer, constValue) :
                                new LongConstVector(column.getName(), constValue);
                    }
                }
                case FLOAT:{
                    var dataStreamMetadata = StreamMetadataDecoder.decode(data, offset);
                    boolean isNullable = dataStreamMetadata.numValues() != numFeatures;
                    if(!isNullable){
                        var dataStream = VectorizedFloatDecoder.decodeFloatStream(data, offset, dataStreamMetadata);
                        return new FloatFlatVector(column.getName(), dataStream);
                    }

                    var dataStream = VectorizedFloatDecoder.decodeNullableFloatStream(data, offset, dataStreamMetadata,
                            nullabilityBuffer);
                    return new FloatFlatVector(column.getName(), nullabilityBuffer, dataStream);
                }
                /*case DOUBLE:{
                    break;
                }*/
                case STRING: {
                    return VectorizedStringDecoder.decodeToRandomAccessFormat(
                            column.getName(), data, offset, numStreams - 1, nullabilityBuffer, numFeatures);
                }
                default:
                    throw new IllegalArgumentException("The specified data type for the field is currently not supported.");
            }
        }

        /* Handle struct which currently only supports strings as nested fields for supporting shared dictionary encoding */
        if (numStreams == 1) {
            throw new IllegalArgumentException("Present stream currently not supported for Structs.");
        }

        return VectorizedStringDecoder.decodeSharedDictionaryToRandomAccessFormat(data, offset, column, numFeatures);
    }

}
