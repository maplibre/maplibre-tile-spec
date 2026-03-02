//! Rust synthetic MLT file generator.
//!
//! This generates synthetic MLT files for testing and validation.
//! The goal is to produce byte-for-byte identical output to the Java generator.

mod layer;

use std::fmt::Write as _;
use std::path::Path;
use std::sync::LazyLock;
use std::{f64, fs};

use geo_types::{
    Coord, LineString, MultiLineString, MultiPoint, MultiPolygon, Point, Polygon, coord,
    line_string as line, wkt,
};
use mlt_core::geojson::Geom32;
use mlt_core::v01::{
    IdEncoder, IdWidth, IntegerEncoder as E, LogicalEncoder as L, PresenceStream as O, PropValue,
    ScalarEncoder as S, StringEncoding as SE, VertexBufferType,
};

use crate::layer::{Layer, SynthWriter};

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

// Geometry builder macros matching Java definitions
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

fn generate_geometry(w: &SynthWriter) {
    p0(w).write("point");
    w.geo_varint().geo(line1()).write("line");
    // Morton (Z-order) line: de-interleave index bits into x/y (even/odd bits).
    let num_points = 16; // 4x4 complete Morton block
    let scale = 8;
    let morton_bits = 4;
    let mut morton_curve = Vec::with_capacity(num_points);
    for i in 0..num_points {
        let i = i32::try_from(i).unwrap();
        let mut x = 0_i32;
        let mut y = 0_i32;
        for b in 0..morton_bits {
            x |= ((i >> (2 * b)) & 1) << b;
            y |= ((i >> (2 * b + 1)) & 1) << b;
        }
        morton_curve.push(c(x * scale, y * scale));
    }
    w.geo_varint()
        .geo(LineString::new(morton_curve))
        .vertex_buffer_type(VertexBufferType::Morton)
        .vertex_offsets(E::delta_rle_varint())
        .write("line_morton");
    w.geo_varint().geo(poly1()).write("polygon");
    w.geo_fastpfor().geo(poly1()).write("polygon_fpf");
    w.geo_varint().tessellated(poly1()).write("polygon_tes");
    w.geo_fastpfor()
        .tessellated(poly1())
        .write("polygon_fpf_tes");
    w.geo_varint()
        .parts_ring(E::rle_varint())
        .geo(poly1h())
        .write("polygon_hole");
    w.geo_fastpfor()
        .parts_ring(E::rle_fastpfor())
        .geo(poly1h())
        .write("polygon_hole_fpf");
    w.geo_varint()
        .rings(E::rle_varint())
        .rings2(E::rle_varint())
        .geo(MultiPolygon(vec![poly1(), poly2()]))
        .write("polygon_multi");
    w.geo_fastpfor()
        .rings(E::rle_fastpfor())
        .rings2(E::rle_fastpfor())
        .geo(MultiPolygon(vec![poly1(), poly2()]))
        .write("polygon_multi_fpf");
    w.geo_varint()
        .geo(MultiPoint(vec![P1, P2, P3]))
        .write("multipoint");
    w.geo_varint()
        .no_rings(E::rle_varint())
        .geo(MultiLineString(vec![line1(), line2()]))
        .write("multiline");
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
        .ids(vec![Some(0)], IdEncoder::new(L::None, IdWidth::Id32))
        .write("id0");
    p0(w)
        .ids(vec![Some(100)], IdEncoder::new(L::None, IdWidth::Id32))
        .write("id");
    p0(w)
        .ids(
            vec![Some(9_234_567_890)],
            IdEncoder::new(L::None, IdWidth::Id64),
        )
        .write("id64");

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
}

