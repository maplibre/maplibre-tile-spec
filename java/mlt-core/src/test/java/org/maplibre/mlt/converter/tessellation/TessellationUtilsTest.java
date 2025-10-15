package org.maplibre.mlt.converter.tessellation;

import static org.junit.jupiter.api.Assertions.assertEquals;

import java.util.ArrayList;
import java.util.stream.Collectors;
import java.util.stream.Stream;
import org.locationtech.jts.geom.*;

public class TessellationUtilsTest {
  private TessellationUtilsTest() {}

  // @Test
  public void tessellateMultiPolygon_PolygonsWithoutHoles() {
    var geometryFactory = new GeometryFactory();
    var shell1 =
        new Coordinate[] {
          new Coordinate(0, 0),
          new Coordinate(10, 0),
          new Coordinate(10, 10),
          new Coordinate(0, 10),
          new Coordinate(0, 0)
        };
    var polygon1 = geometryFactory.createPolygon(shell1);
    var shell2 =
        new Coordinate[] {
          new Coordinate(20, 20),
          new Coordinate(40, 20),
          new Coordinate(40, 40),
          new Coordinate(20, 40),
          new Coordinate(20, 20)
        };
    var polygon2 = geometryFactory.createPolygon(shell2);
    var multiPolygon = geometryFactory.createMultiPolygon(new Polygon[] {polygon1, polygon2});

    var tessellatedPolygon = TessellationUtils.tessellateMultiPolygon(multiPolygon, null);

    var expectedIndexBuffer =
        Stream.of(3, 0, 1, 1, 2, 3, 7, 4, 5, 5, 6, 7)
            .collect(Collectors.toCollection(ArrayList::new));

    assertEquals(4, tessellatedPolygon.numTriangles());
    assertEquals(expectedIndexBuffer, tessellatedPolygon.indexBuffer());
  }

  // @Test
  public void tessellateMultiPolygon_PolygonsWithHoles() {
    var geometryFactory = new GeometryFactory();
    var shell1 =
        new Coordinate[] {
          new Coordinate(0, 0),
          new Coordinate(10, 0),
          new Coordinate(10, 10),
          new Coordinate(0, 10),
          new Coordinate(0, 0)
        };
    var hole1 =
        new Coordinate[] {
          new Coordinate(5, 5),
          new Coordinate(5, 7),
          new Coordinate(7, 7),
          new Coordinate(7, 5),
          new Coordinate(5, 5)
        };
    /*var hole2 = new Coordinate[] {
            new Coordinate(8, 8),
            new Coordinate(8, 9),
            new Coordinate(9, 9),
            new Coordinate(9, 8),
            new Coordinate(8, 8)
    };*/
    var polygon1 =
        geometryFactory.createPolygon(
            geometryFactory.createLinearRing(shell1),
            new LinearRing[] {geometryFactory.createLinearRing(hole1)});
    var shell2 =
        new Coordinate[] {
          new Coordinate(20, 20),
          new Coordinate(40, 20),
          new Coordinate(40, 40),
          new Coordinate(20, 40),
          new Coordinate(20, 20)
        };
    var polygon2 = geometryFactory.createPolygon(shell2);
    var multiPolygon = geometryFactory.createMultiPolygon(new Polygon[] {polygon1, polygon2});

    var tessellatedPolygon = TessellationUtils.tessellateMultiPolygon(multiPolygon, null);

    assertEquals(10, tessellatedPolygon.numTriangles());
  }
}
