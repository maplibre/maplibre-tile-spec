//! Rust synthetic MLT file generator.
//!
//! This generates synthetic MLT files for testing and validation.
//! The goal is to produce byte-for-byte identical output to the Java generator.

mod layer;

use std::fs;
use std::path::Path;

use geo::Polygon;
use geo_types::{
    Coord, LineString, MultiLineString, MultiPoint, MultiPolygon, Point, coord, line_string, point,
    polygon,
};
use mlt_core::geojson::Geom32;
use mlt_core::v01::{
    DecodedProperty, Encoder as E, IdEncoder, IdWidth, LogicalEncoder as L, PhysicalEncoder as P,
    PresenceStream as O, PropValue, PropertyEncoder,
};

use crate::layer::{DecodedProp, bool, geo_fastpfor, geo_varint, i32};

const C0: Coord<i32> = c(13, 42);
// triangle 1, clockwise winding, X ends in 1, Y ends in 2
const C1: Coord<i32> = c(11, 52);
const C2: Coord<i32> = c(71, 72);
const C3: Coord<i32> = c(61, 22);
// triangle 2, clockwise winding, X ends in 3, Y ends in 4
const C21: Coord<i32> = c(23, 34);
const C22: Coord<i32> = c(73, 4);
const C23: Coord<i32> = c(13, 24);
// hole in triangle 1 with counter-clockwise winding
const H1: Coord<i32> = c(65, 66);
const H2: Coord<i32> = c(35, 56);
const H3: Coord<i32> = c(55, 36);
const P0: Point<i32> = Point(C0);
const P1: Point<i32> = Point(C1);
const P2: Point<i32> = Point(C2);
const P3: Point<i32> = Point(C3);
// holes as points with same coordinates as the hole vertices
const PH1: Point<i32> = Point(H1);
const PH2: Point<i32> = Point(H2);
const PH3: Point<i32> = Point(H3);

fn line1() -> LineString<i32> {
    line_string![C1, C2, C3]
}

fn line2() -> LineString<i32> {
    line_string![C21, C22, C23]
}

fn poly1() -> Polygon<i32> {
    polygon![C1, C2, C3, C1]
}

fn poly2() -> Polygon<i32> {
    polygon![C21, C22, C23, C21]
}

fn poly1h() -> Polygon<i32> {
    polygon! { exterior: [C1, C2, C3, C1], interiors: [[H1, H2, H3, H1]] }
}

const fn c(x: i32, y: i32) -> Coord<i32> {
    coord! {x:x,y:y}
}

const fn p(x: i32, y: i32) -> Point<i32> {
    Point(c(x, y))
}

fn main() {
    let dir = Path::new("../test/synthetic/0x01-rust/");
    fs::create_dir_all(dir).unwrap_or_else(|_| panic!("to be able to create {}", dir.display()));

    let dir = dir
        .canonicalize()
        .unwrap_or_else(|_| panic!("bad path {}", dir.display()));
    println!("Generating synthetic test data in {}", dir.display());

    generate_geometry(&dir);
    generate_mixed(&dir);
    generate_extent(&dir);
    generate_ids(&dir);
    generate_properties(&dir);
}

fn generate_geometry(d: &Path) {
    geo_varint().geo(P0).write(d, "point");
    geo_varint().geo(line1()).write(d, "line");
    geo_varint().geo(poly1()).write(d, "polygon");
    geo_fastpfor().geo(poly1()).write(d, "polygon_fpf");
    geo_varint().tessellated(poly1()).write(d, "polygon_tes");
    geo_fastpfor()
        .tessellated(poly1())
        .write(d, "polygon_fpf_tes");
    geo_varint()
        .parts_ring(E::rle_varint())
        .geo(poly1h())
        .write(d, "polygon_hole");
    geo_fastpfor()
        .parts_ring(E::rle_fastpfor())
        .geo(poly1h())
        .write(d, "polygon_hole_fpf");
    geo_varint()
        .rings(E::rle_varint())
        .rings2(E::rle_varint())
        .geo(MultiPolygon(vec![poly1(), poly2()]))
        .write(d, "polygon_multi");
    geo_fastpfor()
        .rings(E::rle_fastpfor())
        .rings2(E::rle_fastpfor())
        .geo(MultiPolygon(vec![poly1(), poly2()]))
        .write(d, "polygon_multi_fpf");
    geo_varint()
        .geo(MultiPoint(vec![P1, P2, P3]))
        .write(d, "multipoint");
    geo_varint()
        .geo(MultiLineString(vec![line1(), line2()]))
        .write(d, "multiline");
}

