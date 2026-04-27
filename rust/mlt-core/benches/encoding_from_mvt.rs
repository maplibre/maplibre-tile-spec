use std::hint::black_box;

use criterion::{BatchSize, BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use mlt_core::encoder::EncoderConfig;
use mlt_core::mvt::mvt_to_tile_layers;
use mlt_core::{MltResult, TileLayer};

#[path = "bench_utils.rs"]
mod bench_utils;
use bench_utils::{BENCHMARKED_ZOOM_LEVELS, load_tiles};

/// Load MVT tiles for a given zoom level from the OMT fixture directory.
fn load_mvt_tiles(zoom: u8) -> Vec<(String, Vec<u8>)> {
    load_tiles(zoom, "fixtures/omt", ".mvt")
}

/// Parse MVT bytes into `TileLayer` objects outside the benchmark loop.
///
/// Errors are silently skipped (some fixture tiles may use unsupported geometry
/// types); the benchmark only exercises successfully parsed layers.
fn parse_mvt_to_tile_layers(mvt_files: &[(String, Vec<u8>)]) -> Vec<TileLayer> {
    mvt_files
        .iter()
        .flat_map(|(path, data)| {
            mvt_to_tile_layers(data.clone()).unwrap_or_else(|err| {
                eprintln!("skipping {path}: {err}");
                Vec::new()
            })
        })
        .collect()
}

fn bench_encode_from_mvt(c: &mut Criterion) {
    let mut group = c.benchmark_group("mlt encode from mvt");
    let cfg = EncoderConfig {
        tessellate: true,
        ..Default::default()
    };

    for zoom in BENCHMARKED_ZOOM_LEVELS {
        let mvt_files = load_mvt_tiles(zoom);
        let total_bytes: usize = mvt_files.iter().map(|(_, d)| d.len()).sum();

        // Parse all MVT files into TileLayer once, outside every benchmark iteration.
        let tile_layers: Vec<TileLayer> = parse_mvt_to_tile_layers(&mvt_files);

        group.throughput(Throughput::Bytes(total_bytes as u64));

        group.bench_with_input(BenchmarkId::new("zoom", zoom), &tile_layers, |b, layers| {
            b.iter_batched(
                // Setup: clone pre-parsed layers so encode (which takes `self`) can consume them.
                || layers.clone(),
                // Benchmark: encode every layer.
                |layers| {
                    let result: MltResult<Vec<Vec<u8>>> = layers
                        .into_iter()
                        .map(|layer| layer.encode(black_box(cfg)))
                        .collect();
                    black_box(result.expect("encode failed"));
                },
                BatchSize::LargeInput,
            );
        });
    }

    group.finish();
}

criterion_group!(benches, bench_encode_from_mvt);
criterion_main!(benches);
