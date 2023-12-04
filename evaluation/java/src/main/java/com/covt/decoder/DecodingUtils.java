package com.covt.decoder;

import com.covt.converter.EncodingUtils;
import com.covt.converter.GeometryUtils;
import me.lemire.integercompression.*;
import org.apache.orc.impl.BufferChunk;
import org.apache.orc.impl.InStream;
import org.apache.orc.impl.RunLengthByteReader;
import org.apache.orc.impl.RunLengthIntegerReader;

import java.io.IOException;
import java.nio.ByteBuffer;
import java.nio.ByteOrder;
import java.nio.IntBuffer;
import java.nio.charset.StandardCharsets;
import java.util.Arrays;

public final class DecodingUtils {
    private DecodingUtils(){}

    public static String decodeString(byte[] content, IntWrapper pos) {
        var stringLength = decodeVarint(content, pos)[0];
        var str = new String(content, pos.get(), stringLength, StandardCharsets.UTF_8);
        pos.set(pos.get() + stringLength);
        return str;
    }

    public static String decodeString(byte[] content, IntWrapper pos, int numChars) {
        var str = new String(content, pos.get(), numChars, StandardCharsets.UTF_8);
        pos.set(pos.get() + numChars);
        return str;
    }

    //TODO: quick and dirty -> optimize for performance
    public static int[] decodeVarint(byte[] src, IntWrapper pos, int numValues){
        var values = new int[numValues];
        for(var i = 0; i < numValues; i++){
            var offset = decodeVarint(src, pos.get(), values);
            pos.set(offset);
        }
        return values;
    }

    public static int[] decodeZigZagVarint(byte[] covtBuffer, IntWrapper pos, int numValues){
        var values = new int[numValues];
        for(var i = 0; i < numValues; i++){
            values[i] = decodeZigZagVarint(covtBuffer, pos);
        }

        return values;
    }

    public static int[] decodeZigZagDeltaVarint(byte[] covtBuffer, IntWrapper pos, int numValues){
        var values = new int[numValues];
        var previousValue = 0;
        for(var i = 0; i < numValues; i++){
            var delta = decodeZigZagVarint(covtBuffer, pos);
            var value = previousValue + delta;
            values[i] = value;
            previousValue = value;
        }

        return values;
    }

    /*public static long[] decodeLongVarint(byte[] src, IntWrapper pos, int numValues, int byteSize){
        //TODO: get rid of that byte buffer creation
        var values = new long[numValues];
        for(var i = 0; i < numValues; i++){
            values[i] = decodeLongVarint(ByteBuffer.wrap(src, pos.get() ));

        }

        //TODO: refactor -> get byte size from decoding
        pos.set(pos.get() + byteSize);
        return values;
    }

    public static long[] decodeZigZagLongVarint(byte[] src, IntWrapper pos, int numValues, int byteSize){
        //TODO: get rid of that byte buffer creation
        var values = new long[numValues];
        for(var i = 0; i < numValues; i++){
            var varint = decodeLongVarint(ByteBuffer.wrap())
            values[i] = decodeZigZagLongVarint(ByteBuffer.wrap(src));

        }

        //TODO: refactor -> get byte size from decoding
        pos.set(pos.get() + byteSize);
        return values;
    }*/

    public static int[] decodeZigZagDeltaVarintCoordinates(byte[] covtBuffer, IntWrapper pos, int numValues){
        var values = new int[numValues];
        var previousValueX = 0;
        var previousValueY = 0;
        for(var i = 0; i < numValues; i+=2){
            var deltaX = decodeZigZagVarint(covtBuffer, pos);
            var deltaY = decodeZigZagVarint(covtBuffer, pos);
            var x = previousValueX + deltaX;
            var y = previousValueY + deltaY;
            values[i] = x;
            values[i+1] = y;

            previousValueX = x;
            previousValueY = y;
        }

        return values;
    }

    //Source: https://github.com/bazelbuild/bazel/blob/master/src/main/java/com/google/devtools/build/lib/util/VarInt.java
    /**
     * Reads a varint from src, places its values into the first element of dst and returns the offset
     * in to src of the first byte after the varint.
     *
     * @param src source buffer to retrieve from
     * @param offset offset within src
     * @param dst the resulting int values
     * @return the updated offset after reading the varint
     */
    private static int decodeVarint(byte[] src, int offset, int[] dst) {
        var dstOffset = 0;

        /*
         * Max 4 bytes supported.
         * */
        var b= src[offset++];
        var value = b & 0x7f;
        if ((b & 0x80) == 0) {
            dst[dstOffset] = value;
            return offset;
        }

        b = src[offset++];
        value |= (b & 0x7f) << 7;
        if ((b & 0x80) == 0) {
            dst[dstOffset] = value;
            return offset;
        }

        b = src[offset++];
        value |= (b & 0x7f) << 14;
        if ((b & 0x80) == 0) {
            dst[dstOffset] = value;
            return offset;
        }

        b = src[offset++];
        value |= (b & 0x7f) << 21;
        dst[dstOffset] = value;
        return offset;
    }

