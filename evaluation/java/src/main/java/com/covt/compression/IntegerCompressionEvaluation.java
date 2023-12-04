package com.covt.compression;

import com.covt.compression.geometry.Point;
import com.covt.compression.utils.TestOutputCatcher;
import com.fasterxml.jackson.databind.ObjectMapper;
import me.lemire.integercompression.BinaryPacking;
import me.lemire.integercompression.FastPFOR128;
import me.lemire.integercompression.IntWrapper;
import me.lemire.integercompression.NewPFD;
import me.lemire.integercompression.OptPFD;
import me.lemire.integercompression.VariableByte;
import org.apache.orc.impl.OutStream;
import org.apache.orc.impl.RunLengthIntegerWriter;
import org.apache.orc.impl.RunLengthIntegerWriterV2;
import org.apache.orc.impl.writer.StreamOptions;
import org.apache.parquet.bytes.DirectByteBufferAllocator;
import org.apache.parquet.column.values.delta.DeltaBinaryPackingValuesWriterForInteger;
import org.apache.parquet.column.values.rle.RunLengthBitPackingHybridValuesWriter;
import java.io.FileInputStream;
import java.io.IOException;
import java.io.InputStream;
import java.util.Arrays;
import java.util.List;
import java.util.Map;
import java.util.stream.IntStream;

public class IntegerCompressionEvaluation {
    private static final Map<String, String> geometryFileNames = Map.of("Transportation_DeltaFeature_IdSorted",
            "../data/transportationZoom5_deltaFeature_idSorted.json");
    private static final List<String> idFileNames = Arrays.asList("../js/data/boundary_zoom4_id_unsorted.json",
            "../js/data/boundary_zoom4_id_sorted.json", "../js/data/boundary_zoom4_id_sorted_delta.json",
            "../js/data/boundary_zoom4_id_unsorted_delta.json", "../js/data/transportation_zoom5_id_unsorted.json",
            "../js/data/transportation_zoom5_id_sorted.json",  "../js/data/transportation_zoom5_id_sorted_delta.json",
            "../js/data/transportation_zoom5_id_unsorted_delta.json",
            //"../js/data/poi_zoom14_id_unsorted.json",
            //"../js/data/poi_zoom14_id_sorted.json",
            "../js/data/poi_zoom14_id_sorted_delta.json"
            );

    private static final List<String> pointFileNames = Arrays.asList("../js/data/poi_zoom14_point_unsorted.json",
            "../js/data/poi_zoom14_point_sorted.json", "../js/data/poi_zoom14_point_sorted_delta.json"
    );

    /**
     *
     * - Varint Encoding -> Reference
     *      -> Delta coordinates
     * - Parquet Delta
     *      -> original coordinates are needed
     *      -> x and y has to be separated to be effective -> can not be interleaved -> slower to decode
     *      -> what effect in compression ratio when interleaving
     * - ORC RLEv2
     *      -> Delta coordinates
     * - FastPfor128
     *      -> Delta coordinates
     *      -> For benchmarking the patched version has to be used
     *      -> on realistic data, SIMDFastPFOR is better than BP32 on two key metrics: decoding speed and compression ratio
     * - NetPFD
     * - OptPFD -> patching scheme with the best compression ratio
     * - BinaryPacking -> SIMD-BP128?
     * - DeltaZigzagBinaryPacking
     *      -> Original data
     * - DeltaZigzagVariableByte
     *      -> Original data
     * - BitPacking -> Parquet is based on -> used by the other schemes?
     */
    public static void main(String[] args) throws IOException {
        analyzeIds();
        //analyzePoints();
    }

