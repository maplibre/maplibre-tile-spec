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
//!
//! ## Files NOT YET GENERATED:
//! - Mixed geometry: `mixed_pt_line`, `mixed_pt_poly`, `mixed_line_poly`, `mixed_pt_mline`, `mixed_all`
//!   (requires fixing geometry offset arrays in mlt-core for mixed types)
//! - Tessellation variants: `polygon_tes`, `polygon_morton_tes` (tessellation compute not implemented)
//! - FSST: `props_str_fsst` (FSST encoding not implemented)
//! - Shared dictionary: `props_shared_dict`, `props_shared_dict_fsst` (shared dict encoding not implemented)

mod geometry;
mod layer;

use std::fs;
use std::path::Path;

use geo_types::{Coord, LineString, MultiLineString, MultiPoint, MultiPolygon, Point, Polygon};
use mlt_core::v01::{Encoder, IdEncoder, IdWidth, LogicalEncoder, PropValue, PropertyEncoder};

use crate::layer::{Feature, Layer};

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

fn generate_geometry(dir: &Path) {
    use Encoder as E;

    Feature::point(Point(C0), E::varint(), E::varint()).write(dir, "point");

    Feature::linestring(
        LineString(vec![C1, C2]),
        E::varint(),
        E::varint(),
        E::varint(),
    )
    .write(dir, "line");

    let pol = || Polygon::new(LineString(vec![C1, C2, C3, C1]), vec![]);
    Feature::polygon(pol(), E::varint(), E::varint(), E::varint(), E::varint())
        .write(dir, "polygon");

    Feature::polygon(
        pol(),
        E::fastpfor(),
        E::fastpfor(),
        E::fastpfor(),
        E::fastpfor(),
    )
    .write(dir, "polygon_fpf");

    // TODO: polygon_tes, polygon_morton_tes need tessellation support
    // Polygon with hole - Java: feat(poly(ring(c1, c2, c3, c1), ring(h1, h2, h3, h1)));
    // Java uses RLE encoding for the rings stream
    let pol_hole = || {
        Polygon::new(
            LineString(vec![C1, C2, C3, C1]),
            vec![LineString(vec![H1, H2, H3, H1])],
        )
    };
    Feature::polygon(
        pol_hole(),
        E::varint(),
        E::varint(),
        E::varint(),
        E::rle_varint(),
    )
    .write(dir, "polygon_hole");

    // polygon_hole_fpf - Polygon with hole using FastPFOR + RLE for rings
    Feature::polygon(
        pol_hole(),
        E::fastpfor(),
        E::fastpfor(),
        E::fastpfor(),
        E::rle_fastpfor(),
    )
    .write(dir, "polygon_hole_fpf");

    // MultiPolygon - Java: feat(multi(poly(c1, c2, c3, c1), poly(h1, h3, c2, h1)));
    // Java uses RLE encoding for both parts and rings streams
    let mpol = || {
        MultiPolygon(vec![
            Polygon::new(LineString(vec![C1, C2, C3, C1]), vec![]),
            Polygon::new(LineString(vec![H1, H3, C2, H1]), vec![]),
        ])
    };
    Feature::multi_polygon(
        mpol(),
        E::varint(),
        E::varint(),
        E::varint(),
        E::rle_varint(),
        E::rle_varint(),
    )
    .write(dir, "polygon_multi");

    // polygon_multi_fpf - MultiPolygon with FastPFOR + RLE for parts/rings
    Feature::multi_polygon(
        mpol(),
        E::fastpfor(),
        E::fastpfor(),
        E::fastpfor(),
        E::rle_fastpfor(),
        E::rle_fastpfor(),
    )
    .write(dir, "polygon_multi_fpf");

    // MultiPoint - Java: write("multipoint", feat(multi(p1, p2, p3)), cfg());
    let mpt = MultiPoint(vec![Point(C1), Point(C2), Point(C3)]);
    Feature::multi_point(mpt, E::varint(), E::varint(), E::varint()).write(dir, "multipoint");

    // MultiLineString - Java: write("multiline", feat(multi(line(c1, c2), line(h1, h2, h3))), cfg());
    let mline = MultiLineString(vec![LineString(vec![C1, C2]), LineString(vec![H1, H2, H3])]);
    Feature::multi_linestring(mline, E::varint(), E::varint(), E::varint(), E::varint())
        .write(dir, "multiline");
}