    //TODO: quick and dirty -> optimize for performance
    private static int[] decodeVarint(byte[] src, IntWrapper pos){
        var values = new int[1];
        var offset = decodeVarint(src, pos.get(), values);
        pos.set(offset);
        return values;
    }

    /* Source: https://github.com/bazelbuild/bazel/blob/master/src/main/java/com/google/devtools/build/lib/util/VarInt.java */
    private static long decodeLongVarint(ByteBuffer src) {
        long tmp;
        if ((tmp = src.get()) >= 0) {
            return tmp;
        }
        long result = tmp & 0x7f;
        if ((tmp = src.get()) >= 0) {
            result |= tmp << 7;
        } else {
            result |= (tmp & 0x7f) << 7;
            if ((tmp = src.get()) >= 0) {
                result |= tmp << 14;
            } else {
                result |= (tmp & 0x7f) << 14;
                if ((tmp = src.get()) >= 0) {
                    result |= tmp << 21;
                } else {
                    result |= (tmp & 0x7f) << 21;
                    if ((tmp = src.get()) >= 0) {
                        result |= tmp << 28;
                    } else {
                        result |= (tmp & 0x7f) << 28;
                        if ((tmp = src.get()) >= 0) {
                            result |= tmp << 35;
                        } else {
                            result |= (tmp & 0x7f) << 35;
                            if ((tmp = src.get()) >= 0) {
                                result |= tmp << 42;
                            } else {
                                result |= (tmp & 0x7f) << 42;
                                if ((tmp = src.get()) >= 0) {
                                    result |= tmp << 49;
                                } else {
                                    result |= (tmp & 0x7f) << 49;
                                    if ((tmp = src.get()) >= 0) {
                                        result |= tmp << 56;
                                    } else {
                                        result |= (tmp & 0x7f) << 56;
                                        result |= ((long) src.get()) << 63;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        return result;
    }

    private static int decodeZigZagVarint(byte[] src, IntWrapper pos){
        var value = decodeVarint(src, pos);
        return decodeZigZag(value[0]);
    }

    private static int decodeZigZag(int encoded) {
        return (encoded >>> 1) ^ (-(encoded & 1));
    }

    /* Based on ORC RLE V1 encoding */
    public static long[] decodeRle(byte[] buffer, int numValues, IntWrapper pos, boolean signed) throws IOException {
        var inStream = InStream.create
                ("test", new BufferChunk(ByteBuffer.wrap(buffer), 0), pos.get(), buffer.length);
        var reader =
                new RunLengthIntegerReader(inStream, signed);

        var values = new long[numValues];
        for(var i = 0; i < numValues; i++){
            values[i] = reader.next();
        }

        //TODO: quick and dirty -> find proper and performant solution of how to get the offset
        var size = getRleChunkSize(values, signed);
        pos.set(pos.get() + size);
        return values;
    }

    /* Based on ORC Byte RLE V1 encoding */
    public static byte[] decodeByteRle(byte[] buffer, int numValues, IntWrapper pos, int byteLength) throws IOException {
        var inStream = InStream.create
                ("test", new BufferChunk(ByteBuffer.wrap(buffer), 0), pos.get(), buffer.length);
        var reader =
                new RunLengthByteReader(inStream);

        var values = new byte[numValues];
        for(var i = 0; i < numValues; i++){
            values[i] = reader.next();
        }

        pos.set(pos.get() + byteLength);
        return values;
    }

    private static int getRleChunkSize(long[] values, boolean signed) throws IOException {
        return EncodingUtils.encodeRle(values, signed).length;
    }

    public static int[] decodeFastPfor128ZigZagDelta(byte[] encodedValues, int numValues, int byteLength, IntWrapper pos){
        var encodedValuesSlice = Arrays.copyOfRange(encodedValues, pos.get(), pos.get() + byteLength);
        //TODO: get rid of that conversion
        IntBuffer intBuf =
                ByteBuffer.wrap(encodedValuesSlice)
                        //TODO: change to little endian
                        .order(ByteOrder.BIG_ENDIAN)
                        .asIntBuffer();
        int[] intValues = new int[(int)Math.ceil(byteLength / 4)];
        for(var i = 0; i < intValues.length; i++){
            intValues[i] = intBuf.get(i);
        }

        int[] decompressedValues = new int[numValues];
        var inputOffset = new IntWrapper(0);
        var outputOffset = new IntWrapper(0);
        IntegerCODEC ic = new Composition(new FastPFOR(), new VariableByte());
        ic.uncompress(intValues, inputOffset, intValues.length, decompressedValues, outputOffset);

        var decodedValues = new int[numValues];
        var previousValue = 0;
        for(var i = 0; i < numValues; i++){
            var zigZagValue = decompressedValues[i];
            var deltaValue = decodeZigZag(zigZagValue);
            var value = previousValue + deltaValue;
            decodedValues[i] = value;
            previousValue = value;
        }

        pos.set(pos.get() +  byteLength);
        return decodedValues;
    }

    public static int[] decodeFastPfor128DeltaCoordinates(byte[] encodedValues, int numValues, int byteLength, IntWrapper pos){
        var encodedValuesSlice = Arrays.copyOfRange(encodedValues, pos.get(), pos.get() + byteLength);
        //TODO: get rid of that conversion
        IntBuffer intBuf =
                ByteBuffer.wrap(encodedValuesSlice)
                        //TODO: change to little endian
                        .order(ByteOrder.BIG_ENDIAN)
                        .asIntBuffer();
        int[] intValues = new int[(int)Math.ceil(byteLength / 4)];
        for(var i = 0; i < intValues.length; i++){
            intValues[i] = intBuf.get(i);
        }

        int[] decompressedValues = new int[numValues];
        var inputOffset = new IntWrapper(0);
        var outputOffset = new IntWrapper(0);
        IntegerCODEC ic = new Composition(new FastPFOR(), new VariableByte());
        ic.uncompress(intValues, inputOffset, intValues.length, decompressedValues, outputOffset);

        var decodedValues = new int[numValues];
        for(var i = 0; i < numValues; i++){
            var zigZagValue = decompressedValues[i];
            decodedValues[i]  = (zigZagValue >>> 1) ^ (-(zigZagValue & 1));
        }

        pos.set(pos.get() +  byteLength);

        var values = new int[numValues];
        var previousValueX = 0;
        var previousValueY = 0;
        for(var i = 0; i < numValues; i+=2){
            var deltaX = decodedValues[i];
            var deltaY = decodedValues[i+1];
            var x = previousValueX + deltaX;
            var y = previousValueY + deltaY;
            values[i] = x;
            values[i+1] = y;

            previousValueX = x;
            previousValueY = y;
        }

        return values;
    }

    public static int[] decodeDeltaVarintMortonCodes(byte[] covtBuffer, IntWrapper pos, int numVertices, int numBits){
        var vertices = new int[numVertices * 2];
        var previousMortonCode = 0;
        for(var i = 0; i < numVertices; i++){
            var delta = decodeVarint(covtBuffer, pos)[0];
            var mortonCode = previousMortonCode + delta;

            var vertex = GeometryUtils.decodeMorton(mortonCode, numBits);
            vertices[i * 2] = vertex[0];
            vertices[i * 2 + 1] = vertex[1];

            previousMortonCode = mortonCode;
        }

        return vertices;
    }

    public static int[] decodeFastPfor128DeltaMortonCodes(byte[] encodedValues, int numVertices, int byteLength, IntWrapper pos, int numBits){
        var encodedValuesSlice = Arrays.copyOfRange(encodedValues, pos.get(), pos.get() + byteLength);
        //TODO: get rid of that conversion
        IntBuffer intBuf =
                ByteBuffer.wrap(encodedValuesSlice)
                        //TODO: change to little endian
                        .order(ByteOrder.BIG_ENDIAN)
                        .asIntBuffer();
        int[] intValues = new int[(int)Math.ceil(byteLength / 4)];
        for(var i = 0; i < intValues.length; i++){
            intValues[i] = intBuf.get(i);
        }

        int[] decompressedValues = new int[numVertices];
        var inputOffset = new IntWrapper(0);
        var outputOffset = new IntWrapper(0);
        IntegerCODEC ic = new Composition(new FastPFOR(), new VariableByte());
        ic.uncompress(intValues, inputOffset, intValues.length, decompressedValues, outputOffset);

        pos.set(pos.get() +  byteLength);

        var vertices = new int[numVertices * 2];
        var previousMortonCode = 0;
        for(var i = 0; i < numVertices; i++){
            var mortonCode = previousMortonCode + decompressedValues[i];
            var vertex = GeometryUtils.decodeMorton(mortonCode, numBits);
            vertices[i * 2] = vertex[0];
            vertices[i * 2 + 1] = vertex[1];

            previousMortonCode = mortonCode;
        }

        return vertices;
    }

    public static float[] decodeFloatsLE(byte[] encodedValues, IntWrapper pos, int numValues){
        var fb = ByteBuffer.wrap(encodedValues, pos.get(), numValues * 4).
                order(ByteOrder.LITTLE_ENDIAN).asFloatBuffer();
        pos.set(pos.get() + numValues * 4);
        var decodedValues = new float[fb.limit()];
        fb.get(decodedValues);
        return decodedValues;
    }
}