    private static void analyzeIds() throws IOException {
        for(var fileName : idFileNames){
            InputStream inputStream = new FileInputStream(fileName);
            var values = new ObjectMapper().readValue(inputStream, int[].class);

            System.out.println(String.format("%s -----------------------------------", fileName));
            final var varintSize = varintEncode(values);
            System.out.println(String.format("Varint Encoding: %s kb", (double)varintSize / 1024));

            final var orcRleEncoding = IntegerCompressionEvaluation.orcRleEncodingV1(values);
            System.out.println(String.format("ORC RLE V1 Encoding: %s kb", (double)orcRleEncoding / 1024));

            final var orcRleEncoding2 = IntegerCompressionEvaluation.orcRleEncodingV2(values);
            System.out.println(String.format("ORC RLE V2 Encoding: %s kb", (double)orcRleEncoding2 / 1024));

            final var parquetRleSize = IntegerCompressionEvaluation.parquetRLEBitpackingHybridEncoding(values);
            System.out.println(String.format("Parquet RLE Bitpacking Hybrid Encoding: %s kb", (double)parquetRleSize / 1024));

            final var parquetDeltaSize = IntegerCompressionEvaluation.parquetDeltaEncoding(values);
            System.out.println(String.format("Parquet Delta Encoding: %s kb", (double)parquetDeltaSize / 1024));

            final var fastPfor128Size = IntegerCompressionEvaluation.fastPfor128Encode(values);
            System.out.println(String.format("FastPfor128 Encoding: %s kb", (double)fastPfor128Size / 1024));

            final var binaryPackingPointSize = IntegerCompressionEvaluation.binaryPacking(values);
            System.out.println(String.format("Binary Packing Encoding: %s kb", (double)binaryPackingPointSize / 1024));

            final var netPfdSize = IntegerCompressionEvaluation.netPFDEncode(values);
            System.out.println(String.format("NetPFD Encoding: %s kb", (double)netPfdSize / 1024));

            final var optPfdSize = IntegerCompressionEvaluation.optPFDEncode(values);
            System.out.println(String.format("OptPFD Encoding: %s kb", (double)optPfdSize / 1024));
        }
    }

    private static void analyzePoints() throws IOException {
        for(var fileName : pointFileNames){
            InputStream inputStream = new FileInputStream(fileName);
            var points = new ObjectMapper().readValue(inputStream, Point[].class);

            var xVertices = Arrays.stream(points).mapToInt(v -> (v.x >> 31) ^ (v.x << 1)).toArray();
            var yVertices = Arrays.stream(points).mapToInt(v -> (v.y >> 31) ^ (v.y << 1)).toArray();
            var vertices = Arrays.stream(points).flatMapToInt(v -> {
                var x = (v.x >> 31) ^ (v.x << 1);
                var y = (v.y >> 31) ^ (v.y << 1);
                return IntStream.of(x, y);
            }).toArray();

            System.out.println(String.format("%s -----------------------------------", fileName));
            final var varintSize = varintEncodePoints(vertices);
            System.out.println(String.format("Varint Encoding: %s kb", varintSize));

            final var fastPfor128Size = IntegerCompressionEvaluation.fastPfor128EncodePoints(vertices);
            System.out.println(String.format("FastPfor128 Encoding: %s kb", fastPfor128Size));

            /*final var orcRleEncoding = IntegerCompressionEvaluation.orcRleEncodingV1(values);
            System.out.println(String.format("ORC RLE V1 Encoding: %s kb", orcRleEncoding));

            final var orcRleEncoding2 = IntegerCompressionEvaluation.orcRleEncodingV2(values);
            System.out.println(String.format("ORC RLE V2 Encoding: %s kb", orcRleEncoding2));

            final var parquetRleSize = IntegerCompressionEvaluation.parquetRLEBitpackingHybridEncoding(values);
            System.out.println(String.format("Parquet RLE Bitpacking Hybrid Encoding: %s kb", parquetRleSize));

            final var parquetDeltaSize = IntegerCompressionEvaluation.parquetDeltaEncoding(values);
            System.out.println(String.format("Parquet Delta Encoding: %s kb", parquetDeltaSize));

            final var fastPfor128Size = IntegerCompressionEvaluation.fastPfor128Encode(values);
            System.out.println(String.format("FastPfor128 Encoding: %s kb", fastPfor128Size));

            final var binaryPackingPointSize = IntegerCompressionEvaluation.binaryPacking(values);
            System.out.println(String.format("Binary Packing Encoding: %s kb", binaryPackingPointSize));

            final var netPfdSize = IntegerCompressionEvaluation.netPFDEncode(values);
            System.out.println(String.format("NetPFD Encoding: %s kb", netPfdSize));

            final var optPfdSize = IntegerCompressionEvaluation.optPFDEncode(values);
            System.out.println(String.format("OptPFD Encoding: %s kb", optPfdSize));*/
        }
    }

