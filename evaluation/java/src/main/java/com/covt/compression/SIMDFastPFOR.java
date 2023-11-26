package com.covt.compression;

import com.covt.compression.geometry.Point;
import com.fasterxml.jackson.databind.ObjectMapper;
import me.lemire.integercompression.FastPFOR128;
import me.lemire.integercompression.IntWrapper;
import me.lemire.integercompression.OptPFD;
import me.lemire.integercompression.VariableByte;
import me.lemire.integercompression.differential.IntegratedIntCompressor;
import org.apache.parquet.bytes.DirectByteBufferAllocator;
import org.apache.parquet.column.values.delta.DeltaBinaryPackingValuesWriterForInteger;

import java.io.FileInputStream;
import java.io.IOException;
import java.io.InputStream;
import java.util.Arrays;
import java.util.stream.Collectors;

public class SIMDFastPFOR {

    public static void main(String[] args) throws IOException {
        //InputStream inputStream = new FileInputStream("../data/transportationZoom5_idSorted.json");
        InputStream inputStream = new FileInputStream("../data/transportationZoom5_deltaFeature_idSorted.json");

        ObjectMapper mapper = new ObjectMapper();
        var values = new ObjectMapper().readValue(inputStream, Point[].class);

        /*
        * Note that this does not use differential coding: if you are working on sorted * lists,
        * you should first compute deltas, @see me.lemire.integercompression.differential.Delta#delta
        * */
        //TODO: also test VectorFastPFOR -> patched version which should be faster
        //var xVertices = Arrays.stream(values).mapToInt(v -> v.x).toArray();
        //var yVertices = Arrays.stream(values).mapToInt(v -> v.y).toArray();
        var xVertices = Arrays.stream(values).mapToInt(v -> (v.x >> 31) ^ (v.x << 1)).toArray();
        var yVertices = Arrays.stream(values).mapToInt(v -> (v.y >> 31) ^ (v.y << 1)).toArray();


        //TODO: are negative values handle because values are not sorted?
        var fastPfor = new FastPFOR128();
        IntWrapper inputoffset = new IntWrapper(0);
        IntWrapper outputoffset = new IntWrapper(0);
        int [] compressed = new int[xVertices.length+1024];
        fastPfor.compress(xVertices, inputoffset, xVertices.length, compressed, outputoffset);
        IntWrapper inputoffset2 = new IntWrapper(0);
        IntWrapper outputoffset2 = new IntWrapper(0);
        int [] compressed2 = new int[yVertices.length+1024];
        fastPfor.compress(yVertices, inputoffset2, yVertices.length, compressed2, outputoffset2);
        var totalSize2 = ((outputoffset.intValue()*4) + (outputoffset2.intValue()*4))/1024;
        System.out.println("SIMD-FastPfor compressed from "+ values.length*4/1024+"KB to "+totalSize2+"KB");
        var compressedTest = Arrays.copyOf(compressed,outputoffset.intValue());
        var compressedTest2 = Arrays.copyOf(compressed2,outputoffset2.intValue());


        var optPfd = new OptPFD();
        IntWrapper inputoffset4 = new IntWrapper(0);
        IntWrapper outputoffset4 = new IntWrapper(0);
        IntWrapper inputoffset5 = new IntWrapper(0);
        IntWrapper outputoffset5 = new IntWrapper(0);
        int [] compressed4 = new int[yVertices.length+1024];
        int [] compressed5 = new int[yVertices.length+1024];
        optPfd.compress(xVertices, inputoffset4, yVertices.length, compressed4, outputoffset4);
        optPfd.compress(yVertices, inputoffset5, yVertices.length, compressed5, outputoffset5);
        var totalSize4 = ((outputoffset4.intValue()*4/1024) + (outputoffset5.intValue()*4/1024));
        System.out.println("OptFPD compressed from "+ values.length*4/1024+"KB to "+totalSize4+"KB");

        //IntegratedBinaryPacking codec
        IntegratedIntCompressor iic = new IntegratedIntCompressor();
        int[] compressedXVertices = iic.compress(xVertices);
        int[] compressedYVertices = iic.compress(yVertices);
        var totalSize = (compressedXVertices.length + compressedYVertices.length) / 1024;
        System.out.println("BinaryPacking: " + totalSize);

        /*SkippableIntegratedComposition codec = new SkippableIntegratedComposition(new IntegratedBinaryPacking(),
                new IntegratedVariableByte());
        SkippableIntegratedComposition codec2 = new SkippableIntegratedComposition(new IntegratedBinaryPacking(),
                new IntegratedVariableByte());*/

        //Implementation of variable-byte -> for best performance, use it using the ByteIntegerCODEC interface
        var variableByte = new VariableByte();
        inputoffset = new IntWrapper(0);
        outputoffset = new IntWrapper(0);
        compressed = new int[xVertices.length+1024];
        variableByte.compress(xVertices, inputoffset, xVertices.length, compressed, outputoffset);
        inputoffset2 = new IntWrapper(0);
        outputoffset2 = new IntWrapper(0);
        compressed2 = new int[yVertices.length+1024];
        variableByte.compress(yVertices, inputoffset2, yVertices.length, compressed2, outputoffset2);
        totalSize2 = ((outputoffset.intValue()*4) + (outputoffset2.intValue()*4))/1024;
        System.out.println("Varint compressed from "+ values.length*4/1024+"KB to "+totalSize2+"KB");
    }
}
