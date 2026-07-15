package org.maplibre.mlt.converter.encodings;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.net.URI;
import java.util.ArrayList;
import java.util.List;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.params.ParameterizedTest;
import org.junit.jupiter.params.provider.EnumSource;
import org.junit.jupiter.params.provider.ValueSource;
import org.locationtech.jts.geom.Coordinate;
import org.locationtech.jts.geom.Geometry;
import org.locationtech.jts.geom.GeometryFactory;
import org.locationtech.jts.geom.LineString;
import org.locationtech.jts.geom.Point;
import org.locationtech.jts.geom.Polygon;
import org.maplibre.mlt.converter.geometry.GeometryType;
import org.maplibre.mlt.converter.geometry.Vertex;
import org.maplibre.mlt.data.Feature;

class GeometryEncoderTest {
  @Test
  void nullGeom() {
    assertHasGeometry(null, false);
  }

  @Test
  void nullFeature() {
    assertFalse(GeometryEncoder.hasGeometry((Feature) null));
  }

  @ParameterizedTest
  @EnumSource(GeometryType.class)
  void validGeometries_return_true(final GeometryType type) {
    assertTrue(GeometryEncoder.hasGeometry(makeGeometry(type)), type.name());
  }

  @ParameterizedTest
  @EnumSource(GeometryType.class)
  void emptyGeometries_return_false(final GeometryType type) {
    assertFalse(GeometryEncoder.hasGeometry(makeEmptyGeometry(type)), type.name());
  }

  private Geometry makeGeometry(final GeometryType type) {
    return switch (type) {
      case POINT -> GF.createPoint(coord(0, 0));
      case LINESTRING -> GF.createLineString(new Coordinate[] {coord(0, 0), coord(1, 1)});
      case POLYGON -> makePolygon(SAMPLE_SQUARE);
      case MULTIPOINT -> {
        final var p1 = GF.createPoint(coord(0, 0));
        final var p2 = GF.createPoint(coord(1, 1));
        yield GF.createMultiPoint(new Point[] {p1, p2});
      }
      case MULTILINESTRING -> {
        final var l1 = GF.createLineString(new Coordinate[] {coord(0, 0), coord(1, 1)});
        final var l2 = GF.createLineString(new Coordinate[] {coord(2, 2), coord(3, 3)});
        yield GF.createMultiLineString(new LineString[] {l1, l2});
      }
      case MULTIPOLYGON -> GF.createMultiPolygon(new Polygon[] {makePolygon(SAMPLE_SQUARE)});
    };
  }

  private Geometry makeEmptyGeometry(final GeometryType type) {
    return switch (type) {
      case POINT -> emptyGeom();
      case LINESTRING -> GF.createLineString(new Coordinate[] {});
      case POLYGON -> GF.createPolygon(new Coordinate[] {});
      case MULTIPOINT -> emptyGeom();
      case MULTILINESTRING -> GF.createMultiLineString(new LineString[] {});
      case MULTIPOLYGON -> GF.createMultiPolygon(new Polygon[] {});
    };
  }

  @Test
  void pointCollection() {
    final var collection =
        GF.createGeometryCollection(new Geometry[] {GF.createPoint(coord(0, 0))});
    assertHasGeometry(collection, true);
  }

  @Test
  void lineStringCollection() {
    final var collection =
        GF.createGeometryCollection(
            new Geometry[] {GF.createLineString(new Coordinate[] {coord(0, 0), coord(1, 1)})});
    assertHasGeometry(collection, true);
  }

  @Test
  void polygonCollection() {
    final var collection = GF.createGeometryCollection(new Geometry[] {makePolygon(SAMPLE_SQUARE)});
    assertHasGeometry(collection, true);
  }

  @Test
  void collectionEmpty() {
    assertHasGeometry(GF.createGeometryCollection(new Geometry[] {}), false);
  }

  @Test
  void emptyGeomCollection() {
    final var collection = GF.createGeometryCollection(new Geometry[] {emptyGeom()});
    assertHasGeometry(collection, false);
  }

  @Test
  void collectionAnyNotEmpty() {
    final var collection =
        GF.createGeometryCollection(new Geometry[] {emptyGeom(), GF.createPoint(coord(0, 0))});
    assertHasGeometry(collection, true);
  }

  @Test
  void collectionAllEmpty() {
    final var collection =
        GF.createGeometryCollection(
            new Geometry[] {emptyGeom(), GF.createLineString(new Coordinate[] {})});
    assertHasGeometry(collection, false);
  }

  @ParameterizedTest
  @ValueSource(booleans = {true, false})
  void nestedCollection(final boolean hasContent) {
    final var inner =
        hasContent
            ? GF.createGeometryCollection(new Geometry[] {GF.createPoint(coord(0, 0))})
            : GF.createGeometryCollection(new Geometry[] {emptyGeom()});
    final var outer = GF.createGeometryCollection(new Geometry[] {inner});
    assertHasGeometry(outer, hasContent);
  }