fn generate_properties(w: &SynthWriter) {
    let enc = S::bool(O::Present);
    p0(w)
        .add_prop(enc, "val", PropValue::Bool(vec![Some(true)]))
        .write("prop_bool");
    p0(w)
        .add_prop(enc, "val", PropValue::Bool(vec![Some(false)]))
        .write("prop_bool_false");

    let enc = S::int(O::Present, E::varint());
    p0(w)
        .add_prop(enc, "val", PropValue::I32(vec![Some(42)]))
        .write("prop_i32");
    p0(w)
        .add_prop(enc, "val", PropValue::I32(vec![Some(-42)]))
        .write("prop_i32_neg");
    p0(w)
        .add_prop(enc, "val", PropValue::I32(vec![Some(i32::MIN)]))
        .write("prop_i32_min");
    p0(w)
        .add_prop(enc, "val", PropValue::I32(vec![Some(i32::MAX)]))
        .write("prop_i32_max");

    p0(w)
        .add_prop(enc, "val", PropValue::U32(vec![Some(42)]))
        .write("prop_u32");
    p0(w)
        .add_prop(enc, "val", PropValue::U32(vec![Some(0)]))
        .write("prop_u32_min");
    p0(w)
        .add_prop(enc, "val", PropValue::U32(vec![Some(u32::MAX)]))
        .write("prop_u32_max");

    p0(w)
        .add_prop(enc, "val", PropValue::I64(vec![Some(9_876_543_210)]))
        .write("prop_i64");
    p0(w)
        .add_prop(enc, "val", PropValue::I64(vec![Some(-9_876_543_210)]))
        .write("prop_i64_neg");
    p0(w)
        .add_prop(enc, "val", PropValue::I64(vec![Some(i64::MIN)]))
        .write("prop_i64_min");
    p0(w)
        .add_prop(enc, "val", PropValue::I64(vec![Some(i64::MAX)]))
        .write("prop_i64_max");

    p0(w)
        .add_prop(
            enc,
            "bignum",
            PropValue::U64(vec![Some(1_234_567_890_123_456_789)]),
        )
        .write("prop_u64");
    p0(w)
        .add_prop(enc, "bignum", PropValue::U64(vec![Some(0)]))
        .write("prop_u64_min");
    p0(w)
        .add_prop(enc, "bignum", PropValue::U64(vec![Some(u64::MAX)]))
        .write("prop_u64_max");

    let enc = S::float(O::Present);
    #[expect(clippy::approx_constant)]
    p0(w)
        .add_prop(enc, "val", PropValue::F32(vec![Some(3.14)]))
        .write("prop_f32");
    p0(w)
        .add_prop(enc, "val", PropValue::F32(vec![Some(f32::NEG_INFINITY)]))
        .write("prop_f32_neg_inf");
    p0(w)
        .add_prop(enc, "val", PropValue::F32(vec![Some(f32::from_bits(1))]))
        .write("prop_f32_min_val");
    p0(w)
        .add_prop(enc, "val", PropValue::F32(vec![Some(f32::MIN_POSITIVE)]))
        .write("prop_f32_min_norm");
    p0(w)
        .add_prop(enc, "val", PropValue::F32(vec![Some(0.0)]))
        .write("prop_f32_zero");
    p0(w)
        .add_prop(enc, "val", PropValue::F32(vec![Some(-0.0)]))
        .write("prop_f32_neg_zero");
    p0(w)
        .add_prop(enc, "val", PropValue::F32(vec![Some(f32::MAX)]))
        .write("prop_f32_max");
    p0(w)
        .add_prop(enc, "val", PropValue::F32(vec![Some(f32::INFINITY)]))
        .write("prop_f32_pos_inf");
    p0(w)
        .add_prop(enc, "val", PropValue::F32(vec![Some(f32::NAN)]))
        .write("prop_f32_nan");
    p0(w)
        .add_prop(enc, "val", PropValue::F64(vec![Some(f64::consts::PI)]))
        .write("prop_f64");
    p0(w)
        .add_prop(enc, "val", PropValue::F64(vec![Some(f64::NAN)]))
        .write("prop_f64_nan");
    p0(w)
        .add_prop(enc, "val", PropValue::F64(vec![Some(f64::NEG_INFINITY)]))
        .write("prop_f64_neg_inf");
    p0(w)
        .add_prop(enc, "val", PropValue::F64(vec![Some(f64::from_bits(1))]))
        .write("prop_f64_min_val");
    p0(w)
        .add_prop(enc, "val", PropValue::F64(vec![Some(f64::MIN_POSITIVE)]))
        .write("prop_f64_min_norm");
    p0(w)
        .add_prop(enc, "val", PropValue::F64(vec![Some(-0.0)]))
        .write("prop_f64_neg_zero");
    p0(w)
        .add_prop(enc, "val", PropValue::F64(vec![Some(0.0)]))
        .write("prop_f64_zero");
    p0(w)
        .add_prop(enc, "val", PropValue::F64(vec![Some(f64::MAX)]))
        .write("prop_f64_max");
    p0(w)
        .add_prop(enc, "val", PropValue::F64(vec![Some(f64::INFINITY)]))
        .write("prop_f64_pos_inf");

    let enc = S::str(O::Present, E::varint());
    p0(w)
        .add_prop(enc, "val", PropValue::Str(vec![Some(String::new())]))
        .write("prop_str_empty");
    p0(w)
        .add_prop(enc, "val", PropValue::Str(vec![Some("42".to_string())]))
        .write("prop_str_ascii");
    p0(w)
        .add_prop(
            enc,
            "val",
            PropValue::Str(vec![Some("Line1\n\t\"quoted\"\\path".to_string())]),
        )
        .write("prop_str_escape");
    p0(w)
        .add_prop(
            enc,
            "val",
            PropValue::Str(vec![Some("München 📍 cafe\u{0301}".to_string())]),
        )
        .write("prop_str_unicode");

    p0(w)
        .add_prop(
            S::str(O::Present, E::varint()),
            "name",
            PropValue::Str(vec![Some("Test Point".to_string())]),
        )
        .add_prop(
            S::bool(O::Present),
            "active",
            PropValue::Bool(vec![Some(true)]),
        )
        //FIXME in java
        //.add_prop(enc, "tiny-count", PropValue::I8(vec![Some(42)]))
        //.add_prop(enc, "tiny-count", PropValue::U8(vec![Some(100)]))
        .add_prop(
            S::int(O::Present, E::varint()),
            "count",
            PropValue::I32(vec![Some(42)]),
        )
        .add_prop(
            S::int(O::Present, E::varint()),
            "medium",
            PropValue::U32(vec![Some(100)]),
        )
        .add_prop(
            S::int(O::Present, E::varint()),
            "bignum",
            PropValue::I32(vec![Some(42)]),
        )
        .add_prop(
            S::int(O::Present, E::varint()),
            "biggest",
            PropValue::U64(vec![Some(0)]),
        ) // FIXME: this should be u64, but java does it it this way
        .add_prop(
            S::float(O::Present),
            "temp",
            PropValue::F32(vec![Some(25.5)]),
        )
        .add_prop(
            S::float(O::Present),
            "precision",
            PropValue::F64(vec![Some(0.123_456_789)]),
        )
        .write("props_mixed");

    generate_props_i32(w);
    generate_props_u32(w);
    generate_props_u64(w);
    generate_props_str(w);
    generate_shared_dictionaries(w);
}

