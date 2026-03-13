//! Rust synthetic MLT file generator.
//!
//! This generates synthetic MLT files for testing and validation.
//! The goal is to produce byte-for-byte identical output to the Java generator.

mod layer;

use std::fmt::Write as _;
use std::fs;
use std::path::Path;
use std::sync::LazyLock;

use geo_types::{
    Coord, LineString, MultiLineString, MultiPoint, MultiPolygon, Point, Polygon, coord,
    line_string as line, wkt,
};
use mlt_core::geojson::Geom32;
use mlt_core::v01::{
    ParsedProperty as P, IdEncoder, IdWidth, IntEncoder as E, LogicalEncoder as L,
    PresenceStream as O, ScalarEncoder as S, StrEncoder as SE, VertexBufferType,
};

use crate::layer::{Layer, SharedDict, SynthWriter};

const C0: Coord<i32> = coord! { x: 13, y: 42 };
// triangle 1, clockwise winding, X ends in 1, Y ends in 2
const C1: Coord<i32> = coord! { x: 11, y: 52 };
const C2: Coord<i32> = coord! { x: 71, y: 72 };
const C3: Coord<i32> = coord! { x: 61, y: 22 };
// hole in triangle 1 with counter-clockwise winding
const H1: Coord<i32> = coord! { x: 65, y: 66 };
const H2: Coord<i32> = coord! { x: 35, y: 56 };
const H3: Coord<i32> = coord! { x: 55, y: 36 };

const P0: Point<i32> = Point(C0);
const P1: Point<i32> = Point(C1);
const P2: Point<i32> = Point(C2);
const P3: Point<i32> = Point(C3);
// holes as points with same coordinates as the hole vertices
const PH1: Point<i32> = Point(H1);
const PH2: Point<i32> = Point(H2);
const PH3: Point<i32> = Point(H3);

const fn c(x: i32, y: i32) -> Coord<i32> {
    coord! { x: x, y: y }
}

fn p0(w: &SynthWriter) -> Layer {
    w.geo_varint().geo(P0)
}

static MIX_TYPES: LazyLock<[(&'static str, Geom32); 7]> = LazyLock::new(|| {
    [
        ("pt", wkt!(POINT(38 29)).into()),
        ("line", wkt!(LINESTRING(5 38, 12 45, 9 70)).into()),
        ("poly", wkt!(POLYGON((55 5, 58 28, 75 22, 55 5))).into()),
        (
            "polyh",
            wkt!(POLYGON((52 35, 14 55, 60 72, 52 35),(32 50, 36 60, 24 54, 32 50))).into(),
        ),
        ("mpt", wkt!(MULTIPOINT(6 25, 21 41, 23 69)).into()),
        (
            "mline",
            wkt!(MULTILINESTRING((24 10, 42 18),(30 36, 48 52, 35 62))).into(),
        ),
        (
            "mpoly",
            wkt!(MULTIPOLYGON(((7 20, 21 31, 26 9, 7 20),(15 20, 20 15, 18 25, 15 20)),((69 57, 71 66, 73 64, 69 57)))).into(),
        ),
    ]
});

fn main() {
    let dir = Path::new("../test/synthetic/0x01-rust/");
    fs::create_dir_all(dir)
        .unwrap_or_else(|e| panic!("to be able to create {}: {e:?}", dir.display()));

    let dir = dir
        .canonicalize()
        .unwrap_or_else(|e| panic!("bad path {}: {e:?}", dir.display()));
    println!("Generating synthetic test data in {}", dir.display());

    let writer = SynthWriter::new(dir);
    generate_geometry(&writer);
    generate_mixed(&writer);
    generate_extent(&writer);
    generate_ids(&writer);
    generate_properties(&writer);
}

// Geometry builder functions matching Java definitions
fn line1() -> LineString<i32> {
    wkt!(LINESTRING(11 52, 71 72, 61 22))
}
fn line2() -> LineString<i32> {
    wkt!(LINESTRING(23 34, 73 4, 13 24))
}
fn poly1() -> Polygon<i32> {
    wkt!(POLYGON((11 52, 71 72, 61 22, 11 52)))
}
fn poly2() -> Polygon<i32> {
    wkt!(POLYGON((23 34, 73 4, 13 24, 23 34)))
}
fn poly1h() -> Polygon<i32> {
    wkt!(POLYGON((11 52, 71 72, 61 22, 11 52),(65 66, 35 56, 55 36, 65 66)))
}
fn poly_colinear() -> Polygon<i32> {
    wkt!(POLYGON((0 0, 10 0, 20 0, 0 0)))
}
fn poly_self_intersect() -> Polygon<i32> {
    wkt!(POLYGON((0 0, 10 10, 0 10, 10 0, 0 0)))
}
fn poly_hole_touching() -> Polygon<i32> {
    wkt!(POLYGON((0 0, 10 0, 10 10, 0 10, 0 0),(0 0, 2 2, 5 2, 0 0)))
}

