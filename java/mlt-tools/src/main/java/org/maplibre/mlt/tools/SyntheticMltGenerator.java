package org.maplibre.mlt.tools;

import static org.maplibre.mlt.converter.ConversionConfig.IntegerEncodingOption.DELTA;
import static org.maplibre.mlt.converter.ConversionConfig.IntegerEncodingOption.DELTA_RLE;
import static org.maplibre.mlt.converter.ConversionConfig.IntegerEncodingOption.PLAIN;
import static org.maplibre.mlt.converter.ConversionConfig.IntegerEncodingOption.RLE;
import static org.maplibre.mlt.tools.SyntheticMltUtil.SYNTHETICS_DIR;
import static org.maplibre.mlt.tools.SyntheticMltUtil.array;
import static org.maplibre.mlt.tools.SyntheticMltUtil.c;
import static org.maplibre.mlt.tools.SyntheticMltUtil.c1;
import static org.maplibre.mlt.tools.SyntheticMltUtil.c2;
import static org.maplibre.mlt.tools.SyntheticMltUtil.c3;
import static org.maplibre.mlt.tools.SyntheticMltUtil.cfg;
import static org.maplibre.mlt.tools.SyntheticMltUtil.feat;
import static org.maplibre.mlt.tools.SyntheticMltUtil.gf;
import static org.maplibre.mlt.tools.SyntheticMltUtil.idFeat;
import static org.maplibre.mlt.tools.SyntheticMltUtil.kv;
import static org.maplibre.mlt.tools.SyntheticMltUtil.layer;
import static org.maplibre.mlt.tools.SyntheticMltUtil.line;
import static org.maplibre.mlt.tools.SyntheticMltUtil.line1;
import static org.maplibre.mlt.tools.SyntheticMltUtil.line2;
import static org.maplibre.mlt.tools.SyntheticMltUtil.mortonCurve;
import static org.maplibre.mlt.tools.SyntheticMltUtil.multi;
import static org.maplibre.mlt.tools.SyntheticMltUtil.p0;
import static org.maplibre.mlt.tools.SyntheticMltUtil.p1;
import static org.maplibre.mlt.tools.SyntheticMltUtil.p2;
import static org.maplibre.mlt.tools.SyntheticMltUtil.p3;
import static org.maplibre.mlt.tools.SyntheticMltUtil.ph1;
import static org.maplibre.mlt.tools.SyntheticMltUtil.ph2;
import static org.maplibre.mlt.tools.SyntheticMltUtil.ph3;
import static org.maplibre.mlt.tools.SyntheticMltUtil.poly;
import static org.maplibre.mlt.tools.SyntheticMltUtil.poly1;
import static org.maplibre.mlt.tools.SyntheticMltUtil.poly1h;
import static org.maplibre.mlt.tools.SyntheticMltUtil.poly2;
import static org.maplibre.mlt.tools.SyntheticMltUtil.prop;
import static org.maplibre.mlt.tools.SyntheticMltUtil.props;
import static org.maplibre.mlt.tools.SyntheticMltUtil.ring;
import static org.maplibre.mlt.tools.SyntheticMltUtil.write;

