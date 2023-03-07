package com.comt.compression;

import com.covt.compression.utils.TestOutputCatcher;
import com.fasterxml.jackson.databind.ObjectMapper;
import me.lemire.integercompression.*;
import org.apache.orc.impl.*;
import org.apache.orc.impl.writer.StreamOptions;
import org.apache.parquet.bytes.DirectByteBufferAllocator;
import org.apache.parquet.column.values.delta.DeltaBinaryPackingValuesWriterForInteger;
import org.apache.parquet.column.values.rle.RunLengthBitPackingHybridValuesWriter;
import java.io.*;
import java.util.Arrays;
import java.util.List;
import java.util.zip.GZIPOutputStream;

public class ComtPyramidCompressionEvaluation {
    private static final List<String> fileNames = List.of("data/rootPyramid.json", "data/rootPyramidZigZagDeltaCoded.json",
            "data/rootPyramidSorted.json", "data/rootPyramidZigZagDeltaCodedSorted.json");
    //private static final List<String> fileNames = List.of("data/rootPyramidSorted.json");
    private static final String rleFileName = "data/rootPyramidRLECodedSorted.json";

    public static void main(String[] args) throws IOException {
        analyzeRootPyramid();
    }

    private static void analyzeRootPyramid() throws IOException {
        for(var fileName :  fileNames){
            InputStream inputStream = new FileInputStream(fileName);
            var tileInfoRecords = new ObjectMapper().readValue(inputStream, TileInfoRecord[].class);
            var tileSizes = Arrays.stream(tileInfoRecords).mapToInt(tileInfoRecord -> tileInfoRecord.size).toArray();
            //TODO: Order on hilbert curve

            System.out.println(String.format("%s -----------------------------------------", fileName));
            final var varintSize = varintEncode(tileSizes);
            System.out.println(String.format("Varint Encoding: %s kb", varintSize / 1024));

            final var orcRleEncoding = ComtPyramidCompressionEvaluation.orcRleEncodingV1(tileSizes);
            System.out.println(String.format("ORC RLE V1 Encoding: %s kb", orcRleEncoding / 1024));

            final var orcRleEncoding2 = ComtPyramidCompressionEvaluation.orcRleEncodingV2(tileSizes);
            System.out.println(String.format("ORC RLE V2 Encoding: %s kb", orcRleEncoding2 / 1024));

            final var parquetRleSize = ComtPyramidCompressionEvaluation.parquetRLEBitpackingHybridEncoding(tileSizes);
            System.out.println(String.format("Parquet RLE Bitpacking Hybrid Encoding: %s kb", parquetRleSize / 1024));

            final var parquetDeltaSize = ComtPyramidCompressionEvaluation.parquetDeltaEncoding(tileSizes);
            System.out.println(String.format("Parquet Delta Encoding: %s kb", parquetDeltaSize / 1024));

            final var fastPfor128Size = ComtPyramidCompressionEvaluation.fastPfor128Encode(tileSizes);
            System.out.println(String.format("FastPfor128 Encoding: %s kb", fastPfor128Size / 1024));

            final var binaryPackingPointSize = ComtPyramidCompressionEvaluation.binaryPacking(tileSizes);
            System.out.println(String.format("Binary Packing Encoding: %s kb", binaryPackingPointSize / 1024));

            final var netPfdSize = ComtPyramidCompressionEvaluation.netPFDEncode(tileSizes);
            System.out.println(String.format("NetPFD Encoding: %s kb", netPfdSize / 1024));

            final var optPfdSize = ComtPyramidCompressionEvaluation.optPFDEncode(tileSizes);
            System.out.println(String.format("OptPFD Encoding: %s kb", optPfdSize / 1024));
        }

        InputStream inputStream = new FileInputStream(rleFileName);
        var rleCodedSizes = new ObjectMapper().readValue(inputStream, int[].class);
        final var varintSize = varintEncode(rleCodedSizes);
        System.out.println(String.format("RLE Varint Encoding: %s kb", varintSize / 1024));
    }

    private static int varintEncode(int[] values) throws IOException {
        //for best performance, use it using the ByteIntegerCODEC interface
        var variableByte = new VariableByte();
        var inputoffset = new IntWrapper(0);
        var outputoffset = new IntWrapper(0);
        var compressed = new int[values.length+1024];
        variableByte.compress(values, inputoffset, values.length, compressed, outputoffset);
        var totalSize = (outputoffset.intValue()*4);

        /*var baos = new FileOutputStream("data/pyramidParquetRLEBitpackingHypbrid.zip");
        GZIPOutputStream gzipOut = new GZIPOutputStream(baos);
        gzipOut.write(compressed);
        gzipOut.close();
        baos.close();*/

        return  totalSize;
    }