/// Morton (Z-order) curve: de-interleave index bits into x/y (even/odd bits).
/// Produces a 4×4 complete Morton block (16 points, scale 8).
fn morton_curve() -> Vec<Coord<i32>> {
    let num_points = 16usize;
    let scale = 8_i32;
    let morton_bits = 4u32;
    let mut curve = Vec::with_capacity(num_points);
    for i in 0..num_points {
        let i = i32::try_from(i).unwrap();
        let mut x = 0_i32;
        let mut y = 0_i32;
        for b in 0..morton_bits {
            x |= ((i >> (2 * b)) & 1) << b;
            y |= ((i >> (2 * b + 1)) & 1) << b;
        }
        curve.push(c(x * scale, y * scale));
    }
    curve
}

fn generate_geometry(w: &SynthWriter) {
    p0(w).write("point");
    w.geo_varint().geo(line1()).write("line");

    let mc = morton_curve();

    w.geo_varint()
        .geo(LineString::new(mc.clone()))
        .vertex_buffer_type(VertexBufferType::Morton)
        .vertex_offsets(E::delta_rle_varint())
        .write("line_morton_curve_morton");
    w.geo_varint()
        .geo(LineString::new(mc.clone()))
        .vertex_buffer_type(VertexBufferType::Vec2)
        .vertex_offsets(E::delta_rle_varint())
        .write("line_morton_curve_no_morton");
    w.geo_varint()
        .geo(LineString::new(vec![c(6_i32, 6), c(6, 6)]))
        .write("line_zero_length");

    w.geo_varint().geo(poly1()).write("poly");
    w.geo_fastpfor().geo(poly1()).write("poly_fpf");
    w.geo_varint().tessellated(poly1()).write("poly_tes");
    w.geo_fastpfor().tessellated(poly1()).write("poly_fpf_tes");

    w.geo_varint().geo(poly_colinear()).write("poly_colinear");
    w.geo_fastpfor()
        .geo(poly_colinear())
        .write("poly_colinear_fpf");
    w.geo_varint()
        .tessellated(poly_colinear())
        .write("poly_colinear_tes");
    w.geo_fastpfor()
        .tessellated(poly_colinear())
        .write("poly_colinear_fpf_tes");

    w.geo_varint()
        .geo(poly_self_intersect())
        .write("poly_self_intersect");
    w.geo_fastpfor()
        .geo(poly_self_intersect())
        .write("poly_self_intersect_fpf");
    w.geo_varint()
        .tessellated(poly_self_intersect())
        .write("poly_self_intersect_tes");
    w.geo_fastpfor()
        .tessellated(poly_self_intersect())
        .write("poly_self_intersect_fpf_tes");

    w.geo_varint()
        .parts_ring(E::rle_varint())
        .geo(poly1h())
        .write("poly_hole");
    w.geo_fastpfor()
        .parts_ring(E::rle_fastpfor())
        .geo(poly1h())
        .write("poly_hole_fpf");
    w.geo_varint()
        .parts_ring(E::rle_varint())
        .tessellated(poly1h())
        .write("poly_hole_tes");
    w.geo_fastpfor()
        .parts_ring(E::rle_fastpfor())
        .tessellated(poly1h())
        .write("poly_hole_fpf_tes");

    w.geo_varint()
        .parts_ring(E::varint())
        .geo(poly_hole_touching())
        .write("poly_hole_touching");
    w.geo_fastpfor()
        .parts_ring(E::fastpfor())
        .geo(poly_hole_touching())
        .write("poly_hole_touching_fpf");

    w.geo_varint()
        .rings(E::rle_varint())
        .rings2(E::rle_varint())
        .geo(MultiPolygon(vec![poly1(), poly2()]))
        .write("poly_multi");
    w.geo_fastpfor()
        .rings(E::rle_fastpfor())
        .rings2(E::rle_fastpfor())
        .geo(MultiPolygon(vec![poly1(), poly2()]))
        .write("poly_multi_fpf");

    // Close the shared Morton curve into a ring to test Morton encoding for polygons.
    let mut morton_ring = mc.clone();
    morton_ring.push(mc[0]);
    let morton_poly = Polygon::new(LineString::new(morton_ring.clone()), vec![]);
    w.geo_varint()
        .geo(morton_poly.clone())
        .write("poly_morton_ring_no_morton");
    w.geo_varint()
        .vertex_buffer_type(VertexBufferType::Morton)
        .vertex_offsets(E::delta_rle_varint())
        .geo(morton_poly)
        .write("poly_morton_ring_morton");

    // Split the Morton curve into two halves and close each into a ring to form a MultiPolygon.
    let half = mc.len() / 2;
    let mut mr1 = mc[..half].to_vec();
    mr1.push(mr1[0]);
    let mut mr2 = mc[half..].to_vec();
    mr2.push(mr2[0]);
    let mp_morton = MultiPolygon(vec![
        Polygon::new(LineString::new(mr1), vec![]),
        Polygon::new(LineString::new(mr2), vec![]),
    ]);
    w.geo_varint()
        .rings(E::rle_varint())
        .rings2(E::rle_varint())
        .geo(mp_morton.clone())
        .write("poly_multi_morton_ring_no_morton");
    w.geo_varint()
        .rings(E::rle_varint())
        .rings2(E::rle_varint())
        .vertex_buffer_type(VertexBufferType::Morton)
        .vertex_offsets(E::delta_rle_varint())
        .geo(mp_morton)
        .write("poly_multi_morton_ring_morton");

    w.geo_varint()
        .geo(MultiPoint(vec![P1, P2, P3]))
        .write("multipoint");
    w.geo_varint()
        .no_rings(E::rle_varint())
        .geo(MultiLineString(vec![line1(), line2()]))
        .write("multiline");

    // Split the Morton curve into two halves to form a MultiLineString with Morton encoding.
    let mline1 = LineString::new(mc[..half].to_vec());
    let mline2 = LineString::new(mc[half..].to_vec());
    w.geo_varint()
        .no_rings(E::rle_varint())
        .vertex_buffer_type(VertexBufferType::Morton)
        .vertex_offsets(E::delta_rle_varint())
        .geo(MultiLineString(vec![mline1, mline2]))
        .write("multiline_morton");
}

