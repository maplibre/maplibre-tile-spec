//! Rust synthetic MLT file generator.
//!
//! This generates synthetic MLT files for testing and validation.
//! The goal is to produce byte-for-byte identical output to the Java generator.
//!
//! # Current Status
//!
//! ## Files that MATCH Java output (byte-for-byte identical):
//! - All basic geometries, extents, IDs, and properties
//! - `FastPFOR` variants: `polygon_fpf`, `polygon_hole_fpf`, `polygon_multi_fpf`
//! - Mixed geometries, tessellation variants
//!
//! ## Files with DIFFERENT output (semantically equivalent, different FSST algorithm):
//! - `props_str_fsst`: Uses `fsst-rs` crate which may produce different symbol tables
//!   than Java's FSST implementation, but both decode to the same strings.
//!
//! ## Files NOT YET GENERATED:
//! - Shared dictionary: `props_shared_dict`, `props_shared_dict_fsst` (shared dict encoding not implemented)

mod layer;

use std::fs;
use std::path::Path;

use geo_types::{Coord, LineString, MultiLineString, MultiPoint, MultiPolygon, Point, Polygon};
use mlt_core::v01::{
    DecodedProperty, Encoder, IdEncoder, IdWidth, LogicalEncoder, PropValue, PropertyEncoder,
};

use crate::layer::{DecodedProp, bool, geo_fastpfor, geo_varint, i32};

// Common coordinates matching Java synthetics
const C0: Coord<i32> = Coord { x: 13, y: 42 };
const C1: Coord<i32> = Coord { x: 4, y: 47 };
const C2: Coord<i32> = Coord { x: 12, y: 53 };
const C3: Coord<i32> = Coord { x: 18, y: 45 };
const H1: Coord<i32> = Coord { x: 13, y: 48 };
const H2: Coord<i32> = Coord { x: 12, y: 50 };
const H3: Coord<i32> = Coord { x: 10, y: 49 };

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
    use Encoder as E;

    let pol = || Polygon::new(LineString(vec![C1, C2, C3, C1]), vec![]);
    let pol_hole = || {
        Polygon::new(
            LineString(vec![C1, C2, C3, C1]),
            vec![LineString(vec![H1, H2, H3, H1])],
        )
    };
    let mpol = || {
        MultiPolygon(vec![
            Polygon::new(LineString(vec![C1, C2, C3, C1]), vec![]),
            Polygon::new(LineString(vec![H1, H3, C2, H1]), vec![]),
        ])
    };

    geo_varint().geo(Point(C0)).write(d, "point");
    geo_varint().geo(LineString(vec![C1, C2])).write(d, "line");
    geo_varint().geo(pol()).write(d, "polygon");
    geo_fastpfor().geo(pol()).write(d, "polygon_fpf");
    geo_varint().tessellated(pol()).write(d, "polygon_tes");
    geo_fastpfor()
        .tessellated(pol())
        .write(d, "polygon_morton_tes");
    geo_varint()
        .parts_ring(E::rle_varint())
        .geo(pol_hole())
        .write(d, "polygon_hole");
    geo_fastpfor()
        .parts_ring(E::rle_fastpfor())
        .geo(pol_hole())
        .write(d, "polygon_hole_fpf");
    geo_varint()
        .rings(E::rle_varint())
        .rings2(E::rle_varint())
        .geo(mpol())
        .write(d, "polygon_multi");
    geo_fastpfor()
        .rings(E::rle_fastpfor())
        .rings2(E::rle_fastpfor())
        .geo(mpol())
        .write(d, "polygon_multi_fpf");
    geo_varint()
        .geo(MultiPoint(vec![Point(C1), Point(C2), Point(C3)]))
        .write(d, "multipoint");
    geo_varint()
        .geo(MultiLineString(vec![
            LineString(vec![C1, C2]),
            LineString(vec![H1, H2, H3]),
        ]))
        .write(d, "multiline");
}

fn generate_mixed(d: &Path) {
    let line1 = || LineString(vec![C1, C2]);
    let line2 = || LineString(vec![H1, H2, H3]);
    let pol1 = || Polygon::new(LineString(vec![C1, C2, C3, C1]), vec![]);
    let pol2 = || Polygon::new(LineString(vec![H1, H3, H2, H1]), vec![]);

    geo_varint()
        .geo(Point(C0))
        .geo(line1())
        .write(d, "mixed_pt_line");
    geo_varint()
        .geo(Point(C0))
        .geo(pol1())
        .write(d, "mixed_pt_poly");
    geo_varint()
        .geo(line1())
        .geo(pol1())
        .write(d, "mixed_line_poly");
    geo_varint()
        .geo(Point(C0))
        .geo(MultiLineString(vec![line1(), line2()]))
        .write(d, "mixed_pt_mline");
    geo_varint()
        .parts(Encoder::rle_varint())
        .rings(Encoder::rle_varint())
        .geo(Point(C0))
        .geo(line1())
        .geo(pol1())
        .geo(MultiPolygon(vec![pol1(), pol2()]))
        .write(d, "mixed_all");
}

