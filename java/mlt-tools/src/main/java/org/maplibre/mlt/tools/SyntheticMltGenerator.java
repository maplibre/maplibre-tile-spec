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
    generateExtent();
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
    var pol = feat(poly(c1, c2, c3, c1));
    write("polygon", pol, cfg());
    write("polygon_fpf", pol, cfg().fastPFOR());
    write("polygon_tes", pol, cfg().tessellate());
    write("polygon_morton_tes", pol, cfg().fastPFOR().tessellate());

    // Polygon with hole
    var polWithHole = feat(poly(ring(c1, c2, c3, c1), ring(h1, h2, h3, h1)));
    write("polygon_hole", polWithHole, cfg());
    write("polygon_hole_fpf", polWithHole, cfg().fastPFOR());

    // MultiPolygon
    var multiPol = feat(multi(poly(c1, c2, c3, c1), poly(h1, h3, c2, h1)));
    write("polygon_multi", multiPol, cfg());
    write("polygon_multi_fpf", multiPol, cfg().fastPFOR());
  }

  private static void generateMultiPoints() throws IOException {
    write("multipoint", feat(multi(p1, p2, p3)), cfg());
  }

  private static void generateMultiLineStrings() throws IOException {
    write("multiline", feat(multi(line(c1, c2), line(h1, h2, h3))), cfg());
  }

  private static void generateMixed() throws IOException {
    write(layer("mixed_pt_line", feat(p0), feat(line(c1, c2))), cfg());
    write(layer("mixed_pt_poly", feat(p0), feat(poly(c1, c2, c3, c1))), cfg());
    write(layer("mixed_line_poly", feat(line(c1, c2)), feat(poly(c1, c2, c3, c1))), cfg());
    write(layer("mixed_pt_mline", feat(p0), feat(multi(line(c1, c2), line(h1, h2, h3)))), cfg());

    write(
        layer(
            "mixed_all",
            feat(p0),
            feat(line(c1, c2)),
            feat(poly(c1, c2, c3, c1)),
            feat(multi(poly(c1, c2, c3, c1), poly(h1, h3, h2, h1)))),
        cfg());
  }

  private static void generateExtent() throws IOException {
    var e9 = 512;
    write(layer("extent_" + e9, e9, feat(line(c(0, 0), c(e9 - 1, e9 - 1)))), cfg());
    write(layer("extent_buf_" + e9, e9, feat(line(c(-42, -42), c(e9 + 42, e9 + 42)))), cfg());
    var e12 = 4096;
    write(layer("extent_" + e12, e12, feat(line(c(0, 0), c(e12 - 1, e12 - 1)))), cfg());
    write(layer("extent_buf_" + e12, e12, feat(line(c(-42, -42), c(e12 + 42, e12 + 42)))), cfg());
    var e17 = 131072;
    write(layer("extent_" + e17, e17, feat(line(c(0, 0), c(e17 - 1, e17 - 1)))), cfg());
    write(layer("extent_buf_" + e17, e17, feat(line(c(-42, -42), c(e17 + 42, e17 + 42)))), cfg());
    var e30 = 1073741824;
    write(layer("extent_" + e30, e30, feat(line(c(0, 0), c(e30 - 1, e30 - 1)))), cfg());
    write(layer("extent_buf_" + e30, e30, feat(line(c(-42, -42), c(e30 + 42, e30 + 42)))), cfg());
  }

  private static void generateIds() throws IOException {
    write("id0", idFeat(0), cfg().ids());
    write("id", idFeat(100), cfg().ids());
    write("id64", idFeat(9_234_567_890L), cfg().ids());

    var ids32 = array(idFeat(103), idFeat(103), idFeat(103), idFeat(103));
    write(layer("ids", ids32), cfg().ids());
    write(layer("ids_delta", ids32), cfg(DELTA).ids());
    write(layer("ids_rle", ids32), cfg(RLE).ids());
    write(layer("ids_delta_rle", ids32), cfg(DELTA_RLE).ids());

    var ids64 =
        array(
            idFeat(9_234_567_890L),
            idFeat(9_234_567_890L),
            idFeat(9_234_567_890L),
            idFeat(9_234_567_890L));
    write(layer("ids64", ids64), cfg().ids());
    write(layer("ids64_delta", ids64), cfg(DELTA).ids());
    write(layer("ids64_rle", ids64), cfg(RLE).ids());
    write(layer("ids64_delta_rle", ids64), cfg(DELTA_RLE).ids());
  }

  @SuppressWarnings("cast")
  private static void generateProperties() throws IOException {
    // Scalar property types
    write("prop_bool", feat(p0, prop("val", true)), cfg());
    write("prop_bool_false", feat(p0, prop("val", false)), cfg());
    // FIXME: not yet supported
    // write("prop_i8", feat(p0, prop("val", (byte) 42)), cfg());
    // write("prop_i8_neg", feat(p0, prop("val", (byte) -42)), cfg());
    // write("prop_i8_min", feat(p0, prop("val", Byte.MIN_VALUE)), cfg());
    // write("prop_i8_max", feat(p0, prop("val", Byte.MAX_VALUE)), cfg());
    // write("prop_i16", feat(p0, prop("val", (short) 42)), cfg());
    // write("prop_i16_neg", feat(p0, prop("val", (short) -42)), cfg());
    // write("prop_i16_min", feat(p0, prop("val", Short.MIN_VALUE)), cfg());
    // write("prop_i16_max", feat(p0, prop("val", Short.MAX_VALUE)), cfg());
    write("prop_i32", feat(p0, prop("val", (int) 42)), cfg());
    write("prop_i32_neg", feat(p0, prop("val", (int) -42)), cfg());
    write("prop_i32_min", feat(p0, prop("val", Integer.MIN_VALUE)), cfg());
    write("prop_i32_max", feat(p0, prop("val", Integer.MAX_VALUE)), cfg());
    write("prop_i64", feat(p0, prop("val", (long) 9_876_543_210L)), cfg());
    write("prop_i64_neg", feat(p0, prop("val", (long) -9_876_543_210L)), cfg());
    write("prop_i64_min", feat(p0, prop("val", Long.MIN_VALUE)), cfg());
    write("prop_i64_max", feat(p0, prop("val", Long.MAX_VALUE)), cfg());
    write("prop_f32", feat(p0, prop("val", (float) 3.14f)), cfg());
    write("prop_f32_min", feat(p0, prop("val", Float.MIN_VALUE)), cfg());
    write("prop_f32_neg_inf", feat(p0, prop("val", Float.NEGATIVE_INFINITY)), cfg());
    write("prop_f32_max", feat(p0, prop("val", Float.MAX_VALUE)), cfg());
    write("prop_f32_pos_inf", feat(p0, prop("val", Float.POSITIVE_INFINITY)), cfg());
    write("prop_f32_nan", feat(p0, prop("val", Float.NaN)), cfg());
    write("prop_f64", feat(p0, prop("val", (double) 3.141592653589793)), cfg());
    write("prop_f64_neg_inf", feat(p0, prop("val", Double.NEGATIVE_INFINITY)), cfg());
    write("prop_f64_min", feat(p0, prop("val", Double.MIN_VALUE)), cfg());
    write("prop_f64_max", feat(p0, prop("val", Double.MAX_VALUE)), cfg());
    // FIXME: Fails in Java as it Produces the same output as prop_f64_max
    // write("prop_f64_pos_inf", feat(p0, prop("val", Double.POSITIVE_INFINITY)), cfg());
    write("prop_f64_nan", feat(p0, prop("val", Double.NaN)), cfg());

    // Mixed properties - single feature demonstrating multiple property types
    write(
        "props_mixed",
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
            feat(p1, prop("val", 42)),
            feat(p2, prop("val", 42)),
            feat(p3, prop("val", 42)),
            feat(ph1, prop("val", 42)));
    write(layer("props_i32", feat_ints), cfg());
    write(layer("props_i32_delta", feat_ints), cfg(DELTA));
    write(layer("props_i32_rle", feat_ints), cfg(RLE));
    write(layer("props_i32_delta_rle", feat_ints), cfg(DELTA_RLE));

    var feat_str =
        array(
            feat(p1, prop("val", "residential_zone_north_sector_1")),
            feat(p2, prop("val", "commercial_zone_south_sector_2")),
            feat(p3, prop("val", "industrial_zone_east_sector_3")),
            feat(ph1, prop("val", "park_zone_west_sector_4")),
            feat(ph2, prop("val", "water_zone_north_sector_5")),
            feat(ph3, prop("val", "residential_zone_south_sector_6")));
    write(layer("props_str", feat_str), cfg());
    write(layer("props_str_fsst", feat_str), cfg().fsst());
  }
}