fn write_mix(w: &SynthWriter, current: &[usize]) {
    let mut builder = w.geo_varint();
    let mut name = format!("mix_{}", current.len());
    for idx in current {
        let mix_type = &MIX_TYPES[*idx];
        builder = builder.geo(mix_type.1.clone());
        write!(&mut name, "_{}", mix_type.0).unwrap();
    }
    builder.write(&name);
}

fn generate_combinations(w: &SynthWriter, k: usize, start: usize, current: &mut Vec<usize>) {
    if current.len() == k {
        write_mix(w, current);
    } else {
        for i in start..MIX_TYPES.len() {
            current.push(i);
            generate_combinations(w, k, i + 1, current);
            current.pop();
        }
    }
}

fn generate_mixed(w: &SynthWriter) {
    // Generate all combinations of MIX_TYPES with length 2 or more
    for k in 2..=MIX_TYPES.len() {
        generate_combinations(w, k, 0, &mut Vec::new());
    }
    // Generate A-A (duplicate) and A-B-A patterns
    for idx in 0..MIX_TYPES.len() {
        write_mix(w, &[idx, idx]); // A-A variant
        for idx2 in 0..MIX_TYPES.len() {
            if idx != idx2 {
                write_mix(w, &[idx, idx2, idx]); // A-B-A variant
            }
        }
    }
}

fn generate_extent(w: &SynthWriter) {
    for e in [512_i32, 4096, 131_072, 1_073_741_824] {
        w.geo_varint()
            .extent(e.cast_unsigned())
            .geo(line![c(0_i32, 0), c(e - 1, e - 1)])
            .write(format!("extent_{e}"));
        w.geo_varint()
            .extent(e.cast_unsigned())
            .geo(line![c(-42_i32, -42), c(e + 42, e + 42)])
            .write(format!("extent_buf_{e}"));
    }
}

