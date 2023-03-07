package com.covt.compression;
import com.covt.compression.geometry.Point;
import com.fasterxml.jackson.databind.ObjectMapper;
import org.apache.parquet.bytes.DirectByteBufferAllocator;
import org.apache.parquet.column.values.delta.DeltaBinaryPackingValuesWriterForInteger;

import java.io.FileInputStream;
import java.io.IOException;
import java.io.InputStream;

public class ParquetCompression {

    public static void main(String[] args) throws IOException {
        //InputStream inputStream = new FileInputStream("../data/transportationZoom5_idSorted.json");
        InputStream inputStream = new FileInputStream("../data/transportationZoom5_deltaFeature_idSorted.json");
        ObjectMapper mapper = new ObjectMapper();
        var values = new ObjectMapper().readValue(inputStream, Point[].class);

        var blockSize = 128;
        var miniBlockNum = 4;
        var writer = new DeltaBinaryPackingValuesWriterForInteger(
                blockSize, miniBlockNum, 100, 200, new DirectByteBufferAllocator());

        /* To get a good compression ratio the x and y component of a vertex has to be stored separately
        *  so that delta encoding is efficient -> not so efficient for decoding -> separate copy step when
        *  before transferring to the GPU */
        for(var value : values){
            writer.writeInteger(value.x);
        }

        for(var value : values){
            writer.writeInteger(value.y);
        }

        //TODO: current is this Delta of Delta
        var buffer = writer.getBytes();
        var bufferLength = buffer.toByteArray().length;
        System.out.println("Parquet size: " + bufferLength/1024);
    }

}
