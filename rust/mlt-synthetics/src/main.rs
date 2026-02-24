mod geometry;
mod layer;

use std::fs;
use std::path::Path;

use layer::Feature;
use mlt_core::v01::{Encoder, IdEncoder, IdWidth, LogicalEncoder};

use crate::geometry::{C0, C1, C2, C3, H1, H2, H3, Point};

fn main() {
    // fixme: replace real synthetics
    let synthetics_dir = Path::new("../test/synthetic/rust/");
    if synthetics_dir.exists() {
        fs::remove_dir_all(synthetics_dir)
            .unwrap_or_else(|_| panic!("to be able to delete {}", synthetics_dir.display()));
    }
    fs::create_dir_all(synthetics_dir)
        .unwrap_or_else(|_| panic!("to be able to create {}", synthetics_dir.display()));

    generate_geometry(synthetics_dir);
}

fn generate_geometry(dir: &Path) {
    use {Encoder as Enc, Feature as Feat};

    // points
    Feat::point(C0, Enc::varint(), Enc::varint()).write(dir, "point");
    Feat::multi_point(&[C1, C2, C3]).write(dir, "multipoint");

    Feat::linestring(&[C1, C2], Enc::varint(), Enc::varint(), Enc::varint()).write(dir, "line");
    Feat::multi_linestring(&[&[C1, C2], &[H1, H2, H3]]).write(dir, "multiline");

    // polygon
    let pol1: &[Point] = &[C1, C2, C3, C1];
    Feat::polygon(pol1).write(dir, "polygon");
    Feat::polygon(pol1).write(dir, "polygon_fpf");
    Feat::polygon(pol1).write(dir, "polygon_tes");
    Feat::polygon(pol1).write(dir, "polygon_morton_tes");

    // Polygon with hole
    let pol2: &[Point] = &[H1, H2, H3, H1];
    Feat::polygon_with_hole(pol2, pol2).write(dir, "polygon_hole");
    Feat::polygon_with_hole(pol2, pol2).write(dir, "polygon_hole_fpf");

    // multipolygon
    Feat::multi_polygon(&[pol2, pol2]).write(dir, "polygon_multi");
    Feat::multi_polygon(&[pol2, pol2]).write(dir, "polygon_multi_fpf");
}

fn generate_mixed(dir: &Path) {
    use {Encoder as Enc, Feature as Feat};

    let line1: &[Point] = &[C1, C2];
    Feat::point(C0, Enc::varint(), Enc::varint())
        .and_linestring(line1, Enc::varint(), Enc::varint(), Enc::varint())
        .write(dir, "mixed_pt_line");
    let pol1: &[Point] = &[C1, C2, C3, C1];
    Feat::point(C0, Enc::varint(), Enc::varint())
        .and_polygon(pol1)
        .write(dir, "mixed_pt_poly");
    Feat::linestring(line1, Enc::varint(), Enc::varint(), Enc::varint())
        .and_polygon(pol1)
        .write(dir, "mixed_line_poly");
    let line2: &[Point] = &[H1, H2, H3];
    Feat::point(C0, Enc::varint(), Enc::varint())
        .and_multi_linestring(&[line1, line2])
        .write(dir, "mixed_pt_mline");

    let pol2: &[Point] = &[H1, H2, H3, H1];
    Feat::point(C0, Enc::varint(), Enc::varint())
        .and_linestring(line1, Enc::varint(), Enc::varint(), Enc::varint())
        .and_polygon(pol1)
        .and_multi_polygon(&[pol2, pol2])
        .write(dir, "mixed_all");
}

fn generate_extent(dir: &Path) {
    use {Encoder as Enc, Feature as Feat};

    for extent in [512, 4096, 131072, 1073741824] {
        Feat::linestring(
            &[[0, 0], [extent - 1, extent - 1]],
            Enc::varint(),
            Enc::varint(),
            Enc::varint(),
        )
        .extent(extent as u32)
        .write(dir, &format!("extent_{extent}"));
        Feat::linestring(
            &[[-42, -42], [extent + 42, extent + 42]],
            Enc::varint(),
            Enc::varint(),
            Enc::varint(),
        )
        .extent(extent as u32)
        .write(dir, &format!("extent_buf_{extent}"));
    }
}

fn generate_ids(dir: &Path) {
    use {Encoder as Enc, Feature as Feat};
    let p0 = Feat::point(C0, Enc::varint(), Enc::varint());

    p0.clone()
        .id(0, LogicalEncoder::None, IdWidth::Id32)
        .write(dir, "id_0");
    p0.clone()
        .id(100, LogicalEncoder::None, IdWidth::Id32)
        .write(dir, "id");
    p0.clone()
        .id(9_234_567_890, LogicalEncoder::None, IdWidth::Id64)
        .write(dir, "id64");

    for (enc, suffix) in [
        (LogicalEncoder::None, ""),
        (LogicalEncoder::Delta, "_delta"),
        (LogicalEncoder::Rle, "_rle"),
        (LogicalEncoder::DeltaRle, "_delta_rle"),
    ] {
        let ids32 = vec![Some(103), Some(103), Some(103), Some(103)];
        p0.clone()
            .ids(ids32, IdEncoder::new(enc, IdWidth::Id32))
            .write(dir, &format!("ids{suffix}"));

        let ids64 = vec![
            Some(9_234_567_890),
            Some(9_234_567_890),
            Some(9_234_567_890),
            Some(9_234_567_890),
        ];
        p0.clone()
            .ids(ids64, IdEncoder::new(enc, IdWidth::Id64))
            .write(dir, &format!("id64{suffix}"));

        let opt_ids32 = vec![Some(100), Some(101), None, Some(105), Some(106)];
        p0.clone()
            .ids(opt_ids32, IdEncoder::new(enc, IdWidth::OptId32))
            .write(dir, &format!("ids_opt{suffix}"));

        let opt_ids64 = vec![None, Some(9_234_567_890), Some(101), Some(105), Some(106)];
        p0.clone()
            .ids(opt_ids64, IdEncoder::new(enc, IdWidth::Id64))
            .write(dir, &format!("id64_opt{suffix}"));
    }
}
