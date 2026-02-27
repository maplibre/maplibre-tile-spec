//! Rust synthetic MLT file generator.
//!
//! This generates synthetic MLT files for testing and validation.
//! The goal is to produce byte-for-byte identical output to the Java generator.

mod layer;

use std::fs;
use std::path::Path;

use geo_types::{
    Coord, MultiLineString, MultiPoint, MultiPolygon, Point, coord, line_string, point, polygon,
};
use mlt_core::geojson::Geom32;
use mlt_core::v01::{
    DecodedProperty, Encoder as E, IdEncoder, IdWidth, LogicalEncoder as L, PhysicalEncoder as P,
    PresenceStream as O, PropValue, PropertyEncoder,
};

use crate::layer::{DecodedProp, bool, geo_fastpfor, geo_varint, i32};

const C0: Coord<i32> = coord! { x: 13, y: 42 };
const C1: Coord<i32> = coord! { x: 4, y: 47 };
const C2: Coord<i32> = coord! { x: 12, y: 53 };
const C3: Coord<i32> = coord! { x: 18, y: 45 };
const H1: Coord<i32> = coord! { x: 13, y: 48 };
const H2: Coord<i32> = coord! { x: 12, y: 50 };
const H3: Coord<i32> = coord! { x: 10, y: 49 };

const P0: Point<i32> = Point(C0);
const P1: Point<i32> = Point(C1);
const P2: Point<i32> = Point(C2);
const P3: Point<i32> = Point(C3);

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
    geo_varint().geo(line_string![C1, C2]).write(d, "line");
    geo_varint()
        .geo(polygon![C1, C2, C3, C1])
        .write(d, "polygon");
    geo_fastpfor()
        .geo(polygon![C1, C2, C3, C1])
        .write(d, "polygon_fpf");
    geo_varint()
        .tessellated(polygon![C1, C2, C3, C1])
        .write(d, "polygon_tes");
    geo_fastpfor()
        .tessellated(polygon![C1, C2, C3, C1])
        .write(d, "polygon_fpf_tes");
    geo_varint()
        .parts_ring(E::rle_varint())
        .geo(polygon! { exterior: [C1, C2, C3, C1], interiors: [[H1, H2, H3, H1]] })
        .write(d, "polygon_hole");
    geo_fastpfor()
        .parts_ring(E::rle_fastpfor())
        .geo(polygon! { exterior: [C1, C2, C3, C1], interiors: [[H1, H2, H3, H1]] })
        .write(d, "polygon_hole_fpf");
    geo_varint()
        .rings(E::rle_varint())
        .rings2(E::rle_varint())
        .geo(MultiPolygon(vec![
            polygon![C1, C2, C3, C1],
            polygon![H1, H3, C2, H1],
        ]))
        .write(d, "polygon_multi");
    geo_fastpfor()
        .rings(E::rle_fastpfor())
        .rings2(E::rle_fastpfor())
        .geo(MultiPolygon(vec![
            polygon![C1, C2, C3, C1],
            polygon![H1, H3, C2, H1],
        ]))
        .write(d, "polygon_multi_fpf");
    geo_varint()
        .geo(MultiPoint(vec![P1, P2, P3]))
        .write(d, "multipoint");
    geo_varint()
        .geo(MultiLineString(vec![
            line_string![C1, C2],
            line_string![H1, H2, H3],
        ]))
        .write(d, "multiline");
}