fn generate_ids(w: &SynthWriter) {
    p0(w)
        .ids(vec![Some(100)], IdEncoder::new(L::None, IdWidth::Id32))
        .write("id");
    p0(w)
        .ids(
            vec![Some(u64::from(u32::MIN))],
            IdEncoder::new(L::None, IdWidth::Id32),
        )
        .write("id_min");
    p0(w)
        .ids(
            vec![Some(u64::from(u32::MAX))],
            IdEncoder::new(L::None, IdWidth::Id32),
        )
        .write("id_max-rust");
    p0(w)
        .ids(
            vec![Some(9_234_567_890)],
            IdEncoder::new(L::None, IdWidth::Id64),
        )
        .write("id64");
    p0(w)
        .ids(vec![Some(u64::MAX)], IdEncoder::new(L::None, IdWidth::Id64))
        .write("id64_max-rust");

    let four_p0 = || w.geo_varint().meta(E::rle_varint()).geos([P0, P0, P0, P0]);
    four_p0()
        .ids(
            vec![Some(103), Some(103), Some(103), Some(103)],
            IdEncoder::new(L::None, IdWidth::Id32),
        )
        .write("ids");
    four_p0()
        .ids(
            vec![Some(103), Some(103), Some(103), Some(103)],
            IdEncoder::new(L::Delta, IdWidth::Id32),
        )
        .write("ids_delta");
    four_p0()
        .ids(
            vec![Some(103), Some(103), Some(103), Some(103)],
            IdEncoder::new(L::Rle, IdWidth::Id32),
        )
        .write("ids_rle");
    four_p0()
        .ids(
            vec![Some(103), Some(103), Some(103), Some(103)],
            IdEncoder::new(L::DeltaRle, IdWidth::Id32),
        )
        .write("ids_delta_rle");
    four_p0()
        .ids(
            vec![
                Some(9_234_567_890),
                Some(9_234_567_890),
                Some(9_234_567_890),
                Some(9_234_567_890),
            ],
            IdEncoder::new(L::None, IdWidth::Id64),
        )
        .write("ids64");
    four_p0()
        .ids(
            vec![
                Some(9_234_567_890),
                Some(9_234_567_890),
                Some(9_234_567_890),
                Some(9_234_567_890),
            ],
            IdEncoder::new(L::Delta, IdWidth::Id64),
        )
        .write("ids64_delta");
    four_p0()
        .ids(
            vec![
                Some(9_234_567_890),
                Some(9_234_567_890),
                Some(9_234_567_890),
                Some(9_234_567_890),
            ],
            IdEncoder::new(L::Rle, IdWidth::Id64),
        )
        .write("ids64_rle");
    four_p0()
        .ids(
            vec![
                Some(9_234_567_890),
                Some(9_234_567_890),
                Some(9_234_567_890),
                Some(9_234_567_890),
            ],
            IdEncoder::new(L::DeltaRle, IdWidth::Id64),
        )
        .write("ids64_delta_rle");

    let five_p0 = || {
        w.geo_varint()
            .meta(E::rle_varint())
            .geos([P0, P0, P0, P0, P0])
    };
    five_p0()
        .ids(
            vec![Some(100), Some(101), None, Some(105), Some(106)],
            IdEncoder::new(L::None, IdWidth::OptId32),
        )
        .write("ids_opt");
    five_p0()
        .ids(
            vec![Some(100), Some(101), None, Some(105), Some(106)],
            IdEncoder::new(L::Delta, IdWidth::OptId32),
        )
        .write("ids_opt_delta");
    five_p0()
        .ids(
            vec![None, Some(9_234_567_890), Some(101), Some(105), Some(106)],
            IdEncoder::new(L::None, IdWidth::OptId64),
        )
        .write("ids64_opt");
    five_p0()
        .ids(
            vec![None, Some(9_234_567_890), Some(101), Some(105), Some(106)],
            IdEncoder::new(L::Delta, IdWidth::OptId64),
        )
        .write("ids64_opt_delta");

    let min_max = || {
        vec![
            Some(u64::MIN),
            Some(u64::MAX),
            Some(u64::MIN),
            Some(u64::MAX),
        ]
    };
    four_p0()
        .ids(min_max(), IdEncoder::new(L::None, IdWidth::Id64))
        .write("ids64_minmax-rust");
    four_p0()
        .ids(min_max(), IdEncoder::new(L::Delta, IdWidth::Id64))
        .write("ids64_minmax_delta-rust");
}