fn generate_mixed(d: &Path) {
    let types: Vec<(&str, Geom32)> = vec![
        ("pt", p(38, 29).into()),
        ("line", line_string![c(5, 38), c(12, 45), c(9, 70)].into()),
        (
            "poly",
            polygon![c(55, 5), c(58, 28), c(75, 22), c(55, 5)].into(),
        ),
        (
            "polyh",
            polygon! {
                exterior: [c(52, 35), c(14, 55), c(60, 72), c(52, 35)],
                interiors: [[c(32, 50), c(36, 60), c(24, 54), c(32, 50)]]
            }
            .into(),
        ),
        (
            "mpoint",
            MultiPoint(vec![p(6, 25), p(21, 41), p(23, 69)]).into(),
        ),
        (
            "mline",
            MultiLineString(vec![
                line_string![c(24, 10), c(42, 18)],
                line_string![c(30, 36), c(48, 52), c(35, 62)],
            ])
            .into(),
        ),
        (
            "mpoly",
            MultiPolygon(vec![
                polygon! {
                    exterior: [c(7, 20), c(21, 31), c(26, 9), c(7, 20)],
                    interiors: [[c(15, 20), c(20, 15), c(18, 25), c(15, 20)]]
                }
                .into(),
                polygon![c(69, 57), c(71, 66), c(73, 64), c(69, 57)],
            ])
            .into(),
        ),
    ];

    for k in 2..=types.len() {
        generate_combinations(&types, k, 0, &mut Vec::new(), d);
    }
}

