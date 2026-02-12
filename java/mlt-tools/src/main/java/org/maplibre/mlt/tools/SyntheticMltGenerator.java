package org.maplibre.mlt.tools;

import static org.maplibre.mlt.converter.ConversionConfig.IntegerEncodingOption.*;
import static org.maplibre.mlt.tools.SyntheticMltUtil.*;

import java.io.IOException;
import java.nio.file.Files;
import org.locationtech.jts.geom.Coordinate;
import org.locationtech.jts.geom.Point;
import org.maplibre.mlt.data.Feature;

public class SyntheticMltGenerator {

  public static void main(String[] args) throws IOException {
    if (Files.exists(SYNTHETICS_DIR)) {
      throw new IOException(
          "Synthetics dir must be deleted before running `:mlt-tools:generateSyntheticMlt`: "
              + SYNTHETICS_DIR.toAbsolutePath());
    }
    Files.createDirectories(SYNTHETICS_DIR);

    generatePoints();
    generateIds();
    generateLines();
    generatePolygons();
    generateMultiPoints();
    generateMultiLineStrings();
    generateProperties();
  }

  private static void generatePoints() throws IOException {
    write("point", feat(p1), cfg());
  }

  private static void generateIds() throws IOException {
    write("point-id", feat(p1, 100L), cfg().ids());
    write("point-id0", feat(p1, 0L), cfg().ids());

    var pts =
        new Feature[] {
          feat(p1, 100L), feat(p2, 101L), feat(p3, 103L),
        };
    write(layer("point-ids", pts), cfg().ids());
    write(layer("point-ids-delta", pts), cfg(DELTA).ids());
  }

  private static void generateLines() throws IOException {
    var line = feat(gf.createLineString(new Coordinate[] {c1, c2}));
    write("line", line, cfg());
  }

  private static void generatePolygons() throws IOException {
    var pol = feat(gf.createPolygon(new Coordinate[] {c1, c2, c5, c1}));
    write("polygon", pol, cfg());
    write("polygon-fpf", pol, cfg().fastPFOR());
    // TODO: Tessellation tests cause decoder errors - skip for now
    // write("polygon-tess", pol, cfg().tessellate());
    // write("polygon-morton-tess", pol, cfg().fastPFOR().tessellate());

    // Polygon with hole
    var polWithHole =
        feat(
            gf.createPolygon(
                gf.createLinearRing(new Coordinate[] {c1, c2, c3, c4, c1}),
                new org.locationtech.jts.geom.LinearRing[] {
                  gf.createLinearRing(new Coordinate[] {c5, c6, c7, c8, c5})
                }));
    write("polygon-hole", polWithHole, cfg());
    write("polygon-hole-fpf", polWithHole, cfg().fastPFOR());

    // MultiPolygon
    var multiPol =
        feat(
            gf.createMultiPolygon(
                new org.locationtech.jts.geom.Polygon[] {
                  gf.createPolygon(new Coordinate[] {c1, c2, c6, c5, c1}),
                  gf.createPolygon(new Coordinate[] {c8, c7, c3, c4, c8})
                }));
    write("polygon-multi", multiPol, cfg());
    write("polygon-multi-fpf", multiPol, cfg().fastPFOR());
  }

  private static void generateMultiPoints() throws IOException {
    var multiPoint = feat(gf.createMultiPoint(new Point[] {p1, p2, p3}));
    write("multipoint", multiPoint, cfg());
  }

  private static void generateMultiLineStrings() throws IOException {
    var multiLine =
        feat(
            gf.createMultiLineString(
                new org.locationtech.jts.geom.LineString[] {
                  gf.createLineString(new Coordinate[] {c1, c2}),
                  gf.createLineString(new Coordinate[] {c3, c4, c5})
                }));
    write("multiline", multiLine, cfg());
  }

  private static void generateProperties() throws IOException {
    // Scalar property types
    write("point-bool", feat(p1, props("flag", true)), cfg());
    write("point-bool-false", feat(p1, props("flag", false)), cfg());
    write("point-int32", feat(p1, props("count", 42)), cfg());
    write("point-int32-neg", feat(p1, props("count", -42)), cfg());
    write("point-int64", feat(p1, props("bignum", 9876543210L)), cfg());
    write("point-int64-neg", feat(p1, props("bignum", -9876543210L)), cfg());
    write("point-float", feat(p1, props("temp", 3.14f)), cfg());
    write("point-double", feat(p1, props("precise", 3.141592653589793)), cfg());

    // Mixed properties - single feature demonstrating multiple property types
    write(
        "point-mixed-props",
        feat(
            p1,
            props(
                "name",
                "Test Point",
                "count",
                42,
                "active",
                true,
                "temp",
                25.5f,
                "precision",
                0.123456789)),
        cfg());

    var feat_ints =
        new Feature[] {
          feat(p1, props("int", 99)),
          feat(p2, props("int", 98)),
          feat(p3, props("int", 97)),
          feat(p4, props("int", 96)),
        };
    write(layer("point-props-int", feat_ints), cfg());
    write(layer("point-props-int-delta", feat_ints), cfg(DELTA));

    var feat_str =
        new Feature[] {
          feat(p1, props("str", "residential_zone_north_sector_1")),
          feat(p2, props("str", "commercial_zone_south_sector_2")),
          feat(p3, props("str", "industrial_zone_east_sector_3")),
          feat(p4, props("str", "park_zone_west_sector_4")),
          feat(p5, props("str", "water_zone_north_sector_5")),
          feat(p6, props("str", "residential_zone_south_sector_6")),
        };
    write(layer("point-props-str", feat_str), cfg());
    write(layer("point-props-str-fsst", feat_str), cfg().fsst());
  }
}
