package com.covt.converter;

import com.covt.evaluation.compression.TestOutputCatcher;
import me.lemire.integercompression.FastPFOR128;
import me.lemire.integercompression.IntWrapper;
import org.apache.commons.lang3.ArrayUtils;
import org.apache.orc.impl.OutStream;
import org.apache.orc.impl.RunLengthByteWriter;
import org.apache.orc.impl.RunLengthIntegerWriter;
import org.apache.orc.impl.writer.StreamOptions;

import java.io.ByteArrayOutputStream;
import java.io.IOException;
import java.nio.charset.StandardCharsets;
import java.util.Arrays;
import java.util.BitSet;
import java.util.List;
import java.util.zip.GZIPOutputStream;

public class EncodingUtils {

    //Source: https://github.com/bazelbuild/bazel/blob/master/src/main/java/com/google/devtools/build/lib/util/VarInt.java
    public static byte[] encodeVarints(long[] values) {
        var varintBuffer = new byte[values.length * 8];
        var i = 0;
        for(var value : values){
            i = putVarInt(value, varintBuffer, i);
        }
        return Arrays.copyOfRange(varintBuffer, 0, i);
    }

    public static byte[] encodeDeltaVarints(long[] values){
        var zigZagDeltaValues = encodeZigZagDelta(values);
        return EncodingUtils.encodeVarints(zigZagDeltaValues);
    }

    public static byte[] encodeZigZagVarints(int[] values){
        var zigZagValues = new long[values.length];
        var i = 0;
        for(var value : values){
            zigZagValues[i++] = EncodingUtils.encodeZigZag(value);
        }

        return EncodingUtils.encodeVarints(zigZagValues);
    }

    public static byte[] encodeZigZagVarints(long[] values){
        var zigZagValues = new long[values.length];
        var i = 0;
        for(var value : values){
            zigZagValues[i++] = EncodingUtils.encodeZigZag(value);
        }

        return EncodingUtils.encodeVarints(zigZagValues);
    }

    public static long encodeZigZag(long value){
        return (value << 1) ^ (value >> 63);
    }

    public static int encodeZigZag(int value){
        return (value >> 31) ^ (value << 1);
    }