fn generate_mixed(dir: &Path) {
    use Encoder as E;

    let line1 = || LineString(vec![C1, C2]);
    let line2 = || LineString(vec![H1, H2, H3]);
    let pol1 = || Polygon::new(LineString(vec![C1, C2, C3, C1]), vec![]);
    let pol2 = || Polygon::new(LineString(vec![H1, H3, H2, H1]), vec![]);

    // mixed_pt_line: layer with point feature + linestring feature
    Layer::new(vec![
        Feature::point(Point(C0), E::varint(), E::varint()),
        Feature::linestring(line1(), E::varint(), E::varint(), E::varint()),
    ])
    .write(dir, "mixed_pt_line");

    // mixed_pt_poly: layer with point feature + polygon feature
    Layer::new(vec![
        Feature::point(Point(C0), E::varint(), E::varint()),
        Feature::polygon(pol1(), E::varint(), E::varint(), E::varint(), E::varint()),
    ])
    .write(dir, "mixed_pt_poly");

    // mixed_line_poly: layer with linestring feature + polygon feature
    Layer::new(vec![
        Feature::linestring(line1(), E::varint(), E::varint(), E::varint()),
        Feature::polygon(pol1(), E::varint(), E::varint(), E::varint(), E::varint()),
    ])
    .write(dir, "mixed_line_poly");

    // mixed_pt_mline: layer with point feature + multi-linestring feature
    Layer::new(vec![
        Feature::point(Point(C0), E::varint(), E::varint()),
        Feature::multi_linestring(
            MultiLineString(vec![line1(), line2()]),
            E::varint(),
            E::varint(),
            E::varint(),
            E::varint(),
        ),
    ])
    .write(dir, "mixed_pt_mline");

    // mixed_all: layer with point + linestring + polygon + multi-polygon features
    // Java auto-detects const streams and uses RLE, so we use RLE for parts since all values are 1
    Layer::new(vec![
        Feature::point(Point(C0), E::varint(), E::varint()),
        Feature::linestring(line1(), E::varint(), E::varint(), E::varint()),
        Feature::polygon(
            pol1(),
            E::varint(),
            E::varint(),
            E::rle_varint(), // parts: all values are 1, so Java uses RLE
            E::varint(),
        ),
        Feature::multi_polygon(
            MultiPolygon(vec![pol1(), pol2()]),
            E::varint(),
            E::varint(),
            E::varint(),
            E::rle_varint(), // parts: all values are 1, so Java uses RLE
            E::varint(),
        ),
    ])
    .write(dir, "mixed_all");
}

fn generate_extent(dir: &Path) {
    use Encoder as E;

    for extent in [512_i32, 4096, 131_072, 1_073_741_824] {
        let line = LineString(vec![
            Coord { x: 0, y: 0 },
            Coord {
                x: extent - 1,
                y: extent - 1,
            },
        ]);
        Feature::linestring(line, E::varint(), E::varint(), E::varint())
            .extent(extent.cast_unsigned())
            .write(dir, &format!("extent_{extent}"));

        let line_buf = LineString(vec![
            Coord { x: -42, y: -42 },
            Coord {
                x: extent + 42,
                y: extent + 42,
            },
        ]);
        Feature::linestring(line_buf, E::varint(), E::varint(), E::varint())
            .extent(extent.cast_unsigned())
            .write(dir, &format!("extent_buf_{extent}"));
    }
}