fn generate_extent(d: &Path) {
    geo_varint()
        .extent(512)
        .geo(LineString(vec![
            Coord { x: 0, y: 0 },
            Coord { x: 511, y: 511 },
        ]))
        .write(d, "extent_512");
    geo_varint()
        .extent(512)
        .geo(LineString(vec![
            Coord { x: -42, y: -42 },
            Coord { x: 554, y: 554 },
        ]))
        .write(d, "extent_buf_512");
    geo_varint()
        .extent(4096)
        .geo(LineString(vec![
            Coord { x: 0, y: 0 },
            Coord { x: 4095, y: 4095 },
        ]))
        .write(d, "extent_4096");
    geo_varint()
        .extent(4096)
        .geo(LineString(vec![
            Coord { x: -42, y: -42 },
            Coord { x: 4138, y: 4138 },
        ]))
        .write(d, "extent_buf_4096");
    geo_varint()
        .extent(131_072)
        .geo(LineString(vec![
            Coord { x: 0, y: 0 },
            Coord {
                x: 131_071,
                y: 131_071,
            },
        ]))
        .write(d, "extent_131072");
    geo_varint()
        .extent(131_072)
        .geo(LineString(vec![
            Coord { x: -42, y: -42 },
            Coord {
                x: 131_114,
                y: 131_114,
            },
        ]))
        .write(d, "extent_buf_131072");
    geo_varint()
        .extent(1_073_741_824)
        .geo(LineString(vec![
            Coord { x: 0, y: 0 },
            Coord {
                x: 1_073_741_823,
                y: 1_073_741_823,
            },
        ]))
        .write(d, "extent_1073741824");
    geo_varint()
        .extent(1_073_741_824)
        .geo(LineString(vec![
            Coord { x: -42, y: -42 },
            Coord {
                x: 1_073_741_866,
                y: 1_073_741_866,
            },
        ]))
        .write(d, "extent_buf_1073741824");
}

fn generate_ids(d: &Path) {
    use Encoder as E;

    let p0 = || geo_varint().geo(Point(C0));
    p0().ids(
        vec![Some(0)],
        IdEncoder::new(LogicalEncoder::None, IdWidth::Id32),
    )
    .write(d, "id0");
    p0().ids(
        vec![Some(100)],
        IdEncoder::new(LogicalEncoder::None, IdWidth::Id32),
    )
    .write(d, "id");
    p0().ids(
        vec![Some(9_234_567_890)],
        IdEncoder::new(LogicalEncoder::None, IdWidth::Id64),
    )
    .write(d, "id64");

    let four_p0 = || {
        geo_varint()
            .meta(E::rle_varint())
            .geo(Point(C0))
            .geo(Point(C0))
            .geo(Point(C0))
            .geo(Point(C0))
    };
    four_p0()
        .ids(
            vec![Some(103), Some(103), Some(103), Some(103)],
            IdEncoder::new(LogicalEncoder::None, IdWidth::Id32),
        )
        .write(d, "ids");
    four_p0()
        .ids(
            vec![Some(103), Some(103), Some(103), Some(103)],
            IdEncoder::new(LogicalEncoder::Delta, IdWidth::Id32),
        )
        .write(d, "ids_delta");
    four_p0()
        .ids(
            vec![Some(103), Some(103), Some(103), Some(103)],
            IdEncoder::new(LogicalEncoder::Rle, IdWidth::Id32),
        )
        .write(d, "ids_rle");
    four_p0()
        .ids(
            vec![Some(103), Some(103), Some(103), Some(103)],
            IdEncoder::new(LogicalEncoder::DeltaRle, IdWidth::Id32),
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
            IdEncoder::new(LogicalEncoder::None, IdWidth::Id64),
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
            IdEncoder::new(LogicalEncoder::Delta, IdWidth::Id64),
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
            IdEncoder::new(LogicalEncoder::Rle, IdWidth::Id64),
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
            IdEncoder::new(LogicalEncoder::DeltaRle, IdWidth::Id64),
        )
        .write(d, "ids64_delta_rle");

    let five_p0 = || {
        geo_varint()
            .meta(E::rle_varint())
            .geo(Point(C0))
            .geo(Point(C0))
            .geo(Point(C0))
            .geo(Point(C0))
            .geo(Point(C0))
    };
    five_p0()
        .ids(
            vec![Some(100), Some(101), None, Some(105), Some(106)],
            IdEncoder::new(LogicalEncoder::None, IdWidth::OptId32),
        )
        .write(d, "ids_opt");
    five_p0()
        .ids(
            vec![Some(100), Some(101), None, Some(105), Some(106)],
            IdEncoder::new(LogicalEncoder::Delta, IdWidth::OptId32),
        )
        .write(d, "ids_opt_delta");
    five_p0()
        .ids(
            vec![None, Some(9_234_567_890), Some(101), Some(105), Some(106)],
            IdEncoder::new(LogicalEncoder::None, IdWidth::OptId64),
        )
        .write(d, "ids64_opt");
    five_p0()
        .ids(
            vec![None, Some(9_234_567_890), Some(101), Some(105), Some(106)],
            IdEncoder::new(LogicalEncoder::Delta, IdWidth::OptId64),
        )
        .write(d, "ids64_opt_delta");
}

