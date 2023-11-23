package com.covt.evaluation;

import com.covt.evaluation.compression.IntegerCompression;
import me.lemire.integercompression.IntWrapper;
import org.apache.commons.lang3.ArrayUtils;
import org.apache.orc.impl.BufferChunk;
import org.apache.orc.impl.RunLengthByteReader;
import org.apache.orc.impl.RunLengthIntegerReader;
import org.apache.parquet.bytes.ByteBufferInputStream;
import org.apache.parquet.column.values.rle.RunLengthBitPackingHybridDecoder;

import java.io.IOException;
import java.nio.ByteBuffer;
import java.nio.charset.StandardCharsets;
import java.util.Arrays;

import static org.apache.orc.impl.InStream.create;

public class ConversionUtils {

    //Source: https://github.com/bazelbuild/bazel/blob/master/src/main/java/com/google/devtools/build/lib/util/VarInt.java
    public static byte[] varintEncode(int[] values) {
        var varintBuffer = new byte[values.length * 4];
        var i = 0;
        for(var value : values){
            i = putVarInt(value, varintBuffer, i);
        }
        return Arrays.copyOfRange(varintBuffer, 0, i);
    }

    //Source: https://github.com/bazelbuild/bazel/blob/master/src/main/java/com/google/devtools/build/lib/util/VarInt.java
    /**
     * Encodes an integer in a variable-length encoding, 7 bits per byte, into a destination byte[],
     * following the protocol buffer convention.
     *
     * @param v the int value to write to sink
     * @param sink the sink buffer to write to
     * @param offset the offset within sink to begin writing
     * @return the updated offset after writing the varint
     */
    private static int putVarInt(int v, byte[] sink, int offset) {
        do {
            // Encode next 7 bits + terminator bit
            int bits = v & 0x7F;
            v >>>= 7;
            byte b = (byte) (bits + ((v != 0) ? 0x80 : 0));
            sink[offset++] = b;
        } while (v != 0);
        return offset;
    }

    public static byte[] varintEncode(long[] values) {
        var varintBuffer = new byte[values.length * 4];
        var i = 0;
        for(var value : values){
            i = putVarInt(value, varintBuffer, i);
        }
        return Arrays.copyOfRange(varintBuffer, 0, i);
    }

    public static int zigZagEncode(int value){
        return (value >> 31) ^ (value << 1);
    }

    //Source: https://github.com/bazelbuild/bazel/blob/master/src/main/java/com/google/devtools/build/lib/util/VarInt.java
    /**
     * Encodes an integer in a variable-length encoding, 7 bits per byte, into a destination byte[],
     * following the protocol buffer convention.
     *
     * @param v the int value to write to sink
     * @param sink the sink buffer to write to
     * @param offset the offset within sink to begin writing
     * @return the updated offset after writing the varint
     */
    private static int putVarInt(long v, byte[] sink, int offset) {
        do {
            // Encode next 7 bits + terminator bit
            long bits = v & 0x7F;
            v >>>= 7;
            byte b = (byte) (bits + ((v != 0) ? 0x80 : 0));
            sink[offset++] = b;
        } while (v != 0);
        return offset;
    }

    /*
    * Varint encode the length of the string and UTF-8 encode the string content.
    * */
    public static byte[] encodeString(String value) throws IOException {
        var utf8Data = value.getBytes(StandardCharsets.UTF_8);
        var stringLength = varintEncode(new int[]{utf8Data.length});
        return ArrayUtils.addAll(stringLength, utf8Data);
    }

    public static String decodeString(byte[] content, IntWrapper pos) throws IOException {
        var stringLength = decodeVarint(content, pos)[0];
        var str = new String(content, pos.get(), stringLength, StandardCharsets.UTF_8);
        pos.set(pos.get() + stringLength);
        return str;
    }

    public static String decodeString(byte[] content, IntWrapper pos, int numChars) throws IOException {
        var str = new String(content, pos.get(), numChars, StandardCharsets.UTF_8);
        pos.set(pos.get() + numChars);
        return str;
    }

    //TODO: quick and dirty -> optimize for performance
    public static int[] decodeVarint(byte[] src, IntWrapper pos){
        var values = new int[1];
        var offset = decodeVarint(src, pos.get(), values);
        pos.set(offset);
        return values;
    }