    public static byte[] encodeString(String value) throws IOException {
        var utf8Data = value.getBytes(StandardCharsets.UTF_8);
        var stringLength = encodeVarints(new long[]{utf8Data.length});
        return ArrayUtils.addAll(stringLength, utf8Data);
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

    public static byte[] encodeRle(long[] values, boolean signed) throws IOException {
        var testOutputCatcher = new TestOutputCatcher();
        var writer =
                new RunLengthIntegerWriter(new OutStream("test", new StreamOptions(1), testOutputCatcher), signed);

        for(var value: values) {
            writer.write(value);
        }

        writer.flush();
        return testOutputCatcher.getBuffer();
    }

    public static byte[] encodeDeltaRle(long[] values) throws IOException {
        /*var zigZagDeltaValues = encodeZigZagDelta(values);
        return encodeRle(zigZagDeltaValues, true);*/
        //TODO: check if this work
        return encodeRle(values, true);
    }

    public static byte[] encodeByteRle(byte[] values) throws IOException {
        var testOutputCatcher = new TestOutputCatcher();
        var writer =
                new RunLengthByteWriter(new OutStream("test", new StreamOptions(1), testOutputCatcher));

        for(var value: values) {
            writer.write(value);
        }

        writer.flush();
        return testOutputCatcher.getBuffer();
    }

    public static long[] encodeZigZagDelta(long[] values){
        var zigZagDeltaValues = new long[values.length];
        var previousValue = 0l;
        var i = 0;
        for(var value : values){
            var delta = value - previousValue;
            zigZagDeltaValues[i++] = EncodingUtils.encodeZigZag(delta);
            previousValue = value;
        }

        return  zigZagDeltaValues;
    }

    public static byte[] encodeDeltaFastPfor128(long[] values){
        var previousValue = 0;
        var deltaValues = new int[values.length];
        var j = 0;
        for(var value : values){
            var delta = (int)value - previousValue;
            var zigZagDelta = encodeZigZag(delta);
            deltaValues[j++] = zigZagDelta;
            previousValue = (int)value;
        }

        /*
         * Note that this does not use differential coding: if you are working on sorted * lists,
         * you should first compute deltas, @see me.lemire.integercompression.differential.Delta#delta
         * */
        //TODO: also test VectorFastPFOR -> patched version which should be faster

        var fastPfor = new FastPFOR128();
        IntWrapper inputoffset = new IntWrapper(0);
        IntWrapper outputoffset = new IntWrapper(0);
        int [] compressed = new int[deltaValues.length+1024];
        fastPfor.compress(deltaValues, inputoffset, deltaValues.length, compressed, outputoffset);
        var totalSize = outputoffset.intValue()*4;

        var compressedBuffer = new byte[totalSize];
        var valueCounter = 0;
        for(var i = 0; i < totalSize; i+=4){
            var value = compressed[valueCounter++];
            var val1 = (byte)(value >>> 24);
            var val2 = (byte)(value >>> 16);
            var val3 = (byte)(value >>> 8);
            var val4 = (byte)value;

            compressedBuffer[i] = val1;
            compressedBuffer[i+1] = val2;
            compressedBuffer[i+2] = val3;
            compressedBuffer[i+3] = val4;
        }

        return compressedBuffer;
    }

    public static byte[] encodeZigZagFastPfor128(int[] values){
        var previousValue = 0;
        var deltaValues = new int[values.length];
        var j = 0;
        for(var value : values){
            var zigZagDelta = encodeZigZag(value);
            deltaValues[j++] = (int)zigZagDelta;
            previousValue = (int)value;
        }

        /*
         * Note that this does not use differential coding: if you are working on sorted * lists,
         * you should first compute deltas, @see me.lemire.integercompression.differential.Delta#delta
         * */
        //TODO: also test VectorFastPFOR -> patched version which should be faster

        var fastPfor = new FastPFOR128();
        IntWrapper inputoffset = new IntWrapper(0);
        IntWrapper outputoffset = new IntWrapper(0);
        int [] compressed = new int[deltaValues.length+1024];
        fastPfor.compress(deltaValues, inputoffset, deltaValues.length, compressed, outputoffset);
        var totalSize = outputoffset.intValue()*4;

        var compressedBuffer = new byte[totalSize];
        var valueCounter = 0;
        for(var i = 0; i < totalSize; i+=4){
            var value = compressed[valueCounter++];
            var val1 = (byte)(value >>> 24);
            var val2 = (byte)(value >>> 16);
            var val3 = (byte)(value >>> 8);
            var val4 = (byte)value;

            compressedBuffer[i] = val1;
            compressedBuffer[i+1] = val2;
            compressedBuffer[i+2] = val3;
            compressedBuffer[i+3] = val4;
        }

        return compressedBuffer;
    }

    public static byte[] encodeBooleans(List<Boolean> present) throws IOException {
        BitSet bitSet = new BitSet(present.size());
        var j = 0;
        for(var p : present){
            bitSet.set(j++, p);
        }

        var presentStream = bitSet.toByteArray();
        /* The BitSet only returns the bytes until the last set bit */
        var numMissingBytes = (int)Math.ceil(present.size() / 8d) - presentStream.length;
        if(numMissingBytes != 0){
            var paddingBytes = new byte[numMissingBytes];
            Arrays.fill(paddingBytes, (byte)0);
            presentStream = ArrayUtils.addAll(presentStream, paddingBytes);
        }

        return EncodingUtils.encodeByteRle(presentStream);
    }

    public static byte[] gzipCompress(byte[] buffer) throws IOException {
        ByteArrayOutputStream baos = new ByteArrayOutputStream();
        GZIPOutputStream gzipOut = new GZIPOutputStream(baos);
        gzipOut.write(buffer);
        gzipOut.close();
        baos.close();

        return baos.toByteArray();
    }

}
