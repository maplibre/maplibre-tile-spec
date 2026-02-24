mod geometry;
mod layer;

use std::fs;
use std::path::Path;

use layer::Feature;
use mlt_core::v01::{Encoder, IdWidth, LogicalEncoder, PhysicalEncoder};

use crate::geometry::C0;

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
    Feature::point(
        C0,
        Encoder {
            logical: LogicalEncoder::None,
            physical: PhysicalEncoder::VarInt,
        },
        Encoder {
            logical: LogicalEncoder::None,
            physical: PhysicalEncoder::VarInt,
        },
    )
    .id(0, LogicalEncoder::None, IdWidth::Id32)
    .write(dir, "point");
}
