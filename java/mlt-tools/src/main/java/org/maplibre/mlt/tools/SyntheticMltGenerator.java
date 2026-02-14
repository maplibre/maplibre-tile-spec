package org.maplibre.mlt.tools;

import static org.maplibre.mlt.converter.ConversionConfig.IntegerEncodingOption.*;
import static org.maplibre.mlt.tools.SyntheticMltUtil.*;

import java.io.IOException;
import java.nio.file.Files;

public class SyntheticMltGenerator {

  public static void main(String[] args) throws IOException {
    if (Files.exists(SYNTHETICS_DIR)) {
      throw new IOException(
          "Synthetics dir must be deleted before running `:mlt-tools:generateSyntheticMlt`: "
              + SYNTHETICS_DIR.toAbsolutePath());
    }
    Files.createDirectories(SYNTHETICS_DIR);

    generatePoints();
    generateLines();
    generatePolygons();
    generateMultiPoints();
    generateMultiLineStrings();
    generateMixed();
    generateIds();
    generateProperties();
  }

  private static void generatePoints() throws IOException {
    write("point", feat(p0), cfg());
  }

  private static void generateLines() throws IOException {
    write("line", feat(line(c1, c2)), cfg());
  }

  private static void generatePolygons() throws IOException {
    var pol = feat(poly(c1, c2, c5, c1));
    write("polygon", pol, cfg());
    write("polygon-fpf", pol, cfg().fastPFOR());
    write("polygon-tes", pol, cfg().tessellate());
    write("polygon-morton-tes", pol, cfg().fastPFOR().tessellate());

    // Polygon with hole
    var polWithHole = feat(poly(ring(c1, c2, c3, c4, c1), ring(c5, c6, c7, c8, c5)));
    write("polygon-hole", polWithHole, cfg());
    write("polygon-hole-fpf", polWithHole, cfg().fastPFOR());

    // MultiPolygon
    var multiPol = feat(multi(poly(c1, c2, c6, c5, c1), poly(c8, c7, c3, c4, c8)));
    write("polygon-multi", multiPol, cfg());
    write("polygon-multi-fpf", multiPol, cfg().fastPFOR());
  }

  private static void generateMultiPoints() throws IOException {
    write("multipoint", feat(multi(p1, p2, p3)), cfg());
  }

  private static void generateMultiLineStrings() throws IOException {
    write("multiline", feat(multi(line(c1, c2), line(c3, c4, c5))), cfg());
  }

  private static void generateMixed() throws IOException {
    write(layer("mixed-pt-line", feat(p0), feat(line(c1, c2))), cfg());
    write(layer("mixed-pt-poly", feat(p0), feat(poly(c1, c2, c5, c1))), cfg());
    write(layer("mixed-line-poly", feat(line(c1, c2)), feat(poly(c1, c2, c5, c1))), cfg());
    write(layer("mixed-pt-mline", feat(p0), feat(multi(line(c1, c2), line(c3, c4, c5)))), cfg());

    write(
        layer(
            "mixed-all",
            feat(p0),
            feat(line(c1, c2)),
            feat(poly(c1, c2, c5, c1)),
            feat(multi(poly(c1, c2, c6, c5, c1), poly(c8, c7, c3, c4, c8)))),
        cfg());
  }

  private static void generateIds() throws IOException {
    write("id0", idFeat(0), cfg().ids());
    write("id", idFeat(100), cfg().ids());
    write("id64", idFeat(9_234_567_890L), cfg().ids());

    var ids32 = array(idFeat(103), idFeat(103), idFeat(103), idFeat(103));
    write(layer("ids", ids32), cfg().ids());
    write(layer("ids-delta", ids32), cfg(DELTA).ids());
    write(layer("ids-rle", ids32), cfg(RLE).ids());
    write(layer("ids-delta-rle", ids32), cfg(DELTA_RLE).ids());

    var ids64 =
        array(
            idFeat(9_234_567_890L),
            idFeat(9_234_567_890L),
            idFeat(9_234_567_890L),
            idFeat(9_234_567_890L));
    write(layer("ids64", ids64), cfg().ids());
    write(layer("ids64-delta", ids64), cfg(DELTA).ids());
    write(layer("ids64-rle", ids64), cfg(RLE).ids());
    write(layer("ids64-delta-rle", ids64), cfg(DELTA_RLE).ids());
  }

  private static void generateProperties() throws IOException {
    // Scalar property types
    write("prop-bool", feat(p0, prop("flag", true)), cfg());
    write("prop-bool-false", feat(p0, prop("flag", false)), cfg());
    write("prop-int32", feat(p0, prop("count", 42)), cfg());
    write("prop-int32-neg", feat(p0, prop("count", -42)), cfg());
    write("prop-int64", feat(p0, prop("bignum", 9_876_543_210L)), cfg());
    write("prop-int64-neg", feat(p0, prop("bignum", -9_876_543_210L)), cfg());
    write("prop-float", feat(p0, prop("temp", 3.14f)), cfg());
    write("prop-double", feat(p0, prop("precise", 3.141592653589793)), cfg());

    // Mixed properties - single feature demonstrating multiple property types
    write(
        "props-mixed",
        feat(
            p1,
            props(
                kv("name", "Test Point"),
                kv("count", 42),
                kv("active", true),
                kv("temp", 25.5f),
                kv("precision", 0.123456789))),
        cfg());

    var feat_ints =
        array(
            feat(p1, prop("int", 42)),
            feat(p2, prop("int", 42)),
            feat(p3, prop("int", 42)),
            feat(p4, prop("int", 42)));
    write(layer("props-int", feat_ints), cfg());
    write(layer("props-int-delta", feat_ints), cfg(DELTA));
    write(layer("props-int-rle", feat_ints), cfg(RLE));
    write(layer("props-int-delta-rle", feat_ints), cfg(DELTA_RLE));

    var feat_str =
        array(
            feat(p1, prop("str", "residential_zone_north_sector_1")),
            feat(p2, prop("str", "commercial_zone_south_sector_2")),
            feat(p3, prop("str", "industrial_zone_east_sector_3")),
            feat(p4, prop("str", "park_zone_west_sector_4")),
            feat(p5, prop("str", "water_zone_north_sector_5")),
            feat(p6, prop("str", "residential_zone_south_sector_6")));
    write(layer("props-str", feat_str), cfg());
    write(layer("props-str-fsst", feat_str), cfg().fsst());
  }
}