  @Test
  void collectionLineString() throws Exception {
    final var point = GF.createPoint(coord(10, 20));
    final var lineString = GF.createLineString(new Coordinate[] {coord(30, 40), coord(50, 60)});
    final var geometryCollection = GF.createGeometryCollection(new Geometry[] {point, lineString});

    assertFlattenedEquals(
        runPrepareGeometry(List.of(geometryCollection)),
        runPrepareGeometry(List.of(point, lineString)));
  }

  @Test
  void collectionNested() throws Exception {
    final var point = GF.createPoint(coord(5, 10));
    final var innerCollection = GF.createGeometryCollection(new Geometry[] {point});
    final var outerCollection = GF.createGeometryCollection(new Geometry[] {innerCollection});

    assertEquals(
        runPrepareGeometry(List.of(outerCollection)).geometryTypes,
        runPrepareGeometry(List.of(point)).geometryTypes);
    assertEquals(
        runPrepareGeometry(List.of(outerCollection)).numGeometries,
        runPrepareGeometry(List.of(point)).numGeometries);
    assertEquals(
        runPrepareGeometry(List.of(outerCollection)).vertexBuffer,
        runPrepareGeometry(List.of(point)).vertexBuffer);
  }

  @Test
  void collectionMultiPolygon() throws Exception {
    final var multiPolygon =
        GF.createMultiPolygon(
            new Polygon[] {makePolygon(SAMPLE_SQUARE), makePolygon(SAMPLE_SQUARE_2)});
    final var geometryCollection = GF.createGeometryCollection(new Geometry[] {multiPolygon});

    assertFlattenedEquals(
        runPrepareGeometry(List.of(geometryCollection)), runPrepareGeometry(List.of(multiPolygon)));
  }

  @Test
  void collectionPolygon() throws Exception {
    final var geometryCollection =
        GF.createGeometryCollection(
            new Geometry[] {makePolygon(SAMPLE_SQUARE), makePolygon(SAMPLE_SQUARE_2)});

    assertFlattenedEquals(
        runPrepareGeometry(List.of(geometryCollection)),
        runPrepareGeometry(List.of(makePolygon(SAMPLE_SQUARE), makePolygon(SAMPLE_SQUARE_2))));
  }

  private static final GeometryFactory GF = new GeometryFactory();
  private static final Coordinate[] SAMPLE_SQUARE = {
    coord(0, 0), coord(1, 0), coord(1, 1), coord(0, 1), coord(0, 0)
  };
  private static final Coordinate[] SAMPLE_SQUARE_2 = {
    coord(2, 2), coord(3, 2), coord(3, 3), coord(2, 3), coord(2, 2)
  };

  private static Coordinate coord(final double x, final double y) {
    return new Coordinate(x, y);
  }

  private static Polygon makePolygon(final Coordinate[] coords) {
    return GF.createPolygon(coords);
  }

  private static Geometry emptyGeom() {
    return GF.createEmpty(2);
  }

  private static void assertHasGeometry(final Geometry geom, final boolean expected) {
    assertEquals(expected, GeometryEncoder.hasGeometry(geom));
  }

  private static void assertFlattenedEquals(final PrepareResult a, final PrepareResult b) {
    assertEquals(a.geometryTypes, b.geometryTypes);
    assertEquals(a.numGeometries, b.numGeometries);
    assertEquals(a.vertexBuffer, b.vertexBuffer);
    assertEquals(a.numParts, b.numParts);
    assertEquals(a.numRings, b.numRings);
  }

  private record PrepareResult(
      List<Integer> geometryTypes,
      List<Integer> numGeometries,
      List<Vertex> vertexBuffer,
      List<Integer> numParts,
      List<Integer> numRings) {}

  private PrepareResult runPrepareGeometry(final List<Geometry> geometries) throws Exception {
    final var method =
        GeometryEncoder.class.getDeclaredMethod(
            "prepareGeometry",
            List.class,
            ArrayList.class,
            ArrayList.class,
            ArrayList.class,
            ArrayList.class,
            ArrayList.class,
            ArrayList.class,
            ArrayList.class,
            URI.class);
    method.setAccessible(true);

    final var numGeometries = new ArrayList<Integer>();
    final var geometryTypes = new ArrayList<Integer>();
    final var vertexBuffer = new ArrayList<Vertex>();
    final var numParts = new ArrayList<Integer>();
    final var numRings = new ArrayList<Integer>();
    final var numTriangles = new ArrayList<Integer>();
    final var indexBuffer = new ArrayList<Integer>();

    method.invoke(
        null,
        geometries,
        numGeometries,
        geometryTypes,
        vertexBuffer,
        numParts,
        numRings,
        numTriangles,
        indexBuffer,
        (URI) null);

    return new PrepareResult(geometryTypes, numGeometries, vertexBuffer, numParts, numRings);
  }
}