fn generate_properties(w: &SynthWriter) {
    // Properties with special names
    p0(w)
        .add_prop(S::bool(O::Present), P::bool("", vec![Some(true)]))
        .write("prop_empty_name");
    p0(w)
        .add_prop(
            S::bool(O::Present),
            P::bool("hello\u{0000} world\n", vec![Some(true)]),
        )
        .write("prop_special_name");

    let enc = S::bool(O::Present);
    p0(w)
        .add_prop(enc, P::bool("val", vec![Some(true)]))
        .write("prop_bool");
    p0(w)
        .add_prop(enc, P::bool("val", vec![Some(false)]))
        .write("prop_bool_false");
    // Two-feature optional bool variants
    w.geo_varint()
        .meta(E::rle_varint())
        .geos([P0, P0])
        .add_prop(enc, P::bool("val", vec![Some(true), None]))
        .write("prop_bool_true_null");
    w.geo_varint()
        .meta(E::rle_varint())
        .geos([P0, P0])
        .add_prop(enc, P::bool("val", vec![None, Some(true)]))
        .write("prop_bool_null_true");
    w.geo_varint()
        .meta(E::rle_varint())
        .geos([P0, P0])
        .add_prop(enc, P::bool("val", vec![Some(false), None]))
        .write("prop_bool_false_null");
    w.geo_varint()
        .meta(E::rle_varint())
        .geos([P0, P0])
        .add_prop(enc, P::bool("val", vec![None, Some(false)]))
        .write("prop_bool_null_false");

    let enc = S::int(O::Present, E::varint());
    p0(w)
        .add_prop(enc, P::i32("val", vec![Some(42)]))
        .write("prop_i32");
    p0(w)
        .add_prop(enc, P::i32("val", vec![Some(-42)]))
        .write("prop_i32_neg");
    p0(w)
        .add_prop(enc, P::i32("val", vec![Some(i32::MIN)]))
        .write("prop_i32_min");
    p0(w)
        .add_prop(enc, P::i32("val", vec![Some(i32::MAX)]))
        .write("prop_i32_max");
    // Two-feature optional i32 variants
    w.geo_varint()
        .meta(E::rle_varint())
        .geos([P0, P0])
        .add_prop(enc, P::i32("val", vec![Some(42), None]))
        .write("prop_i32_val_null");
    w.geo_varint()
        .meta(E::rle_varint())
        .geos([P0, P0])
        .add_prop(enc, P::i32("val", vec![None, Some(42)]))
        .write("prop_i32_null_val");

    p0(w)
        .add_prop(enc, P::u32("val", vec![Some(42)]))
        .write("prop_u32");
    p0(w)
        .add_prop(enc, P::u32("val", vec![Some(0)]))
        .write("prop_u32_min");
    p0(w)
        .add_prop(enc, P::u32("val", vec![Some(u32::MAX)]))
        .write("prop_u32_max");
    // Two-feature optional u32 variants
    w.geo_varint()
        .meta(E::rle_varint())
        .geos([P0, P0])
        .add_prop(enc, P::u32("val", vec![Some(42), None]))
        .write("prop_u32_val_null");
    w.geo_varint()
        .meta(E::rle_varint())
        .geos([P0, P0])
        .add_prop(enc, P::u32("val", vec![None, Some(42)]))
        .write("prop_u32_null_val");

    p0(w)
        .add_prop(enc, P::i64("val", vec![Some(9_876_543_210)]))
        .write("prop_i64");
    p0(w)
        .add_prop(enc, P::i64("val", vec![Some(-9_876_543_210)]))
        .write("prop_i64_neg");
    p0(w)
        .add_prop(enc, P::i64("val", vec![Some(i64::MIN)]))
        .write("prop_i64_min");
    p0(w)
        .add_prop(enc, P::i64("val", vec![Some(i64::MAX)]))
        .write("prop_i64_max");
    // Two-feature optional i64 variants
    w.geo_varint()
        .meta(E::rle_varint())
        .geos([P0, P0])
        .add_prop(enc, P::i64("val", vec![Some(9_876_543_210), None]))
        .write("prop_i64_val_null");
    w.geo_varint()
        .meta(E::rle_varint())
        .geos([P0, P0])
        .add_prop(enc, P::i64("val", vec![None, Some(9_876_543_210)]))
        .write("prop_i64_null_val");

    p0(w)
        .add_prop(enc, P::u64("bignum", vec![Some(1_234_567_890_123_456_789)]))
        .write("prop_u64");
    p0(w)
        .add_prop(enc, P::u64("bignum", vec![Some(0)]))
        .write("prop_u64_min");
    p0(w)
        .add_prop(enc, P::u64("bignum", vec![Some(u64::MAX)]))
        .write("prop_u64_max");
    // Two-feature optional u64 variants (key is "val" to match Java)
    w.geo_varint()
        .meta(E::rle_varint())
        .geos([P0, P0])
        .add_prop(
            enc,
            P::u64("val", vec![Some(1_234_567_890_123_456_789), None]),
        )
        .write("prop_u64_val_null");
    w.geo_varint()
        .meta(E::rle_varint())
        .geos([P0, P0])
        .add_prop(
            enc,
            P::u64("val", vec![None, Some(1_234_567_890_123_456_789)]),
        )
        .write("prop_u64_null_val");

    let enc = S::float(O::Present);
    #[expect(clippy::approx_constant)]
    p0(w)
        .add_prop(enc, P::f32("val", vec![Some(3.14)]))
        .write("prop_f32");
    p0(w)
        .add_prop(enc, P::f32("val", vec![Some(f32::NEG_INFINITY)]))
        .write("prop_f32_neg_inf");
    p0(w)
        .add_prop(enc, P::f32("val", vec![Some(f32::from_bits(1))]))
        .write("prop_f32_min_val");
    p0(w)
        .add_prop(enc, P::f32("val", vec![Some(f32::MIN_POSITIVE)]))
        .write("prop_f32_min_norm");
    p0(w)
        .add_prop(enc, P::f32("val", vec![Some(0.0)]))
        .write("prop_f32_zero");
    p0(w)
        .add_prop(enc, P::f32("val", vec![Some(-0.0)]))
        .write("prop_f32_neg_zero");
    p0(w)
        .add_prop(enc, P::f32("val", vec![Some(f32::MAX)]))
        .write("prop_f32_max");
    p0(w)
        .add_prop(enc, P::f32("val", vec![Some(f32::INFINITY)]))
        .write("prop_f32_pos_inf");
    p0(w)
        .add_prop(enc, P::f32("val", vec![Some(f32::NAN)]))
        .write("prop_f32_nan");
    // Two-feature optional f32 variants
    #[expect(clippy::approx_constant)]
    w.geo_varint()
        .meta(E::rle_varint())
        .geos([P0, P0])
        .add_prop(enc, P::f32("val", vec![Some(3.14), None]))
        .write("prop_f32_val_null");
    #[expect(clippy::approx_constant)]
    w.geo_varint()
        .meta(E::rle_varint())
        .geos([P0, P0])
        .add_prop(enc, P::f32("val", vec![None, Some(3.14)]))
        .write("prop_f32_null_val");

    p0(w)
        .add_prop(enc, P::f64("val", vec![Some(std::f64::consts::PI)]))
        .write("prop_f64");
    p0(w)
        .add_prop(enc, P::f64("val", vec![Some(f64::NAN)]))
        .write("prop_f64_nan");
    p0(w)
        .add_prop(enc, P::f64("val", vec![Some(f64::NEG_INFINITY)]))
        .write("prop_f64_neg_inf");
    p0(w)
        .add_prop(enc, P::f64("val", vec![Some(f64::from_bits(1))]))
        .write("prop_f64_min_val");
    p0(w)
        .add_prop(enc, P::f64("val", vec![Some(f64::MIN_POSITIVE)]))
        .write("prop_f64_min_norm");
    p0(w)
        .add_prop(enc, P::f64("val", vec![Some(-0.0)]))
        .write("prop_f64_neg_zero");
    p0(w)
        .add_prop(enc, P::f64("val", vec![Some(0.0)]))
        .write("prop_f64_zero");
    p0(w)
        .add_prop(enc, P::f64("val", vec![Some(f64::MAX)]))
        .write("prop_f64_max");
    p0(w)
        .add_prop(enc, P::f64("val", vec![Some(f64::INFINITY)]))
        .write("prop_f64_pos_inf");
    // Two-feature optional f64 variants
    w.geo_varint()
        .meta(E::rle_varint())
        .geos([P0, P0])
        .add_prop(enc, P::f64("val", vec![Some(std::f64::consts::PI), None]))
        .write("prop_f64_val_null");
    w.geo_varint()
        .meta(E::rle_varint())
        .geos([P0, P0])
        .add_prop(enc, P::f64("val", vec![None, Some(std::f64::consts::PI)]))
        .write("prop_f64_null_val");

    let enc = S::str(O::Present, E::varint());
    p0(w)
        .add_prop(enc, P::str("val", vec![Some(String::new())]))
        .write("prop_str_empty");
    p0(w)
        .add_prop(enc, P::str("val", vec![Some("42".to_string())]))
        .write("prop_str_ascii");
    p0(w)
        .add_prop(
            enc,
            P::str("val", vec![Some("Line1\n\t\"quoted\"\\path".to_string())]),
        )
        .write("prop_str_escape");
    p0(w)
        .add_prop(
            enc,
            P::str("val", vec![Some("München 📍 cafe\u{0301}".to_string())]),
        )
        .write("prop_str_unicode");
    p0(w)
        .add_prop(
            enc,
            P::str("val", vec![Some("hello\u{0000} world\n".to_string())]),
        )
        .write("prop_str_special");
    // Two-feature optional str variants
    w.geo_varint()
        .meta(E::rle_varint())
        .geos([P0, P0])
        .add_prop(enc, P::str("val", vec![Some("42".to_string()), None]))
        .write("prop_str_val_null");
    w.geo_varint()
        .meta(E::rle_varint())
        .geos([P0, P0])
        .add_prop(enc, P::str("val", vec![None, Some("42".to_string())]))
        .write("prop_str_null_val");
    w.geo_varint()
        .meta(E::rle_varint())
        .geos([P0, P0])
        .add_prop(enc, P::str("val", vec![Some(String::new()), None]))
        .write("prop_str_val_empty");
    w.geo_varint()
        .meta(E::rle_varint())
        .geos([P0, P0])
        .add_prop(enc, P::str("val", vec![None, Some(String::new())]))
        .write("prop_str_empty_val");

    p0(w)
        .add_prop(S::bool(O::Present), P::bool("active", vec![Some(true)]))
        .add_prop(
            S::int(O::Present, E::varint()),
            P::u64("biggest", vec![Some(0)]),
        ) // FIXME: this should be u64, but java does it it this way
        .add_prop(
            S::int(O::Present, E::varint()),
            P::i32("bignum", vec![Some(42)]),
        )
        .add_prop(
            S::int(O::Present, E::varint()),
            P::i32("count", vec![Some(42)]),
        )
        .add_prop(
            S::int(O::Present, E::varint()),
            P::u32("medium", vec![Some(100)]),
        )
        .add_prop(
            S::str(O::Present, E::varint()),
            P::str("name", vec![Some("Test Point".to_string())]),
        )
        .add_prop(
            S::float(O::Present),
            P::f64("precision", vec![Some(0.123_456_789)]),
        )
        .add_prop(S::float(O::Present), P::f32("temp", vec![Some(25.5)]))
        //FIXME in java
        //.add_prop(enc, "tiny-count", PropValue::I8(vec![Some(42)]))
        //.add_prop(enc, "tiny-count", PropValue::U8(vec![Some(100)]))
        .write("props_mixed");

    generate_props_i32(w);
    generate_props_u32(w);
    generate_props_u64(w);
    generate_props_str(w);
    generate_shared_dictionaries(w);
}