import java.io.IOException;
import java.math.BigInteger;
import java.nio.file.Files;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.List;
import java.util.stream.Collectors;
import org.locationtech.jts.geom.Geometry;
import org.locationtech.jts.geom.MultiPolygon;
import org.locationtech.jts.geom.Polygon;
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

    write("line_morton_curve_no_morton", feat(line(mortonCurve)), cfg());
    write("line_morton_curve_morton", feat(line(mortonCurve)), cfg().morton());

    write("line_zero_length", feat(line(c(6, 6), c(6, 6))), cfg());
  }

  private static void generatePolygons() throws IOException {
    var pol = feat(poly1);
    write("poly", pol, cfg());
    write("poly_fpf", pol, cfg().fastPFOR());
    write("poly_tes", pol, cfg().tessellate());
    write("poly_fpf_tes", pol, cfg().fastPFOR().tessellate());

    var polCollinear = feat(poly(c(0, 0), c(10, 0), c(20, 0), c(0, 0)));
    write("poly_collinear", polCollinear, cfg());
    write("poly_collinear_fpf", polCollinear, cfg().fastPFOR());
    write("poly_collinear_tes", polCollinear, cfg().tessellate());
    write("poly_collinear_fpf_tes", polCollinear, cfg().fastPFOR().tessellate());

    var polSelfIntersect = feat(poly(c(0, 0), c(10, 10), c(0, 10), c(10, 0), c(0, 0)));
    write("poly_self_intersect", polSelfIntersect, cfg());
    write("poly_self_intersect_fpf", polSelfIntersect, cfg().fastPFOR());
    write("poly_self_intersect_tes", polSelfIntersect, cfg().tessellate());
    write("poly_self_intersect_fpf_tes", polSelfIntersect, cfg().fastPFOR().tessellate());

    // Polygon with hole
    var polWithHole = feat(poly1h);
    write("poly_hole", polWithHole, cfg());
    write("poly_hole_fpf", polWithHole, cfg().fastPFOR());
    write("poly_hole_tes", polWithHole, cfg().tessellate());
    write("poly_hole_fpf_tes", polWithHole, cfg().fastPFOR().tessellate());

    var polyHoleTouching =
        feat(
            poly(
                ring(c(0, 0), c(10, 0), c(10, 10), c(0, 10), c(0, 0)),
                ring(c(0, 0), c(2, 2), c(5, 2), c(0, 0))));
    write("poly_hole_touching", polyHoleTouching, cfg());
    write("poly_hole_touching_fpf", polyHoleTouching, cfg().fastPFOR());
    write("poly_hole_touching_tes", polyHoleTouching, cfg().tessellate());
    write("poly_hole_touching_fpf_tes", polyHoleTouching, cfg().fastPFOR().tessellate());

    // MultiPolygon
    var multiPol = feat(multi(poly1, poly2));
    write("poly_multi", multiPol, cfg());
    write("poly_multi_fpf", multiPol, cfg().fastPFOR());
    write("poly_multi_tes", multiPol, cfg().tessellate());
    write("poly_multi_fpf_tes", multiPol, cfg().fastPFOR().tessellate());

    // Close the shared Morton curve into a ring to test Morton encoding for polygons.
    var mortonRing = Arrays.copyOf(mortonCurve, mortonCurve.length + 1);
    mortonRing[mortonCurve.length] = mortonRing[0];
    write("poly_morton_ring_no_morton", feat(poly(mortonRing)), cfg());
    write("poly_morton_ring_morton", feat(poly(mortonRing)), cfg().morton());

    // Split the Morton curve into two halves (bottom 2 rows / top 2 rows of the 4x4 grid)
    // and close each into a ring to form a MultiPolygon.
    var half = mortonCurve.length / 2;
    var mortonRing1 = Arrays.copyOf(mortonCurve, half + 1);
    mortonRing1[half] = mortonRing1[0];
    var mortonRing2src = Arrays.copyOfRange(mortonCurve, half, mortonCurve.length);
    var mortonRing2 = Arrays.copyOf(mortonRing2src, mortonRing2src.length + 1);
    mortonRing2[mortonRing2src.length] = mortonRing2[0];
    write(
        "poly_multi_morton_ring_no_morton",
        feat(multi(poly(mortonRing1), poly(mortonRing2))),
        cfg());
    write(
        "poly_multi_morton_ring_morton",
        feat(multi(poly(mortonRing1), poly(mortonRing2))),
        cfg().morton());

    // Split the Morton curve at a different place so that the rings are different lengths,
    // use one as the shell and one as the hole of a single and multi-polygon.
    final var quarter = mortonCurve.length / 4;
    mortonRing1 = Arrays.copyOf(mortonCurve, quarter + 1);
    mortonRing1[quarter] = mortonRing1[0];
    mortonRing2src = Arrays.copyOfRange(mortonCurve, quarter, mortonCurve.length);
    mortonRing2 = Arrays.copyOf(mortonRing2src, mortonRing2src.length + 1);
    mortonRing2[mortonRing2src.length] = mortonRing2[0];
    write(
        "poly_morton_hole_morton",
        feat(poly(ring(mortonRing1), ring(mortonRing2))),
        cfg().morton());
    write(
        "poly_multi_morton_hole_morton",
        feat(multi(poly(ring(mortonRing1), ring(mortonRing2)))),
        cfg().morton());
  }

  private static void generateMultiPoints() throws IOException {
    write("multipoint", feat(multi(c1, c2, c3)), cfg());
  }

  private static void generateMultiLineStrings() throws IOException {
    write("multiline", feat(multi(line1, line2)), cfg());

    var half = mortonCurve.length / 2;
    var mortonLine1 = Arrays.copyOf(mortonCurve, half);
    var mortonLine2 = Arrays.copyOfRange(mortonCurve, half, mortonCurve.length);
    write("multiline_morton", feat(multi(line(mortonLine1), line(mortonLine2))), cfg().morton());
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

    // if all geometries are of polygon type, add tessellated variant
    if (current.stream()
        .allMatch(
            t -> {
              Geometry geo = t.feat.getGeometry();
              return geo instanceof Polygon || geo instanceof MultiPolygon;
            })) {
      write(layer(name + "_tes", feats), cfg().geomEnc(PLAIN).tessellate());
    }
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
    write("id", idFeat(100), cfg().ids());
    write("id_min", idFeat(0), cfg().ids());
    // FIXME: serialises as -1
    // write("id_max", idFeat(0xFFFFFFFF), cfg().ids());
    write("id64", idFeat(9_234_567_890L), cfg().ids());
    // FIXME: writes as Id32 instead of Id64
    // write("id64_max", idFeat(0xFFFFFFFFFFFFFFFFL), cfg().ids());

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

    // Java does not generate this as an ID64 if none of them are above the U32::Max threshold
    // FIXME: serialises as i64
    // var ids64MinMax = array(idFeat(0L), idFeat(0xFFFFFFFFFFFFFFFFL), idFeat(0L),
    // idFeat(0xFFFFFFFFFFFFFFFFL));
    // write(layer("ids64_minmax", ids64MinMax), cfg().ids());
    // write(layer("ids64_minmax_delta", ids64MinMax), cfg(DELTA).ids());
  }

  @SuppressWarnings("cast")
  private static void generateProperties() throws IOException {
    write("prop_empty_name", feat(p0, prop("", true)), cfg());
    write("prop_special_name", feat(p0, prop("hello\u0000 world\n", true)), cfg());

    write("prop_bool", feat(p0, prop("val", true)), cfg());
    write("prop_bool_false", feat(p0, prop("val", false)), cfg());
    write(layer("prop_bool_true_null", feat(p0, prop("val", true)), feat(p0)), cfg());
    write(layer("prop_bool_null_true", feat(p0), feat(p0, prop("val", true))), cfg());
    write(layer("prop_bool_false_null", feat(p0, prop("val", false)), feat(p0)), cfg());
    write(layer("prop_bool_null_false", feat(p0), feat(p0, prop("val", false))), cfg());

    // FIXME: needs support in the Java decoder + encoder
    // write("prop_i8", feat(p0, prop("val", (byte) 42)), cfg());
    // write("prop_i8_neg", feat(p0, prop("val", (byte) -42)), cfg());
    // write("prop_i8_min", feat(p0, prop("val", Byte.MIN_VALUE)), cfg());
    // write("prop_i8_max", feat(p0, prop("val", Byte.MAX_VALUE)), cfg());
    // write(layer("prop_i8_val_null", feat(p0, prop("val", (byte) 42)), feat(p0)), cfg());
    // write(layer("prop_i8_null_val", feat(p0), feat(p0, prop("val", (byte) 42))), cfg());

    // write("prop_u8", feat(p0, prop("tinynum", U8.of(100))), cfg());
    // write("prop_u8_min", feat(p0, prop("tinynum", U8.of(0))), cfg());
    // write("prop_u8_max", feat(p0, prop("tinynum", U8.of(255))), cfg());
    // write(layer("prop_u8_val_null", feat(p0, prop("val", U8.of(100))), feat(p0)), cfg());
    // write(layer("prop_u8_null_val", feat(p0), feat(p0, prop("val", U8.of(100)))), cfg());

    // write("prop_i16", feat(p0, prop("val", (short) 42)), cfg());
    // write("prop_i16_neg", feat(p0, prop("val", (short) -42)), cfg());
    // write("prop_i16_min", feat(p0, prop("val", Short.MIN_VALUE)), cfg());
    // write("prop_i16_max", feat(p0, prop("val", Short.MAX_VALUE)), cfg());
    // write(layer("prop_i16_val_null", feat(p0, prop("val", (short) 42)), feat(p0)), cfg());
    // write(layer("prop_i16_null_val", feat(p0), feat(p0, prop("val", (short) 42))), cfg());

    write("prop_i32", feat(p0, prop("val", (int) 42)), cfg());
    write("prop_i32_neg", feat(p0, prop("val", (int) -42)), cfg());
    write("prop_i32_min", feat(p0, prop("val", Integer.MIN_VALUE)), cfg());
    write("prop_i32_max", feat(p0, prop("val", Integer.MAX_VALUE)), cfg());
    write(layer("prop_i32_val_null", feat(p0, prop("val", (int) 42)), feat(p0)), cfg());
    write(layer("prop_i32_null_val", feat(p0), feat(p0, prop("val", (int) 42))), cfg());

    write("prop_u32", feat(p0, prop("val", U32.of(42L))), cfg());
    write("prop_u32_min", feat(p0, prop("val", U32.of(0L))), cfg());
    write("prop_u32_max", feat(p0, prop("val", U32.of(0xFFFFFFFFL))), cfg());
    write(layer("prop_u32_val_null", feat(p0, prop("val", U32.of(42L))), feat(p0)), cfg());
    write(layer("prop_u32_null_val", feat(p0), feat(p0, prop("val", U32.of(42L)))), cfg());

    long i64_value = 9_876_543_210L;
    write("prop_i64", feat(p0, prop("val", i64_value)), cfg());
    write("prop_i64_neg", feat(p0, prop("val", (long) -9_876_543_210L)), cfg());
    write("prop_i64_min", feat(p0, prop("val", Long.MIN_VALUE)), cfg());
    write("prop_i64_max", feat(p0, prop("val", Long.MAX_VALUE)), cfg());
    write(layer("prop_i64_val_null", feat(p0, prop("val", i64_value)), feat(p0)), cfg());
    write(layer("prop_i64_null_val", feat(p0), feat(p0, prop("val", i64_value))), cfg());

    U64 u64_value = U64.of(BigInteger.valueOf(1234567890123456789L));
    U64 u64_max = U64.of(new BigInteger("18446744073709551615"));
    write("prop_u64", feat(p0, prop("bignum", u64_value)), cfg());
    write("prop_u64_min", feat(p0, prop("bignum", U64.of(BigInteger.ZERO))), cfg());
    write("prop_u64_max", feat(p0, prop("bignum", u64_max)), cfg());
    write(layer("prop_u64_val_null", feat(p0, prop("val", u64_value)), feat(p0)), cfg());
    write(layer("prop_u64_null_val", feat(p0), feat(p0, prop("val", u64_value))), cfg());

    write("prop_f32", feat(p0, prop("val", (float) 3.14f)), cfg());
    write("prop_f32_neg_inf", feat(p0, prop("val", Float.NEGATIVE_INFINITY)), cfg());
    write("prop_f32_min_norm", feat(p0, prop("val", Float.MIN_NORMAL)), cfg());
    write("prop_f32_min_val", feat(p0, prop("val", Float.MIN_VALUE)), cfg());
    write("prop_f32_neg_zero", feat(p0, prop("val", (float) -0.0f)), cfg());
    write("prop_f32_zero", feat(p0, prop("val", (float) 0.0f)), cfg());
    write("prop_f32_max", feat(p0, prop("val", Float.MAX_VALUE)), cfg());
    write("prop_f32_pos_inf", feat(p0, prop("val", Float.POSITIVE_INFINITY)), cfg());
    write("prop_f32_nan", feat(p0, prop("val", Float.NaN)), cfg());
    write(layer("prop_f32_val_null", feat(p0, prop("val", 3.14f)), feat(p0)), cfg());
    write(layer("prop_f32_null_val", feat(p0), feat(p0, prop("val", 3.14f))), cfg());

    write("prop_f64", feat(p0, prop("val", (Double) Math.PI)), cfg());
    write("prop_f64_neg_inf", feat(p0, prop("val", Double.NEGATIVE_INFINITY)), cfg());
    write("prop_f64_min_norm", feat(p0, prop("val", Double.MIN_NORMAL)), cfg());
    write("prop_f64_min_val", feat(p0, prop("val", Double.MIN_VALUE)), cfg());
    write("prop_f64_neg_zero", feat(p0, prop("val", (double) -0.0)), cfg());
    write("prop_f64_zero", feat(p0, prop("val", (double) 0.0)), cfg());
    write("prop_f64_max", feat(p0, prop("val", Double.MAX_VALUE)), cfg());
    write("prop_f64_pos_inf", feat(p0, prop("val", Double.POSITIVE_INFINITY)), cfg());
    write("prop_f64_nan", feat(p0, prop("val", Double.NaN)), cfg());
    write(layer("prop_f64_val_null", feat(p0, prop("val", (Double) Math.PI)), feat(p0)), cfg());
    write(layer("prop_f64_null_val", feat(p0), feat(p0, prop("val", (Double) Math.PI))), cfg());

    write("prop_str_empty", feat(p0, prop("val", "")), cfg());
    write("prop_str_ascii", feat(p0, prop("val", "42")), cfg());
    write("prop_str_escape", feat(p0, prop("val", "Line1\n\t\"quoted\"\\path")), cfg());
    write("prop_str_unicode", feat(p0, prop("val", "München 📍 cafe\u0301")), cfg());
    write("prop_str_special", feat(p0, prop("val", "hello\u0000 world\n")), cfg());
    write(layer("prop_str_val_null", feat(p0, prop("val", "42")), feat(p0)), cfg());
    write(layer("prop_str_null_val", feat(p0), feat(p0, prop("val", "42"))), cfg());
    write(layer("prop_str_val_empty", feat(p0, prop("val", "")), feat(p0)), cfg());
    write(layer("prop_str_empty_val", feat(p0), feat(p0, prop("val", ""))), cfg());

    // Multiple properties - single feature demonstrating multiple property types
    write(
        "props_mixed",
        feat(
            p0,
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

    // The actual frame size is 256, so cover multiples of its half
    for (var multiplier : new int[] {1, 2, 3, 4}) {
      for (var offset : new int[] {-1, 0, 1}) {
        var features = new Feature[128 * multiplier + offset];
        for (var i = 0; i < features.length; i++) {
          // Sequence 0,1,2, 0,1,2, 0,1,2, 0,1,2, ...
          features[i] = feat(p0, prop("val", U32.of(i % 3)));
        }
        write(layer("props_u32_fpf_" + features.length, features), cfg().fastPFOR());
      }
    }

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

    // 30 because otherwise fsst is skipped
    var val = "A".repeat(30);

    // If there are many identical strings in the same column,
    // an offset directory is used to share them
    // Because the directory we are indexing into has lengths assosciated with it and the offsets
    // are indexes,
    // we cannot do overlap-optimization where one ABBA contains BB -> only ABBA would be in the
    // dict.
    // -> ABBABB would need to be in the dict
    var feat_two_str_eq = array(feat(p1, prop("val", val)), feat(p2, prop("val", val)));
    write(layer("props_offset_str", feat_two_str_eq), cfg());
    write(layer("props_offset_str_fsst", feat_two_str_eq), cfg().fsst());
  }

  private static void generateSharedDictionaries() throws IOException {
    // 30 because otherwise fsst is skipped
    var val = "A".repeat(30);
    var feat_names = array(feat(p0, props(kv("name:en", val), kv("name:de", val))));

    write(layer("props_no_shared_dict", feat_names), cfg());
    write(layer("props_shared_dict", feat_names), cfg().sharedDictPrefix("name", ":"));
    write(layer("props_shared_dict_fsst", feat_names), cfg().sharedDictPrefix("name", ":").fsst());

    var feat_one_child = array(feat(p0, props(kv("name:en", val), kv("place", val))));
    write(
        layer("props_shared_dict_one_child", feat_one_child), cfg().sharedDictPrefix("name", ":"));
    write(
        layer("props_shared_dict_one_child_fsst", feat_one_child),
        cfg().sharedDictPrefix("name", ":").fsst());

    var feat_distinct_keys_same_value = array(feat(p0, props(kv("a", val), kv("b", val))));
    write(
        layer("props_shared_dict_no_struct_name", feat_distinct_keys_same_value),
        cfg().sharedDictPrefix("", ""));
    write(
        layer("props_shared_dict_no_struct_name_fsst", feat_distinct_keys_same_value),
        cfg().sharedDictPrefix("", "").fsst());

    var feat_names_eq_val_eq = array(feat(p0, props(kv("a", val), kv("a", val))));
    write(
        layer("props_shared_dict_no_child_name", feat_names_eq_val_eq),
        cfg().sharedDictPrefix("a", ""));
    write(
        layer("props_shared_dict_no_child_name_fsst", feat_names_eq_val_eq),
        cfg().sharedDictPrefix("a", "").fsst());

    // Two separate shared dicts with the same "name" prefix, split by explicit column groups
    var feat_names_4 =
        array(
            feat(
                p0,
                props(
                    kv("name:de", val),
                    kv("name_en", val),
                    kv("name_fr", val),
                    kv("name:he", val))));
    write(
        layer("props_shared_dict_2_same_prefix", feat_names_4),
        cfg()
            .sharedDictColumnGroups(
                List.of(List.of("name:de", "name_en"), List.of("name_fr", "name:he"))));
  }
}
