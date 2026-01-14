package org.maplibre.mlt.converter.tessellation;

import com.google.gson.Gson;
import com.google.gson.JsonObject;
import jakarta.annotation.Nullable;
import java.io.BufferedReader;
import java.io.InputStreamReader;
import java.io.OutputStream;
import java.net.HttpURLConnection;
import java.net.URI;
import java.nio.charset.StandardCharsets;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.List;
import java.util.stream.Collectors;
import java.util.stream.Stream;
import org.maplibre.earcut4j.Earcut;
import org.jetbrains.annotations.NotNull;
import org.locationtech.jts.geom.*;
import org.maplibre.mlt.converter.geometry.Vertex;

public class TessellationUtils {
  private TessellationUtils() {}

  public static TessellatedPolygon tessellatePolygon(
      Polygon polygon, int indexOffset, @Nullable URI tessellateSource) {
    final var flattenedCoordinates = flatCoordinatesWithoutClosingPoint(polygon);

    final var holeIndices = new ArrayList<Integer>();
    var numVertices = polygon.getExteriorRing().getCoordinates().length - 1;
    for (int i = 0; i < polygon.getNumInteriorRing(); i++) {
      holeIndices.add(numVertices);
      numVertices += polygon.getInteriorRingN(i).getCoordinates().length - 1;
    }

    final var holeIndicesArray =
        !holeIndices.isEmpty() ? holeIndices.stream().mapToInt(i -> i).toArray() : null;

    Stream<Integer> indices;
    if (tessellateSource != null) {
      indices =
          Arrays.stream(
                  tessellatePolygonRemote(flattenedCoordinates, holeIndicesArray, tessellateSource))
              .boxed();
    } else {
      indices = Earcut.earcut(flattenedCoordinates, holeIndicesArray, 2).stream();
    }

    final var indexList = indices.map(index -> index + indexOffset).toList();
    final var numTriangles = indexList.size() / 3;
    return new TessellatedPolygon(indexList, numTriangles, numVertices);
  }

  private static int[] tessellatePolygonRemote(
      double[] flattenedCoordinates, int[] holeIndicesArray, @NotNull URI tessellateSource) {
    try {
      HttpURLConnection conn = (HttpURLConnection) tessellateSource.toURL().openConnection();
      conn.setRequestMethod("POST");
      conn.setRequestProperty("Content-Type", "application/json");
      conn.setRequestProperty("Accept", "application/json");
      conn.setDoOutput(true);

      JsonObject requestBody = new JsonObject();
      requestBody.add("vertices", new Gson().toJsonTree(flattenedCoordinates));
      requestBody.add("holes", new Gson().toJsonTree(holeIndicesArray));

      try (OutputStream os = conn.getOutputStream()) {
        byte[] input = requestBody.toString().getBytes(StandardCharsets.UTF_8);
        os.write(input, 0, input.length);
      }

      try (BufferedReader br =
          new BufferedReader(
              new InputStreamReader(conn.getInputStream(), StandardCharsets.UTF_8))) {
        StringBuilder response = new StringBuilder();
        String responseLine;
        while ((responseLine = br.readLine()) != null) {
          response.append(responseLine.trim());
        }

        Gson gson = new Gson();
        JsonObject jsonResponse = gson.fromJson(response.toString(), JsonObject.class);
        return gson.fromJson(jsonResponse.get("indices"), int[].class);
      }
    } catch (Exception e) {
      throw new RuntimeException(e);
    }
  }

  public static TessellatedPolygon tessellateMultiPolygon(
      MultiPolygon multiPolygon, @Nullable URI tessellateSource) {
    List<Integer> indexBuffer = new ArrayList<>();
    var numTriangles = 0;

    var numVertices = 0;
    /* The range of the values of the indices are created per MultiPolygon,
     *  which means the min index of every new MultiPolygon is 0.
     *  Because of the filtering happening on the map renderer side the indices have
     *  to be adjusted with an offset.
     *  */
    for (int i = 0; i < multiPolygon.getNumGeometries(); i++) {
      var polygon = (Polygon) multiPolygon.getGeometryN(i);
      var tessellatedPolygon = tessellatePolygon(polygon, numVertices, tessellateSource);
      indexBuffer.addAll(tessellatedPolygon.indexBuffer());

      numTriangles += tessellatedPolygon.numTriangles();
      // indexOffset = tessellatedPolygon.indexBuffer().stream().max(Integer::compareTo).get() + 1;
      numVertices += tessellatedPolygon.numVertices();
    }

    return new TessellatedPolygon(indexBuffer, numTriangles, numVertices);
  }