fn generate_mixed(d: &Path) {
    let line1 = || line_string![C1, C2];
    let line2 = || line_string![H1, H2, H3];
    let pol1 = || polygon![C1, C2, C3, C1];
    let pol2 = || polygon![H1, H3, H2, H1];
    let types: Vec<(&str, Geom32)> = vec![
        ("pt", P0.into()),
        ("line", line1().into()),
        ("poly", pol1().into()),
        ("mpoint", MultiPoint(vec![P1, P2, P3]).into()),
        ("mline", MultiLineString(vec![line1(), line2()]).into()),
        ("mpoly", MultiPolygon(vec![pol1(), pol2()]).into()),
    ];

    // FIXME: this is a bug, all of these panic
    let non_working = vec![
        "mixed_pt_mpoint",
        "mixed_line_pt_line",
        "mixed_line_mpoly",
        "mixed_poly_pt_poly",
        "mixed_poly_line_poly",
        "mixed_poly_mpoint_poly",
        "mixed_poly_mline",
        "mixed_mpoint_pt",
        "mixed_mpoint_mline",
        "mixed_mpoint_mpoly",
        "mixed_mline_mpoint_mline",
        "mixed_mline_mpoly",
        "mixed_mpoly_mpoint_mpoly",
        "mixed_mpoly_mline",
    ];

    for (n1, geo1) in &types {
        for (n2, geo2) in &types {
            let name = format!("mixed_{n1}_{n2}");
            // FIXME: remove this
            if non_working.contains(&name.as_str()) {
                continue;
            }
            geo_varint()
                .geo(geo1.clone())
                .geo(geo2.clone())
                .write(d, &name);
            if n1 != n2 {
                let name = format!("{name}_{n1}");
                // FIXME: remove this
                if non_working.contains(&name.as_str()) {
                    continue;
                }
                geo_varint()
                    .geo(geo1.clone())
                    .geo(geo2.clone())
                    .geo(geo1.clone())
                    .write(d, name);
            }
        }
    }

    // FIXME: panics
    //let geos = types.iter().map(|(_, geo)| geo.clone()).collect::<Vec<_>>();
    //geo_varint()
    //  .parts(E::rle_varint())
    //.rings(E::rle_varint())
    //.geos(geos)
    //.write(d, "mixed_all");
}

fn generate_extent(d: &Path) {
    geo_varint()
        .extent(512)
        .geo(line_string![
            coord! { x: 0, y: 0 },
            coord! { x: 511, y: 511 }
        ])
        .write(d, "extent_512");
    geo_varint()
        .extent(512)
        .geo(line_string![
            coord! { x: -42, y: -42 },
            coord! { x: 554, y: 554 }
        ])
        .write(d, "extent_buf_512");
    geo_varint()
        .extent(4096)
        .geo(line_string![
            coord! { x: 0, y: 0 },
            coord! { x: 4095, y: 4095 }
        ])
        .write(d, "extent_4096");
    geo_varint()
        .extent(4096)
        .geo(line_string![
            coord! { x: -42, y: -42 },
            coord! { x: 4138, y: 4138 }
        ])
        .write(d, "extent_buf_4096");
    geo_varint()
        .extent(131_072)
        .geo(line_string![
            coord! { x: 0, y: 0 },
            coord! { x: 131_071, y: 131_071 },
        ])
        .write(d, "extent_131072");
    geo_varint()
        .extent(131_072)
        .geo(line_string![
            coord! { x: -42, y: -42 },
            coord! { x: 131_114, y: 131_114 },
        ])
        .write(d, "extent_buf_131072");
    geo_varint()
        .extent(1_073_741_824)
        .geo(line_string![
            coord! { x: 0, y: 0 },
            coord! { x: 1_073_741_823, y: 1_073_741_823 },
        ])
        .write(d, "extent_1073741824");
    geo_varint()
        .extent(1_073_741_824)
        .geo(line_string![
            coord! { x: -42, y: -42 },
            coord! { x: 1_073_741_866, y: 1_073_741_866 },
        ])
        .write(d, "extent_buf_1073741824");
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
            name: "count".to_string(),
            values: PropValue::I32(vec![Some(42)]),
        },
        enc,
    ))
    .add_prop(bool("active", enc).add(true))
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
    let four_points = || {
        geo_varint()
            .meta(E::rle_varint())
            .geos([P1, P2, P3, point!(H1)])
    };
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
            .geos([P1, P2, P3, point!(H1), point!(H2), point!(H3)])
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