fn generate_properties(d: &Path) {
    use mlt_core::v01::{LogicalEncoder as L, PhysicalEncoder as P, PresenceStream as O};

    let p0 = || geo_varint().geo(Point(C0));
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

    let p1 = || geo_varint().geo(Point(C1));
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
    use mlt_core::v01::{PhysicalEncoder, PresenceStream};

    let four_p0 = || {
        geo_varint()
            .meta(Encoder::rle_varint())
            .geo(Point(C1))
            .geo(Point(C2))
            .geo(Point(C3))
            .geo(Point(H1))
    };
    let values = vec![Some(42), Some(42), Some(42), Some(42)];

    four_p0()
        .add_prop(DecodedProp::new(
            DecodedProperty {
                name: "val".to_string(),
                values: PropValue::I32(values.clone()),
            },
            PropertyEncoder::new(
                PresenceStream::Present,
                LogicalEncoder::None,
                PhysicalEncoder::VarInt,
            ),
        ))
        .write(d, "props_i32");
    four_p0()
        .add_prop(DecodedProp::new(
            DecodedProperty {
                name: "val".to_string(),
                values: PropValue::I32(values.clone()),
            },
            PropertyEncoder::new(
                PresenceStream::Present,
                LogicalEncoder::Delta,
                PhysicalEncoder::VarInt,
            ),
        ))
        .write(d, "props_i32_delta");
    four_p0()
        .add_prop(DecodedProp::new(
            DecodedProperty {
                name: "val".to_string(),
                values: PropValue::I32(values.clone()),
            },
            PropertyEncoder::new(
                PresenceStream::Present,
                LogicalEncoder::Rle,
                PhysicalEncoder::VarInt,
            ),
        ))
        .write(d, "props_i32_rle");
    four_p0()
        .add_prop(DecodedProp::new(
            DecodedProperty {
                name: "val".to_string(),
                values: PropValue::I32(values),
            },
            PropertyEncoder::new(
                PresenceStream::Present,
                LogicalEncoder::DeltaRle,
                PhysicalEncoder::VarInt,
            ),
        ))
        .write(d, "props_i32_delta_rle");
}

fn generate_props_u32(d: &Path) {
    use mlt_core::v01::{PhysicalEncoder, PresenceStream};

    let four_p0 = || {
        geo_varint()
            .meta(Encoder::rle_varint())
            .geo(Point(C0))
            .geo(Point(C1))
            .geo(Point(C2))
            .geo(Point(C3))
    };
    let values = vec![Some(9_000), Some(9_000), Some(9_000), Some(9_000)];

    four_p0()
        .add_prop(DecodedProp::new(
            DecodedProperty {
                name: "val".to_string(),
                values: PropValue::U32(values.clone()),
            },
            PropertyEncoder::new(
                PresenceStream::Present,
                LogicalEncoder::None,
                PhysicalEncoder::VarInt,
            ),
        ))
        .write(d, "props_u32");
    four_p0()
        .add_prop(DecodedProp::new(
            DecodedProperty {
                name: "val".to_string(),
                values: PropValue::U32(values.clone()),
            },
            PropertyEncoder::new(
                PresenceStream::Present,
                LogicalEncoder::Delta,
                PhysicalEncoder::VarInt,
            ),
        ))
        .write(d, "props_u32_delta");
    four_p0()
        .add_prop(DecodedProp::new(
            DecodedProperty {
                name: "val".to_string(),
                values: PropValue::U32(values.clone()),
            },
            PropertyEncoder::new(
                PresenceStream::Present,
                LogicalEncoder::Rle,
                PhysicalEncoder::VarInt,
            ),
        ))
        .write(d, "props_u32_rle");
    four_p0()
        .add_prop(DecodedProp::new(
            DecodedProperty {
                name: "val".to_string(),
                values: PropValue::U32(values),
            },
            PropertyEncoder::new(
                PresenceStream::Present,
                LogicalEncoder::DeltaRle,
                PhysicalEncoder::VarInt,
            ),
        ))
        .write(d, "props_u32_delta_rle");
}

