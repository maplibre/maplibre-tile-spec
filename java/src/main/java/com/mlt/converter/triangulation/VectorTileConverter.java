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
import no.ecc.vectortile.VectorTileDecoder;
import org.locationtech.jts.geom.Geometry;
import org.locationtech.jts.geom.MultiPolygon;
import org.locationtech.jts.geom.Polygon;

import java.io.IOException;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.List;

public class VectorTileConverter {
    private final ArrayList<Integer> numTrianglesPerPolygon = new ArrayList<>();

    private final ArrayList<Integer> indexBuffer = new ArrayList<>();

    public VectorTileConverter(Path vectorTilePath) throws IOException {
        var vectorTile = MvtUtils.decodeMvt(vectorTilePath);
        this.triangulatePolygons(vectorTile);
    }

    public VectorTileConverter(byte[] mvtTile) throws IOException {
        var vectorTile = MvtUtils.decodeMvtFast(mvtTile);
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

    private void triangulatePolygons(List<VectorTileDecoder.Feature> decodedTile) {
        for (VectorTileDecoder.Feature feature: decodedTile) {
            var geometry = feature.getGeometry().toString();
            if (geometry.contains(Geometry.TYPENAME_MULTIPOLYGON.toUpperCase())) {
                triangulateMultiPolygon((MultiPolygon) feature.getGeometry());
            } else if (geometry.contains(Geometry.TYPENAME_POLYGON.toUpperCase())) {
                triangulatePolygon((Polygon) feature.getGeometry());
            }
        }
    }

    private void triangulatePolygonFeature(Feature feature) {
        if (feature.geometry().getGeometryType().equals(Geometry.TYPENAME_POLYGON)) {
            var triangulatedPolygon = TriangulationUtils.triangulatePolygon((Polygon) feature.geometry());
            this.indexBuffer.addAll(triangulatedPolygon.getIndexBuffer());
            this.numTrianglesPerPolygon.add(triangulatedPolygon.getNumTrianglesPerPolygon());
        } else if (feature.geometry().getGeometryType().equals(Geometry.TYPENAME_MULTIPOLYGON)) {
            var triangulatedPolygon = TriangulationUtils.triangulatePolygonWithHoles((MultiPolygon) feature.geometry());
            this.indexBuffer.addAll(triangulatedPolygon.getIndexBuffer());
            this.numTrianglesPerPolygon.add(triangulatedPolygon.getNumTrianglesPerPolygon());        }
    }

    private void triangulatePolygon(Polygon polygon) {
        var triangulatedPolygon = TriangulationUtils.triangulatePolygon(polygon);
        this.indexBuffer.addAll(triangulatedPolygon.getIndexBuffer());
        this.numTrianglesPerPolygon.add(triangulatedPolygon.getNumTrianglesPerPolygon());
    }

    private void triangulateMultiPolygon(MultiPolygon multiPolygon) {
        var triangulatedPolygon = TriangulationUtils.triangulatePolygonWithHoles(multiPolygon);
        this.indexBuffer.addAll(triangulatedPolygon.getIndexBuffer());
        this.numTrianglesPerPolygon.add(triangulatedPolygon.getNumTrianglesPerPolygon());
    }
}
