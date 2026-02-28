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
    line_string as line, polygon as poly,
};
use mlt_core::geojson::Geom32;
use mlt_core::v01::{
    DecodedProperty, Encoder as E, IdEncoder, IdWidth, LogicalEncoder as L, PhysicalEncoder as P,
    PresenceStream as O, PropValue, PropertyEncoder,
};

use crate::layer::{DecodedProp, SynthWriter, bool, i32};

const C0: Coord<i32> = coord! { x: 13, y: 42 };
// triangle 1, clockwise winding, X ends in 1, Y ends in 2
const C1: Coord<i32> = coord! { x: 11, y: 52 };
const C2: Coord<i32> = coord! { x: 71, y: 72 };
const C3: Coord<i32> = coord! { x: 61, y: 22 };
// triangle 2, clockwise winding, X ends in 3, Y ends in 4
const C21: Coord<i32> = coord! { x: 23, y: 34 };
const C22: Coord<i32> = coord! { x: 73, y: 4 };
const C23: Coord<i32> = coord! { x: 13, y: 24 };
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

const fn p(x: i32, y: i32) -> Point<i32> {
    Point(c(x, y))
}

static MIX_TYPES: LazyLock<[(&'static str, Geom32); 7]> = LazyLock::new(|| {
    [
        ("pt", p(38, 29).into()),
        ("line", line![c(5, 38), c(12, 45), c(9, 70)].into()),
        (
            "poly",
            poly![c(55, 5), c(58, 28), c(75, 22), c(55, 5)].into(),
        ),
        (
            "polyh",
            poly! {
                exterior: [c(52, 35), c(14, 55), c(60, 72), c(52, 35)],
                interiors: [[c(32, 50), c(36, 60), c(24, 54), c(32, 50)]]
            }
            .into(),
        ),
        (
            "mpt",
            MultiPoint(vec![p(6, 25), p(21, 41), p(23, 69)]).into(),
        ),
        (
            "mline",
            MultiLineString(vec![
                line![c(24, 10), c(42, 18)],
                line![c(30, 36), c(48, 52), c(35, 62)],
            ])
            .into(),
        ),
        (
            "mpoly",
            MultiPolygon(vec![
                poly! {
                    exterior: [c(7, 20), c(21, 31), c(26, 9), c(7, 20)],
                    interiors: [[c(15, 20), c(20, 15), c(18, 25), c(15, 20)]]
                },
                poly![c(69, 57), c(71, 66), c(73, 64), c(69, 57)],
            ])
            .into(),
        ),
    ]
});

fn main() {
    let dir = Path::new("../test/synthetic/0x01-rust/");
    fs::create_dir_all(dir).unwrap_or_else(|_| panic!("to be able to create {}", dir.display()));

    let dir = dir
        .canonicalize()
        .unwrap_or_else(|_| panic!("bad path {}", dir.display()));
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
    line![C1, C2, C3]
}
fn line2() -> LineString<i32> {
    line![C21, C22, C23]
}
fn poly1() -> Polygon<i32> {
    poly![C1, C2, C3, C1]
}
fn poly2() -> Polygon<i32> {
    poly![C21, C22, C23, C21]
}
fn poly1h() -> Polygon<i32> {
    poly! { exterior: [C1, C2, C3, C1], interiors: [[H1, H2, H3, H1]] }
}

