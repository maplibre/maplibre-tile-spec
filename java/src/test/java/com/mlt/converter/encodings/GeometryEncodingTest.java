package com.mlt.converter.encodings;

import com.mlt.TestSettings;
import com.mlt.converter.mvt.MvtUtils;
import com.mlt.converter.triangulation.TriangulationUtils;
import com.mlt.decoder.vectorized.VectorizedGeometryDecoder;
import com.mlt.metadata.stream.PhysicalLevelTechnique;
import java.io.IOException;
import java.nio.file.Path;
import java.nio.file.Paths;
import java.util.ArrayList;
import java.util.Arrays;
import me.lemire.integercompression.IntWrapper;
import org.junit.jupiter.api.Test;
import org.locationtech.jts.geom.Geometry;
import org.locationtech.jts.geom.MultiPolygon;
import org.locationtech.jts.geom.Polygon;
import org.locationtech.jts.util.Assert;

public class GeometryEncodingTest {
  Path mvtFilePath = Paths.get(TestSettings.BING_MVT_PATH, "4-8-5" + ".mvt");

  PhysicalLevelTechnique physicalLevelTechnique = PhysicalLevelTechnique.FAST_PFOR;

  @Test
  public void testTriangulatedGeometryEncodingForTile() throws IOException {
    var decodedMvTile = MvtUtils.decodeMvt(mvtFilePath);

    var geometries = new ArrayList<Geometry>();
    var featureIds = new ArrayList<Long>();

    decodedMvTile
        .layers()
        .forEach(
            layer -> {
              layer
                  .features()
                  .forEach(
                      feature -> {
                        geometries.add(feature.geometry());
                        featureIds.add(feature.id());
                      });
            });

    var sortSettings = new GeometryEncoder.SortSettings(false, featureIds);
    var encodedGeometryColumn =
        GeometryEncoder.encodeGeometryColumn(
            geometries, physicalLevelTechnique, sortSettings, true);
    var decodedGeometryColumn =
        VectorizedGeometryDecoder.decodeGeometryColumn(
            encodedGeometryColumn.encodedValues(),
            encodedGeometryColumn.numStreams(),
            new IntWrapper(0));

    Assert.isTrue(decodedGeometryColumn.indexBuffer().isPresent());
    Assert.isTrue(decodedGeometryColumn.numTrianglesPerPolygonBuffer().isPresent());
  }

  @Test
  public void testTriangulateGeometryColumnForPolygonLayer() throws IOException {
    var decodedMvTile = MvtUtils.decodeMvt(mvtFilePath);

    var geometries = new ArrayList<Geometry>();
    var featureIds = new ArrayList<Long>();
    var polygonFeature = decodedMvTile.layers().get(5).features().get(0);
    geometries.add(polygonFeature.geometry());
    featureIds.add(polygonFeature.id());

    var sortSettings = new GeometryEncoder.SortSettings(false, featureIds);
    var encodedGeometryColumn =
        GeometryEncoder.encodeGeometryColumn(
            geometries, physicalLevelTechnique, sortSettings, true);
    var decodedGeometryColumn =
        VectorizedGeometryDecoder.decodeGeometryColumn(
            encodedGeometryColumn.encodedValues(),
            encodedGeometryColumn.numStreams(),
            new IntWrapper(0));

    var expectedTriangulatedPolygon =
        TriangulationUtils.triangulatePolygon((Polygon) polygonFeature.geometry());
    var indexBuffer = expectedTriangulatedPolygon.getIndexBuffer();

    var expectedIndexBuffer = new int[indexBuffer.size()];
    for (int i = 0; i < indexBuffer.size(); i++) {
      expectedIndexBuffer[i] = indexBuffer.get(i);
    }

    Assert.isTrue(decodedGeometryColumn.indexBuffer().isPresent());
    Assert.isTrue(decodedGeometryColumn.numTrianglesPerPolygonBuffer().isPresent());
    Assert.equals(
        decodedGeometryColumn.numTrianglesPerPolygonBuffer().get().array()[0],
        expectedTriangulatedPolygon.getNumTrianglesPerPolygon());
    Assert.isTrue(
        Arrays.equals(decodedGeometryColumn.indexBuffer().get().array(), expectedIndexBuffer));
  }

  @Test
  public void testTriangulateGeometryColumnForMultiPolygonLayer() throws IOException {
    var decodedMvTile = MvtUtils.decodeMvt(mvtFilePath);

    var geometries = new ArrayList<Geometry>();
    var featureIds = new ArrayList<Long>();
    var polygonFeature = decodedMvTile.layers().get(0).features().get(0);
    geometries.add(polygonFeature.geometry());
    featureIds.add(polygonFeature.id());
    var sortSettings = new GeometryEncoder.SortSettings(false, featureIds);

    var encodedGeometryColumn =
        GeometryEncoder.encodeGeometryColumn(
            geometries, physicalLevelTechnique, sortSettings, true);
    var decodedGeometryColumn =
        VectorizedGeometryDecoder.decodeGeometryColumn(
            encodedGeometryColumn.encodedValues(),
            encodedGeometryColumn.numStreams(),
            new IntWrapper(0));

    var expectedTriangulatedMultiPolygon =
        TriangulationUtils.triangulatePolygonWithHoles((MultiPolygon) polygonFeature.geometry());
    var indexBuffer = expectedTriangulatedMultiPolygon.getIndexBuffer();

    var expectedIndexBuffer = new int[indexBuffer.size()];
    for (int i = 0; i < indexBuffer.size(); i++) {
      expectedIndexBuffer[i] = indexBuffer.get(i);
    }

    Assert.isTrue(decodedGeometryColumn.indexBuffer().isPresent());
    Assert.isTrue(decodedGeometryColumn.numTrianglesPerPolygonBuffer().isPresent());
    Assert.equals(
        decodedGeometryColumn.numTrianglesPerPolygonBuffer().get().array()[0],
        expectedTriangulatedMultiPolygon.getNumTrianglesPerPolygon());
    Assert.isTrue(
        Arrays.equals(decodedGeometryColumn.indexBuffer().get().array(), expectedIndexBuffer));
  }
}
