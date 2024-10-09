package com.mlt.earcut;

import com.mlt.converter.triangulation.VectorTileConverter;
import org.junit.jupiter.api.Test;

import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Paths;


class EarcutVectorizationTest {
    @Test
    void vectorizeVectorTile() throws IOException {
        var path = Paths.get("../test/fixtures/amazon/5_5_11.pbf").toAbsolutePath();

        var polygonConverter = new VectorTileConverter(path);

        assert polygonConverter.getNumTrianglesPerPolygon().size() > 0;
        assert polygonConverter.getIndexBuffer().size() > 0;

        logVectorizationInformation(polygonConverter);
    }

    @Test
    void verifyVectorizedData() throws IOException {
        var path = Paths.get("../test/fixtures/omt/13_4266_5468.mvt").toAbsolutePath();
        var encodedTile = Files.readAllBytes(path);

        var polygonConverter = new VectorTileConverter(encodedTile);

        logVectorizationInformation(polygonConverter);
    }

    private int getByteSize(int[] array) {
        return 4 * array.length;
    }

    private void logVectorizationInformation(VectorTileConverter vectorTileConverter) throws IOException {
        var indexBufferSize = getByteSize(vectorTileConverter.getIndexBuffer().stream().mapToInt(i -> i).toArray());
        var encodedIndexBuffer = vectorTileConverter.getEncodedIndexBuffer();
        var gzippedIndexBuffer = vectorTileConverter.getGzippedIndexBuffer();

        var encodedPercentageOfOriginalSize = (double) encodedIndexBuffer.length / (double) indexBufferSize * 100;
        var gzippedPercentageOfOriginalSize = (double) gzippedIndexBuffer.length / (double) indexBufferSize * 100;


        System.out.println("------------ IndexBuffer result ------------");
        System.out.println("#### Byte size of integer index array: " + indexBufferSize);
        System.out.println("#### Byte size of encoded index array: " + encodedIndexBuffer.length);
        System.out.println("#### Byte size of gzipped index array: " + gzippedIndexBuffer.length);
        System.out.println("------");
        System.out.println("#### Array length of index integer array: " + vectorTileConverter.getIndexBuffer().size());
        System.out.println("#### Array length of encoded index byte array: " + encodedIndexBuffer.length);
        System.out.println("#### Array length of gzipped index byte array: " + gzippedIndexBuffer.length);
        System.out.println("---> Encoded vertices are " + encodedPercentageOfOriginalSize + " % the size of the original vertex buffer.");
        System.out.println("---> Gzipped vertices are " + gzippedPercentageOfOriginalSize + " % the size of the original vertex buffer.");
        System.out.println();

        var numTrianglesSize = getByteSize(vectorTileConverter.getNumTrianglesPerPolygon().stream().mapToInt(i -> i).toArray());
        var encodedNumTriangles = vectorTileConverter.getEncodedNumberOfTrianglesPerPolygon();
        var gzippedNumTriangles = vectorTileConverter.getGzippedNumberOfTrianglesPerPolygon();
        var percentageOfOriginalNumTrianglesSize = (double) encodedNumTriangles.length / (double) numTrianglesSize * 100;
        var gzippedPercentageOfOriginalNumTrianglesSize = (double) gzippedNumTriangles.length / (double) numTrianglesSize * 100;

        System.out.println("------------ NumTriangles result ------------");
        System.out.println("#### Byte size of integer index array: " + numTrianglesSize);
        System.out.println("#### Byte size of encoded index array: " + encodedNumTriangles.length);
        System.out.println("#### Byte size of gzipped index array: " + gzippedNumTriangles.length);
        System.out.println("------");
        System.out.println("#### Array length of index integer array: " + vectorTileConverter.getNumTrianglesPerPolygon().size());
        System.out.println("#### Array length of encoded index byte array: " + encodedNumTriangles.length);
        System.out.println("#### Array length of gzipped index byte array: " + gzippedNumTriangles.length);
        System.out.println("---> Encoded numTriangles are " + percentageOfOriginalNumTrianglesSize + " % the size of the original vertex buffer.");
        System.out.println("---> Gzipped numTriangles are " + gzippedPercentageOfOriginalNumTrianglesSize + " % the size of the original vertex buffer.");
        System.out.println();
    }
    // TODO: verify results with simple vector tile
}