    /* Id encoding --------------------------------------------- */

    private static int varintEncode(int[] values){
        //for best performance, use it using the ByteIntegerCODEC interface
        var variableByte = new VariableByte();
        var inputoffset = new IntWrapper(0);
        var outputoffset = new IntWrapper(0);
        var compressed = new int[values.length+1024];
        variableByte.compress(values, inputoffset, values.length, compressed, outputoffset);
        var totalSize = (outputoffset.intValue()*4);
        return  totalSize;
    }

    public static int fastPfor128Encode(int[] values){
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

    public static byte[] fastPfor128EncodeBuffer(int[] values){
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

    private static int binaryPacking(int[] values){
        IntWrapper inputoffset = new IntWrapper(0);
        IntWrapper outputoffset = new IntWrapper(0);
        int [] compressed = new int[values.length+1024];
        var binaryPacking = new BinaryPacking();
        binaryPacking.compress(values, inputoffset, values.length, compressed, outputoffset);
        var totalSize = outputoffset.intValue()*4;
        return totalSize;
    }

    public static int netPFDEncode(int[] values){
        var newPFD = new NewPFD();
        IntWrapper inputoffset = new IntWrapper(0);
        IntWrapper outputoffset = new IntWrapper(0);
        int [] compressed = new int[values.length+1024];
        newPFD.compress(values, inputoffset, values.length, compressed, outputoffset);
        return outputoffset.intValue()*4;
    }

    public static int optPFDEncode(int[] values){
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
        //TODO: check if this is valid
        var maxValue = Arrays.stream(values).max().getAsInt();
        var bitWidth = (int)Math.ceil(Math.log(maxValue) + 1 );
        var initialCapacity = 1;
        var writer = new RunLengthBitPackingHybridValuesWriter(bitWidth, initialCapacity, 10, new DirectByteBufferAllocator());

        for(var value : values){
            writer.writeInteger(value);
        }

        var buffer = writer.getBytes();
        var bufferLength = buffer.toByteArray().length;
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

        return testOutputCatcher.getBufferSize();
    }


    /* Point encoding --------------------------------------------- */

    private static double varintEncodePoints(int[] vertices){
        //for best performance, use it using the ByteIntegerCODEC interface
        var variableByte = new VariableByte();
        var inputoffset = new IntWrapper(0);
        var outputoffset = new IntWrapper(0);
        var compressed = new int[vertices.length];
        variableByte.compress(vertices, inputoffset, vertices.length, compressed, outputoffset);
        var totalSize = (double)outputoffset.intValue()*4/1024;
        return  totalSize;
    }

    private static double fastPfor128EncodePoints(int[] vertices){
        /*
         * Note that this does not use differential coding: if you are working on sorted * lists,
         * you should first compute deltas, @see me.lemire.integercompression.differential.Delta#delta
         * */
        //TODO: also test VectorFastPFOR -> patched version which should be faster

        var fastPfor = new FastPFOR128();
        IntWrapper inputoffset = new IntWrapper(0);
        IntWrapper outputoffset = new IntWrapper(0);
        int [] compressed = new int[vertices.length];
        fastPfor.compress(vertices, inputoffset, vertices.length, compressed, outputoffset);
        var totalSize = (double)outputoffset.intValue()*4 /1024;
        return totalSize;
    }

    private static int fastPfor128Encode(int[] xVertices, int[] yVertices){
        /*
         * Note that this does not use differential coding: if you are working on sorted * lists,
         * you should first compute deltas, @see me.lemire.integercompression.differential.Delta#delta
         * */
        //TODO: also test VectorFastPFOR -> patched version which should be faster
        var fastPfor = new FastPFOR128();
        IntWrapper inputoffset = new IntWrapper(0);
        IntWrapper outputoffset = new IntWrapper(0);
        int [] compressed = new int[xVertices.length+1024];
        fastPfor.compress(xVertices, inputoffset, xVertices.length, compressed, outputoffset);
        IntWrapper inputoffset2 = new IntWrapper(0);
        IntWrapper outputoffset2 = new IntWrapper(0);
        int [] compressed2 = new int[yVertices.length+1024];
        fastPfor.compress(yVertices, inputoffset2, yVertices.length, compressed2, outputoffset2);
        var totalSize = ((outputoffset.intValue()*4) + (outputoffset2.intValue()*4))/1024;
        return totalSize;
    }

    /*private static int binaryPacking(int[] xVertices, int[] yVertices){
        IntegratedIntCompressor iic = new IntegratedIntCompressor();
        int[] compressedXVertices = iic.compress(xVertices);
        int[] compressedYVertices = iic.compress(yVertices);
        var totalSize = (compressedXVertices.length + compressedYVertices.length) / 1024;
        return totalSize;
    }*/

    private static int binaryPacking(Point[] points){
        var vertices = Arrays.stream(points).flatMapToInt(v -> {
            var x = (v.x >> 31) ^ (v.x << 1);
            var y = (v.y >> 31) ^ (v.y << 1);
            return  Arrays.stream(new int[]{ x,y});
        }).toArray();

        IntWrapper inputoffset = new IntWrapper(0);
        IntWrapper outputoffset = new IntWrapper(0);
        int [] compressed = new int[vertices.length+1024];
        var binaryPacking = new BinaryPacking();
        binaryPacking.compress(vertices, inputoffset, vertices.length, compressed, outputoffset);
        var totalSize = outputoffset.intValue()*4 /1024;
        return totalSize;
    }

    /*private static int bitPacking(Point[] points){
        var vertices = Arrays.stream(points).flatMapToInt(v -> {
            var x = (v.x >> 31) ^ (v.x << 1);
            var y = (v.y >> 31) ^ (v.y << 1);
            return  Arrays.stream(new int[]{ x,y});
        }).toArray();

        IntWrapper inputoffset = new IntWrapper(0);
        IntWrapper outputoffset = new IntWrapper(0);
        int [] compressed = new int[vertices.length+1024];

        //Parquet is adapting this scheme
        BitPacking.fastpack(vertices, 0, vertices.length, compressed, outputoffset);
        //bitPacking.compress(vertices, inputoffset, vertices.length, compressed, outputoffset);
        var totalSize = outputoffset.intValue()*4 /1024;
        return totalSize;
    }*/

    private static int netPFDEncode(Point[] points){
        var vertices = Arrays.stream(points).flatMapToInt(v -> {
            var x = (v.x >> 31) ^ (v.x << 1);
            var y = (v.y >> 31) ^ (v.y << 1);
            return  Arrays.stream(new int[]{ x,y});
        }).toArray();

        var newPFD = new NewPFD();
        IntWrapper inputoffset = new IntWrapper(0);
        IntWrapper outputoffset = new IntWrapper(0);
        int [] compressed = new int[vertices.length+1024];
        newPFD.compress(vertices, inputoffset, vertices.length, compressed, outputoffset);
        return outputoffset.intValue()*4/1024;
    }

    private static int optPFDEncode(Point[] points){
        var vertices = Arrays.stream(points).flatMapToInt(v -> {
            var x = (v.x >> 31) ^ (v.x << 1);
            var y = (v.y >> 31) ^ (v.y << 1);
            return  Arrays.stream(new int[]{ x,y});
        }).toArray();

        var optPfd = new OptPFD();
        IntWrapper inputoffset = new IntWrapper(0);
        IntWrapper outputoffset = new IntWrapper(0);
        int [] compressed = new int[vertices.length+1024];
        optPfd.compress(vertices, inputoffset, vertices.length, compressed, outputoffset);
        return outputoffset.intValue()*4/1024;
    }

    private static int parquetDeltaEncoding(Point[] points) throws IOException {
        var blockSize = 128;
        var miniBlockNum = 4;
        //TODO: play around with the settings to achive a better compression ratio
        /*var writer = new DeltaBinaryPackingValuesWriterForInteger(
                blockSize, miniBlockNum, 100, 200, new DirectByteBufferAllocator());*/
        var writer = new DeltaBinaryPackingValuesWriterForInteger(
                blockSize, miniBlockNum, 100, 200, new DirectByteBufferAllocator());

        /* To get a good compression ratio the x and y component of a vertex has to be stored separately
         *  so that delta encoding is efficient -> not so efficient for decoding -> separate copy step when
         *  before transferring to the GPU */
        var lastPointX = 0;
        for(var point : points){
            var x = lastPointX + point.x;
            writer.writeInteger(x);
            lastPointX = x;
        }

        var lastPointY = 0;
        for(var point : points){
            var y = lastPointY + point.y;
            writer.writeInteger(y);
            lastPointY = y;
        }

        //TODO: current is this Delta of Delta
        var buffer = writer.getBytes();
        var bufferLength = buffer.toByteArray().length;
        return bufferLength/1024;
    }

    private static int orcRleEncodingV1(Point[] points) throws IOException {
        var vertices = Arrays.stream(points).flatMapToInt(v -> {
            var x = (v.x >> 31) ^ (v.x << 1);
            var y = (v.y >> 31) ^ (v.y << 1);
            return  Arrays.stream(new int[]{ x,y});
        }).toArray();

        var signed = false;
        var testOutputCatcher = new TestOutputCatcher();
        RunLengthIntegerWriterV2 writer =
                new RunLengthIntegerWriterV2(new OutStream("test", new StreamOptions(10000), testOutputCatcher), signed, false);

        for(var vertex: vertices) {
            writer.write(vertex);
        }

        writer.flush();

        return testOutputCatcher.getBufferSize()/1024;
    }

    private static int orcRleEncodingWithSign(Point[] points) throws IOException {
        var vertices = Arrays.stream(points).flatMapToInt(v -> {
            return  Arrays.stream(new int[]{ v.x,v.y});
        }).toArray();

        var signed = true;
        var testOutputCatcher = new TestOutputCatcher();
        RunLengthIntegerWriterV2 writer =
                new RunLengthIntegerWriterV2(new OutStream("test", new StreamOptions(10000), testOutputCatcher), signed, false);

        for(var vertex: vertices) {
            writer.write(vertex);
        }

        writer.flush();

        return testOutputCatcher.getBufferSize()/1024;
    }

    private static int orcRleWithoutDeltaEncoding(Point[] points) throws IOException {
        var lastPointX = 0;
        var lastPointY = 0;

        var convertedVertices = new int[points.length * 2];
        var i = 0;
        for(var point : points){
            var x = lastPointX + point.x;
            var y = lastPointY + point.y;
            lastPointX = x;
            lastPointY = y;

            x = (x >> 31) ^ (x << 1);
            y = (y >> 31) ^ (y << 1);
            convertedVertices[i++] = x;
            convertedVertices[i++] = y;
        }

        //TODO: test with signed true
        var signed = false;
        var testOutputCatcher = new TestOutputCatcher();
        RunLengthIntegerWriterV2 writer =
                new RunLengthIntegerWriterV2(new OutStream("test", new StreamOptions(10000), testOutputCatcher), signed, false);

        for(var vertex: convertedVertices) {
            writer.write(vertex);
        }

        writer.flush();

        return testOutputCatcher.getBufferSize()/1024;
    }

    /*private static void analyzeGeometries() throws IOException {
        for(final var entry : geometryFileNames.entrySet()){
            final var fileName = entry.getValue();
            InputStream inputStream = new FileInputStream(fileName);
            var points = new ObjectMapper().readValue(inputStream, Point[].class);

            var xVertices = Arrays.stream(points).mapToInt(v -> (v.x >> 31) ^ (v.x << 1)).toArray();
            var yVertices = Arrays.stream(points).mapToInt(v -> (v.y >> 31) ^ (v.y << 1)).toArray();

            System.out.println(String.format("%s -----------------------------------", entry.getKey()));
            final var varintSize = IntegerCompressionEvaluation.varintEncode(xVertices, yVertices);
            System.out.println(String.format("Varint Encoding: %s kb", varintSize));

            final var fastPfor128Size = IntegerCompressionEvaluation.fastPfor128Encode(xVertices, yVertices);
            System.out.println(String.format("FastPfor128 Encoding: %s kb", fastPfor128Size));

            final var fastPfor128PointSize = IntegerCompressionEvaluation.fastPfor128Encode(points);
            System.out.println(String.format("FastPfor128 Point Encoding: %s kb", fastPfor128PointSize));

            final var parquetDeltaSize = IntegerCompressionEvaluation.parquetDeltaEncoding(points);
            System.out.println(String.format("Parquet Delta Encoding: %s kb", parquetDeltaSize));

            final var netPfdSize = IntegerCompressionEvaluation.netPFDEncode(points);
            System.out.println(String.format("NetPFD Encoding: %s kb", netPfdSize));

            final var optPfdSize = IntegerCompressionEvaluation.optPFDEncode(points);
            System.out.println(String.format("OptPFD Encoding: %s kb", optPfdSize));


            final var binaryPackingPointSize = IntegerCompressionEvaluation.binaryPacking(points);
            System.out.println(String.format("Binary Packing Point Encoding: %s kb", binaryPackingPointSize));

            final var orcRleEncoding = IntegerCompressionEvaluation.orcRleEncodingV1(points);
            System.out.println(String.format("ORC RLE Point Encoding: %s kb", orcRleEncoding));

            final var orcRleWithSignEncoding = IntegerCompressionEvaluation.orcRleEncodingWithSign(points);
            System.out.println(String.format("ORC RLE Point With Sign Encoding: %s kb", orcRleWithSignEncoding));

            final var orcRleWithoutDeltaEncoding = IntegerCompressionEvaluation.orcRleWithoutDeltaEncoding(points);
            System.out.println(String.format("ORC RLE Point Without Delta Encoding: %s kb", orcRleWithoutDeltaEncoding));

            //DeltaZigzagBinaryPacking
            //DeltaZigzagVariableByte
            //BitPacking
        }
    }*/

    /*private static int compress(int[] xVertices, int[] yVertices, FiveParameterFunction function){
        //for best performance, use it using the ByteIntegerCODEC interface
        var variableByte = new VariableByte();
        var inputoffset = new IntWrapper(0);
        var outputoffset = new IntWrapper(0);
        var compressed = new int[xVertices.length+1024];
        variableByte.compress(xVertices, inputoffset, xVertices.length, compressed, outputoffset);
        var inputoffset2 = new IntWrapper(0);
        var outputoffset2 = new IntWrapper(0);
        var compressed2 = new int[yVertices.length+1024];
        variableByte.compress(yVertices, inputoffset2, yVertices.length, compressed2, outputoffset2);
        var totalSize = ((outputoffset.intValue()*4) + (outputoffset2.intValue()*4))/1024;
        //System.out.println("Varint compressed from "+ values.length*4/1024+"KB to "+totalSize+"KB");
        return  totalSize;
    }*/

}