fn generate_props_i32(w: &SynthWriter) {
    let four_points = || w.geo_varint().meta(E::rle_varint()).geos([P0, P1, P2, P3]);
    let values = || P::i32("val", vec![Some(42), Some(42), Some(42), Some(42)]);

    four_points()
        .add_prop(S::int(O::Present, E::varint()), values())
        .write("props_i32");
    four_points()
        .add_prop(S::int(O::Present, E::delta_varint()), values())
        .write("props_i32_delta");
    four_points()
        .add_prop(S::int(O::Present, E::rle_varint()), values())
        .write("props_i32_rle");
    four_points()
        .add_prop(S::int(O::Present, E::delta_rle_varint()), values())
        .write("props_i32_delta_rle");
}

fn generate_props_u32(w: &SynthWriter) {
    let four_points = || w.geo_varint().meta(E::rle_varint()).geos([P0, P1, P2, P3]);
    let values = || {
        P::u32(
            "val",
            vec![Some(9_000), Some(9_000), Some(9_000), Some(9_000)],
        )
    };

    four_points()
        .add_prop(S::int(O::Present, E::varint()), values())
        .write("props_u32");
    four_points()
        .add_prop(S::int(O::Present, E::delta_varint()), values())
        .write("props_u32_delta");
    four_points()
        .add_prop(S::int(O::Present, E::rle_varint()), values())
        .write("props_u32_rle");
    four_points()
        .add_prop(S::int(O::Present, E::delta_rle_varint()), values())
        .write("props_u32_delta_rle");
}