fn generate_ids(dir: &Path) {
    use Encoder as E;

    // Single point uses varint for meta stream
    let p0 = || Feature::point(Point(C0), E::varint(), E::varint());

    p0().id(0, LogicalEncoder::None, IdWidth::Id32)
        .write(dir, "id0");
    p0().id(100, LogicalEncoder::None, IdWidth::Id32)
        .write(dir, "id");
    p0().id(9_234_567_890, LogicalEncoder::None, IdWidth::Id64)
        .write(dir, "id64");

    // Helper to create 4 identical points - uses RLE for meta stream since all types are identical
    // Java auto-selects RLE for const streams, so we explicitly specify it
    let four_p0 = || {
        Feature::point(Point(C0), E::rle_varint(), E::varint())
            .and_point(Point(C0), E::rle_varint(), E::varint())
            .and_point(Point(C0), E::rle_varint(), E::varint())
            .and_point(Point(C0), E::rle_varint(), E::varint())
    };

    // ids with various encodings
    for (enc, suffix) in [
        (LogicalEncoder::None, ""),
        (LogicalEncoder::Delta, "_delta"),
        (LogicalEncoder::Rle, "_rle"),
        (LogicalEncoder::DeltaRle, "_delta_rle"),
    ] {
        let ids32 = vec![Some(103), Some(103), Some(103), Some(103)];
        four_p0()
            .ids(ids32, IdEncoder::new(enc, IdWidth::Id32))
            .write(dir, &format!("ids{suffix}"));

        let ids64 = vec![
            Some(9_234_567_890),
            Some(9_234_567_890),
            Some(9_234_567_890),
            Some(9_234_567_890),
        ];
        four_p0()
            .ids(ids64, IdEncoder::new(enc, IdWidth::Id64))
            .write(dir, &format!("ids64{suffix}"));
    }

    // Helper to create 5 identical points for optional IDs - uses RLE for meta stream
    let five_p0 = || {
        Feature::point(Point(C0), E::rle_varint(), E::varint())
            .and_point(Point(C0), E::rle_varint(), E::varint())
            .and_point(Point(C0), E::rle_varint(), E::varint())
            .and_point(Point(C0), E::rle_varint(), E::varint())
            .and_point(Point(C0), E::rle_varint(), E::varint())
    };

    // Optional IDs (only None and Delta - Java doesn't generate RLE/DeltaRle for optional)
    for (enc, suffix) in [
        (LogicalEncoder::None, ""),
        (LogicalEncoder::Delta, "_delta"),
    ] {
        let opt_ids32 = vec![Some(100), Some(101), None, Some(105), Some(106)];
        five_p0()
            .ids(opt_ids32, IdEncoder::new(enc, IdWidth::OptId32))
            .write(dir, &format!("ids_opt{suffix}"));

        let opt_ids64 = vec![None, Some(9_234_567_890), Some(101), Some(105), Some(106)];
        five_p0()
            .ids(opt_ids64, IdEncoder::new(enc, IdWidth::OptId64))
            .write(dir, &format!("ids64_opt{suffix}"));
    }
}

