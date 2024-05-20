package com.covt.converter;

import com.covt.evaluation.compression.TestOutputCatcher;
import me.lemire.integercompression.*;
import org.apache.commons.lang3.ArrayUtils;
import org.apache.orc.impl.OutStream;
import org.apache.orc.impl.RunLengthByteWriter;
import org.apache.orc.impl.RunLengthIntegerWriter;
import org.apache.orc.impl.RunLengthIntegerWriterV2;
import org.apache.orc.impl.writer.StreamOptions;
import org.apache.parquet.bytes.DirectByteBufferAllocator;
import org.apache.parquet.column.values.delta.DeltaBinaryPackingValuesWriterForInteger;

import java.io.ByteArrayOutputStream;
import java.io.IOException;
import java.nio.ByteBuffer;
import java.nio.ByteOrder;
import java.nio.IntBuffer;
import java.nio.charset.StandardCharsets;
import java.util.Arrays;
import java.util.BitSet;
import java.util.List;
import java.util.zip.GZIPOutputStream;

public class EncodingUtils {

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

    public static byte[] encodeString(String value) throws IOException {
        var utf8Data = value.getBytes(StandardCharsets.UTF_8);
        var stringLength = encodeVarints(new long[]{utf8Data.length}, false, false);
        return ArrayUtils.addAll(stringLength, utf8Data);
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

    public static byte[] encodeFastPfor128(int[] values, boolean zigZagEncode, boolean deltaEncode){
        /*
         * Note that this does not use differential coding: if you are working on sorted * lists,
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

    public static int[] encodeZigZagDeltaCoordinates(List<Integer> coordinates){
        var previousValueX = 0;
        var previousValueY = 0;
        var deltaValues = new int[coordinates.size()];
        var j = 0;
        for(var coordinate : coordinates){
            if(j % 2 == 0){
                var delta = coordinate - previousValueX;
                var zigZagDelta = encodeZigZag(delta);
                deltaValues[j++] = zigZagDelta;
                previousValueX = coordinate;
            }
            else{
                var delta = coordinate - previousValueY;
                var zigZagDelta = encodeZigZag(delta);
                deltaValues[j++] = zigZagDelta;
                previousValueY = coordinate;
            }
        }

        return deltaValues;
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

    public static byte[] encodeOptPFD(int[] values){
        var zigZagValues = new int[values.length];
        var j = 0;
        for(var value : values){
            zigZagValues[j++] = encodeZigZag(value);
        }

        var optPfd = new OptPFD();
        IntWrapper inputoffset = new IntWrapper(0);
        IntWrapper outputoffset = new IntWrapper(0);
        int [] compressed = new int[zigZagValues.length+1024];
        optPfd.compress(zigZagValues, inputoffset, zigZagValues.length, compressed, outputoffset);

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

    public static byte [] encodeParquetDelta(int[] values) throws IOException {
        var blockSize = 128;
        var miniBlockNum = 4;
        //var slabSize = 100;
        //var pageSize = 200;
        var slabSize = 1;
        var pageSize = 1;
        var writer = new DeltaBinaryPackingValuesWriterForInteger(
                blockSize, miniBlockNum, slabSize,  pageSize, new DirectByteBufferAllocator());

        for(var value : values){
            writer.writeInteger(value);
        }

        return writer.getBytes().toByteArray();
    }

    public static byte[] encodeOrcRleV2(int[] values) throws IOException {
        var testOutputCatcher = new com.covt.compression.utils.TestOutputCatcher();
        var writer =
                new RunLengthIntegerWriterV2(new OutStream("test", new StreamOptions(1), testOutputCatcher), true, false);

        for(var value: values) {
            writer.write(value);
        }

        writer.flush();
        return testOutputCatcher.getBuffer();
    }


    /* Decoding */

    public static int[] deocdeFastPfor128Delta(byte[] encodedValues, int numValues){
        IntBuffer intBuf =
                ByteBuffer.wrap(encodedValues)
                        .order(ByteOrder.BIG_ENDIAN)
                        .asIntBuffer();
        int[] intValues = new int[encodedValues.length / 4];
        for(var i = 0; i < intValues.length; i++){
            intValues[i] = intBuf.get(i);
        }

        int[] decompressedValues = new int[numValues];
        var inputOffset = new IntWrapper(0);
        var outputOffset = new IntWrapper(0);
        var fastPfor = new FastPFOR();
        fastPfor.uncompress(intValues, inputOffset, intValues.length, decompressedValues, outputOffset);

        var decodedValues = new int[numValues];
        for(var i = 0; i < numValues; i++){
            var zigZagValue = decompressedValues[i];
            decodedValues[i]  = (zigZagValue >>> 1) ^ (-(zigZagValue & 1));
        }

        return decodedValues;
    }

    public static int[] deocdeFastPfor128DeltaWithoutZigZag(byte[] encodedValues, int numValues){
        IntBuffer intBuf =
                ByteBuffer.wrap(encodedValues)
                        .order(ByteOrder.BIG_ENDIAN)
                        .asIntBuffer();
        int[] intValues = new int[encodedValues.length / 4];
        for(var i = 0; i < intValues.length; i++){
            intValues[i] = intBuf.get(i);
        }

        int[] decompressedValues = new int[numValues];
        var inputOffset = new IntWrapper(0);
        var outputOffset = new IntWrapper(0);
        var fastPfor = new FastPFOR();
        fastPfor.uncompress(intValues, inputOffset, intValues.length, decompressedValues, outputOffset);

        var decodedValues = new int[numValues];
        var previousValue = 0;
        for(var i = 0; i < numValues; i++){
            var deltaValue = decompressedValues[i];
            decodedValues[i]  = previousValue + deltaValue;
            previousValue = decodedValues[i];
        }

        return decodedValues;
    }

    public static int[] deocdeFastPfor128(byte[] encodedValues, int numValues){
        IntBuffer intBuf =
                ByteBuffer.wrap(encodedValues)
                        .order(ByteOrder.BIG_ENDIAN)
                        .asIntBuffer();
        int[] intValues = new int[encodedValues.length / 4];
        for(var i = 0; i < intValues.length; i++){
            intValues[i] = intBuf.get(i);
        }

        int[] decompressedValues = new int[numValues];
        var inputOffset = new IntWrapper(0);
        var outputOffset = new IntWrapper(0);
        var fastPfor = new FastPFOR();
        fastPfor.uncompress(intValues, inputOffset, intValues.length, decompressedValues, outputOffset);

        return decompressedValues;
    }

    public static byte[] deocdeFastPfor128Delta2(int[] values){
        var zigZagValues = new int[values.length];
        var j = 0;
        for(var value : values){
            zigZagValues[j++] = encodeZigZag(value);
        }


        /*

        int [] compressed = new int[zigZagValues.length+1024];
        IntegerCODEC codec =  new
                Composition(
                new FastPFOR(),
                new VariableByte());
        IntWrapper inputoffset = new IntWrapper(0);
        IntWrapper outputoffset = new IntWrapper(0);
        codec.compress(zigZagValues, inputoffset, zigZagValues.length, compressed, outputoffset);
        compressed = Arrays.copyOf(compressed,outputoffset.intValue());

        int[] recovered = new int[values.length+1024];
        IntWrapper recoffset = new IntWrapper(0);
        codec.uncompress(compressed,new IntWrapper(0),compressed.length,recovered,recoffset);*/


        var fastPfor = new FastPFOR128();
        IntWrapper inputOffset = new IntWrapper(0);
        IntWrapper outputOffset = new IntWrapper(0);
        int [] compressed = new int[zigZagValues.length+1024];
        fastPfor.compress(zigZagValues, inputOffset, zigZagValues.length, compressed, outputOffset);
        compressed = Arrays.copyOf(compressed,outputOffset.intValue());

        ByteBuffer byteBuffer = ByteBuffer.allocate(compressed.length * 4);
        IntBuffer intBuffer = byteBuffer.asIntBuffer();
        intBuffer.put(compressed);
        var compressedBuffer = byteBuffer.array();


        IntBuffer intBuf =
                ByteBuffer.wrap(compressedBuffer)
                        .order(ByteOrder.BIG_ENDIAN)
                        .asIntBuffer();
        int[] intBuffer2 = new int[intBuffer.capacity()];
        for(var i = 0; i < intBuf.capacity(); i++){
            intBuffer2[i] = intBuf.get(i);
        }


        int[] recovered = new int[zigZagValues.length];
        inputOffset = new IntWrapper(0);
        outputOffset = new IntWrapper(0);
        fastPfor.uncompress(intBuffer2, inputOffset, compressed.length, recovered, outputOffset);

        var decodedValues = new int[zigZagValues.length];
        var i = 0;
        for(var v : recovered){
            decodedValues[i++]  = (v >>> 1) ^ (-(v & 1));
        }

        return compressedBuffer;
    }

}