fn generate_props_u64(w: &SynthWriter) {
    let four_points = || w.geo_varint().meta(E::rle_varint()).geos([P0, P1, P2, P3]);
    let property = || {
        P::u64(
            "val",
            vec![Some(9_000), Some(9_000), Some(9_000), Some(9_000)],
        )
    };

    four_points()
        .add_prop(S::int(O::Present, E::varint()), property())
        .write("props_u64");
    four_points()
        .add_prop(S::int(O::Present, E::delta_varint()), property())
        .write("props_u64_delta");
    four_points()
        .add_prop(S::int(O::Present, E::rle_varint()), property())
        .write("props_u64_rle");
    four_points()
        .add_prop(S::int(O::Present, E::delta_rle_varint()), property())
        .write("props_u64_delta_rle");
}

fn generate_props_str(w: &SynthWriter) {
    let six_points = || {
        w.geo_varint()
            .meta(E::rle_varint())
            .geos([P1, P2, P3, PH1, PH2, PH3])
    };
    let values = || {
        P::str(
            "val",
            vec![
                Some("residential_zone_north_sector_1".to_string()),
                Some("commercial_zone_south_sector_2".to_string()),
                Some("industrial_zone_east_sector_3".to_string()),
                Some("park_zone_west_sector_4".to_string()),
                Some("water_zone_north_sector_5".to_string()),
                Some("residential_zone_south_sector_6".to_string()),
            ],
        )
    };

    six_points()
        .add_prop(S::str(O::Present, E::varint()), values())
        .write("props_str");
    six_points()
        .add_prop(S::str_fsst(O::Present, E::varint(), E::varint()), values())
        .write("props_str_fsst-rust"); // FSST compression output is not byte-for-byte consistent with Java's
}

