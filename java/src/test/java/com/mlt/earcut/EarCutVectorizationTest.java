package com.mlt.earcut;

import com.mlt.converter.triangulation.PolygonConverter;
import org.junit.jupiter.api.Test;

import java.io.IOException;
import java.nio.file.Paths;


class EarcutVectorizationTest {
    @Test
    void vectorizeVectorTile() throws IOException {
        var path = Paths.get("../test/fixtures/amazon/5_5_11.pbf").toAbsolutePath();

        var polygonConverter = new PolygonConverter(path);

        assert polygonConverter.getNumTrianglesPerPolygon().size() > 0;
        assert polygonConverter.getIndexBuffer().size() > 0;

        logVectorizationInformation(polygonConverter);
    }

    private int getByteSize(int[] array) {
        return 4 * array.length;
    }

    private void logVectorizationInformation(PolygonConverter polygonConverter) throws IOException {
        var indexBufferSize = getByteSize(polygonConverter.getIndexBuffer().stream().mapToInt(i -> i).toArray());
        var encodedIndexBuffer = polygonConverter.getEncodedIndexBuffer();
        var percentageOfOriginalSize = (double) encodedIndexBuffer.length / (double) indexBufferSize * 100;

        var gzippedIndexBuffer = polygonConverter.getGzippedIndexBuffer();

        System.out.println("------------ IndexBuffer result ------------");
        System.out.println("#### Byte size of integer index array: " + indexBufferSize);
        System.out.println("#### Byte size of encoded index array: " + encodedIndexBuffer.length);
        System.out.println("#### Byte size of gzipped index array: " + gzippedIndexBuffer.length);
        System.out.println("------");
        System.out.println("#### Array length of index integer array: " + polygonConverter.getIndexBuffer().size());
        System.out.println("#### Array length of encoded index byte array: " + encodedIndexBuffer.length);
        System.out.println("#### Array length of gzipped index byte array: " + gzippedIndexBuffer.length);
        System.out.println("---> Encoded vertices are " + percentageOfOriginalSize + " % the size of the original vertex buffer.");
        System.out.println();

        var numTrianglesSize = getByteSize(polygonConverter.getNumTrianglesPerPolygon().stream().mapToInt(i -> i).toArray());
        var encodedNumTriangles = polygonConverter.getEncodedNumberOfTrianglesPerPolygon();
        var percentageOfOriginalNumTrianglesSize = (double) encodedNumTriangles.length / (double) numTrianglesSize * 100;
        var gzippedNumTriangles = polygonConverter.getGzippedNumberOfTrianglesPerPolygon();

        System.out.println("------------ NumTriangles result ------------");
        System.out.println("#### Byte size of integer index array: " + numTrianglesSize);
        System.out.println("#### Byte size of encoded index array: " + encodedNumTriangles.length);
        System.out.println("#### Byte size of gzipped index array: " + gzippedNumTriangles.length);
        System.out.println("------");
        System.out.println("#### Array length of index integer array: " + polygonConverter.getNumTrianglesPerPolygon().size());
        System.out.println("#### Array length of encoded index byte array: " + encodedNumTriangles.length);
        System.out.println("#### Array length of gzipped index byte array: " + gzippedNumTriangles.length);
        System.out.println("---> Encoded vertices are " + percentageOfOriginalNumTrianglesSize + " % the size of the original vertex buffer.");
        System.out.println();
    }

    // TODO: verify log statements
    // TODO: verify results with simple vector tile
}
