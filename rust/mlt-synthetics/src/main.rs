mod geometry;
mod layer;

use std::fs;
use std::path::Path;

use geo_types::{Coord, LineString, MultiLineString, MultiPoint, MultiPolygon, Point, Polygon};
use mlt_core::v01::{Encoder, IdEncoder, IdWidth, LogicalEncoder};

use crate::geometry::ValidatingGeometryEncoder;
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
    use Encoder as Enc;
    type GEnc = ValidatingGeometryEncoder;

    // Point - Java: write("point", feat(p0), cfg());
    let pt_enc = GEnc::default().point(Enc::varint(), Enc::varint());
    Feature::from_geom(Point(C0), pt_enc).write(dir, "point");

    // Line - Java: write("line", feat(line(c1, c2)), cfg());
    let line_enc = GEnc::default().linestring(Enc::varint(), Enc::varint(), Enc::varint());
    Feature::from_geom(LineString(vec![C1, C2]), line_enc).write(dir, "line");

    // Polygon - Java: var pol = feat(poly(c1, c2, c3, c1));
    let poly_enc =
        GEnc::default().polygon(Enc::varint(), Enc::varint(), Enc::varint(), Enc::varint());
    let pol = Polygon::new(LineString(vec![C1, C2, C3, C1]), vec![]);
    Feature::from_geom(pol.clone(), poly_enc).write(dir, "polygon");
    // TODO: polygon_fpf, polygon_tes, polygon_morton_tes need different encoding configs

    // Polygon with hole - Java: feat(poly(ring(c1, c2, c3, c1), ring(h1, h2, h3, h1)));
    let pol_hole = Polygon::new(
        LineString(vec![C1, C2, C3, C1]),
        vec![LineString(vec![H1, H2, H3, H1])],
    );
    Feature::from_geom(pol_hole.clone(), poly_enc).write(dir, "polygon_hole");
    // TODO: polygon_hole_fpf needs different encoding config

    // MultiPolygon - Java: feat(multi(poly(c1, c2, c3, c1), poly(h1, h3, c2, h1)));
    let mpoly_enc = GEnc::default().multi_polygon(
        Enc::varint(),
        Enc::varint(),
        Enc::varint(),
        Enc::varint(),
        Enc::varint(),
    );
    let mpol = MultiPolygon(vec![
        Polygon::new(LineString(vec![C1, C2, C3, C1]), vec![]),
        Polygon::new(LineString(vec![H1, H3, C2, H1]), vec![]),
    ]);
    Feature::from_geom(mpol.clone(), mpoly_enc).write(dir, "polygon_multi");
    // TODO: polygon_multi_fpf needs different encoding config

    // MultiPoint - Java: write("multipoint", feat(multi(p1, p2, p3)), cfg());
    let mpt_enc = GEnc::default().multi_point(Enc::varint(), Enc::varint(), Enc::varint());
    let mpt = MultiPoint(vec![Point(C1), Point(C2), Point(C3)]);
    Feature::from_geom(mpt, mpt_enc).write(dir, "multipoint");

    // MultiLineString - Java: write("multiline", feat(multi(line(c1, c2), line(h1, h2, h3))), cfg());
    let mline_enc = GEnc::default().multi_linestring(
        Enc::varint(),
        Enc::varint(),
        Enc::varint(),
        Enc::varint(),
    );
    let mline = MultiLineString(vec![LineString(vec![C1, C2]), LineString(vec![H1, H2, H3])]);
    Feature::from_geom(mline, mline_enc).write(dir, "multiline");
}

#[expect(dead_code)]
fn generate_mixed(dir: &Path) {
    use Encoder as Enc;
    type GEnc = ValidatingGeometryEncoder;

    let pt_enc = GEnc::default().point(Enc::varint(), Enc::varint());
    let line_enc = GEnc::default().linestring(Enc::varint(), Enc::varint(), Enc::varint());
    let poly_enc =
        GEnc::default().polygon(Enc::varint(), Enc::varint(), Enc::varint(), Enc::varint());
    let mline_enc = GEnc::default().multi_linestring(
        Enc::varint(),
        Enc::varint(),
        Enc::varint(),
        Enc::varint(),
    );
    let mpoly_enc = GEnc::default().multi_polygon(
        Enc::varint(),
        Enc::varint(),
        Enc::varint(),
        Enc::varint(),
        Enc::varint(),
    );

    let line1 = LineString(vec![C1, C2]);
    let line2 = LineString(vec![H1, H2, H3]);
    let pol1 = Polygon::new(LineString(vec![C1, C2, C3, C1]), vec![]);
    let pol2 = Polygon::new(LineString(vec![H1, H2, H3, H1]), vec![]);

    // mixed_pt_line: point + linestring
    Feature::from_geom(Point(C0), pt_enc)
        .and(line1.clone(), line_enc)
        .write(dir, "mixed_pt_line");

    // mixed_pt_poly: point + polygon
    Feature::from_geom(Point(C0), pt_enc)
        .and(pol1.clone(), poly_enc)
        .write(dir, "mixed_pt_poly");

    // mixed_line_poly: linestring + polygon
    Feature::from_geom(line1.clone(), line_enc)
        .and(pol1.clone(), poly_enc)
        .write(dir, "mixed_line_poly");

    // mixed_pt_mline: point + multi-linestring
    Feature::from_geom(Point(C0), pt_enc)
        .and(MultiLineString(vec![line1.clone(), line2]), mline_enc)
        .write(dir, "mixed_pt_mline");

    // mixed_all: point + linestring + polygon + multi-polygon
    Feature::from_geom(Point(C0), pt_enc)
        .and(line1, line_enc)
        .and(pol1.clone(), poly_enc)
        .and(MultiPolygon(vec![pol1, pol2]), mpoly_enc)
        .write(dir, "mixed_all");
}

#[expect(dead_code)]
fn generate_extent(dir: &Path) {
    use Encoder as Enc;
    type GEnc = ValidatingGeometryEncoder;

    let line_enc = GEnc::default().linestring(Enc::varint(), Enc::varint(), Enc::varint());

    for extent in [512_i32, 4096, 131_072, 1_073_741_824] {
        let line = LineString(vec![
            Coord { x: 0, y: 0 },
            Coord {
                x: extent - 1,
                y: extent - 1,
            },
        ]);
        Feature::from_geom(line, line_enc)
            .extent(extent.cast_unsigned())
            .write(dir, &format!("extent_{extent}"));

        let line_buf = LineString(vec![
            Coord { x: -42, y: -42 },
            Coord {
                x: extent + 42,
                y: extent + 42,
            },
        ]);
        Feature::from_geom(line_buf, line_enc)
            .extent(extent.cast_unsigned())
            .write(dir, &format!("extent_buf_{extent}"));
    }
}

#[expect(dead_code)]
fn generate_ids(dir: &Path) {
    use Encoder as Enc;
    type GEnc = ValidatingGeometryEncoder;

    let pt_enc = GEnc::default().point(Enc::varint(), Enc::varint());
    let p0 = || Feature::from_geom(Point(C0), pt_enc);

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
