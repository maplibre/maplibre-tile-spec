package com.mlt.converter.encodings;

import me.lemire.integercompression.*;
import org.apache.commons.lang3.ArrayUtils;
import org.apache.commons.lang3.tuple.Pair;
import org.apache.orc.PhysicalWriter;
import org.apache.orc.impl.OutStream;
import org.apache.orc.impl.RunLengthByteWriter;
import org.apache.orc.impl.RunLengthIntegerWriter;
import org.apache.orc.impl.writer.StreamOptions;
import java.io.ByteArrayInputStream;
import java.io.ByteArrayOutputStream;
import java.io.IOException;
import java.nio.ByteBuffer;
import java.nio.ByteOrder;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.BitSet;
import java.util.List;
import java.util.zip.GZIPInputStream;
import java.util.zip.GZIPOutputStream;


public class EncodingUtils {

    public static byte[] gzip(byte[] buffer) throws IOException {
        ByteArrayOutputStream baos = new ByteArrayOutputStream();
        GZIPOutputStream gzipOut = new GZIPOutputStream(baos);
        gzipOut.write(buffer);
        gzipOut.close();
        baos.close();

        return baos.toByteArray();
    }

    public static byte[] unzip(byte[] buffer) throws IOException {
        try(var inputStream = new ByteArrayInputStream(buffer)){
            try(var gZIPInputStream = new GZIPInputStream(inputStream)){
                return gZIPInputStream.readAllBytes();
            }
        }
    }

    /**
     * Convert the floats to IEEE754 floating point numbers in Little Endian byte order.
     */
    public static byte[] encodeFloatsLE(float[] values){
        var buffer = ByteBuffer.allocate(values.length * 4).order(ByteOrder.LITTLE_ENDIAN);
        for(var value : values){
            buffer.putFloat(value);
        }
        return buffer.array();
    }