fn generate_shared_dictionaries(w: &SynthWriter) {
    let long_string_value = || "A".repeat(30);
    let val = long_string_value();
    p0(w)
        .add_prop(
            S::str(O::Present, E::varint()),
            P::str("name:de", vec![Some(long_string_value())]),
        )
        .add_prop(
            S::str(O::Present, E::varint()),
            P::str("name:en", vec![Some(long_string_value())]),
        )
        .write("props_no_shared_dict");

    p0(w)
        .add_shared_dict(
            SharedDict::new("name:", SE::plain(E::varint()))
                .column("de", O::Present, E::varint(), [Some(long_string_value())])
                .column("en", O::Present, E::varint(), [Some(long_string_value())]),
        )
        .write("props_shared_dict-rust"); // For some reason Java hallucinates another stream count at the start, so starts counting the stream count at 1

    p0(w)
        .add_shared_dict(
            SharedDict::new("name:", SE::fsst(E::varint(), E::varint()))
                .column("de", O::Present, E::varint(), [Some(long_string_value())])
                .column("en", O::Present, E::varint(), [Some(long_string_value())]),
        )
        .write("props_shared_dict_fsst-rust"); // Rust FSST is not byte-for-byte consistent with Java's
    p0(w)
        // column names MUST be unique, but the shared dict prefix can duplicate
        .add_shared_dict(
            SharedDict::new("name", SE::plain(E::varint()))
                .column(":de", O::Present, E::varint(), [Some(long_string_value())])
                .column(":en", O::Present, E::varint(), [Some(long_string_value())]),
        )
        .add_shared_dict(
            SharedDict::new("name", SE::plain(E::varint()))
                .column(":fr", O::Present, E::varint(), [Some(long_string_value())])
                .column(":he", O::Present, E::varint(), [Some(long_string_value())]),
        )
        .write("props_shared_dict_2_same_prefix-rust");

    // Empty struct name: keys "a" and "b" both become children of the "" struct.
    // FIXME: dump equal, but not binary equal
    // p0(w)
    //     .add_shared_dict(
    //         SharedDict::new("", SE::plain(E::varint()))
    //             .column("a", O::Present, E::varint(), [Some(val.clone())])
    //             .column("b", O::Present, E::varint(), [Some(val.clone())]),
    //     )
    //     .write("props_shared_dict_no_struct_name");
    p0(w)
        .add_shared_dict(
            SharedDict::new("", SE::fsst(E::varint(), E::varint()))
                .column("a", O::Present, E::varint(), [Some(val.clone())])
                .column("b", O::Present, E::varint(), [Some(val.clone())]),
        )
        .write("props_shared_dict_no_struct_name_fsst-rust"); // Rust FSST is not byte-for-byte consistent with Java's
}
