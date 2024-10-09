package com.mlt.converter.triangulation;

import com.mlt.converter.encodings.EncodingUtils;
import com.mlt.converter.encodings.IntegerEncoder;
import com.mlt.converter.mvt.MapboxVectorTile;
import com.mlt.converter.mvt.MvtUtils;
import com.mlt.data.Feature;
import com.mlt.metadata.stream.LogicalStreamType;
import com.mlt.metadata.stream.OffsetType;
import com.mlt.metadata.stream.PhysicalLevelTechnique;
import com.mlt.metadata.stream.PhysicalStreamType;
import earcut4j.Earcut;
import org.apache.commons.lang3.ArrayUtils;
import org.locationtech.jts.geom.Coordinate;
import org.locationtech.jts.geom.Geometry;
import org.locationtech.jts.geom.MultiPolygon;

import java.io.IOException;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.List;

public class PolygonConverter {
    private final ArrayList<Integer> numTrianglesPerPolygon = new ArrayList<>();

    private final ArrayList<Integer> indexBuffer = new ArrayList<>();

    public PolygonConverter(Path vectorTilePath) throws IOException {
        var vectorTile = MvtUtils.decodeMvt(vectorTilePath);

        this.triangulatePolygons(vectorTile);
    }

    public List<Integer> getNumTrianglesPerPolygon() {
        return numTrianglesPerPolygon;
    }

    public List<Integer> getIndexBuffer() {
        return indexBuffer;
    }

    public byte[] getEncodedIndexBuffer() {
        return IntegerEncoder.encodeIntStream(this.indexBuffer, PhysicalLevelTechnique.FAST_PFOR, false, PhysicalStreamType.DATA, new LogicalStreamType(OffsetType.INDEX));
    }

    public byte[] getEncodedNumberOfTrianglesPerPolygon() {
        return IntegerEncoder.encodeIntStream(this.numTrianglesPerPolygon, PhysicalLevelTechnique.FAST_PFOR, false, PhysicalStreamType.DATA, new LogicalStreamType(OffsetType.INDEX));
    }

    public byte[] getGzippedIndexBuffer() throws IOException {
        return EncodingUtils.gzip(this.getEncodedIndexBuffer());
    }

    public byte[] getGzippedNumberOfTrianglesPerPolygon() throws IOException {
        return EncodingUtils.gzip(this.getEncodedNumberOfTrianglesPerPolygon());
    }

    private void triangulatePolygons(MapboxVectorTile vectorTile) {
        vectorTile.layers().forEach(layer -> layer.features().forEach(this::triangulatePolygonFeature));
    }

    private void triangulatePolygonFeature(Feature feature) {
        if (feature.geometry().getGeometryType().equals(Geometry.TYPENAME_POLYGON)) {
            var coordinates = convertCoordinates(feature.geometry().getCoordinates());

            List<Integer> triangles = Earcut.earcut(coordinates, null, 2);

            indexBuffer.addAll(triangles);
            numTrianglesPerPolygon.add(triangles.size() / 3);
        } else if (feature.geometry().getGeometryType().equals(Geometry.TYPENAME_MULTIPOLYGON)) {
            triangulatePolygonWithHoles(feature);
        }
    }

    private void triangulatePolygonWithHoles(Feature feature) {
        var multipolygon = (MultiPolygon) feature.geometry();
        var holeIndex = 0;

        ArrayList<Double> multiPolygonCoordinates = new ArrayList<>();
        ArrayList<Integer> holeIndices = new ArrayList<>();

        for (int i = 0; i < multipolygon.getNumGeometries(); i++) {
            // assertion: first polygon defines the outer linear ring and the other polygons its holes!
            if (i == 0) {
                holeIndex = multipolygon.getGeometryN(i).getCoordinates().length;
                holeIndices.add(holeIndex);
            } else if (i != multipolygon.getNumGeometries() - 1) {
                holeIndex += multipolygon.getGeometryN(i).getCoordinates().length;
                holeIndices.add(holeIndex);
            }

            var coordinates = multipolygon.getGeometryN(i).getCoordinates();
            for (Coordinate coordinate : coordinates) {
                multiPolygonCoordinates.add(coordinate.x);
                multiPolygonCoordinates.add(coordinate.y);
                if (!Double.isNaN(coordinate.z)) {
                    multiPolygonCoordinates.add(coordinate.z);
                }
            }
        }

        var doubleArray = ArrayUtils.toPrimitive(multiPolygonCoordinates.toArray(new Double[0]));
        List<Integer> triangleVertices = Earcut.earcut(doubleArray, holeIndices.stream().mapToInt(Integer::intValue).toArray(), 2);

        indexBuffer.addAll(triangleVertices);
        numTrianglesPerPolygon.add(triangleVertices.size() / 3);
    }

    private double[] convertCoordinates(Coordinate[] coordinates) {
        ArrayList<Double> convertedCoordinates = new ArrayList<>();

        for (Coordinate coordinate : coordinates) {
            convertedCoordinates.add(coordinate.x);
            convertedCoordinates.add(coordinate.y);
            if (!Double.isNaN(coordinate.z)) {
                convertedCoordinates.add(coordinate.z);
            }
        }
        Double[] array = convertedCoordinates.toArray(new Double[0]);

        return ArrayUtils.toPrimitive(array);
    }
}
