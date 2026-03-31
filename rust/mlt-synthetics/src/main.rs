//! Rust synthetic MLT file generator.
//!
//! Verifies non-rust synthetics in-memory against the reference `0x01/` dir.
//! Writes `-rust`-suffixed files to `0x01-rust/` and compares their decoded JSON
//! to the corresponding non-rust reference JSON.

mod layer;
mod writer;

use std::fmt::Write as _;
use std::path::PathBuf;
use std::sync::LazyLock;

use clap::Parser;
use geo_types::{
    Coord, LineString, MultiLineString, MultiPoint, MultiPolygon, Point, Polygon, coord,
    line_string as line, wkt,
};
use mlt_core::geojson::Geom32;
use mlt_core::v01::{
    IdEncoder, IdWidth, IntEncoder as E, LogicalEncoder as L, ScalarEncoder as S,
    StagedProperty as P, StrEncoder as SE, VertexBufferType,
};

use crate::layer::{
    Layer, SharedDict, geo_fastpfor, geo_varint, geo_varint_with_rle, morton_curve,
};
use crate::writer::SynthWriter;

#[derive(Parser)]
#[command(about = "Verify Rust-generated synthetic MLTs against the Java reference")]
struct Args {
    /// Print each verified or written file
    #[arg(long)]
    verbose: bool,

    /// Directory with the reference synthetic MLT files to verify against (must exist)
    #[arg(long, default_value = "../test/synthetic/0x01/")]
    synthetics: PathBuf,

    /// Directory to use for Rust-specific synthetic MLT files (will be created if it doesn't exist)
    #[arg(long, default_value = "../test/synthetic/0x01-rust/")]
    synthetics_rust: PathBuf,
}

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