    public static int decodeZigZagVarint(byte[] src, IntWrapper pos){
        var value = decodeVarint(src, pos);
        return zigZagDecode(value[0]);
    }

    public static int zigZagDecode(int encoded) {
        return (encoded >>> 1) ^ (-(encoded & 1));
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

    public static long[] decodeVarintLong(byte[] src, IntWrapper pos){
        var values = new long[1];
        var offset = decodeVarint(src, pos.get(), values);
        pos.set(pos.get() + offset);
        return values;
    }

    private static int decodeVarint(byte[] src, int offset, long[] dst) {
        int result = 0;
        int shift = 0;
        int b;
        do {
            // Get 7 bits from next byte
            b = src[offset++];
            result |= (b & 0x7F) << shift;
            shift += 7;
        } while ((b & 0x80) != 0);
        dst[0] = result;
        return offset;
    }

    public static long[] decodeOrcRleEncodingV1(byte[] buffer, int numValues, IntWrapper pos) throws IOException {
        var inStream = create
                ("test", new BufferChunk(ByteBuffer.wrap(buffer), 0), pos.get(), buffer.length);
        var reader =
                new RunLengthIntegerReader(inStream, false);

        var values = new long[numValues];
        for(var i = 0; i < numValues; i++){
            values[i] = reader.next();
        }

        //TODO: quick and dirty -> find proper and performant solution of how to get the offset
        var size = getOrcRleV1ChunkSize(values);
        pos.set(pos.get() + size);
        return values;
    }

    private static int getOrcRleV1ChunkSize(long[] values) throws IOException {
        return IntegerCompression.orcRleEncodingV1(values).length;
    }

    public static int[] decodeParquetRleBitpackingHybrid(byte[] src, int numValues, IntWrapper pos) throws IOException {
        ByteBufferInputStream in = ByteBufferInputStream.wrap(ByteBuffer.wrap(src, pos.get(),   src.length - pos.get()));
        var decoder = new RunLengthBitPackingHybridDecoder(3, in);

        var values = new int[numValues];
        for(var i = 0; i < numValues; i++){
            values[i] = decoder.readInt();
        }

        return values;
    }

    public static int[] varintZigZagDecode(byte[] covtBuffer, IntWrapper pos, int numValues){
        var values = new int[numValues];
        for(var i = 0; i < numValues; i++){
            var value = decodeZigZagVarint(covtBuffer, pos);
            values[i] = value;
        }

        return values;
    }

    public static int[] varintZigZagDeltaDecode(byte[] covtBuffer, IntWrapper pos, int numValues){
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

    public static int[] varintZigZagDeltaDecodeCoordinates(byte[] covtBuffer, IntWrapper pos, int numValues){
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

    public static byte[] decodeOrcRleByteEncodingV1(byte[] buffer, int numValues, IntWrapper pos) throws IOException {
        var inStream = create
                ("test", new BufferChunk(ByteBuffer.wrap(buffer), 0), pos.get(), buffer.length);
        var reader =
                new RunLengthByteReader(inStream);

        var values = new byte[numValues];
        for(var i = 0; i < numValues; i++){
            values[i] = reader.next();
        }

        var size = getOrcRleByteV1ChunkSize(values);
        pos.set(pos.get() + size);
        return values;
    }

    private static int getOrcRleByteV1ChunkSize(byte[] values) throws IOException {
        return IntegerCompression.orcRleByteEncodingV1(values).length;
    }

    //TODO:  For best performance, use it using the ByteIntegerCODEC interface.
    /*public static int[] decodeVarint(byte[] varintEncodedBuffer, int numValues, IntWrapper pos){
        var variableByte = new VariableByte();
        var values = new int[numValues];
        var inpos = pos.get();
        var outpos = new IntWrapper(0);
        variableByte.uncompress(varintEncodedBuffer, pos, numValues, values, outpos);
        //TODO: quick and dirty -> find proper approach -> VariableByte seems not to return the valid offset
        var numBytes = 0;
        for(var i = 0; i < numValues; i++){
            numBytes += (int)Math.ceil((log2(values[i]) + 2) / 8);
        }
        pos.set(inpos + numBytes);

        return values;
     }

    public static int log2(int N)
    {
        return (int)(Math.log(N) / Math.log(2));
    }*/

}