    //Source: https://github.com/bazelbuild/bazel/blob/master/src/main/java/com/google/devtools/build/lib/util/VarInt.java
    public static byte[] encodeVarints(long[] values, boolean zigZagEncode, boolean deltaEncode) {
        var encodedValues = values;
        if(deltaEncode){
            encodedValues = encodeDeltas(values);
        }

        if(zigZagEncode){
            encodedValues = encodeZigZag(encodedValues);
        }

        var varintBuffer = new byte[values.length * 8];
        var i = 0;
        for(var value : encodedValues){
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

    public static long[] encodeZigZag(long[] values){
        return Arrays.stream(values).map(value -> EncodingUtils.encodeZigZag(value)).toArray();
    }

    public static int[] encodeZigZag(int[] values){
        return Arrays.stream(values).map(value -> EncodingUtils.encodeZigZag(value)).toArray();
    }

    public static long encodeZigZag(long value){
        return (value << 1) ^ (value >> 63);
    }

    public static int encodeZigZag(int value){
        return (value >> 31) ^ (value << 1);
    }

    public static long[] encodeDeltas(long[] values){
        var deltaValues = new long[values.length];
        var previousValue = 0l;
        for(var i = 0; i < values.length; i++){
            var value = values[i];
            deltaValues[i] = value - previousValue;
            previousValue = value;
        }
        return  deltaValues;
    }

    public static int[] encodeDeltas(int[] values){
        var deltaValues = new int[values.length];
        var previousValue = 0;
        for(var i = 0; i < values.length; i++){
            var value = values[i];
            deltaValues[i] = value - previousValue;
            previousValue = value;
        }
        return  deltaValues;
    }


    /* RLE V1 encoding of the ORC format */
    public static byte[] encodeDeltaRle(long[] values, boolean signed) throws IOException {
        var testOutputCatcher = new OutputCatcher();
        var writer =
                new RunLengthIntegerWriter(new OutStream("test", new StreamOptions(1), testOutputCatcher), signed);
        for(var value: values) {
            writer.write(value);
        }
        writer.flush();
        return testOutputCatcher.getBuffer();
    }

    /**
     * @return Pair of runs and values.
     */
    public static Pair<List<Integer>, List<Integer>> encodeRle(int[] values) {
        var valueBuffer = new ArrayList<Integer>();
        var runsBuffer = new ArrayList<Integer>();
        var previousValue = 0;
        var runs = 0;
        for(var i = 0; i < values.length; i++){
            var value = values[i];
            if(previousValue != value && i != 0){
                valueBuffer.add(previousValue);
                runsBuffer.add(runs);
                runs = 0;
            }

            runs++;
            previousValue = value;
        }

        valueBuffer.add(values[values.length -1]);
        runsBuffer.add(runs);

        return Pair.of(runsBuffer, valueBuffer);
    }

    /**
     * @return Pair of runs and values.
     */
    //TODO: merge this method with the int variant
    public static Pair<List<Integer>, List<Long>> encodeRle(long[] values) {
        var valueBuffer = new ArrayList<Long>();
        var runsBuffer = new ArrayList<Integer>();
        var previousValue = 0l;
        var runs = 0;
        for(var i = 0; i < values.length; i++){
            var value = values[i];
            if(previousValue != value && i != 0){
                valueBuffer.add(previousValue);
                runsBuffer.add(runs);
                runs = 0;
            }

            runs++;
            previousValue = value;
        }

        valueBuffer.add(values[values.length -1]);
        runsBuffer.add(runs);

        return Pair.of(runsBuffer, valueBuffer);
    }

    public static byte[] encodeFastPfor128(int[] values, boolean zigZagEncode, boolean deltaEncode){
        /*
         * Note that this does not use differential coding: if you are working on sorted lists,
         * you should first compute deltas, @see me.lemire.integercompression.differential.Delta#delta
         * */
        //TODO: also test VectorFastPFOR -> patched version which should be faster

        var encodedValues = values;
        if(deltaEncode){
            encodedValues = encodeDeltas(values);
        }

        if(zigZagEncode){
            encodedValues = encodeZigZag(encodedValues);
        }

        IntegerCODEC ic = new Composition(new FastPFOR(), new VariableByte());
        IntWrapper inputoffset = new IntWrapper(0);
        IntWrapper outputoffset = new IntWrapper(0);
        int [] compressed = new int[encodedValues.length+1024];
        ic.compress(encodedValues, inputoffset, encodedValues.length, compressed, outputoffset);
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

    public static byte[] encodeByteRle(byte[] values) throws IOException {
        var outputCatcher = new OutputCatcher();
        var writer =
                new RunLengthByteWriter(new OutStream("test", new StreamOptions(1), outputCatcher));

        for(var value: values) {
            writer.write(value);
        }

        writer.flush();
        return outputCatcher.getBuffer();
    }

    public static byte[] encodeBooleanRle(BitSet bitSet, int numValues) throws IOException {
        var presentStream = bitSet.toByteArray();
        /* The BitSet only returns the bytes until the last set bit */
        var numMissingBytes = (int)Math.ceil(numValues /8d) - (int)Math.ceil(bitSet.length() / 8d);
        if(numMissingBytes != 0){
            var paddingBytes = new byte[numMissingBytes];
            Arrays.fill(paddingBytes, (byte)0);
            presentStream = ArrayUtils.addAll(presentStream, paddingBytes);
        }

        return EncodingUtils.encodeByteRle(presentStream);
    }

    private static class OutputCatcher implements PhysicalWriter.OutputReceiver{
        int currentBuffer = 0;
        List<ByteBuffer> buffers = new ArrayList<>();

        @Override
        public void output(ByteBuffer buffer) throws IOException {
            buffers.add(buffer);
        }

        @Override
        public void suppress() {
        }

        public ByteBuffer getCurrentBuffer() {
            while (currentBuffer < buffers.size() && buffers.get(currentBuffer).remaining() == 0) {
                currentBuffer += 1;
            }
            return currentBuffer < buffers.size() ? buffers.get(currentBuffer) : null;
        }

        public byte[] getBuffer() throws IOException {
            ByteArrayOutputStream outputStream = new ByteArrayOutputStream( );
            for(var buffer : this.buffers){
                outputStream.write(buffer.array());
            }
            return outputStream.toByteArray();
        }

        public int getBufferSize(){
            var size = 0;
            for(var buffer : buffers){
                size  += buffer.array().length;
            }
            return size;
        }
    }
}