fn generate_props_u64(d: &Path) {
    use mlt_core::v01::{PhysicalEncoder, PresenceStream};

    let four_p0 = || {
        geo_varint()
            .meta(Encoder::rle_varint())
            .geo(Point(C0))
            .geo(Point(C1))
            .geo(Point(C2))
            .geo(Point(C3))
    };
    let values = vec![Some(9_000), Some(9_000), Some(9_000), Some(9_000)];

    four_p0()
        .add_prop(DecodedProp::new(
            DecodedProperty {
                name: "val".to_string(),
                values: PropValue::U64(values.clone()),
            },
            PropertyEncoder::new(
                PresenceStream::Present,
                LogicalEncoder::None,
                PhysicalEncoder::VarInt,
            ),
        ))
        .write(d, "props_u64");
    four_p0()
        .add_prop(DecodedProp::new(
            DecodedProperty {
                name: "val".to_string(),
                values: PropValue::U64(values.clone()),
            },
            PropertyEncoder::new(
                PresenceStream::Present,
                LogicalEncoder::Delta,
                PhysicalEncoder::VarInt,
            ),
        ))
        .write(d, "props_u64_delta");
    four_p0()
        .add_prop(DecodedProp::new(
            DecodedProperty {
                name: "val".to_string(),
                values: PropValue::U64(values.clone()),
            },
            PropertyEncoder::new(
                PresenceStream::Present,
                LogicalEncoder::Rle,
                PhysicalEncoder::VarInt,
            ),
        ))
        .write(d, "props_u64_rle");
    four_p0()
        .add_prop(DecodedProp::new(
            DecodedProperty {
                name: "val".to_string(),
                values: PropValue::U64(values),
            },
            PropertyEncoder::new(
                PresenceStream::Present,
                LogicalEncoder::DeltaRle,
                PhysicalEncoder::VarInt,
            ),
        ))
        .write(d, "props_u64_delta_rle");
}

fn generate_props_str(d: &Path) {
    use mlt_core::v01::{PhysicalEncoder, PresenceStream};

    let six_p0 = || {
        geo_varint()
            .meta(Encoder::rle_varint())
            .geo(Point(C1))
            .geo(Point(C2))
            .geo(Point(C3))
            .geo(Point(H1))
            .geo(Point(H2))
            .geo(Point(H3))
    };
    let values = vec![
        Some("residential_zone_north_sector_1".to_string()),
        Some("commercial_zone_south_sector_2".to_string()),
        Some("industrial_zone_east_sector_3".to_string()),
        Some("park_zone_west_sector_4".to_string()),
        Some("water_zone_north_sector_5".to_string()),
        Some("residential_zone_south_sector_6".to_string()),
    ];

    six_p0()
        .add_prop(DecodedProp::new(
            DecodedProperty {
                name: "val".to_string(),
                values: PropValue::Str(values.clone()),
            },
            PropertyEncoder::new(
                PresenceStream::Present,
                LogicalEncoder::None,
                PhysicalEncoder::VarInt,
            ),
        ))
        .write(d, "props_str");
    six_p0()
        .add_prop(DecodedProp::new(
            DecodedProperty {
                name: "val".to_string(),
                values: PropValue::Str(values),
            },
            PropertyEncoder::with_fsst(
                PresenceStream::Present,
                LogicalEncoder::None,
                PhysicalEncoder::VarInt,
            ),
        ))
        .write(d, "props_str_fsst");
}

fn generate_shared_dictionaries(d: &Path) {
    use mlt_core::v01::{PhysicalEncoder, PresenceStream};

    let p1 = || geo_varint().geo(Point(C1));
    let enc = PropertyEncoder::new(
        PresenceStream::Present,
        LogicalEncoder::None,
        PhysicalEncoder::VarInt,
    );
    let val = "A".repeat(30);

    p1().add_prop(DecodedProp::new(
        DecodedProperty {
            name: "name:en".to_string(),
            values: PropValue::Str(vec![Some(val.clone())]),
        },
        enc,
    ))
    .add_prop(DecodedProp::new(
        DecodedProperty {
            name: "name:de".to_string(),
            values: PropValue::Str(vec![Some(val)]),
        },
        enc,
    ))
    .write(d, "props_no_shared_dict");

    // TODO: props_shared_dict and props_shared_dict_fsst need shared dictionary support
}
