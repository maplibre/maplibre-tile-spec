package com.covt.evaluation.compression;

import me.lemire.integercompression.*;
import org.apache.orc.impl.OutStream;
import org.apache.orc.impl.RunLengthIntegerWriter;
import org.apache.orc.impl.RunLengthIntegerWriterV2;
import org.apache.orc.impl.writer.StreamOptions;
import org.apache.parquet.bytes.DirectByteBufferAllocator;
import org.apache.parquet.column.values.delta.DeltaBinaryPackingValuesWriterForInteger;
import org.apache.parquet.column.values.rle.RunLengthBitPackingHybridValuesWriter;
import java.io.ByteArrayOutputStream;
import java.io.IOException;
import java.nio.ByteBuffer;
import java.util.Arrays;
import java.util.zip.GZIPOutputStream;

public class IntegerCompression {


    public static byte[] gzipCompress(byte[] buffer) throws IOException {
        ByteArrayOutputStream baos = new ByteArrayOutputStream();
        GZIPOutputStream gzipOut = new GZIPOutputStream(baos);
        gzipOut.write(buffer);
        gzipOut.close();
        baos.close();

        return baos.toByteArray();
    }

    public static byte[] gzipCompress(int[] buffer) throws IOException {
        ByteArrayOutputStream baos = new ByteArrayOutputStream();
        GZIPOutputStream gzipOut = new GZIPOutputStream(baos);
        for(var i = 0; i < buffer.length; i++){
            ByteBuffer byteBuffer = ByteBuffer.allocate(4);
            byteBuffer.asIntBuffer().put(buffer[i]);

            byte[] array = byteBuffer.array();
            gzipOut.write(array[0]);
            gzipOut.write(array[1]);
            gzipOut.write(array[2]);
            gzipOut.write(array[3]);
        }
        gzipOut.close();
        baos.close();

        return baos.toByteArray();
    }

    public static byte[] varintEncode(int[] values) throws IOException {
        var variableByte = new VariableByte();
        var inputoffset = new IntWrapper(0);
        var outputoffset = new IntWrapper(0);
        var compressed = new byte[values.length * 4];
        variableByte.compress(values, inputoffset, values.length, compressed, outputoffset);
        return Arrays.copyOfRange(compressed, 0, outputoffset.intValue());
    }

    public static byte[] parquetDeltaEncoding(int[] values) throws IOException {
        var blockSize = 128;
        var miniBlockNum = 4;
        var slabSize = 100;
        var pageSize = 30000;
        var writer = new DeltaBinaryPackingValuesWriterForInteger(
                blockSize, miniBlockNum, slabSize,  pageSize, new DirectByteBufferAllocator());

        for(var value : values){
            writer.writeInteger(value);
        }

        return writer.getBytes().toByteArray();
    }

    public static byte[] parquetRLEBitpackingHybridEncoding(int[] values) throws IOException {
        var maxValue = Arrays.stream(values).max().getAsInt();
        var bitWidth = (int)Math.ceil(Math.log(maxValue) + 1 );
        var initialCapacity = 1;
        var writer = new RunLengthBitPackingHybridValuesWriter(bitWidth, initialCapacity, 30000, new DirectByteBufferAllocator());

        for(var value : values){
            writer.writeInteger(value);
        }

        return writer.getBytes().toByteArray();
    }

    public static byte[] orcRleEncodingV1(long[] values) throws IOException {
        var signed = false;
        var testOutputCatcher = new TestOutputCatcher();
        var writer =
                new RunLengthIntegerWriter(new OutStream("test", new StreamOptions(1), testOutputCatcher), signed);

        for(var value: values) {
            writer.write(value);
        }

        writer.flush();
        return testOutputCatcher.getBuffer();
    }

    public static byte[] orcRleEncodingV2(long[] values) throws IOException {
        var signed = false;
        var testOutputCatcher = new TestOutputCatcher();
        var writer =
                new RunLengthIntegerWriterV2(new OutStream("test", new StreamOptions(1), testOutputCatcher), signed, false);

        for(var value: values) {
            writer.write(value);
        }

        writer.flush();
        return testOutputCatcher.getBuffer();
    }
}
