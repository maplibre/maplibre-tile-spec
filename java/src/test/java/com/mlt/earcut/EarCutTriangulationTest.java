package com.mlt.earcut;

import com.mlt.converter.triangulation.TriangulationUtils;
import com.mlt.converter.triangulation.VectorTileConverter;
import org.junit.jupiter.api.Test;
import org.locationtech.jts.geom.*;
import org.locationtech.jts.util.Assert;

import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Paths;

import static org.mockito.Mockito.mock;
import static org.mockito.Mockito.when;


class EarcutTriangulationTest {
    @Test
    void triangulateVectorTile() throws IOException {
        var path = Paths.get("../test/fixtures/amazon/5_5_11.pbf").toAbsolutePath();

        var polygonConverter = new VectorTileConverter(path);

        Assert.isTrue(polygonConverter.getNumTrianglesPerPolygon().size() > 0);
        Assert.isTrue(polygonConverter.getIndexBuffer().size() > 0);

        logVectorizationInformation(polygonConverter);
    }

    @Test
    void triangulateVectorTile2() throws IOException {
        var path = Paths.get("../test/fixtures/omt/13_4266_5468.mvt").toAbsolutePath();
        var encodedTile = Files.readAllBytes(path);

        var polygonConverter = new VectorTileConverter(encodedTile);

        Assert.isTrue(polygonConverter.getNumTrianglesPerPolygon().size() > 0);
        Assert.isTrue(polygonConverter.getIndexBuffer().size() > 0);
        logVectorizationInformation(polygonConverter);
    }

    @Test
    void verifyPolygonTriangulation() {
        Polygon polygon = mock(Polygon.class);

        when(polygon.getCoordinates()).thenReturn(getQuadraticPolygonCoordinates());
        var triangulatedPolygon = TriangulationUtils.triangulatePolygon(polygon);

        Assert.isTrue(triangulatedPolygon.getNumTrianglesPerPolygon() == 2);
        Assert.isTrue(triangulatedPolygon.getIndexBuffer().size() == 6);
    }

    @Test
    void verifyMultiPolygonTriangulation() {
        MultiPolygon multiPolygon = mock(MultiPolygon.class);
        Polygon outerPolygon = mock(Polygon.class);
        when(outerPolygon.getCoordinates()).thenReturn(getQuadraticPolygonCoordinates());

        Polygon innerPolygon = mock(Polygon.class);
        Coordinate[] innerPolygonCoordinates = new Coordinate[4];
        innerPolygonCoordinates[0] = new Coordinate(0.25, 0.25);
        innerPolygonCoordinates[1] = new Coordinate(0.75, 0.25);
        innerPolygonCoordinates[2] = new Coordinate(0.75, 0.75);
        innerPolygonCoordinates[3] = new Coordinate(0.25, 0.75);

        when(innerPolygon.getCoordinates()).thenReturn(innerPolygonCoordinates);

        when(multiPolygon.getNumGeometries()).thenReturn(2);
        when(multiPolygon.getGeometryN(0)).thenReturn(outerPolygon);
        when(multiPolygon.getGeometryN(1)).thenReturn(innerPolygon);

        var triangulatedPolygon = TriangulationUtils.triangulatePolygonWithHoles(multiPolygon);

        Assert.isTrue(triangulatedPolygon.getNumTrianglesPerPolygon() == 8);
        Assert.isTrue(triangulatedPolygon.getIndexBuffer().size() == 24);
    }

    private Coordinate[] getQuadraticPolygonCoordinates() {
        Coordinate[] coordinates = new Coordinate[4];
        coordinates[0] = new Coordinate(0,0);
        coordinates[1]= new Coordinate(1,0);
        coordinates[2] = new Coordinate(1,1);
        coordinates[3] = new Coordinate(0,1);
        return coordinates;
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
}