const NON_WORKING_GEOS: &[&str] = &[
    "mixed_2_pt_polyh",
    "mixed_2_line_mpoly",
    "mixed_2_line_polyh",
    "mixed_2_poly_mline",
    "mixed_2_poly_mline",
    "mixed_2_poly_mline",
    "mixed_2_polyh_mline",
    "mixed_2_mpoint_mline",
    "mixed_2_mpoint_mpoly",
    "mixed_2_mline_mpoly",
    "mixed_3_pt_line_polyh",
    "mixed_3_pt_line_mpoly",
    "mixed_3_pt_poly_mline",
    "mixed_3_pt_poly_polyh",
    "mixed_3_pt_mpoint_mline",
    "mixed_3_pt_mpoint_mpoly",
    "mixed_3_pt_mline_mpoly",
    "mixed_3_pt_polyh_mline",
    "mixed_3_line_poly_mline",
    "mixed_3_line_mpoint_mline",
    "mixed_3_line_mpoint_mpoly",
    "mixed_3_line_mline_mpoly",
    "mixed_3_poly_mpoint_mline",
    "mixed_3_poly_mpoint_mpoly",
    "mixed_3_line_poly_polyh",
    "mixed_3_line_polyh_mline",
    "mixed_3_poly_mline_mpoly",
    "mixed_3_poly_polyh_mline",
    "mixed_3_polyh_mpoint_mline",
    "mixed_3_polyh_mpoint_mpoly",
    "mixed_3_polyh_mline_mpoly",
    "mixed_3_mpoint_mline_mpoly",
    "mixed_4_pt_line_poly_mline",
    "mixed_4_pt_line_poly_polyh",
    "mixed_4_pt_line_polyh_mline",
    "mixed_4_pt_line_mpoint_mline",
    "mixed_4_pt_line_mpoint_mpoly",
    "mixed_4_pt_line_mline_mpoly",
    "mixed_4_pt_poly_mpoint_mline",
    "mixed_4_pt_poly_mpoint_mpoly",
    "mixed_4_pt_poly_mline_mpoly",
    "mixed_4_pt_poly_polyh_mline",
    "mixed_4_pt_polyh_mpoint_mline",
    "mixed_4_pt_polyh_mpoint_mpoly",
    "mixed_4_pt_polyh_mline_mpoly",
    "mixed_4_pt_mpoint_mline_mpoly",
    "mixed_4_line_poly_mpoint_mline",
    "mixed_4_line_poly_mpoint_mpoly",
    "mixed_4_line_poly_mline_mpoly",
    "mixed_4_line_poly_polyh_mline",
    "mixed_4_line_polyh_mpoint_mline",
    "mixed_4_line_polyh_mpoint_mpoly",
    "mixed_4_line_polyh_mline_mpoly",
    "mixed_4_line_mpoint_mline_mpoly",
    "mixed_4_poly_mpoint_mline_mpoly",
    "mixed_4_poly_polyh_mpoint_mline",
    "mixed_4_poly_polyh_mpoint_mpoly",
    "mixed_4_poly_polyh_mline_mpoly",
    "mixed_4_polyh_mpoint_mline_mpoly",
    "mixed_5_pt_line_poly_polyh_mline",
    "mixed_5_pt_line_polyh_mpoint_mline",
    "mixed_5_pt_line_polyh_mpoint_mpoly",
    "mixed_5_pt_poly_polyh_mpoint_mline",
    "mixed_5_pt_poly_polyh_mpoint_mpoly",
    "mixed_5_pt_poly_polyh_mline_mpoly",
    "mixed_5_pt_polyh_mpoint_mline_mpoly",
    "mixed_5_pt_line_poly_mpoint_mline",
    "mixed_5_pt_line_poly_mpoint_mpoly",
    "mixed_5_pt_line_mpoint_mline_mpoly",
    "mixed_5_pt_poly_mpoint_mline_mpoly",
    "mixed_5_line_poly_mpoint_mline_mpoly",
    "mixed_5_line_poly_polyh_mpoint_mline",
    "mixed_5_line_poly_polyh_mpoint_mpoly",
    "mixed_5_line_poly_polyh_mline_mpoly",
    "mixed_5_line_poly_mpoint_mline_mpoly",
    "mixed_5_line_polyh_mpoint_mline_mpoly",
    "mixed_5_poly_polyh_mpoint_mline_mpoly",
    "mixed_6_pt_line_poly_polyh_mpoint_mline",
    "mixed_6_pt_line_poly_polyh_mpoint_mpoly",
    "mixed_6_pt_line_poly_mpoint_mline_mpoly",
    "mixed_6_pt_line_polyh_mpoint_mline_mpoly",
    "mixed_6_pt_poly_polyh_mpoint_mline_mpoly",
    "mixed_6_line_poly_polyh_mpoint_mline_mpoly",
    "mixed_7_pt_line_poly_polyh_mpoint_mline_mpoly",
];

fn generate_combinations(
    types: &[(&str, Geom32)],
    k: usize,
    start: usize,
    current: &mut Vec<usize>,
    d: &Path,
) {
    if current.len() == k {
        let name = format!(
            "mixed_{}_{}",
            k,
            current
                .iter()
                .map(|&i| types[i].0)
                .collect::<Vec<_>>()
                .join("_")
        );
        // FIXME: Remove NON_WORKING_GEOS
        if !NON_WORKING_GEOS.contains(&name.as_str()) {
            let mut builder = geo_varint();
            for i in current {
                builder = builder.geo(types[*i].1.clone());
            }
            builder.write(d, &name);
            return;
        }
    }

    for i in start..types.len() {
        current.push(i);
        generate_combinations(types, k, i + 1, current, d);
        current.pop();
    }
}