fn generate_properties(dir: &Path) {
    use Encoder as E;
    use mlt_core::v01::{PhysicalEncoder, PresenceStream};

    let p0 = || Feature::point(Point(C0), E::varint(), E::varint());
    // Java always creates presence stream for properties (isNullable = true)
    // Use VarInt for physical encoding to match Java's default
    let enc = PropertyEncoder::new(
        PresenceStream::Present,
        LogicalEncoder::None,
        PhysicalEncoder::VarInt,
    );

    // Boolean properties
    p0().prop(&"val", PropValue::Bool(vec![Some(true)]), enc)
        .write(dir, "prop_bool");
    p0().prop(&"val", PropValue::Bool(vec![Some(false)]), enc)
        .write(dir, "prop_bool_false");

    // i32 properties
    p0().prop(&"val", PropValue::I32(vec![Some(42)]), enc)
        .write(dir, "prop_i32");
    p0().prop(&"val", PropValue::I32(vec![Some(-42)]), enc)
        .write(dir, "prop_i32_neg");
    p0().prop(&"val", PropValue::I32(vec![Some(i32::MIN)]), enc)
        .write(dir, "prop_i32_min");
    p0().prop(&"val", PropValue::I32(vec![Some(i32::MAX)]), enc)
        .write(dir, "prop_i32_max");

    // u32 properties
    p0().prop(&"val", PropValue::U32(vec![Some(42)]), enc)
        .write(dir, "prop_u32");
    p0().prop(&"val", PropValue::U32(vec![Some(0)]), enc)
        .write(dir, "prop_u32_min");
    p0().prop(&"val", PropValue::U32(vec![Some(u32::MAX)]), enc)
        .write(dir, "prop_u32_max");

    // i64 properties
    p0().prop(&"val", PropValue::I64(vec![Some(9_876_543_210)]), enc)
        .write(dir, "prop_i64");
    p0().prop(&"val", PropValue::I64(vec![Some(-9_876_543_210)]), enc)
        .write(dir, "prop_i64_neg");
    p0().prop(&"val", PropValue::I64(vec![Some(i64::MIN)]), enc)
        .write(dir, "prop_i64_min");
    p0().prop(&"val", PropValue::I64(vec![Some(i64::MAX)]), enc)
        .write(dir, "prop_i64_max");

    // u64 properties
    p0().prop(
        &"bignum",
        PropValue::U64(vec![Some(1_234_567_890_123_456_789)]),
        enc,
    )
    .write(dir, "prop_u64");
    p0().prop(&"bignum", PropValue::U64(vec![Some(0)]), enc)
        .write(dir, "prop_u64_min");
    p0().prop(&"bignum", PropValue::U64(vec![Some(u64::MAX)]), enc)
        .write(dir, "prop_u64_max");

    // f32 properties (explicit values to match Java exactly)
    #[expect(clippy::approx_constant)]
    p0().prop(&"val", PropValue::F32(vec![Some(3.14)]), enc)
        .write(dir, "prop_f32");
    p0().prop(&"val", PropValue::F32(vec![Some(f32::NEG_INFINITY)]), enc)
        .write(dir, "prop_f32_neg_inf");
    // Java Float.MIN_VALUE is the smallest positive denormalized float (0x00000001)
    // not the smallest normalized float (f32::MIN_POSITIVE = 0x00800000)
    p0().prop(&"val", PropValue::F32(vec![Some(f32::from_bits(1))]), enc)
        .write(dir, "prop_f32_min");
    p0().prop(&"val", PropValue::F32(vec![Some(0.0)]), enc)
        .write(dir, "prop_f32_zero");
    p0().prop(&"val", PropValue::F32(vec![Some(f32::MAX)]), enc)
        .write(dir, "prop_f32_max");
    p0().prop(&"val", PropValue::F32(vec![Some(f32::INFINITY)]), enc)
        .write(dir, "prop_f32_pos_inf");
    p0().prop(&"val", PropValue::F32(vec![Some(f32::NAN)]), enc)
        .write(dir, "prop_f32_nan");

    // f64 properties (explicit values to match Java exactly)
    #[expect(clippy::approx_constant)]
    p0().prop(
        &"val",
        PropValue::F64(vec![Some(3.141_592_653_589_793)]),
        enc,
    )
    .write(dir, "prop_f64");
    p0().prop(&"val", PropValue::F64(vec![Some(f64::NEG_INFINITY)]), enc)
        .write(dir, "prop_f64_neg_inf");
    p0().prop(&"val", PropValue::F64(vec![Some(f64::MIN_POSITIVE)]), enc)
        .write(dir, "prop_f64_min");
    p0().prop(&"val", PropValue::F64(vec![Some(-0.0)]), enc)
        .write(dir, "prop_f64_neg_zero");
    p0().prop(&"val", PropValue::F64(vec![Some(f64::MAX)]), enc)
        .write(dir, "prop_f64_max");
    p0().prop(&"val", PropValue::F64(vec![Some(f64::NAN)]), enc)
        .write(dir, "prop_f64_nan");

    // String properties
    p0().prop(&"val", PropValue::Str(vec![Some(String::new())]), enc)
        .write(dir, "prop_str_empty");
    p0().prop(&"val", PropValue::Str(vec![Some("42".to_string())]), enc)
        .write(dir, "prop_str_ascii");
    p0().prop(
        &"val",
        PropValue::Str(vec![Some("Line1\n\t\"quoted\"\\path".to_string())]),
        enc,
    )
    .write(dir, "prop_str_escape");
    // Use decomposed form (cafe + U+0301) to match Java's "cafe\u0301"
    p0().prop(
        &"val",
        PropValue::Str(vec![Some("München 📍 cafe\u{0301}".to_string())]),
        enc,
    )
    .write(dir, "prop_str_unicode");

    // Multiple features with same property - props_mixed
    let p1 = || Feature::point(Point(C1), E::varint(), E::varint());
    p1().prop(
        &"name",
        PropValue::Str(vec![Some("Test Point".to_string())]),
        enc,
    )
    .and_prop(&"count", PropValue::I32(vec![Some(42)]), enc)
    .and_prop(&"active", PropValue::Bool(vec![Some(true)]), enc)
    .and_prop(&"temp", PropValue::F32(vec![Some(25.5)]), enc)
    .and_prop(&"precision", PropValue::F64(vec![Some(0.123_456_789)]), enc)
    .write(dir, "props_mixed");

    // Generate property arrays with various encodings
    generate_props_i32(dir);
    generate_props_u32(dir);
    generate_props_u64(dir);
    generate_props_str(dir);
    generate_shared_dictionaries(dir);
}

