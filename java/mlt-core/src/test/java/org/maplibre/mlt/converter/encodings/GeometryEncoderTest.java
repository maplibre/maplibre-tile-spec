package org.maplibre.mlt.converter.encodings;

import static org.junit.jupiter.api.Assertions.assertArrayEquals;
import static org.junit.jupiter.api.Assertions.assertEquals;

import java.io.IOException;
import java.net.URI;
import java.util.List;
import org.junit.jupiter.api.Test;
import org.locationtech.jts.geom.*;
import org.maplibre.mlt.converter.MLTStreamObserverDefault;
import org.maplibre.mlt.metadata.stream.PhysicalLevelTechnique;

public class GeometryEncoderTest {

  private static final GeometryFactory GEOM = new GeometryFactory();
  private static final Polygon polygon1 =
      GEOM.createPolygon(
          GEOM.createLinearRing(
              new Coordinate[] {
                new Coordinate(1, 2),
                new Coordinate(3, 4),
                new Coordinate(5, 6),
                new Coordinate(1, 2),
              }));
  private static final Polygon polygon2 =
      GEOM.createPolygon(
          GEOM.createLinearRing(
              new Coordinate[] {
                new Coordinate(9, 8),
                new Coordinate(7, 6),
                new Coordinate(5, 4),
                new Coordinate(9, 8),
              }));

  private static final GeometryCollection nestedPolygon2 =
      GEOM.createGeometryCollection(
          new Geometry[] {GEOM.createGeometryCollection(new Geometry[] {polygon2})});

  private static final List<Geometry> flatGeometries = List.of(polygon1, polygon2);
  private static final List<Geometry> nestedGeometries = List.of(polygon1, nestedPolygon2);

  @Test
  public void encodeGeometryColumn_GeometryCollections_EncodeIdenticalToFlatList()
      throws IOException {
    var physicalLevelTechnique = PhysicalLevelTechnique.VARINT;
    var sortSettings = new GeometryEncoder.SortSettings(true, List.of());
    var useMortonEncoding = false;
    var streamObserver = new MLTStreamObserverDefault();

    var flatResult =
        GeometryEncoder.encodeGeometryColumn(
            flatGeometries,
            physicalLevelTechnique,
            sortSettings,
            useMortonEncoding,
            streamObserver);
    var nestedResult =
        GeometryEncoder.encodeGeometryColumn(
            nestedGeometries,
            physicalLevelTechnique,
            sortSettings,
            useMortonEncoding,
            streamObserver);

    assertEquals(
        flatResult.numStreams(),
        nestedResult.numStreams(),
        "GeometryCollection must not influence number of streams");
    assertArrayEquals(
        flatResult.encodedValues(),
        nestedResult.encodedValues(),
        "GeometryCollection must result in same encodedValues bytes");
    assertEquals(
        flatResult.geometryColumnSorted(),
        nestedResult.geometryColumnSorted(),
        "GeometryCollection influence geometryColumnSorted");
  }

  @Test
  public void encodePretessellatedGeometryColumn_GeometryCollections_EncodeIdenticalToFlatList()
      throws IOException {
    var physicalLevelTechnique = PhysicalLevelTechnique.VARINT;
    var sortSettings = new GeometryEncoder.SortSettings(true, List.of());
    var useMortonEncoding = false;
    var encodePolygonOutlines = false;
    URI tessellateSource = null;
    var streamObserver = new MLTStreamObserverDefault();

    var flatResult =
        GeometryEncoder.encodePretessellatedGeometryColumn(
            flatGeometries,
            physicalLevelTechnique,
            sortSettings,
            useMortonEncoding,
            encodePolygonOutlines,
            tessellateSource,
            streamObserver);
    var nestedResult =
        GeometryEncoder.encodePretessellatedGeometryColumn(
            nestedGeometries,
            physicalLevelTechnique,
            sortSettings,
            useMortonEncoding,
            encodePolygonOutlines,
            tessellateSource,
            streamObserver);

    assertEquals(
        flatResult.numStreams(),
        nestedResult.numStreams(),
        "GeometryCollection must not influence number of streams");
    assertArrayEquals(
        flatResult.encodedValues(),
        nestedResult.encodedValues(),
        "GeometryCollection must result in same encodedValues bytes");
    assertEquals(
        flatResult.geometryColumnSorted(),
        nestedResult.geometryColumnSorted(),
        "GeometryCollection influence geometryColumnSorted");
  }
}