fn generate_extent(d: &Path) {
    for e in [512_i32, 4096, 131072, 1073741824] {
        geo_varint()
            .extent(e as u32)
            .geo(line_string![
                coord! { x: 0_i32, y: 0 },
                coord! { x: e - 1, y: e - 1 }
            ])
            .write(d, format!("extent_{e}"));
        geo_varint()
            .extent(e as u32)
            .geo(line_string![
                coord! { x: -42_i32, y: -42 },
                coord! { x: e + 42, y: e + 42 }
            ])
            .write(d, format!("extent_buf_{e}"));
    }
}

fn generate_ids(d: &Path) {
    let p0 = || geo_varint().geo(P0);
    p0().ids(vec![Some(0)], IdEncoder::new(L::None, IdWidth::Id32))
        .write(d, "id0");
    p0().ids(vec![Some(100)], IdEncoder::new(L::None, IdWidth::Id32))
        .write(d, "id");
    p0().ids(
        vec![Some(9_234_567_890)],
        IdEncoder::new(L::None, IdWidth::Id64),
    )
    .write(d, "id64");

    let four_p0 = || geo_varint().meta(E::rle_varint()).geos([P0, P0, P0, P0]);
    four_p0()
        .ids(
            vec![Some(103), Some(103), Some(103), Some(103)],
            IdEncoder::new(L::None, IdWidth::Id32),
        )
        .write(d, "ids");
    four_p0()
        .ids(
            vec![Some(103), Some(103), Some(103), Some(103)],
            IdEncoder::new(L::Delta, IdWidth::Id32),
        )
        .write(d, "ids_delta");
    four_p0()
        .ids(
            vec![Some(103), Some(103), Some(103), Some(103)],
            IdEncoder::new(L::Rle, IdWidth::Id32),
        )
        .write(d, "ids_rle");
    four_p0()
        .ids(
            vec![Some(103), Some(103), Some(103), Some(103)],
            IdEncoder::new(L::DeltaRle, IdWidth::Id32),
        )
        .write(d, "ids_delta_rle");
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
        .write(d, "ids64");
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
        .write(d, "ids64_delta");
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
        .write(d, "ids64_rle");
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
        .write(d, "ids64_delta_rle");

    let five_p0 = || {
        geo_varint()
            .meta(E::rle_varint())
            .geos([P0, P0, P0, P0, P0])
    };
    five_p0()
        .ids(
            vec![Some(100), Some(101), None, Some(105), Some(106)],
            IdEncoder::new(L::None, IdWidth::OptId32),
        )
        .write(d, "ids_opt");
    five_p0()
        .ids(
            vec![Some(100), Some(101), None, Some(105), Some(106)],
            IdEncoder::new(L::Delta, IdWidth::OptId32),
        )
        .write(d, "ids_opt_delta");
    five_p0()
        .ids(
            vec![None, Some(9_234_567_890), Some(101), Some(105), Some(106)],
            IdEncoder::new(L::None, IdWidth::OptId64),
        )
        .write(d, "ids64_opt");
    five_p0()
        .ids(
            vec![None, Some(9_234_567_890), Some(101), Some(105), Some(106)],
            IdEncoder::new(L::Delta, IdWidth::OptId64),
        )
        .write(d, "ids64_opt_delta");
}

