//! Profile the fast-mvt decoder.
//!
//! Usage:
//! ```
//! cargo run --release -p fast-mvt --example profile_decoder -- ../test/fixtures/omt/4_8_10.mvt
//! cargo flamegraph -p fast-mvt --example profile_decoder -- ../test/fixtures/omt/4_8_10.mvt
//! ```
use std::env;
use std::hint::black_box;
use std::path::PathBuf;
use std::time::Instant;

use fast_mvt::MvtReaderRef;
use usize_cast::FromUsize;

fn main() {
    let path = env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .expect("usage: cargo run --release -p fast-mvt --example profile_decoder -- <tile.mvt> [iterations]");
    let iterations = env::args().nth(2).map_or(100, |v| {
        v.parse::<usize>().expect("iterations must be a number")
    });
    let data = std::fs::read(&path).expect("read fixture");

    let mut checksum = 0_u64;
    let started = Instant::now();
    for _ in 0..iterations {
        checksum = checksum.wrapping_add(traverse_fast_mvt(black_box(&data)));
    }
    println!(
        "decoded {} bytes from {} for {iterations} iterations in {:.3}s; checksum={checksum}",
        data.len(),
        path.display(),
        started.elapsed().as_secs_f64()
    );
}

fn traverse_fast_mvt(data: &[u8]) -> u64 {
    let reader = MvtReaderRef::new(data).expect("fast-mvt parse");
    let mut visited = u64::from_usize(reader.layer_count());
    for layer in reader.layers() {
        visited = visited.wrapping_add(u64::from_usize(layer.feature_count()));
        for feature in layer.features() {
            black_box(feature.id());
            visited = visited.wrapping_add(1);

            black_box(feature.geometry().expect("fast-mvt geometry"));
            visited = visited.wrapping_add(1);

            for property in feature.properties() {
                black_box(property.expect("fast-mvt property"));
                visited = visited.wrapping_add(1);
            }
        }
    }
    visited
}