fn generate_props_i32(dir: &Path) {
    use Encoder as E;
    use mlt_core::v01::{PhysicalEncoder, PresenceStream};

    // p1, p2, p3, ph1 - matching Java's points
    // Use RLE for meta stream since all 4 geometry types are identical (Point)
    let coords = [C1, C2, C3, H1];
    let values: Vec<Option<i32>> = vec![Some(42), Some(42), Some(42), Some(42)];

    for (log_enc, suffix) in [
        (LogicalEncoder::None, ""),
        (LogicalEncoder::Delta, "_delta"),
        (LogicalEncoder::Rle, "_rle"),
        (LogicalEncoder::DeltaRle, "_delta_rle"),
    ] {
        // Java always creates presence stream for properties (isNullable = true)
        let enc = PropertyEncoder::new(PresenceStream::Present, log_enc, PhysicalEncoder::VarInt);
        let mut feat = Feature::point(Point(coords[0]), E::rle_varint(), E::varint());
        for &c in &coords[1..] {
            feat = feat.and_point(Point(c), E::rle_varint(), E::varint());
        }
        feat.prop(&"val", PropValue::I32(values.clone()), enc)
            .write(dir, &format!("props_i32{suffix}"));
    }
}

fn generate_props_u32(dir: &Path) {
    use Encoder as E;
    use mlt_core::v01::{PhysicalEncoder, PresenceStream};

    // p0, p1, p2, p3
    // Use RLE for meta stream since all 4 geometry types are identical (Point)
    let coords = [C0, C1, C2, C3];
    let values: Vec<Option<u32>> = vec![Some(9_000), Some(9_000), Some(9_000), Some(9_000)];

    for (log_enc, suffix) in [
        (LogicalEncoder::None, ""),
        (LogicalEncoder::Delta, "_delta"),
        (LogicalEncoder::Rle, "_rle"),
        (LogicalEncoder::DeltaRle, "_delta_rle"),
    ] {
        // Java always creates presence stream for properties (isNullable = true)
        let enc = PropertyEncoder::new(PresenceStream::Present, log_enc, PhysicalEncoder::VarInt);
        let mut feat = Feature::point(Point(coords[0]), E::rle_varint(), E::varint());
        for &c in &coords[1..] {
            feat = feat.and_point(Point(c), E::rle_varint(), E::varint());
        }
        feat.prop(&"val", PropValue::U32(values.clone()), enc)
            .write(dir, &format!("props_u32{suffix}"));
    }
}