fn generate_properties(d: &Path) {
    let p0 = || geo_varint().geo(P0);
    let enc = PropertyEncoder::new(O::Present, L::None, P::VarInt);

    p0().add_prop(bool("val", enc).add(true))
        .write(d, "prop_bool");
    p0().add_prop(bool("val", enc).add(false))
        .write(d, "prop_bool_false");

    p0().add_prop(i32("val", enc).add(42)).write(d, "prop_i32");
    p0().add_prop(DecodedProp::new(
        DecodedProperty {
            name: "val".to_string(),
            values: PropValue::I32(vec![Some(-42)]),
        },
        enc,
    ))
    .write(d, "prop_i32_neg");
    p0().add_prop(DecodedProp::new(
        DecodedProperty {
            name: "val".to_string(),
            values: PropValue::I32(vec![Some(i32::MIN)]),
        },
        enc,
    ))
    .write(d, "prop_i32_min");
    p0().add_prop(DecodedProp::new(
        DecodedProperty {
            name: "val".to_string(),
            values: PropValue::I32(vec![Some(i32::MAX)]),
        },
        enc,
    ))
    .write(d, "prop_i32_max");

    p0().add_prop(DecodedProp::new(
        DecodedProperty {
            name: "val".to_string(),
            values: PropValue::U32(vec![Some(42)]),
        },
        enc,
    ))
    .write(d, "prop_u32");
    p0().add_prop(DecodedProp::new(
        DecodedProperty {
            name: "val".to_string(),
            values: PropValue::U32(vec![Some(0)]),
        },
        enc,
    ))
    .write(d, "prop_u32_min");
    p0().add_prop(DecodedProp::new(
        DecodedProperty {
            name: "val".to_string(),
            values: PropValue::U32(vec![Some(u32::MAX)]),
        },
        enc,
    ))
    .write(d, "prop_u32_max");

    p0().add_prop(DecodedProp::new(
        DecodedProperty {
            name: "val".to_string(),
            values: PropValue::I64(vec![Some(9_876_543_210)]),
        },
        enc,
    ))
    .write(d, "prop_i64");
    p0().add_prop(DecodedProp::new(
        DecodedProperty {
            name: "val".to_string(),
            values: PropValue::I64(vec![Some(-9_876_543_210)]),
        },
        enc,
    ))
    .write(d, "prop_i64_neg");
    p0().add_prop(DecodedProp::new(
        DecodedProperty {
            name: "val".to_string(),
            values: PropValue::I64(vec![Some(i64::MIN)]),
        },
        enc,
    ))
    .write(d, "prop_i64_min");
    p0().add_prop(DecodedProp::new(
        DecodedProperty {
            name: "val".to_string(),
            values: PropValue::I64(vec![Some(i64::MAX)]),
        },
        enc,
    ))
    .write(d, "prop_i64_max");

    p0().add_prop(DecodedProp::new(
        DecodedProperty {
            name: "bignum".to_string(),
            values: PropValue::U64(vec![Some(1_234_567_890_123_456_789)]),
        },
        enc,
    ))
    .write(d, "prop_u64");
    p0().add_prop(DecodedProp::new(
        DecodedProperty {
            name: "bignum".to_string(),
            values: PropValue::U64(vec![Some(0)]),
        },
        enc,
    ))
    .write(d, "prop_u64_min");
    p0().add_prop(DecodedProp::new(
        DecodedProperty {
            name: "bignum".to_string(),
            values: PropValue::U64(vec![Some(u64::MAX)]),
        },
        enc,
    ))
    .write(d, "prop_u64_max");

    #[expect(clippy::approx_constant)]
    p0().add_prop(DecodedProp::new(
        DecodedProperty {
            name: "val".to_string(),
            values: PropValue::F32(vec![Some(3.14)]),
        },
        enc,
    ))
    .write(d, "prop_f32");
    p0().add_prop(DecodedProp::new(
        DecodedProperty {
            name: "val".to_string(),
            values: PropValue::F32(vec![Some(f32::NEG_INFINITY)]),
        },
        enc,
    ))
    .write(d, "prop_f32_neg_inf");
    p0().add_prop(DecodedProp::new(
        DecodedProperty {
            name: "val".to_string(),
            values: PropValue::F32(vec![Some(f32::from_bits(1))]),
        },
        enc,
    ))
    .write(d, "prop_f32_min");
    p0().add_prop(DecodedProp::new(
        DecodedProperty {
            name: "val".to_string(),
            values: PropValue::F32(vec![Some(0.0)]),
        },
        enc,
    ))
    .write(d, "prop_f32_zero");
    p0().add_prop(DecodedProp::new(
        DecodedProperty {
            name: "val".to_string(),
            values: PropValue::F32(vec![Some(f32::MAX)]),
        },
        enc,
    ))
    .write(d, "prop_f32_max");
    p0().add_prop(DecodedProp::new(
        DecodedProperty {
            name: "val".to_string(),
            values: PropValue::F32(vec![Some(f32::INFINITY)]),
        },
        enc,
    ))
    .write(d, "prop_f32_pos_inf");
    p0().add_prop(DecodedProp::new(
        DecodedProperty {
            name: "val".to_string(),
            values: PropValue::F32(vec![Some(f32::NAN)]),
        },
        enc,
    ))
    .write(d, "prop_f32_nan");

    #[expect(clippy::approx_constant)]
    p0().add_prop(DecodedProp::new(
        DecodedProperty {
            name: "val".to_string(),
            values: PropValue::F64(vec![Some(3.141_592_653_589_793)]),
        },
        enc,
    ))
    .write(d, "prop_f64");
    p0().add_prop(DecodedProp::new(
        DecodedProperty {
            name: "val".to_string(),
            values: PropValue::F64(vec![Some(f64::NEG_INFINITY)]),
        },
        enc,
    ))
    .write(d, "prop_f64_neg_inf");
    p0().add_prop(DecodedProp::new(
        DecodedProperty {
            name: "val".to_string(),
            values: PropValue::F64(vec![Some(f64::MIN_POSITIVE)]),
        },
        enc,
    ))
    .write(d, "prop_f64_min");
    p0().add_prop(DecodedProp::new(
        DecodedProperty {
            name: "val".to_string(),
            values: PropValue::F64(vec![Some(-0.0)]),
        },
        enc,
    ))
    .write(d, "prop_f64_neg_zero");
    p0().add_prop(DecodedProp::new(
        DecodedProperty {
            name: "val".to_string(),
            values: PropValue::F64(vec![Some(f64::MAX)]),
        },
        enc,
    ))
    .write(d, "prop_f64_max");
    p0().add_prop(DecodedProp::new(
        DecodedProperty {
            name: "val".to_string(),
            values: PropValue::F64(vec![Some(f64::NAN)]),
        },
        enc,
    ))
    .write(d, "prop_f64_nan");

    p0().add_prop(DecodedProp::new(
        DecodedProperty {
            name: "val".to_string(),
            values: PropValue::Str(vec![Some(String::new())]),
        },
        enc,
    ))
    .write(d, "prop_str_empty");
    p0().add_prop(DecodedProp::new(
        DecodedProperty {
            name: "val".to_string(),
            values: PropValue::Str(vec![Some("42".to_string())]),
        },
        enc,
    ))
    .write(d, "prop_str_ascii");
    p0().add_prop(DecodedProp::new(
        DecodedProperty {
            name: "val".to_string(),
            values: PropValue::Str(vec![Some("Line1\n\t\"quoted\"\\path".to_string())]),
        },
        enc,
    ))
    .write(d, "prop_str_escape");
    p0().add_prop(DecodedProp::new(
        DecodedProperty {
            name: "val".to_string(),
            values: PropValue::Str(vec![Some("München 📍 cafe\u{0301}".to_string())]),
        },
        enc,
    ))
    .write(d, "prop_str_unicode");

    let p1 = || geo_varint().geo(P1);
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
            values: PropValue::I64(vec![Some(42)]),
        },
        enc,
    ))
    .add_prop(DecodedProp::new(
        DecodedProperty {
            name: "biggest".to_string(),
            values: PropValue::U32(vec![Some(0)]), // FIXME: this should be u64, but java does it it this way
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
    .write(d, "props_mixed");

    generate_props_i32(d);
    generate_props_u32(d);
    generate_props_u64(d);
    generate_props_str(d);
    generate_shared_dictionaries(d);
}

fn generate_props_i32(d: &Path) {
    let four_points = || geo_varint().meta(E::rle_varint()).geos([P0, P1, P2, P3]);
    let values = || DecodedProperty {
        name: "val".to_string(),
        values: PropValue::I32(vec![Some(42), Some(42), Some(42), Some(42)]),
    };

    four_points()
        .add_prop(DecodedProp::new(
            values(),
            PropertyEncoder::new(O::Present, L::None, P::VarInt),
        ))
        .write(d, "props_i32");
    four_points()
        .add_prop(DecodedProp::new(
            values(),
            PropertyEncoder::new(O::Present, L::Delta, P::VarInt),
        ))
        .write(d, "props_i32_delta");
    four_points()
        .add_prop(DecodedProp::new(
            values(),
            PropertyEncoder::new(O::Present, L::Rle, P::VarInt),
        ))
        .write(d, "props_i32_rle");
    four_points()
        .add_prop(DecodedProp::new(
            values(),
            PropertyEncoder::new(O::Present, L::DeltaRle, P::VarInt),
        ))
        .write(d, "props_i32_delta_rle");
}

fn generate_props_u32(d: &Path) {
    let four_points = || geo_varint().meta(E::rle_varint()).geos([P0, P1, P2, P3]);
    let values = || DecodedProperty {
        name: "val".to_string(),
        values: PropValue::U32(vec![Some(9_000), Some(9_000), Some(9_000), Some(9_000)]),
    };

    four_points()
        .add_prop(DecodedProp::new(
            values(),
            PropertyEncoder::new(O::Present, L::None, P::VarInt),
        ))
        .write(d, "props_u32");
    four_points()
        .add_prop(DecodedProp::new(
            values(),
            PropertyEncoder::new(O::Present, L::Delta, P::VarInt),
        ))
        .write(d, "props_u32_delta");
    four_points()
        .add_prop(DecodedProp::new(
            values(),
            PropertyEncoder::new(O::Present, L::Rle, P::VarInt),
        ))
        .write(d, "props_u32_rle");
    four_points()
        .add_prop(DecodedProp::new(
            values(),
            PropertyEncoder::new(O::Present, L::DeltaRle, P::VarInt),
        ))
        .write(d, "props_u32_delta_rle");
}

fn generate_props_u64(d: &Path) {
    let four_points = || geo_varint().meta(E::rle_varint()).geos([P0, P1, P2, P3]);
    let property = || DecodedProperty {
        name: "val".to_string(),
        values: PropValue::U64(vec![Some(9_000), Some(9_000), Some(9_000), Some(9_000)]),
    };

    four_points()
        .add_prop(DecodedProp::new(
            property(),
            PropertyEncoder::new(O::Present, L::None, P::VarInt),
        ))
        .write(d, "props_u64");
    four_points()
        .add_prop(DecodedProp::new(
            property(),
            PropertyEncoder::new(O::Present, L::Delta, P::VarInt),
        ))
        .write(d, "props_u64_delta");
    four_points()
        .add_prop(DecodedProp::new(
            property(),
            PropertyEncoder::new(O::Present, L::Rle, P::VarInt),
        ))
        .write(d, "props_u64_rle");
    four_points()
        .add_prop(DecodedProp::new(
            property(),
            PropertyEncoder::new(O::Present, L::DeltaRle, P::VarInt),
        ))
        .write(d, "props_u64_delta_rle");
}

fn generate_props_str(d: &Path) {
    let six_points = || {
        geo_varint()
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
        .write(d, "props_str");
    six_points()
        .add_prop(DecodedProp::new(
            DecodedProperty {
                name: "val".to_string(),
                values: PropValue::Str(values()),
            },
            PropertyEncoder::with_fsst(O::Present, L::None, P::VarInt),
        ))
        .write(d, "props_str_fsst-rust"); // FSST compression output is not byte-for-byte consistent with Java's
}

fn generate_shared_dictionaries(d: &Path) {
    geo_varint()
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
        .write(d, "props_no_shared_dict");

    // TODO: props_shared_dict and props_shared_dict_fsst need shared dictionary support
}