fn generate_props_i32(w: &SynthWriter) {
    let four_points = || w.geo_varint().meta(E::rle_varint()).geos([P0, P1, P2, P3]);
    let values = || PropValue::I32(vec![Some(42), Some(42), Some(42), Some(42)]);

    four_points()
        .add_prop(S::int(O::Present, E::varint()), "val", values())
        .write("props_i32");
    four_points()
        .add_prop(S::int(O::Present, E::delta_varint()), "val", values())
        .write("props_i32_delta");
    four_points()
        .add_prop(S::int(O::Present, E::rle_varint()), "val", values())
        .write("props_i32_rle");
    four_points()
        .add_prop(S::int(O::Present, E::delta_rle_varint()), "val", values())
        .write("props_i32_delta_rle");
}

fn generate_props_u32(w: &SynthWriter) {
    let four_points = || w.geo_varint().meta(E::rle_varint()).geos([P0, P1, P2, P3]);
    let values = || PropValue::U32(vec![Some(9_000), Some(9_000), Some(9_000), Some(9_000)]);

    four_points()
        .add_prop(S::int(O::Present, E::varint()), "val", values())
        .write("props_u32");
    four_points()
        .add_prop(S::int(O::Present, E::delta_varint()), "val", values())
        .write("props_u32_delta");
    four_points()
        .add_prop(S::int(O::Present, E::rle_varint()), "val", values())
        .write("props_u32_rle");
    four_points()
        .add_prop(S::int(O::Present, E::delta_rle_varint()), "val", values())
        .write("props_u32_delta_rle");
}