fn generate_geometry(w: &SynthWriter) {
    w.geo_varint().geo(P0).write("point");
    w.geo_varint().geo(line1()).write("line");
    // Morton (Z-order) line: de-interleave index bits into x/y (even/odd bits).
    let num_points = 16; // 4x4 complete Morton block
    let scale = 8;
    let morton_bits = 4;
    let mut morton_curve = Vec::with_capacity(num_points);
    for i in 0..num_points {
        let i = i as i32;
        let mut x = 0_i32;
        let mut y = 0_i32;
        for b in 0..morton_bits {
            x |= ((i >> (2 * b)) & 1) << b;
            y |= ((i >> (2 * b + 1)) & 1) << b;
        }
        morton_curve.push(c(x * scale, y * scale))
    }
    w.geo_varint()
        .geo(LineString::new(morton_curve))
        .morton()
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
    let p0 = || w.geo_varint().geo(P0);
    p0().ids(vec![Some(0)], IdEncoder::new(L::None, IdWidth::Id32))
        .write("id0");
    p0().ids(vec![Some(100)], IdEncoder::new(L::None, IdWidth::Id32))
        .write("id");
    p0().ids(
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
    let p0 = || w.geo_varint().geo(P0);
    let enc = PropertyEncoder::new(O::Present, L::None, P::VarInt);

    p0().add_prop(bool("val", enc).add(true)).write("prop_bool");
    p0().add_prop(bool("val", enc).add(false))
        .write("prop_bool_false");

    p0().add_prop(i32("val", enc).add(42)).write("prop_i32");
    p0().add_prop(DecodedProp::new(
        DecodedProperty {
            name: "val".to_string(),
            values: PropValue::I32(vec![Some(-42)]),
        },
        enc,
    ))
    .write("prop_i32_neg");
    p0().add_prop(DecodedProp::new(
        DecodedProperty {
            name: "val".to_string(),
            values: PropValue::I32(vec![Some(i32::MIN)]),
        },
        enc,
    ))
    .write("prop_i32_min");
    p0().add_prop(DecodedProp::new(
        DecodedProperty {
            name: "val".to_string(),
            values: PropValue::I32(vec![Some(i32::MAX)]),
        },
        enc,
    ))
    .write("prop_i32_max");

    p0().add_prop(DecodedProp::new(
        DecodedProperty {
            name: "val".to_string(),
            values: PropValue::U32(vec![Some(42)]),
        },
        enc,
    ))
    .write("prop_u32");
    p0().add_prop(DecodedProp::new(
        DecodedProperty {
            name: "val".to_string(),
            values: PropValue::U32(vec![Some(0)]),
        },
        enc,
    ))
    .write("prop_u32_min");
    p0().add_prop(DecodedProp::new(
        DecodedProperty {
            name: "val".to_string(),
            values: PropValue::U32(vec![Some(u32::MAX)]),
        },
        enc,
    ))
    .write("prop_u32_max");

    p0().add_prop(DecodedProp::new(
        DecodedProperty {
            name: "val".to_string(),
            values: PropValue::I64(vec![Some(9_876_543_210)]),
        },
        enc,
    ))
    .write("prop_i64");
    p0().add_prop(DecodedProp::new(
        DecodedProperty {
            name: "val".to_string(),
            values: PropValue::I64(vec![Some(-9_876_543_210)]),
        },
        enc,
    ))
    .write("prop_i64_neg");
    p0().add_prop(DecodedProp::new(
        DecodedProperty {
            name: "val".to_string(),
            values: PropValue::I64(vec![Some(i64::MIN)]),
        },
        enc,
    ))
    .write("prop_i64_min");
    p0().add_prop(DecodedProp::new(
        DecodedProperty {
            name: "val".to_string(),
            values: PropValue::I64(vec![Some(i64::MAX)]),
        },
        enc,
    ))
    .write("prop_i64_max");

    p0().add_prop(DecodedProp::new(
        DecodedProperty {
            name: "bignum".to_string(),
            values: PropValue::U64(vec![Some(1_234_567_890_123_456_789)]),
        },
        enc,
    ))
    .write("prop_u64");
    p0().add_prop(DecodedProp::new(
        DecodedProperty {
            name: "bignum".to_string(),
            values: PropValue::U64(vec![Some(0)]),
        },
        enc,
    ))
    .write("prop_u64_min");
    p0().add_prop(DecodedProp::new(
        DecodedProperty {
            name: "bignum".to_string(),
            values: PropValue::U64(vec![Some(u64::MAX)]),
        },
        enc,
    ))
    .write("prop_u64_max");

    #[expect(clippy::approx_constant)]
    p0().add_prop(DecodedProp::new(
        DecodedProperty {
            name: "val".to_string(),
            values: PropValue::F32(vec![Some(3.14)]),
        },
        enc,
    ))
    .write("prop_f32");
    p0().add_prop(DecodedProp::new(
        DecodedProperty {
            name: "val".to_string(),
            values: PropValue::F32(vec![Some(f32::NEG_INFINITY)]),
        },
        enc,
    ))
    .write("prop_f32_neg_inf");
    p0().add_prop(DecodedProp::new(
        DecodedProperty {
            name: "val".to_string(),
            values: PropValue::F32(vec![Some(f32::MIN_POSITIVE)]),
        },
        enc,
    ))
    .write("prop_f32_min_norm");
    p0().add_prop(DecodedProp::new(
        DecodedProperty {
            name: "val".to_string(),
            values: PropValue::F32(vec![Some(0.0)]),
        },
        enc,
    ))
    .write("prop_f32_zero");
    p0().add_prop(DecodedProp::new(
        DecodedProperty {
            name: "val".to_string(),
            values: PropValue::F32(vec![Some(f32::MAX)]),
        },
        enc,
    ))
    .write("prop_f32_max_val");
    p0().add_prop(DecodedProp::new(
        DecodedProperty {
            name: "val".to_string(),
            values: PropValue::F32(vec![Some(f32::INFINITY)]),
        },
        enc,
    ))
    .write("prop_f32_pos_inf");
    p0().add_prop(DecodedProp::new(
        DecodedProperty {
            name: "val".to_string(),
            values: PropValue::F32(vec![Some(f32::NAN)]),
        },
        enc,
    ))
    .write("prop_f32_nan");

    #[expect(clippy::approx_constant)]
    p0().add_prop(DecodedProp::new(
        DecodedProperty {
            name: "val".to_string(),
            values: PropValue::F64(vec![Some(3.141_592_653_589_793)]),
        },
        enc,
    ))
    .write("prop_f64");
    p0().add_prop(DecodedProp::new(
        DecodedProperty {
            name: "val".to_string(),
            values: PropValue::F64(vec![Some(f64::NEG_INFINITY)]),
        },
        enc,
    ))
    .write("prop_f64_neg_inf");
    p0().add_prop(DecodedProp::new(
        DecodedProperty {
            name: "val".to_string(),
            values: PropValue::F64(vec![Some(f64::MIN_POSITIVE)]),
        },
        enc,
    ))
    .write("prop_f64_min_norm");
    p0().add_prop(DecodedProp::new(
        DecodedProperty {
            name: "val".to_string(),
            values: PropValue::F64(vec![Some(-0.0)]),
        },
        enc,
    ))
    .write("prop_f64_neg_zero");
    p0().add_prop(DecodedProp::new(
        DecodedProperty {
            name: "val".to_string(),
            values: PropValue::F64(vec![Some(f64::MAX)]),
        },
        enc,
    ))
    .write("prop_f64_max");
    p0().add_prop(DecodedProp::new(
        DecodedProperty {
            name: "val".to_string(),
            values: PropValue::F64(vec![Some(f64::NAN)]),
        },
        enc,
    ))
    .write("prop_f64_nan");

    p0().add_prop(DecodedProp::new(
        DecodedProperty {
            name: "val".to_string(),
            values: PropValue::Str(vec![Some(String::new())]),
        },
        enc,
    ))
    .write("prop_str_empty");
    p0().add_prop(DecodedProp::new(
        DecodedProperty {
            name: "val".to_string(),
            values: PropValue::Str(vec![Some("42".to_string())]),
        },
        enc,
    ))
    .write("prop_str_ascii");
    p0().add_prop(DecodedProp::new(
        DecodedProperty {
            name: "val".to_string(),
            values: PropValue::Str(vec![Some("Line1\n\t\"quoted\"\\path".to_string())]),
        },
        enc,
    ))
    .write("prop_str_escape");
    p0().add_prop(DecodedProp::new(
        DecodedProperty {
            name: "val".to_string(),
            values: PropValue::Str(vec![Some("München 📍 cafe\u{0301}".to_string())]),
        },
        enc,
    ))
    .write("prop_str_unicode");

    let p1 = || w.geo_varint().geo(P1);
    p1().add_prop(DecodedProp::new(
        DecodedProperty {
            name: "name".to_string(),
            values: PropValue::Str(vec![Some("Test Point".to_string())]),
        },
        enc,
    ))
    .add_prop(DecodedProp::new(
        DecodedProperty {
            name: "active".to_string(),
            values: PropValue::Bool(vec![Some(true)]),
        },
        enc,
    ))
    //FIXME in java
    //.add_prop(DecodedProp::new(
    //    DecodedProperty {
    //        name: "tiny-count".to_string(),
    //        values: PropValue::I8(vec![Some(42)]),
    //    },
    //    enc,
    //))
    //.add_prop(DecodedProp::new(
    //    DecodedProperty {
    //        name: "tiny-count".to_string(),
    //        values: PropValue::U8(vec![Some(100)]),
    //    },
    //    enc,
    //))
    .add_prop(DecodedProp::new(
        DecodedProperty {
            name: "count".to_string(),
            values: PropValue::I32(vec![Some(42)]),
        },
        enc,
    ))
    .add_prop(DecodedProp::new(
        DecodedProperty {
            name: "medium".to_string(),
            values: PropValue::U32(vec![Some(100)]),
        },
        enc,
    ))
    .add_prop(DecodedProp::new(
        DecodedProperty {
            name: "bignum".to_string(),
            values: PropValue::I32(vec![Some(42)]),
        },
        enc,
    ))
    .add_prop(DecodedProp::new(
        DecodedProperty {
            name: "biggest".to_string(),
            values: PropValue::U64(vec![Some(0)]), // FIXME: this should be u64, but java does it it this way
        },
        enc,
    ))
    .add_prop(DecodedProp::new(
        DecodedProperty {
            name: "temp".to_string(),
            values: PropValue::F32(vec![Some(25.5)]),
        },
        enc,
    ))
    .add_prop(DecodedProp::new(
        DecodedProperty {
            name: "precision".to_string(),
            values: PropValue::F64(vec![Some(0.123_456_789)]),
        },
        enc,
    ))
    .write("props_mixed");

    generate_props_i32(w);
    generate_props_u32(w);
    generate_props_u64(w);
    generate_props_str(w);
    generate_shared_dictionaries(w);
}

fn generate_props_i32(w: &SynthWriter) {
    let four_points = || w.geo_varint().meta(E::rle_varint()).geos([P0, P1, P2, P3]);
    let values = || DecodedProperty {
        name: "val".to_string(),
        values: PropValue::I32(vec![Some(42), Some(42), Some(42), Some(42)]),
    };

    four_points()
        .add_prop(DecodedProp::new(
            values(),
            PropertyEncoder::new(O::Present, L::None, P::VarInt),
        ))
        .write("props_i32");
    four_points()
        .add_prop(DecodedProp::new(
            values(),
            PropertyEncoder::new(O::Present, L::Delta, P::VarInt),
        ))
        .write("props_i32_delta");
    four_points()
        .add_prop(DecodedProp::new(
            values(),
            PropertyEncoder::new(O::Present, L::Rle, P::VarInt),
        ))
        .write("props_i32_rle");
    four_points()
        .add_prop(DecodedProp::new(
            values(),
            PropertyEncoder::new(O::Present, L::DeltaRle, P::VarInt),
        ))
        .write("props_i32_delta_rle");
}

fn generate_props_u32(w: &SynthWriter) {
    let four_points = || w.geo_varint().meta(E::rle_varint()).geos([P0, P1, P2, P3]);
    let values = || DecodedProperty {
        name: "val".to_string(),
        values: PropValue::U32(vec![Some(9_000), Some(9_000), Some(9_000), Some(9_000)]),
    };

    four_points()
        .add_prop(DecodedProp::new(
            values(),
            PropertyEncoder::new(O::Present, L::None, P::VarInt),
        ))
        .write("props_u32");
    four_points()
        .add_prop(DecodedProp::new(
            values(),
            PropertyEncoder::new(O::Present, L::Delta, P::VarInt),
        ))
        .write("props_u32_delta");
    four_points()
        .add_prop(DecodedProp::new(
            values(),
            PropertyEncoder::new(O::Present, L::Rle, P::VarInt),
        ))
        .write("props_u32_rle");
    four_points()
        .add_prop(DecodedProp::new(
            values(),
            PropertyEncoder::new(O::Present, L::DeltaRle, P::VarInt),
        ))
        .write("props_u32_delta_rle");
}

fn generate_props_u64(w: &SynthWriter) {
    let four_points = || w.geo_varint().meta(E::rle_varint()).geos([P0, P1, P2, P3]);
    let property = || DecodedProperty {
        name: "val".to_string(),
        values: PropValue::U64(vec![Some(9_000), Some(9_000), Some(9_000), Some(9_000)]),
    };

    four_points()
        .add_prop(DecodedProp::new(
            property(),
            PropertyEncoder::new(O::Present, L::None, P::VarInt),
        ))
        .write("props_u64");
    four_points()
        .add_prop(DecodedProp::new(
            property(),
            PropertyEncoder::new(O::Present, L::Delta, P::VarInt),
        ))
        .write("props_u64_delta");
    four_points()
        .add_prop(DecodedProp::new(
            property(),
            PropertyEncoder::new(O::Present, L::Rle, P::VarInt),
        ))
        .write("props_u64_rle");
    four_points()
        .add_prop(DecodedProp::new(
            property(),
            PropertyEncoder::new(O::Present, L::DeltaRle, P::VarInt),
        ))
        .write("props_u64_delta_rle");
}

fn generate_props_str(w: &SynthWriter) {
    let six_points = || {
        w.geo_varint()
            .meta(E::rle_varint())
            .geos([P1, P2, P3, PH1, PH2, PH3])
    };
    let values = || {
        vec![
            Some("residential_zone_north_sector_1".to_string()),
            Some("commercial_zone_south_sector_2".to_string()),
            Some("industrial_zone_east_sector_3".to_string()),
            Some("park_zone_west_sector_4".to_string()),
            Some("water_zone_north_sector_5".to_string()),
            Some("residential_zone_south_sector_6".to_string()),
        ]
    };

    six_points()
        .add_prop(DecodedProp::new(
            DecodedProperty {
                name: "val".to_string(),
                values: PropValue::Str(values()),
            },
            PropertyEncoder::new(O::Present, L::None, P::VarInt),
        ))
        .write("props_str");
    six_points()
        .add_prop(DecodedProp::new(
            DecodedProperty {
                name: "val".to_string(),
                values: PropValue::Str(values()),
            },
            PropertyEncoder::with_fsst(O::Present, L::None, P::VarInt),
        ))
        .write("props_str_fsst-rust"); // FSST compression output is not byte-for-byte consistent with Java's
}

fn generate_shared_dictionaries(w: &SynthWriter) {
    w.geo_varint()
        .geo(P1)
        .add_prop(DecodedProp::new(
            DecodedProperty {
                name: "name:en".to_string(),
                values: PropValue::Str(vec![Some("A".repeat(30))]),
            },
            PropertyEncoder::new(O::Present, L::None, P::VarInt),
        ))
        .add_prop(DecodedProp::new(
            DecodedProperty {
                name: "name:de".to_string(),
                values: PropValue::Str(vec![Some("A".repeat(30))]),
            },
            PropertyEncoder::new(O::Present, L::None, P::VarInt),
        ))
        .write("props_no_shared_dict");

    // TODO: props_shared_dict and props_shared_dict_fsst need shared dictionary support
}
