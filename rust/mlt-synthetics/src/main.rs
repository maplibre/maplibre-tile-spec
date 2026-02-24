mod geometry;
mod layer;

use layer::Feature;
use std::fs;
use std::path::Path;
use mlt_core::v01::Encoder;
use crate::geometry::C0;

fn main() {
    let synthetics_dir = Path::new("../test/synthetic/0x01");
    if synthetics_dir.exists() {
        fs::remove_dir_all(synthetics_dir).unwrap_or_else(|_| panic!("to be able to delete {}", synthetics_dir.display()));
    }
    fs::create_dir_all(synthetics_dir).unwrap_or_else(|_| panic!("to be able to create {}", synthetics_dir.display()));

    generate_geometry(synthetics_dir);
}

fn generate_geometry(dir: &Path){
    Feature::point(C0, Encoder::plain(), Encoder::plain()).write(dir,"point");
}