  private static double[] flatCoordinatesWithoutClosingPoint(Polygon polygon) {
    var shell = polygon.getExteriorRing();
    var shellCoordinates = shell.getCoordinates();
    var coordinates =
        new ArrayList<>(Arrays.asList(shellCoordinates).subList(0, shellCoordinates.length - 1));

    for (var i = 0; i < polygon.getNumInteriorRing(); ++i) {
      var hole = polygon.getInteriorRingN(i);
      var childCoordinates = hole.getCoordinates();
      coordinates.addAll(Arrays.asList(childCoordinates).subList(0, childCoordinates.length - 1));
    }

    return coordinates.stream()
        .flatMapToDouble(c -> Arrays.stream(new double[] {c.x, c.y}))
        .toArray();
  }

  private static double[] flatCoordinates(Polygon polygon) {
    var shell = polygon.getExteriorRing();
    var shellCoordinates = shell.getCoordinates();
    var coordinates =
        new ArrayList<>(Arrays.asList(shellCoordinates).subList(0, shellCoordinates.length));

    for (var i = 0; i < polygon.getNumInteriorRing(); ++i) {
      var hole = polygon.getInteriorRingN(i);
      var childCoordinates = hole.getCoordinates();
      coordinates.addAll(Arrays.asList(childCoordinates).subList(0, childCoordinates.length));
    }

    return coordinates.stream()
        .flatMapToDouble(c -> Arrays.stream(new double[] {c.x, c.y}))
        .toArray();
  }

  private static double[] flatPolygonWithClosingPoint(Polygon polygon) {
    var numRings = polygon.getNumInteriorRing() + 1;

    var exteriorRing = polygon.getExteriorRing();
    var shell =
        new GeometryFactory()
            .createLineString(
                Arrays.copyOf(exteriorRing.getCoordinates(), exteriorRing.getCoordinates().length));
    var shellVertices = flatLineString(shell);
    var vertexBuffer = new ArrayList<>(shellVertices);

    for (var i = 0; i < polygon.getNumInteriorRing(); i++) {
      var interiorRing = polygon.getInteriorRingN(i);
      var ring =
          new GeometryFactory()
              .createLineString(
                  Arrays.copyOf(
                      interiorRing.getCoordinates(), interiorRing.getCoordinates().length));

      var ringVertices = flatLineString(ring);
      vertexBuffer.addAll(ringVertices);
    }

    return vertexBuffer.stream()
        .flatMapToDouble(v -> Arrays.stream(new double[] {v.x(), v.y()}))
        .toArray();
  }

  private static List<Vertex> flatLineString(LineString lineString) {
    return Arrays.stream(lineString.getCoordinates())
        .map(v -> new Vertex((int) v.x, (int) v.y))
        .collect(Collectors.toList());
  }

  /*public static TessellatedPolygon tessellatePolygon(Polygon polygon, int indexOffset) {
    //var flattenedCoordinates = flatCoordinatesWithoutClosingPoint(polygon);
    var flattenedCoordinates = flatPolygonWithClosingPoint(polygon);

    var holeIndices = new ArrayList<Integer>();
    //var numVertices = polygon.getExteriorRing().getCoordinates().length - 1;
    var numVertices = polygon.getExteriorRing().getCoordinates().length;
    for(int i = 0; i < polygon.getNumInteriorRing(); i++) {
        holeIndices.add(numVertices);
        //numVertices += polygon.getInteriorRingN(i).getCoordinates().length - 1;
        numVertices += polygon.getInteriorRingN(i).getCoordinates().length;
    }

    var holeIndicesArray = !holeIndices.isEmpty() ? holeIndices.stream().mapToInt(i -> i).toArray() : null;
    var indices = Earcut.earcut(flattenedCoordinates, holeIndicesArray, 2);
    indices = indices.stream().map(index -> index + indexOffset).toList();
    var numTriangles = indices.size() / 3;
    return new TessellatedPolygon(indices, numTriangles);
  }*/

}