fn p0() -> Layer {
    geo_varint().geo(P0)
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
    let mut writer = SynthWriter::new(Args::parse());

    generate_geometry(&mut writer);
    generate_mixed(&mut writer);
    generate_extent(&mut writer);
    generate_ids(&mut writer);
    generate_properties(&mut writer);

    writer.report_ungenerated();

    if writer.failures > 0 {
        eprintln!("{} synthetics failed", writer.failures);
        std::process::exit(1);
    }
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
fn poly_collinear() -> Polygon<i32> {
    wkt!(POLYGON((0 0, 10 0, 20 0, 0 0)))
}
fn poly_self_intersect() -> Polygon<i32> {
    wkt!(POLYGON((0 0, 10 10, 0 10, 10 0, 0 0)))
}
fn poly_hole_touching() -> Polygon<i32> {
    wkt!(POLYGON((0 0, 10 0, 10 10, 0 10, 0 0),(0 0, 2 2, 5 2, 0 0)))
}

fn generate_geometry(w: &mut SynthWriter) {
    p0().write(w, "point");
    geo_varint().geo(line1()).write(w, "line");

    geo_varint()
        .geo(LineString::new(morton_curve()))
        .vertex_buffer_type(VertexBufferType::Morton)
        .vertex_offsets(E::delta_rle_varint())
        .write(w, "line_morton_curve_morton");
    geo_varint()
        .geo(LineString::new(morton_curve()))
        .vertex_buffer_type(VertexBufferType::Vec2)
        .vertex_offsets(E::delta_rle_varint())
        .write(w, "line_morton_curve_no_morton");
    geo_varint()
        .geo(LineString::new(vec![c(6_i32, 6), c(6, 6)]))
        .write(w, "line_zero_length");

    geo_varint().geo(poly1()).write(w, "poly");
    geo_fastpfor().geo(poly1()).write(w, "poly_fpf");
    geo_varint().tessellate().geo(poly1()).write(w, "poly_tes");
    geo_fastpfor()
        .tessellate()
        .geo(poly1())
        .write(w, "poly_fpf_tes");

    geo_varint()
        .geo(poly_collinear())
        .write(w, "poly_collinear");
    geo_fastpfor()
        .geo(poly_collinear())
        .write(w, "poly_collinear_fpf");
    geo_varint()
        .tessellate()
        .geo(poly_collinear())
        .write(w, "poly_collinear_tes");
    geo_fastpfor()
        .tessellate()
        .geo(poly_collinear())
        .write(w, "poly_collinear_fpf_tes");

    geo_varint()
        .geo(poly_self_intersect())
        .write(w, "poly_self_intersect");
    geo_fastpfor()
        .geo(poly_self_intersect())
        .write(w, "poly_self_intersect_fpf");
    geo_varint()
        .tessellate()
        .geo(poly_self_intersect())
        .write(w, "poly_self_intersect_tes");
    geo_fastpfor()
        .tessellate()
        .geo(poly_self_intersect())
        .write(w, "poly_self_intersect_fpf_tes");

    geo_varint()
        .parts_ring(E::rle_varint())
        .geo(poly1h())
        .write(w, "poly_hole");
    geo_fastpfor()
        .parts_ring(E::rle_fastpfor())
        .geo(poly1h())
        .write(w, "poly_hole_fpf");
    geo_varint()
        .parts_ring(E::rle_varint())
        .tessellate()
        .geo(poly1h())
        .write(w, "poly_hole_tes");
    geo_fastpfor()
        .parts_ring(E::rle_fastpfor())
        .tessellate()
        .geo(poly1h())
        .write(w, "poly_hole_fpf_tes");

    geo_varint()
        .parts_ring(E::varint())
        .geo(poly_hole_touching())
        .write(w, "poly_hole_touching");
    geo_fastpfor()
        .parts_ring(E::fastpfor())
        .geo(poly_hole_touching())
        .write(w, "poly_hole_touching_fpf");
    geo_varint()
        .parts_ring(E::varint())
        .tessellate()
        .geo(poly_hole_touching())
        .write(w, "poly_hole_touching_tes");
    geo_fastpfor()
        .parts_ring(E::fastpfor())
        .tessellate()
        .geo(poly_hole_touching())
        .write(w, "poly_hole_touching_fpf_tes");

    geo_varint()
        .rings(E::rle_varint())
        .rings2(E::rle_varint())
        .geo(MultiPolygon(vec![poly1(), poly2()]))
        .write(w, "poly_multi");
    geo_fastpfor()
        .rings(E::rle_fastpfor())
        .rings2(E::rle_fastpfor())
        .geo(MultiPolygon(vec![poly1(), poly2()]))
        .write(w, "poly_multi_fpf");
    geo_varint()
        .rings(E::rle_varint())
        .rings2(E::rle_varint())
        .tessellate()
        .geo(MultiPolygon(vec![poly1(), poly2()]))
        .write(w, "poly_multi_tes");
    geo_fastpfor()
        .rings(E::rle_fastpfor())
        .rings2(E::rle_fastpfor())
        .tessellate()
        .geo(MultiPolygon(vec![poly1(), poly2()]))
        .write(w, "poly_multi_fpf_tes");

    // Close the shared Morton curve into a ring to test Morton encoding for polygons.
    let mut morton_ring = morton_curve();
    morton_ring.push(morton_ring[0]);
    let morton_poly = Polygon::new(LineString::new(morton_ring), vec![]);
    geo_varint()
        .geo(morton_poly.clone())
        .write(w, "poly_morton_ring_no_morton");
    geo_varint()
        .vertex_buffer_type(VertexBufferType::Morton)
        .vertex_offsets(E::delta_rle_varint())
        .geo(morton_poly)
        .write(w, "poly_morton_ring_morton");

    // Split the Morton curve into two halves and close each into a ring to form a MultiPolygon.
    let mc = morton_curve();
    let half = mc.len() / 2;
    let mut mr1 = mc[..half].to_vec();
    mr1.push(mr1[0]);
    let mut mr2 = mc[half..].to_vec();
    mr2.push(mr2[0]);
    let mp_morton = MultiPolygon(vec![
        Polygon::new(LineString::new(mr1), vec![]),
        Polygon::new(LineString::new(mr2), vec![]),
    ]);
    geo_varint()
        .rings(E::rle_varint())
        .rings2(E::rle_varint())
        .geo(mp_morton.clone())
        .write(w, "poly_multi_morton_ring_no_morton");
    geo_varint()
        .rings(E::rle_varint())
        .rings2(E::rle_varint())
        .vertex_buffer_type(VertexBufferType::Morton)
        .vertex_offsets(E::delta_rle_varint())
        .geo(mp_morton)
        .write(w, "poly_multi_morton_ring_morton");

    geo_varint()
        .geo(MultiPoint(vec![P1, P2, P3]))
        .write(w, "multipoint");
    geo_varint()
        .no_rings(E::rle_varint())
        .geo(MultiLineString(vec![line1(), line2()]))
        .write(w, "multiline");

    // Split the Morton curve into two halves to form a MultiLineString with Morton encoding.
    let mline1 = LineString::new(mc[..half].to_vec());
    let mline2 = LineString::new(mc[half..].to_vec());
    geo_varint()
        .no_rings(E::rle_varint())
        .vertex_buffer_type(VertexBufferType::Morton)
        .vertex_offsets(E::delta_rle_varint())
        .geo(MultiLineString(vec![mline1, mline2]))
        .write(w, "multiline_morton");
}

fn write_mix(w: &mut SynthWriter, current: &[usize]) {
    let mut builder = geo_varint();
    let mut builder_t = Some(geo_varint().tessellate());
    let mut name = format!("mix_{}", current.len());
    for idx in current {
        let mix_type = &MIX_TYPES[*idx];
        builder = builder.geo(mix_type.1.clone());
        write!(&mut name, "_{}", mix_type.0).unwrap();
        if let Some(bldr) = builder_t {
            if matches!(mix_type.1, Geom32::Polygon(_) | Geom32::MultiPolygon(_)) {
                builder_t = Some(bldr.geo(mix_type.1.clone()));
            } else {
                builder_t = None;
            }
        }
    }
    if let Some(bldr) = builder_t {
        bldr.write(w, format!("{name}_tes"));
    }
    builder.write(w, &name);
}

fn generate_combinations(w: &mut SynthWriter, k: usize, start: usize, current: &mut Vec<usize>) {
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

fn generate_mixed(w: &mut SynthWriter) {
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

fn generate_extent(w: &mut SynthWriter) {
    for e in [512_i32, 4096, 131_072, 1_073_741_824] {
        geo_varint()
            .extent(e.cast_unsigned())
            .geo(line![c(0_i32, 0), c(e - 1, e - 1)])
            .write(w, format!("extent_{e}"));
        geo_varint()
            .extent(e.cast_unsigned())
            .geo(line![c(-42_i32, -42), c(e + 42, e + 42)])
            .write(w, format!("extent_buf_{e}"));
    }
}

fn generate_ids(w: &mut SynthWriter) {
    p0().ids(vec![Some(100)], IdEncoder::new(L::None, IdWidth::Id32))
        .write(w, "id");
    p0().ids(
        vec![Some(u64::from(u32::MIN))],
        IdEncoder::new(L::None, IdWidth::Id32),
    )
    .write(w, "id_min");
    p0().ids(
        vec![Some(u64::from(u32::MAX))],
        IdEncoder::new(L::None, IdWidth::Id32),
    )
    .write(w, "id_max-rust");
    p0().ids(
        vec![Some(9_234_567_890)],
        IdEncoder::new(L::None, IdWidth::Id64),
    )
    .write(w, "id64");
    p0().ids(vec![Some(u64::MAX)], IdEncoder::new(L::None, IdWidth::Id64))
        .write(w, "id64_max-rust");

    let four_p0 = || geo_varint_with_rle().geos([P0, P0, P0, P0]);
    four_p0()
        .ids(
            vec![Some(103), Some(103), Some(103), Some(103)],
            IdEncoder::new(L::None, IdWidth::Id32),
        )
        .write(w, "ids");
    four_p0()
        .ids(
            vec![Some(103), Some(103), Some(103), Some(103)],
            IdEncoder::new(L::Delta, IdWidth::Id32),
        )
        .write(w, "ids_delta");
    four_p0()
        .ids(
            vec![Some(103), Some(103), Some(103), Some(103)],
            IdEncoder::new(L::Rle, IdWidth::Id32),
        )
        .write(w, "ids_rle");
    four_p0()
        .ids(
            vec![Some(103), Some(103), Some(103), Some(103)],
            IdEncoder::new(L::DeltaRle, IdWidth::Id32),
        )
        .write(w, "ids_delta_rle");
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
        .write(w, "ids64");
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
        .write(w, "ids64_delta");
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
        .write(w, "ids64_rle");
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
        .write(w, "ids64_delta_rle");

    let five_p0 = || geo_varint_with_rle().geos([P0, P0, P0, P0, P0]);
    five_p0()
        .ids(
            vec![Some(100), Some(101), None, Some(105), Some(106)],
            IdEncoder::new(L::None, IdWidth::OptId32),
        )
        .write(w, "ids_opt");
    five_p0()
        .ids(
            vec![Some(100), Some(101), None, Some(105), Some(106)],
            IdEncoder::new(L::Delta, IdWidth::OptId32),
        )
        .write(w, "ids_opt_delta");
    five_p0()
        .ids(
            vec![None, Some(9_234_567_890), Some(101), Some(105), Some(106)],
            IdEncoder::new(L::None, IdWidth::OptId64),
        )
        .write(w, "ids64_opt");
    five_p0()
        .ids(
            vec![None, Some(9_234_567_890), Some(101), Some(105), Some(106)],
            IdEncoder::new(L::Delta, IdWidth::OptId64),
        )
        .write(w, "ids64_opt_delta");

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
        .write(w, "ids64_minmax-rust");
    four_p0()
        .ids(min_max(), IdEncoder::new(L::Delta, IdWidth::Id64))
        .write(w, "ids64_minmax_delta-rust");
}

fn generate_properties(w: &mut SynthWriter) {
    // Properties with special names
    p0().add_prop(
        S::bool().with_forced_presence(true),
        P::bool("", vec![Some(true)]),
    )
    .write(w, "prop_empty_name");
    p0().add_prop(
        S::bool().with_forced_presence(true),
        P::bool("hello\u{0000} world\n", vec![Some(true)]),
    )
    .write(w, "prop_special_name");

    let enc = S::bool().with_forced_presence(true);
    p0().add_prop(enc, P::bool("val", vec![Some(true)]))
        .write(w, "prop_bool");
    p0().add_prop(enc, P::bool("val", vec![Some(false)]))
        .write(w, "prop_bool_false");
    // Two-feature optional bool variants
    geo_varint_with_rle()
        .geos([P0, P0])
        .add_prop(enc, P::bool("val", vec![Some(true), None]))
        .write(w, "prop_bool_true_null");
    geo_varint_with_rle()
        .geos([P0, P0])
        .add_prop(enc, P::bool("val", vec![None, Some(true)]))
        .write(w, "prop_bool_null_true");
    geo_varint_with_rle()
        .geos([P0, P0])
        .add_prop(enc, P::bool("val", vec![Some(false), None]))
        .write(w, "prop_bool_false_null");
    geo_varint_with_rle()
        .geos([P0, P0])
        .add_prop(enc, P::bool("val", vec![None, Some(false)]))
        .write(w, "prop_bool_null_false");

    let enc = S::int(E::varint()).with_forced_presence(true);
    p0().add_prop(enc, P::i32("val", vec![Some(42)]))
        .write(w, "prop_i32");
    p0().add_prop(enc, P::i32("val", vec![Some(-42)]))
        .write(w, "prop_i32_neg");
    p0().add_prop(enc, P::i32("val", vec![Some(i32::MIN)]))
        .write(w, "prop_i32_min");
    p0().add_prop(enc, P::i32("val", vec![Some(i32::MAX)]))
        .write(w, "prop_i32_max");
    // Two-feature optional i32 variants
    geo_varint_with_rle()
        .geos([P0, P0])
        .add_prop(enc, P::i32("val", vec![Some(42), None]))
        .write(w, "prop_i32_val_null");
    geo_varint_with_rle()
        .geos([P0, P0])
        .add_prop(enc, P::i32("val", vec![None, Some(42)]))
        .write(w, "prop_i32_null_val");

    p0().add_prop(enc, P::u32("val", vec![Some(42)]))
        .write(w, "prop_u32");
    p0().add_prop(enc, P::u32("val", vec![Some(0)]))
        .write(w, "prop_u32_min");
    p0().add_prop(enc, P::u32("val", vec![Some(u32::MAX)]))
        .write(w, "prop_u32_max");
    // Two-feature optional u32 variants
    geo_varint_with_rle()
        .geos([P0, P0])
        .add_prop(enc, P::u32("val", vec![Some(42), None]))
        .write(w, "prop_u32_val_null");
    geo_varint_with_rle()
        .geos([P0, P0])
        .add_prop(enc, P::u32("val", vec![None, Some(42)]))
        .write(w, "prop_u32_null_val");

    p0().add_prop(enc, P::i64("val", vec![Some(9_876_543_210)]))
        .write(w, "prop_i64");
    p0().add_prop(enc, P::i64("val", vec![Some(-9_876_543_210)]))
        .write(w, "prop_i64_neg");
    p0().add_prop(enc, P::i64("val", vec![Some(i64::MIN)]))
        .write(w, "prop_i64_min");
    p0().add_prop(enc, P::i64("val", vec![Some(i64::MAX)]))
        .write(w, "prop_i64_max");
    // Two-feature optional i64 variants
    geo_varint_with_rle()
        .geos([P0, P0])
        .add_prop(enc, P::i64("val", vec![Some(9_876_543_210), None]))
        .write(w, "prop_i64_val_null");
    geo_varint_with_rle()
        .geos([P0, P0])
        .add_prop(enc, P::i64("val", vec![None, Some(9_876_543_210)]))
        .write(w, "prop_i64_null_val");

    p0().add_prop(enc, P::u64("bignum", vec![Some(1_234_567_890_123_456_789)]))
        .write(w, "prop_u64");
    p0().add_prop(enc, P::u64("bignum", vec![Some(0)]))
        .write(w, "prop_u64_min");
    p0().add_prop(enc, P::u64("bignum", vec![Some(u64::MAX)]))
        .write(w, "prop_u64_max");
    // Two-feature optional u64 variants (key is "val" to match Java)
    geo_varint_with_rle()
        .geos([P0, P0])
        .add_prop(
            enc,
            P::u64("val", vec![Some(1_234_567_890_123_456_789), None]),
        )
        .write(w, "prop_u64_val_null");
    geo_varint_with_rle()
        .geos([P0, P0])
        .add_prop(
            enc,
            P::u64("val", vec![None, Some(1_234_567_890_123_456_789)]),
        )
        .write(w, "prop_u64_null_val");

    let enc = S::float().with_forced_presence(true);
    #[expect(clippy::approx_constant)]
    p0().add_prop(enc, P::f32("val", vec![Some(3.14)]))
        .write(w, "prop_f32");
    p0().add_prop(enc, P::f32("val", vec![Some(f32::NEG_INFINITY)]))
        .write(w, "prop_f32_neg_inf");
    p0().add_prop(enc, P::f32("val", vec![Some(f32::from_bits(1))]))
        .write(w, "prop_f32_min_val");
    p0().add_prop(enc, P::f32("val", vec![Some(f32::MIN_POSITIVE)]))
        .write(w, "prop_f32_min_norm");
    p0().add_prop(enc, P::f32("val", vec![Some(0.0)]))
        .write(w, "prop_f32_zero");
    p0().add_prop(enc, P::f32("val", vec![Some(-0.0)]))
        .write(w, "prop_f32_neg_zero");
    p0().add_prop(enc, P::f32("val", vec![Some(f32::MAX)]))
        .write(w, "prop_f32_max");
    p0().add_prop(enc, P::f32("val", vec![Some(f32::INFINITY)]))
        .write(w, "prop_f32_pos_inf");
    p0().add_prop(enc, P::f32("val", vec![Some(f32::NAN)]))
        .write(w, "prop_f32_nan");
    // Two-feature optional f32 variants
    #[expect(clippy::approx_constant)]
    geo_varint_with_rle()
        .geos([P0, P0])
        .add_prop(enc, P::f32("val", vec![Some(3.14), None]))
        .write(w, "prop_f32_val_null");
    #[expect(clippy::approx_constant)]
    geo_varint_with_rle()
        .geos([P0, P0])
        .add_prop(enc, P::f32("val", vec![None, Some(3.14)]))
        .write(w, "prop_f32_null_val");

    p0().add_prop(enc, P::f64("val", vec![Some(std::f64::consts::PI)]))
        .write(w, "prop_f64");
    p0().add_prop(enc, P::f64("val", vec![Some(f64::NAN)]))
        .write(w, "prop_f64_nan");
    p0().add_prop(enc, P::f64("val", vec![Some(f64::NEG_INFINITY)]))
        .write(w, "prop_f64_neg_inf");
    p0().add_prop(enc, P::f64("val", vec![Some(f64::from_bits(1))]))
        .write(w, "prop_f64_min_val");
    p0().add_prop(enc, P::f64("val", vec![Some(f64::MIN_POSITIVE)]))
        .write(w, "prop_f64_min_norm");
    p0().add_prop(enc, P::f64("val", vec![Some(-0.0)]))
        .write(w, "prop_f64_neg_zero");
    p0().add_prop(enc, P::f64("val", vec![Some(0.0)]))
        .write(w, "prop_f64_zero");
    p0().add_prop(enc, P::f64("val", vec![Some(f64::MAX)]))
        .write(w, "prop_f64_max");
    p0().add_prop(enc, P::f64("val", vec![Some(f64::INFINITY)]))
        .write(w, "prop_f64_pos_inf");
    // Two-feature optional f64 variants
    geo_varint_with_rle()
        .geos([P0, P0])
        .add_prop(enc, P::f64("val", vec![Some(std::f64::consts::PI), None]))
        .write(w, "prop_f64_val_null");
    geo_varint_with_rle()
        .geos([P0, P0])
        .add_prop(enc, P::f64("val", vec![None, Some(std::f64::consts::PI)]))
        .write(w, "prop_f64_null_val");

    let enc = S::str(E::varint()).with_forced_presence(true);
    p0().add_prop(enc, P::str("val", vec![Some(String::new())]))
        .write(w, "prop_str_empty");
    p0().add_prop(enc, P::str("val", vec![Some("42".to_string())]))
        .write(w, "prop_str_ascii");
    p0().add_prop(
        enc,
        P::str("val", vec![Some("Line1\n\t\"quoted\"\\path".to_string())]),
    )
    .write(w, "prop_str_escape");
    p0().add_prop(
        enc,
        P::str("val", vec![Some("München 📍 cafe\u{0301}".to_string())]),
    )
    .write(w, "prop_str_unicode");
    p0().add_prop(
        enc,
        P::str("val", vec![Some("hello\u{0000} world\n".to_string())]),
    )
    .write(w, "prop_str_special");
    // Two-feature optional str variants
    geo_varint_with_rle()
        .geos([P0, P0])
        .add_prop(enc, P::str("val", vec![Some("42".to_string()), None]))
        .write(w, "prop_str_val_null");
    geo_varint_with_rle()
        .geos([P0, P0])
        .add_prop(enc, P::str("val", vec![None, Some("42".to_string())]))
        .write(w, "prop_str_null_val");
    geo_varint_with_rle()
        .geos([P0, P0])
        .add_prop(enc, P::str("val", vec![Some(String::new()), None]))
        .write(w, "prop_str_val_empty");
    geo_varint_with_rle()
        .geos([P0, P0])
        .add_prop(enc, P::str("val", vec![None, Some(String::new())]))
        .write(w, "prop_str_empty_val");

    p0().add_prop(
        S::bool().with_forced_presence(true),
        P::bool("active", vec![Some(true)]),
    )
    .add_prop(
        S::int(E::varint()).with_forced_presence(true),
        P::u64("biggest", vec![Some(0)]),
    ) // FIXME: this should be u64, but java does it it this way
    .add_prop(
        S::int(E::varint()).with_forced_presence(true),
        P::i32("bignum", vec![Some(42)]),
    )
    .add_prop(
        S::int(E::varint()).with_forced_presence(true),
        P::i32("count", vec![Some(42)]),
    )
    .add_prop(
        S::int(E::varint()).with_forced_presence(true),
        P::u32("medium", vec![Some(100)]),
    )
    .add_prop(
        S::str(E::varint()).with_forced_presence(true),
        P::str("name", vec![Some("Test Point".to_string())]),
    )
    .add_prop(
        S::float().with_forced_presence(true),
        P::f64("precision", vec![Some(0.123_456_789)]),
    )
    .add_prop(
        S::float().with_forced_presence(true),
        P::f32("temp", vec![Some(25.5)]),
    )
    //FIXME in java
    //.add_prop(enc, "tiny-count", PropValue::I8(vec![Some(42)]))
    //.add_prop(enc, "tiny-count", PropValue::U8(vec![Some(100)]))
    .write(w, "props_mixed");

    generate_props_i32(w);
    generate_props_u32(w);
    generate_props_u64(w);
    generate_props_str(w);
    generate_shared_dictionaries(w);
}

fn generate_props_i32(w: &mut SynthWriter) {
    let four_points = || geo_varint_with_rle().geos([P0, P1, P2, P3]);
    let values = || P::i32("val", vec![Some(42), Some(42), Some(42), Some(42)]);

    four_points()
        .add_prop(S::int(E::varint()).with_forced_presence(true), values())
        .write(w, "props_i32");
    four_points()
        .add_prop(
            S::int(E::delta_varint()).with_forced_presence(true),
            values(),
        )
        .write(w, "props_i32_delta");
    four_points()
        .add_prop(S::int(E::rle_varint()).with_forced_presence(true), values())
        .write(w, "props_i32_rle");
    four_points()
        .add_prop(
            S::int(E::delta_rle_varint()).with_forced_presence(true),
            values(),
        )
        .write(w, "props_i32_delta_rle");
}

fn generate_props_u32(w: &mut SynthWriter) {
    let four_points = || geo_varint_with_rle().geos([P0, P1, P2, P3]);
    let values = || {
        P::u32(
            "val",
            vec![Some(9_000), Some(9_000), Some(9_000), Some(9_000)],
        )
    };

    four_points()
        .add_prop(S::int(E::varint()).with_forced_presence(true), values())
        .write(w, "props_u32");
    four_points()
        .add_prop(
            S::int(E::delta_varint()).with_forced_presence(true),
            values(),
        )
        .write(w, "props_u32_delta");
    four_points()
        .add_prop(S::int(E::rle_varint()).with_forced_presence(true), values())
        .write(w, "props_u32_rle");
    four_points()
        .add_prop(
            S::int(E::delta_rle_varint()).with_forced_presence(true),
            values(),
        )
        .write(w, "props_u32_delta_rle");

    for multiplier in [1, 2, 3, 4] {
        for offset in [-1, 0, 1] {
            let count = usize::try_from(128 * multiplier + offset).unwrap();
            // Sequence 0,1,2, 0,1,2, 0,1,2, ...
            let vals: Vec<_> = (0..count)
                .map(|i| Some(u32::try_from(i % 3).unwrap()))
                .collect();
            geo_fastpfor()
                .meta(E::rle_fastpfor())
                .geos(vec![P0; count])
                .add_prop(
                    S::int(E::fastpfor()).with_forced_presence(true),
                    P::u32("val", vals),
                )
                .write(w, format!("props_u32_fpf_{count}"));
        }
    }
}

fn generate_props_u64(w: &mut SynthWriter) {
    let four_points = || geo_varint_with_rle().geos([P0, P1, P2, P3]);
    let property = || {
        P::u64(
            "val",
            vec![Some(9_000), Some(9_000), Some(9_000), Some(9_000)],
        )
    };

    four_points()
        .add_prop(S::int(E::varint()).with_forced_presence(true), property())
        .write(w, "props_u64");
    four_points()
        .add_prop(
            S::int(E::delta_varint()).with_forced_presence(true),
            property(),
        )
        .write(w, "props_u64_delta");
    four_points()
        .add_prop(
            S::int(E::rle_varint()).with_forced_presence(true),
            property(),
        )
        .write(w, "props_u64_rle");
    four_points()
        .add_prop(
            S::int(E::delta_rle_varint()).with_forced_presence(true),
            property(),
        )
        .write(w, "props_u64_delta_rle");
}

fn generate_props_str(w: &mut SynthWriter) {
    let six_points = || geo_varint_with_rle().geos([P1, P2, P3, PH1, PH2, PH3]);
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
        .add_prop(S::str(E::varint()).with_forced_presence(true), values())
        .write(w, "props_str");
    six_points()
        .add_prop(S::str_fsst(E::varint(), E::varint()), values())
        .write(w, "props_str_fsst-rust"); // FSST compression output is not byte-for-byte consistent with Java's

    // Two features with the same 30-char value → deduplicated dictionary encoding.
    // 30 chars because otherwise FSST is skipped.
    let long_string = || "A".repeat(30);
    let two_pts = || geo_varint_with_rle().geos([P1, P2]);
    let two_same = || P::str("val", vec![Some(long_string()), Some(long_string())]);

    two_pts()
        .add_prop(
            S::str_dict(E::varint(), E::rle_varint()).with_forced_presence(true),
            two_same(),
        )
        .write(w, "props_offset_str");
    two_pts()
        .add_prop(
            S::str_fsst_dict(E::varint(), E::varint(), E::rle_varint()).with_forced_presence(true),
            two_same(),
        )
        .write(w, "props_offset_str_fsst-rust"); // FSST output may differ from Java
}

fn generate_shared_dictionaries(w: &mut SynthWriter) {
    let long_string = || "A".repeat(30);
    p0().add_prop(
        S::str(E::varint()).with_forced_presence(true),
        P::str("name:de", vec![Some(long_string())]),
    )
    .add_prop(
        S::str(E::varint()).with_forced_presence(true),
        P::str("name:en", vec![Some(long_string())]),
    )
    .write(w, "props_no_shared_dict");

    p0().add_shared_dict(
        SharedDict::new("name:", SE::plain(E::varint()))
            .column_fp("de", E::varint(), [Some(long_string())])
            .column_fp("en", E::varint(), [Some(long_string())]),
    )
    .write(w, "props_shared_dict");

    p0().add_shared_dict(
        SharedDict::new("", SE::plain(E::varint()))
            .column_fp("a", E::varint(), [Some(long_string())])
            .column_fp("b", E::varint(), [Some(long_string())]),
    )
    .write(w, "props_shared_dict_no_struct_name");

    p0().add_prop(
        S::str(E::varint()).with_forced_presence(true),
        P::str("place", vec![Some(long_string())]),
    )
    .add_shared_dict(
        SharedDict::new("name:en", SE::plain(E::varint())).column_fp(
            "",
            E::varint(),
            [Some(long_string())],
        ),
    )
    .write(w, "props_shared_dict_one_child");

    p0().add_shared_dict(SharedDict::new("a", SE::plain(E::varint())).column_fp(
        "",
        E::varint(),
        [Some(long_string())],
    ))
    .write(w, "props_shared_dict_no_child_name");

    p0().add_shared_dict(
        SharedDict::new("name:", SE::fsst(E::varint(), E::varint()))
            .column_fp("de", E::varint(), [Some(long_string())])
            .column_fp("en", E::varint(), [Some(long_string())]),
    )
    .write(w, "props_shared_dict_fsst-rust");

    p0().add_shared_dict(
        SharedDict::new("a", SE::fsst(E::varint(), E::varint())).column_fp(
            "",
            E::varint(),
            [Some(long_string())],
        ),
    )
    .write(w, "props_shared_dict_no_child_name_fsst-rust"); // FSST output differs from Java

    p0().add_prop(
        S::str(E::varint()).with_forced_presence(true),
        P::str("place", vec![Some(long_string())]),
    )
    .add_shared_dict(
        SharedDict::new("name:en", SE::fsst(E::varint(), E::varint())).column_fp(
            "",
            E::varint(),
            [Some(long_string())],
        ),
    )
    .write(w, "props_shared_dict_one_child_fsst-rust"); // FSST output differs from Java
    p0()
        // column names MUST be unique, but the shared dict prefix can duplicate
        .add_shared_dict(
            SharedDict::new("name", SE::plain(E::varint()))
                .column_fp(":de", E::varint(), [Some(long_string())])
                .column_fp(":en", E::varint(), [Some(long_string())]),
        )
        .add_shared_dict(
            SharedDict::new("name", SE::plain(E::varint()))
                .column_fp(":fr", E::varint(), [Some(long_string())])
                .column_fp(":he", E::varint(), [Some(long_string())]),
        )
        .write(w, "props_shared_dict_2_same_prefix-rust");

    p0().add_shared_dict(
        SharedDict::new("", SE::fsst(E::varint(), E::varint()))
            .column_fp("a", E::varint(), [Some(long_string())])
            .column_fp("b", E::varint(), [Some(long_string())]),
    )
    .write(w, "props_shared_dict_no_struct_name_fsst-rust");
}
