package com.mlt.converter.triangulation;

import earcut4j.Earcut;
import java.util.ArrayList;
import java.util.List;
import org.apache.commons.lang3.ArrayUtils;
import org.locationtech.jts.geom.Coordinate;
import org.locationtech.jts.geom.MultiPolygon;
import org.locationtech.jts.geom.Polygon;

public class TriangulationUtils {
  private TriangulationUtils() {}

  public static TriangulatedPolygon triangulatePolygon(Polygon polygon) {
    var convertedCoordinates = convertCoordinates(polygon.getCoordinates());

    List<Integer> triangles = Earcut.earcut(convertedCoordinates, null, 2);

    ArrayList<Integer> indexBuffer = new ArrayList<>(triangles);
    var numTriangles = triangles.size() / 3;

    return new TriangulatedPolygon(indexBuffer, numTriangles);
  }

  public static TriangulatedPolygon triangulatePolygonWithHoles(MultiPolygon multiPolygon) {
    var holeIndex = 0;

    ArrayList<Double> multiPolygonCoordinates = new ArrayList<>();
    ArrayList<Integer> holeIndices = new ArrayList<>();

    for (int i = 0; i < multiPolygon.getNumGeometries(); i++) {
      // assertion: first polygon defines the outer linear ring and the other polygons define its
      // holes!
      if (i == 0) {
        holeIndex = multiPolygon.getGeometryN(i).getCoordinates().length;
        holeIndices.add(holeIndex);
      } else if (i != multiPolygon.getNumGeometries() - 1) {
        holeIndex += multiPolygon.getGeometryN(i).getCoordinates().length;
        holeIndices.add(holeIndex);
      }

      var coordinates = multiPolygon.getGeometryN(i).getCoordinates();
      for (Coordinate coordinate : coordinates) {
        multiPolygonCoordinates.add(coordinate.x);
        multiPolygonCoordinates.add(coordinate.y);
        if (!Double.isNaN(coordinate.z)) {
          multiPolygonCoordinates.add(coordinate.z);
        }
      }
    }

    var doubleArray = ArrayUtils.toPrimitive(multiPolygonCoordinates.toArray(new Double[0]));
    List<Integer> triangleVertices =
        Earcut.earcut(doubleArray, holeIndices.stream().mapToInt(Integer::intValue).toArray(), 2);

    ArrayList<Integer> indexBuffer = new ArrayList<>(triangleVertices);
    var numTriangles = triangleVertices.size() / 3;

    return new TriangulatedPolygon(indexBuffer, numTriangles);
  }

  private static double[] convertCoordinates(Coordinate[] coordinates) {
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
