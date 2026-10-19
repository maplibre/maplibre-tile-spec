package org.maplibre.mlt.converter.geometry;

import java.math.BigInteger;
import org.jetbrains.annotations.NotNull;
import org.locationtech.jts.geom.Coordinate;
import org.locationtech.jts.geom.Geometry;
import org.locationtech.jts.geom.LinearRing;
import org.locationtech.jts.geom.MultiPolygon;
import org.locationtech.jts.geom.Polygon;

/// Winds polygon rings as MVT winds them: an exterior ring with a positive area by the surveyor's
/// formula in tile coordinates, and a hole with a negative one. That is clockwise on a tile whose
/// y-axis points down. A ring that encloses no area keeps its winding.
public final class PolygonWinding {
  private PolygonWinding() {}

  /// The geometry with its polygon rings wound as MVT winds them, or the geometry itself if they
  /// already are or it has no polygons. Geometry collections are wound by their elements.
  public static @NotNull Geometry wind(@NotNull Geometry geometry) {
    return switch (geometry) {
      case Polygon polygon -> wind(polygon);
      case MultiPolygon multiPolygon -> wind(multiPolygon);
      default -> geometry;
    };
  }

  public static @NotNull Polygon wind(@NotNull Polygon polygon) {
    if (!needsWinding(polygon)) {
      return polygon;
    }
    final var holes = new LinearRing[polygon.getNumInteriorRing()];
    for (var i = 0; i < holes.length; i++) {
      holes[i] = wind(polygon.getInteriorRingN(i), false);
    }
    return polygon.getFactory().createPolygon(wind(polygon.getExteriorRing(), true), holes);
  }

  public static @NotNull MultiPolygon wind(@NotNull MultiPolygon multiPolygon) {
    final var polygons = new Polygon[multiPolygon.getNumGeometries()];
    var changed = false;
    for (var i = 0; i < polygons.length; i++) {
      final var polygon = (Polygon) multiPolygon.getGeometryN(i);
      polygons[i] = wind(polygon);
      changed |= polygons[i] != polygon;
    }
    return changed ? multiPolygon.getFactory().createMultiPolygon(polygons) : multiPolygon;
  }

  private static boolean needsWinding(Polygon polygon) {
    if (needsReversal(polygon.getExteriorRing(), true)) {
      return true;
    }
    for (var i = 0; i < polygon.getNumInteriorRing(); i++) {
      if (needsReversal(polygon.getInteriorRingN(i), false)) {
        return true;
      }
    }
    return false;
  }

  /// The ring reversed if it winds against its role. Reversing a closed ring keeps its first
  /// vertex in place.
  private static LinearRing wind(LinearRing ring, boolean exterior) {
    return needsReversal(ring, exterior) ? ring.reverse() : ring;
  }

  private static boolean needsReversal(LinearRing ring, boolean exterior) {
    final var sign = signedAreaSign(ring.getCoordinates());
    return exterior ? sign < 0 : sign > 0;
  }

  /// The sign of the ring's area by the surveyor's formula, as MVT defines winding. The ring is
  /// closed, so pairing each vertex with the next one covers every edge. The sum is exact, falling
  /// back to BigInteger when products of large coordinates overflow a long.
  static int signedAreaSign(Coordinate[] ring) {
    try {
      var sum = 0L;
      for (var i = 0; i + 1 < ring.length; i++) {
        final var a = ring[i];
        final var b = ring[i + 1];
        sum =
            Math.addExact(
                sum,
                Math.subtractExact(
                    Math.multiplyExact((long) a.x, (long) b.y),
                    Math.multiplyExact((long) b.x, (long) a.y)));
      }
      return Long.signum(sum);
    } catch (ArithmeticException e) {
      var sum = BigInteger.ZERO;
      for (var i = 0; i + 1 < ring.length; i++) {
        final var a = ring[i];
        final var b = ring[i + 1];
        sum =
            sum.add(
                BigInteger.valueOf((long) a.x)
                    .multiply(BigInteger.valueOf((long) b.y))
                    .subtract(
                        BigInteger.valueOf((long) b.x).multiply(BigInteger.valueOf((long) a.y))));
      }
      return sum.signum();
    }
  }
}
