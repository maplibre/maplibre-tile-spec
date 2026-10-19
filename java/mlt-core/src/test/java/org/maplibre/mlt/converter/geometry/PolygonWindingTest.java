package org.maplibre.mlt.converter.geometry;

import static org.junit.jupiter.api.Assertions.assertArrayEquals;
import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertSame;

import org.junit.jupiter.api.Test;
import org.locationtech.jts.geom.Coordinate;
import org.locationtech.jts.geom.GeometryFactory;
import org.locationtech.jts.geom.LinearRing;
import org.locationtech.jts.geom.Polygon;

class PolygonWindingTest {
  private static final GeometryFactory FACTORY = new GeometryFactory();

  // Positive area by the surveyor's formula, i.e. clockwise with the y-axis pointing down
  private static final int[] EXTERIOR = {0, 0, 10, 0, 10, 10, 0, 10};
  private static final int[] HOLE = {2, 2, 2, 8, 8, 8, 8, 2};

  @Test
  void keepsRingsWoundAsMvt() {
    final var polygon = polygon(EXTERIOR, HOLE);
    assertSame(polygon, PolygonWinding.wind(polygon));
  }

  @Test
  void reversesExteriorAgainstMvtKeepingItsFirstVertex() {
    final var wound = PolygonWinding.wind(polygon(new int[] {0, 0, 0, 10, 10, 10, 10, 0}));
    assertRing(EXTERIOR, wound.getExteriorRing());
  }

  @Test
  void reversesHoleAgainstMvtKeepingItsFirstVertex() {
    final var wound = PolygonWinding.wind(polygon(EXTERIOR, new int[] {2, 2, 8, 2, 8, 8, 2, 8}));
    assertRing(EXTERIOR, wound.getExteriorRing());
    assertRing(HOLE, wound.getInteriorRingN(0));
  }

  @Test
  void windsEveryPolygonOfMultiPolygon() {
    final var wound =
        PolygonWinding.wind(
            FACTORY.createMultiPolygon(
                new Polygon[] {
                  polygon(EXTERIOR), polygon(new int[] {20, 20, 20, 30, 30, 30, 30, 20})
                }));
    assertRing(EXTERIOR, ((Polygon) wound.getGeometryN(0)).getExteriorRing());
    assertRing(
        new int[] {20, 20, 30, 20, 30, 30, 20, 30},
        ((Polygon) wound.getGeometryN(1)).getExteriorRing());
  }

  @Test
  void keepsRingWithoutArea() {
    final var polygon = polygon(new int[] {0, 0, 10, 0, 20, 0});
    assertSame(polygon, PolygonWinding.wind(polygon));
  }

  @Test
  void signOfAreaIsExactForLargeCoordinates() {
    final var max = Integer.MAX_VALUE;
    final var min = Integer.MIN_VALUE;
    // Products of these coordinates overflow a long when summed
    assertEquals(1, PolygonWinding.signedAreaSign(closed(min, min, max, min, max, max, min, max)));
    assertEquals(-1, PolygonWinding.signedAreaSign(closed(min, min, min, max, max, max, max, min)));
  }

  private static Polygon polygon(int[] exterior, int[]... holes) {
    final var interiors = new LinearRing[holes.length];
    for (var i = 0; i < holes.length; i++) {
      interiors[i] = FACTORY.createLinearRing(closed(holes[i]));
    }
    return FACTORY.createPolygon(FACTORY.createLinearRing(closed(exterior)), interiors);
  }

  private static Coordinate[] closed(int... xy) {
    final var coordinates = new Coordinate[xy.length / 2 + 1];
    for (var i = 0; i < xy.length / 2; i++) {
      coordinates[i] = new Coordinate(xy[2 * i], xy[2 * i + 1]);
    }
    coordinates[coordinates.length - 1] = coordinates[0];
    return coordinates;
  }

  private static void assertRing(int[] expected, LinearRing ring) {
    assertArrayEquals(closed(expected), ring.getCoordinates());
  }
}