fn generate_props_u64(w: &SynthWriter) {
    let four_points = || w.geo_varint().meta(E::rle_varint()).geos([P0, P1, P2, P3]);
    let property = || PropValue::U64(vec![Some(9_000), Some(9_000), Some(9_000), Some(9_000)]);

    four_points()
        .add_prop(S::int(O::Present, E::varint()), "val", property())
        .write("props_u64");
    four_points()
        .add_prop(S::int(O::Present, E::delta_varint()), "val", property())
        .write("props_u64_delta");
    four_points()
        .add_prop(S::int(O::Present, E::rle_varint()), "val", property())
        .write("props_u64_rle");
    four_points()
        .add_prop(S::int(O::Present, E::delta_rle_varint()), "val", property())
        .write("props_u64_delta_rle");
}

fn generate_props_str(w: &SynthWriter) {
    let six_points = || {
        w.geo_varint()
            .meta(E::rle_varint())
            .geos([P1, P2, P3, PH1, PH2, PH3])
    };
    let values = || {
        PropValue::Str(vec![
            Some("residential_zone_north_sector_1".to_string()),
            Some("commercial_zone_south_sector_2".to_string()),
            Some("industrial_zone_east_sector_3".to_string()),
            Some("park_zone_west_sector_4".to_string()),
            Some("water_zone_north_sector_5".to_string()),
            Some("residential_zone_south_sector_6".to_string()),
        ])
    };

    six_points()
        .add_prop(S::str(O::Present, E::varint()), "val", values())
        .write("props_str");
    six_points()
        .add_prop(
            S::str_fsst(O::Present, E::varint(), E::varint()),
            "val",
            values(),
        )
        .write("props_str_fsst-rust"); // FSST compression output is not byte-for-byte consistent with Java's
}

fn generate_shared_dictionaries(w: &SynthWriter) {
    let long_string_value = || "A".repeat(30);
    p0(w)
        .add_prop(
            S::str(O::Present, E::varint()),
            "name:en",
            PropValue::Str(vec![Some(long_string_value())]),
        )
        .add_prop(
            S::str(O::Present, E::varint()),
            "name:de",
            PropValue::Str(vec![Some(long_string_value())]),
        )
        .write("props_no_shared_dict");

    p0(w)
        .add_shared_dict("name:", SE::plain(E::varint()))
        .add_shared_dict_column(
            "name:",
            "de",
            O::Present,
            E::varint(),
            [Some(long_string_value())],
        )
        .add_shared_dict_column(
            "name:",
            "en",
            O::Present,
            E::varint(),
            [Some(long_string_value())],
        )
        .write("props_shared_dict-rust"); // For some reason Java hallucinates another stream count at the start, so starts counting the stream count at 1

    p0(w)
        .add_shared_dict("name:", SE::fsst(E::varint(), E::varint()))
        .add_shared_dict_column(
            "name:",
            "de",
            O::Present,
            E::varint(),
            [Some(long_string_value())],
        )
        .add_shared_dict_column(
            "name:",
            "en",
            O::Present,
            E::varint(),
            [Some(long_string_value())],
        )
        .write("props_shared_dict_fsst-rust"); // Rust FSST is not byte-for-byte consistent with Java's
}
