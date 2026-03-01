package org.maplibre.mlt.tools;

import static org.maplibre.mlt.converter.ConversionConfig.IntegerEncodingOption.*;
import static org.maplibre.mlt.tools.SyntheticMltUtil.*;

import java.io.IOException;
import java.math.BigInteger;
import java.nio.file.Files;
import java.util.ArrayList;
import java.util.List;
import java.util.stream.Collectors;
import org.locationtech.jts.geom.Coordinate;
import org.maplibre.mlt.data.Feature;
import org.maplibre.mlt.data.unsigned.U32;
import org.maplibre.mlt.data.unsigned.U64;

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
    generateSharedDictionaries();
  }

  private static void generatePoints() throws IOException {
    write("point", feat(p0), cfg());
  }

  private static void generateLines() throws IOException {
    write("line", feat(line1), cfg());

    // Morton (Z-order) line: de-interleave index bits into x/y (even/odd bits).
    int numPoints = 16; // 4x4 complete Morton block
    int scale = 8;
    int mortonBits = 4;
    var mortonCurve = new Coordinate[numPoints];
    for (var i = 0; i < mortonCurve.length; i++) {
      var x = 0;
      var y = 0;
      for (var b = 0; b < mortonBits; b++) {
        x |= ((i >> (2 * b)) & 1) << b;
        y |= ((i >> (2 * b + 1)) & 1) << b;
      }
      mortonCurve[i] = c(x * scale, y * scale);
    }
    write("line_morton", feat(line(mortonCurve)), cfg().morton());
  }

  private static void generatePolygons() throws IOException {
    var pol = feat(poly1);
    write("polygon", pol, cfg());
    write("polygon_fpf", pol, cfg().fastPFOR());
    write("polygon_tes", pol, cfg().tessellate());
    write("polygon_fpf_tes", pol, cfg().fastPFOR().tessellate());

    // Polygon with hole
    var polWithHole = feat(poly1h);
    write("polygon_hole", polWithHole, cfg());
    write("polygon_hole_fpf", polWithHole, cfg().fastPFOR());

    // MultiPolygon
    var multiPol = feat(multi(poly1, poly2));
    write("polygon_multi", multiPol, cfg());
    write("polygon_multi_fpf", multiPol, cfg().fastPFOR());
  }

  private static void generateMultiPoints() throws IOException {
    write("multipoint", feat(multi(c1, c2, c3)), cfg());
  }

  private static void generateMultiLineStrings() throws IOException {
    write("multiline", feat(multi(line1, line2)), cfg());
  }

  record GeomType(String sym, Feature feat) {}

  private static void generateMixCombination(List<GeomType> current) throws IOException {
    var name =
        "mix_"
            + current.size()
            + "_"
            + current.stream().map(GeomType::sym).collect(Collectors.joining("_"));
    var feats = current.stream().map(t -> t.feat).toArray(Feature[]::new);
    write(layer(name, feats), cfg().geomEnc(PLAIN));
  }

  private static void generateMixedCombine(GeomType[] arr, int k, int start, List<GeomType> current)
      throws IOException {
    if (current.size() == k) {
      generateMixCombination(current);
    } else {
      for (int i = start; i < arr.length; i++) {
        if (i > start && arr[i] == arr[i - 1]) {
          continue;
        }
        current.add(arr[i]);
        generateMixedCombine(arr, k, i + 1, current);
        current.removeLast();
      }
    }
  }

  private static void generateMixed() throws IOException {
    GeomType[] types = {
      new GeomType("pt", feat(gf.createPoint(c(38, 29)))),
      new GeomType("line", feat(line(c(5, 38), c(12, 45), c(9, 70)))),
      new GeomType("poly", feat(poly(c(55, 5), c(58, 28), c(75, 22), c(55, 5)))),
      new GeomType(
          "polyh",
          feat(
              poly(
                  ring(c(52, 35), c(14, 55), c(60, 72), c(52, 35)),
                  ring(c(32, 50), c(36, 60), c(24, 54), c(32, 50))))),
      new GeomType("mpt", feat(multi(c(6, 25), c(21, 41), c(23, 69)))),
      new GeomType(
          "mline", feat(multi(line(c(24, 10), c(42, 18)), line(c(30, 36), c(48, 52), c(35, 62))))),
      new GeomType(
          "mpoly",
          feat(
              multi(
                  poly(
                      ring(c(7, 20), c(21, 31), c(26, 9), c(7, 20)),
                      ring(c(15, 20), c(20, 15), c(18, 25), c(15, 20))),
                  poly(c(69, 57), c(71, 66), c(73, 64), c(69, 57))))),
    };

    for (int k = 2; k <= types.length; k++) {
      generateMixedCombine(types, k, 0, new ArrayList<>());
    }
    for (var a : types) {
      // Create A-A variants
      generateMixCombination(List.of(a, a));
      for (var b : types) {
        if (!a.sym.equals(b.sym)) {
          // Create A-B-A variants
          generateMixCombination(List.of(a, b, a));
        }
      }
    }
  }

  private static void generateExtent() throws IOException {
    int[] extents = {512, 4096, 131072, 1073741824};
    for (int e : extents) {
      write(layer("extent_" + e, e, feat(line(c(0, 0), c(e - 1, e - 1)))), cfg());
      write(layer("extent_buf_" + e, e, feat(line(c(-42, -42), c(e + 42, e + 42)))), cfg());
    }
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

    var optIds = array(idFeat(100), idFeat(101), idFeat(), idFeat(105), idFeat(106));
    write(layer("ids_opt", optIds), cfg().ids());
    write(layer("ids_opt_delta", optIds), cfg(DELTA).ids());

    var optIds64 = array(idFeat(), idFeat(9_234_567_890L), idFeat(101), idFeat(105), idFeat(106));
    write(layer("ids64_opt", optIds64), cfg().ids());
    write(layer("ids64_opt_delta", optIds64), cfg(DELTA).ids());
  }

  @SuppressWarnings("cast")
  private static void generateProperties() throws IOException {
    // Scalar property types
    write("prop_bool", feat(p0, prop("val", true)), cfg());
    write("prop_bool_false", feat(p0, prop("val", false)), cfg());
    // FIXME: needs support in the Java decoder + encoder
    // write("prop_i8", feat(p0, prop("val", (byte) 42)), cfg());
    // write("prop_i8_neg", feat(p0, prop("val", (byte) -42)), cfg());
    // write("prop_i8_min", feat(p0, prop("val", Byte.MIN_VALUE)), cfg());
    // write("prop_i8_max", feat(p0, prop("val", Byte.MAX_VALUE)), cfg());
    // write("prop_u8", feat(p0, prop("tinynum", U8.of(100))), cfg());
    // write("prop_u8_min", feat(p0, prop("tinynum", U8.of(0))), cfg());
    // write("prop_u8_max", feat(p0, prop("tinynum", U8.of(255))), cfg());
    // write("prop_i16", feat(p0, prop("val", (short) 42)), cfg());
    // write("prop_i16_neg", feat(p0, prop("val", (short) -42)), cfg());
    // write("prop_i16_min", feat(p0, prop("val", Short.MIN_VALUE)), cfg());
    // write("prop_i16_max", feat(p0, prop("val", Short.MAX_VALUE)), cfg());
    write("prop_i32", feat(p0, prop("val", (int) 42)), cfg());
    write("prop_i32_neg", feat(p0, prop("val", (int) -42)), cfg());
    write("prop_i32_min", feat(p0, prop("val", Integer.MIN_VALUE)), cfg());
    write("prop_i32_max", feat(p0, prop("val", Integer.MAX_VALUE)), cfg());
    write("prop_u32", feat(p0, prop("val", U32.of(42L))), cfg());
    write("prop_u32_min", feat(p0, prop("val", U32.of(0L))), cfg());
    write("prop_u32_max", feat(p0, prop("val", U32.of(0xFFFFFFFFL))), cfg());
    write("prop_i64", feat(p0, prop("val", (long) 9_876_543_210L)), cfg());
    write("prop_i64_neg", feat(p0, prop("val", (long) -9_876_543_210L)), cfg());
    write("prop_i64_min", feat(p0, prop("val", Long.MIN_VALUE)), cfg());
    write("prop_i64_max", feat(p0, prop("val", Long.MAX_VALUE)), cfg());
    write(
        "prop_u64",
        feat(p0, prop("bignum", U64.of(BigInteger.valueOf(1234567890123456789L)))),
        cfg());
    write("prop_u64_min", feat(p0, prop("bignum", U64.of(BigInteger.ZERO))), cfg());
    write(
        "prop_u64_max",
        feat(p0, prop("bignum", U64.of(new BigInteger("18446744073709551615")))),
        cfg());
    write("prop_f32", feat(p0, prop("val", (float) 3.14f)), cfg());
    write("prop_f32_neg_inf", feat(p0, prop("val", Float.NEGATIVE_INFINITY)), cfg());
    write("prop_f32_min_norm", feat(p0, prop("val", Float.MIN_NORMAL)), cfg());
    // FIXME: Produces the same output as prop_f32_min_norm
    // write("prop_f32_min_val", feat(p0, prop("val", Float.MIN_VALUE)), cfg());
    // FIXME: Produces the same output as prop_f32_min
    // write("prop_f32_neg_zero", feat(p0, prop("val", (float) -0.0f)), cfg());
    write("prop_f32_zero", feat(p0, prop("val", (float) 0.0f)), cfg());
    write("prop_f32_max_val", feat(p0, prop("val", Float.MAX_VALUE)), cfg());
    write("prop_f32_pos_inf", feat(p0, prop("val", Float.POSITIVE_INFINITY)), cfg());
    write("prop_f32_nan", feat(p0, prop("val", Float.NaN)), cfg());
    // FIXME: ALL f64 are serialized as f32, but should be done properly
    // write("prop_f64", feat(p0, prop("val", (double) 3.141592653589793)), cfg());
    // write("prop_f64_neg_inf", feat(p0, prop("val", Double.NEGATIVE_INFINITY)), cfg());
    // write("prop_f64_min_norm", feat(p0, prop("val", Double.MIN_NORMAL)), cfg());
    // FIXME: Produces the same output as prop_f64_min_norm
    // write("prop_f64_min_val", feat(p0, prop("val", Double.MIN_VALUE)), cfg());
    // write("prop_f64_neg_zero", feat(p0, prop("val", (double) -0.0)), cfg());
    // FIXME: Produces the same output as prop_f64_min
    // write("prop_f64_zero", feat(p0, prop("val", (double) 0.0)), cfg());
    // FIXME: Serializes as f32
    // write("prop_f64_max_val", feat(p0, prop("val", Double.MAX_VALUE)), cfg());
    // FIXME: Fails in Java as it Produces the same output as prop_f64_max
    // write("prop_f64_pos_inf", feat(p0, prop("val", Double.POSITIVE_INFINITY)), cfg());
    // FIXME: Serializes as f32
    // write("prop_f64_nan", feat(p0, prop("val", Double.NaN)), cfg());
    write("prop_str_empty", feat(p0, prop("val", "")), cfg());
    write("prop_str_ascii", feat(p0, prop("val", "42")), cfg());
    write("prop_str_escape", feat(p0, prop("val", "Line1\n\t\"quoted\"\\path")), cfg());
    write("prop_str_unicode", feat(p0, prop("val", "München 📍 cafe\u0301")), cfg());

    // Multiple properties - single feature demonstrating multiple property types
    write(
        "props_mixed",
        feat(
            p1,
            props(
                kv("name", "Test Point"),
                kv("active", true),
                // FIXME: needs support in the Java decoder + encoder
                // kv("tiny-count", (byte) 42),
                // FIXME: needs support in the decoder + encoder
                // kv("tiny", U8.of(100)),
                kv("count", 42),
                kv("medium", U32.of(100)),
                kv("bignum", 42L), // FIXME: this is encoded as i32
                kv("biggest", U64.of(BigInteger.ZERO)),
                kv("temp", 25.5f),
                kv("precision", 0.123456789))),
        cfg());

    // FIXME: needs support in the decoder + encoder
    // var feat_u8s =
    //    array(
    //        feat(p1, prop("val", U8.of(100))),
    //        feat(p2, prop("val", U8.of(100))),
    //        feat(p3, prop("val", U8.of(100))),
    //        feat(p4, prop("val", U8.of(100))));
    // write(layer("props_u8", feat_u8s), cfg());
    // write(layer("props_u8_delta", feat_u8s), cfg(DELTA));
    // write(layer("props_u8_rle", feat_u8s), cfg(RLE));
    // write(layer("props_u8_delta_rle", feat_u8s), cfg(DELTA_RLE));

    var feat_ints =
        array(
            feat(p0, prop("val", 42)),
            feat(p1, prop("val", 42)),
            feat(p2, prop("val", 42)),
            feat(p3, prop("val", 42)));
    write(layer("props_i32", feat_ints), cfg());
    write(layer("props_i32_delta", feat_ints), cfg(DELTA));
    write(layer("props_i32_rle", feat_ints), cfg(RLE));
    write(layer("props_i32_delta_rle", feat_ints), cfg(DELTA_RLE));

    var feat_u32s =
        array(
            feat(p0, prop("val", U32.of(9_000))),
            feat(p1, prop("val", U32.of(9_000))),
            feat(p2, prop("val", U32.of(9_000))),
            feat(p3, prop("val", U32.of(9_000))));
    write(layer("props_u32", feat_u32s), cfg());
    write(layer("props_u32_delta", feat_u32s), cfg(DELTA));
    write(layer("props_u32_rle", feat_u32s), cfg(RLE));
    write(layer("props_u32_delta_rle", feat_u32s), cfg(DELTA_RLE));

    var feat_u64s =
        array(
            feat(p0, prop("val", U64.of(BigInteger.valueOf(9_000L)))),
            feat(p1, prop("val", U64.of(BigInteger.valueOf(9_000L)))),
            feat(p2, prop("val", U64.of(BigInteger.valueOf(9_000L)))),
            feat(p3, prop("val", U64.of(BigInteger.valueOf(9_000L)))));
    write(layer("props_u64", feat_u64s), cfg());
    write(layer("props_u64_delta", feat_u64s), cfg(DELTA));
    write(layer("props_u64_rle", feat_u64s), cfg(RLE));
    write(layer("props_u64_delta_rle", feat_u64s), cfg(DELTA_RLE));

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

  private static void generateSharedDictionaries() throws IOException {
    // 30 because otherwise fsst is skipped
    var val = "A".repeat(30);
    var feat_names = array(feat(p0, props(kv("name:en", val), kv("name:de", val))));

    write(layer("props_no_shared_dict", feat_names), cfg());
    write(layer("props_shared_dict", feat_names), cfg().sharedDictPrefix("name", ":"));
    write(layer("props_shared_dict_fsst", feat_names), cfg().sharedDictPrefix("name", ":").fsst());
  }
}