fn generate_props_u64(dir: &Path) {
    use Encoder as E;
    use mlt_core::v01::{PhysicalEncoder, PresenceStream};

    // p0, p1, p2, p3
    // Use RLE for meta stream since all 4 geometry types are identical (Point)
    let coords = [C0, C1, C2, C3];
    let values: Vec<Option<u64>> = vec![Some(9_000), Some(9_000), Some(9_000), Some(9_000)];

    for (log_enc, suffix) in [
        (LogicalEncoder::None, ""),
        (LogicalEncoder::Delta, "_delta"),
        (LogicalEncoder::Rle, "_rle"),
        (LogicalEncoder::DeltaRle, "_delta_rle"),
    ] {
        // Java always creates presence stream for properties (isNullable = true)
        let enc = PropertyEncoder::new(PresenceStream::Present, log_enc, PhysicalEncoder::VarInt);
        let mut feat = Feature::point(Point(coords[0]), E::rle_varint(), E::varint());
        for &c in &coords[1..] {
            feat = feat.and_point(Point(c), E::rle_varint(), E::varint());
        }
        feat.prop(&"val", PropValue::U64(values.clone()), enc)
            .write(dir, &format!("props_u64{suffix}"));
    }
}

fn generate_props_str(dir: &Path) {
    use Encoder as E;
    use mlt_core::v01::{PhysicalEncoder, PresenceStream};

    // p1, p2, p3, ph1, ph2, ph3
    // Use RLE for meta stream since all 6 geometry types are identical (Point)
    let coords = [C1, C2, C3, H1, H2, H3];
    let values: Vec<Option<String>> = vec![
        Some("residential_zone_north_sector_1".to_string()),
        Some("commercial_zone_south_sector_2".to_string()),
        Some("industrial_zone_east_sector_3".to_string()),
        Some("park_zone_west_sector_4".to_string()),
        Some("water_zone_north_sector_5".to_string()),
        Some("residential_zone_south_sector_6".to_string()),
    ];

    // Java always creates presence stream for properties (isNullable = true)
    let enc = PropertyEncoder::new(
        PresenceStream::Present,
        LogicalEncoder::None,
        PhysicalEncoder::VarInt,
    );
    let mut feat = Feature::point(Point(coords[0]), E::rle_varint(), E::varint());
    for &c in &coords[1..] {
        feat = feat.and_point(Point(c), E::rle_varint(), E::varint());
    }
    feat.prop(&"val", PropValue::Str(values.clone()), enc)
        .write(dir, "props_str");

    // TODO: props_str_fsst needs FSST support
}

fn generate_shared_dictionaries(dir: &Path) {
    use Encoder as E;
    use mlt_core::v01::{PhysicalEncoder, PresenceStream};

    let p1 = || Feature::point(Point(C1), E::varint(), E::varint());
    // Java always creates presence stream for properties (isNullable = true)
    let enc = PropertyEncoder::new(
        PresenceStream::Present,
        LogicalEncoder::None,
        PhysicalEncoder::VarInt,
    );

    // 30 chars so fsst is not skipped
    let val = "A".repeat(30);

    p1().prop(&"name:en", PropValue::Str(vec![Some(val.clone())]), enc)
        .and_prop(&"name:de", PropValue::Str(vec![Some(val)]), enc)
        .write(dir, "props_no_shared_dict");

    // TODO: props_shared_dict and props_shared_dict_fsst need shared dictionary support
}
