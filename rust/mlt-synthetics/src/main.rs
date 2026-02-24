mod geometry;
mod layer;

use std::fs;
use std::path::Path;

use geo_types::{Coord, LineString, MultiLineString, MultiPoint, MultiPolygon, Point, Polygon};
use mlt_core::v01::Encoder;

use crate::layer::Feature;

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
}

fn generate_geometry(dir: &Path) {
    use Encoder as E;

    // Point - Java: write("point", feat(p0), cfg());
    Feature::point(Point(C0), E::varint(), E::varint()).write(dir, "point");

    // Line - Java: write("line", feat(line(c1, c2)), cfg());
    Feature::linestring(
        LineString(vec![C1, C2]),
        E::varint(),
        E::varint(),
        E::varint(),
    )
    .write(dir, "line");

    // Polygon - Java: var pol = feat(poly(c1, c2, c3, c1));
    let pol = Polygon::new(LineString(vec![C1, C2, C3, C1]), vec![]);
    Feature::polygon(
        pol.clone(),
        E::varint(),
        E::varint(),
        E::varint(),
        E::varint(),
    )
    .write(dir, "polygon");
    // TODO: polygon_fpf, polygon_tes, polygon_morton_tes need different encoding configs

    // Polygon with hole - Java: feat(poly(ring(c1, c2, c3, c1), ring(h1, h2, h3, h1)));
    // Java uses RLE encoding for the rings stream
    let pol_hole = Polygon::new(
        LineString(vec![C1, C2, C3, C1]),
        vec![LineString(vec![H1, H2, H3, H1])],
    );
    Feature::polygon(
        pol_hole,
        E::varint(),
        E::varint(),
        E::varint(),
        E::rle_varint(),
    )
    .write(dir, "polygon_hole");
    // TODO: polygon_hole_fpf needs different encoding config

    // MultiPolygon - Java: feat(multi(poly(c1, c2, c3, c1), poly(h1, h3, c2, h1)));
    // Java uses RLE encoding for both parts and rings streams
    let mpol = MultiPolygon(vec![
        Polygon::new(LineString(vec![C1, C2, C3, C1]), vec![]),
        Polygon::new(LineString(vec![H1, H3, C2, H1]), vec![]),
    ]);
    Feature::multi_polygon(
        mpol,
        E::varint(),
        E::varint(),
        E::varint(),
        E::rle_varint(),
        E::rle_varint(),
    )
    .write(dir, "polygon_multi");
    // TODO: polygon_multi_fpf needs different encoding config

    // MultiPoint - Java: write("multipoint", feat(multi(p1, p2, p3)), cfg());
    let mpt = MultiPoint(vec![Point(C1), Point(C2), Point(C3)]);
    Feature::multi_point(mpt, E::varint(), E::varint(), E::varint()).write(dir, "multipoint");

    // MultiLineString - Java: write("multiline", feat(multi(line(c1, c2), line(h1, h2, h3))), cfg());
    let mline = MultiLineString(vec![LineString(vec![C1, C2]), LineString(vec![H1, H2, H3])]);
    Feature::multi_linestring(mline, E::varint(), E::varint(), E::varint(), E::varint())
        .write(dir, "multiline");
}

#[expect(dead_code)]
fn generate_mixed(dir: &Path) {
    use Encoder as E;

    let line1 = LineString(vec![C1, C2]);
    let line2 = LineString(vec![H1, H2, H3]);
    let pol1 = Polygon::new(LineString(vec![C1, C2, C3, C1]), vec![]);
    let pol2 = Polygon::new(LineString(vec![H1, H2, H3, H1]), vec![]);

    // mixed_pt_line: point + linestring
    Feature::point(Point(C0), E::varint(), E::varint())
        .and_linestring(line1.clone(), E::varint(), E::varint(), E::varint())
        .write(dir, "mixed_pt_line");

    // mixed_pt_poly: point + polygon
    Feature::point(Point(C0), E::varint(), E::varint())
        .and_polygon(
            pol1.clone(),
            E::varint(),
            E::varint(),
            E::varint(),
            E::varint(),
        )
        .write(dir, "mixed_pt_poly");

    // mixed_line_poly: linestring + polygon
    Feature::linestring(line1.clone(), E::varint(), E::varint(), E::varint())
        .and_polygon(
            pol1.clone(),
            E::varint(),
            E::varint(),
            E::varint(),
            E::varint(),
        )
        .write(dir, "mixed_line_poly");

    // mixed_pt_mline: point + multi-linestring
    Feature::point(Point(C0), E::varint(), E::varint())
        .and_multi_linestring(
            MultiLineString(vec![line1.clone(), line2]),
            E::varint(),
            E::varint(),
            E::varint(),
            E::varint(),
        )
        .write(dir, "mixed_pt_mline");

    // mixed_all: point + linestring + polygon + multi-polygon
    Feature::point(Point(C0), E::varint(), E::varint())
        .and_linestring(line1, E::varint(), E::varint(), E::varint())
        .and_polygon(
            pol1.clone(),
            E::varint(),
            E::varint(),
            E::varint(),
            E::varint(),
        )
        .and_multi_polygon(
            MultiPolygon(vec![pol1, pol2]),
            E::varint(),
            E::varint(),
            E::varint(),
            E::varint(),
            E::varint(),
        )
        .write(dir, "mixed_all");
}

#[expect(dead_code)]
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

#[expect(dead_code)]
fn generate_ids(dir: &Path) {
    use Encoder as E;
    use mlt_core::v01::{IdEncoder, IdWidth, LogicalEncoder};

    let p0 = || Feature::point(Point(C0), E::varint(), E::varint());

    p0().id(0, LogicalEncoder::None, IdWidth::Id32)
        .write(dir, "id0");
    p0().id(100, LogicalEncoder::None, IdWidth::Id32)
        .write(dir, "id");
    p0().id(9_234_567_890, LogicalEncoder::None, IdWidth::Id64)
        .write(dir, "id64");

    for (enc, suffix) in [
        (LogicalEncoder::None, ""),
        (LogicalEncoder::Delta, "_delta"),
        (LogicalEncoder::Rle, "_rle"),
        (LogicalEncoder::DeltaRle, "_delta_rle"),
    ] {
        let ids32 = vec![Some(103), Some(103), Some(103), Some(103)];
        p0().ids(ids32, IdEncoder::new(enc, IdWidth::Id32))
            .write(dir, &format!("ids{suffix}"));

        let ids64 = vec![
            Some(9_234_567_890),
            Some(9_234_567_890),
            Some(9_234_567_890),
            Some(9_234_567_890),
        ];
        p0().ids(ids64, IdEncoder::new(enc, IdWidth::Id64))
            .write(dir, &format!("id64{suffix}"));

        let opt_ids32 = vec![Some(100), Some(101), None, Some(105), Some(106)];
        p0().ids(opt_ids32, IdEncoder::new(enc, IdWidth::OptId32))
            .write(dir, &format!("ids_opt{suffix}"));

        let opt_ids64 = vec![None, Some(9_234_567_890), Some(101), Some(105), Some(106)];
        p0().ids(opt_ids64, IdEncoder::new(enc, IdWidth::Id64))
            .write(dir, &format!("id64_opt{suffix}"));
    }
}