    private static int fastPfor128Encode(int[] values){
        /*
         * Note that this does not use differential coding: if you are working on sorted * lists,
         * you should first compute deltas, @see me.lemire.integercompression.differential.Delta#delta
         * */
        //TODO: also test VectorFastPFOR -> patched version which should be faster

        var fastPfor = new FastPFOR128();
        IntWrapper inputoffset = new IntWrapper(0);
        IntWrapper outputoffset = new IntWrapper(0);
        int [] compressed = new int[values.length+1024];
        fastPfor.compress(values, inputoffset, values.length, compressed, outputoffset);
        var totalSize = outputoffset.intValue()*4;
        return totalSize;
    }

    private static int binaryPacking(int[] values){
        IntWrapper inputoffset = new IntWrapper(0);
        IntWrapper outputoffset = new IntWrapper(0);
        int [] compressed = new int[values.length+1024];
        var binaryPacking = new BinaryPacking();
        binaryPacking.compress(values, inputoffset, values.length, compressed, outputoffset);
        var totalSize = outputoffset.intValue()*4;
        return totalSize;
    }

    private static int netPFDEncode(int[] values){
        var newPFD = new NewPFD();
        IntWrapper inputoffset = new IntWrapper(0);
        IntWrapper outputoffset = new IntWrapper(0);
        int [] compressed = new int[values.length+1024];
        newPFD.compress(values, inputoffset, values.length, compressed, outputoffset);
        return outputoffset.intValue()*4;
    }

    private static int optPFDEncode(int[] values){
        var optPFD = new OptPFD();
        IntWrapper inputoffset = new IntWrapper(0);
        IntWrapper outputoffset = new IntWrapper(0);
        int [] compressed = new int[values.length+1024];
        optPFD.compress(values, inputoffset, values.length, compressed, outputoffset);
        return outputoffset.intValue()*4;
    }

    private static int parquetDeltaEncoding(int[] values) throws IOException {
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

        var buffer = writer.getBytes();
        var bufferLength = buffer.toByteArray().length;
        return bufferLength;
    }

    private static int parquetRLEBitpackingHybridEncoding(int[] values) throws IOException {
        var maxValue = Arrays.stream(values).max().getAsInt();
        var bitWidth = (int)Math.ceil(Math.log(maxValue) + 1 );
        var initialCapacity = 1;
        var writer = new RunLengthBitPackingHybridValuesWriter(bitWidth, initialCapacity, 10, new DirectByteBufferAllocator());

        for(var value : values){
            writer.writeInteger(value);
        }

        var buffer = writer.getBytes();
        var bufferLength = buffer.toByteArray().length;

        var baos = new FileOutputStream("data/pyramidParquetRLEBitpackingHypbrid.zip");
        GZIPOutputStream gzipOut = new GZIPOutputStream(baos);
        gzipOut.write(buffer.toByteArray());
        gzipOut.close();
        baos.close();

        return bufferLength;
    }

    private static int orcRleEncodingV1(int[] values) throws IOException {
        var signed = false;
        var testOutputCatcher = new TestOutputCatcher();
        var writer =
                new RunLengthIntegerWriter(new OutStream("test", new StreamOptions(1), testOutputCatcher), signed);

        for(var value: values) {
            writer.write(value);
        }

        writer.flush();

        var buffer = testOutputCatcher.getBuffer();
        //ByteArrayOutputStream baos = new ByteArrayOutputStream();
        var baos = new FileOutputStream("data/pyramidRLEV1.zip");
        GZIPOutputStream gzipOut = new GZIPOutputStream(baos);
        gzipOut.write(buffer);
        gzipOut.close();
        baos.close();

        return testOutputCatcher.getBufferSize();
    }

    private static int orcRleEncodingV2(int[] values) throws IOException {
        var signed = false;
        var testOutputCatcher = new TestOutputCatcher();
        var writer =
                new RunLengthIntegerWriterV2(new OutStream("test", new StreamOptions(1), testOutputCatcher), signed, false);

        for(var value: values) {
            writer.write(value);
        }

        writer.flush();

        var buffer = testOutputCatcher.getBuffer();
        //ByteArrayOutputStream baos = new ByteArrayOutputStream();
        var baos = new FileOutputStream("data/pyramidRLEV2.zip");
        GZIPOutputStream gzipOut = new GZIPOutputStream(baos);
        gzipOut.write(buffer);
        gzipOut.close();
        //baos.write(buffer);
        baos.close();

        /*TestInStream.OutputCollector collect = new TestInStream.OutputCollector();
        ByteBuffer inBuf = ByteBuffer.allocate(collect.buffer.size());
        collect.buffer.setByteBuffer(inBuf, 0, collect.buffer.size());
        inBuf.flip();
        RunLengthIntegerReaderV2 in =
                new RunLengthIntegerReaderV2(InStream.create("test",
                        new BufferChunk(inBuf, 0), 0,
                        inBuf.remaining()), false, false);*/

        return testOutputCatcher.getBufferSize();
    }

}
