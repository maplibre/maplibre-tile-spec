mod geometry;
mod layer;

use std::fs;
use std::path::Path;

use layer::Feature;
use mlt_core::v01::{Encoder, LogicalEncoder, PhysicalEncoder};

use crate::geometry::{C0, C1, C2};

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
    Feature::point(C0, Encoder::varint(), Encoder::varint()).write(dir, "point");
    Feature::line([C1, C2], Encoder::varint(), Encoder::varint(), Encoder::varint()).write(dir, "line");
}
