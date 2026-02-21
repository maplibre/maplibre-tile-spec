use std::fs;
use std::hint::black_box;
use std::path::Path;

use criterion::{BatchSize, BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use mlt_core::parse_layers;

const BENCHMARKED_ZOOM_LEVELS: [u8; 3] = [4, 7, 13];

fn load_mlt_tiles(zoom: u8) -> Vec<(String, Vec<u8>)> {
    load_tiles(zoom, "expected/tag0x01/omt", "mlt")
}

fn load_mvt_tiles(zoom: u8) -> Vec<(String, Vec<u8>)> {
    load_tiles(zoom, "fixtures/omt", "mvt")
}

fn load_tiles(zoom: u8, test_subpath: &str, extension: &str) -> Vec<(String, Vec<u8>)> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../test")
        .join(test_subpath);
    let prefix = format!("{zoom}_");
    let mut tiles = Vec::new();
    let entries = fs::read_dir(&dir).unwrap_or_else(|_| panic!("can't read {}", dir.display()));
    for entry in entries.flatten() {
        let file_name = entry.file_name();
        let name = file_name.to_string_lossy();
        if name.starts_with(&prefix)
            && let Some(name) = name.strip_suffix(extension)
            && let Ok(data) = fs::read(entry.path())
        {
            tiles.push((name.to_string(), data));
        }
    }
    assert!(
        !tiles.is_empty(),
        "No tiles found for zoom level {zoom} in {}",
        dir.display()
    );
    tiles.sort_by(|a, b| a.0.cmp(&b.0));
    tiles
}

fn bench_mlt_parse(c: &mut Criterion) {
    let mut group = c.benchmark_group("mlt parse (no decode_all)");

    for zoom in BENCHMARKED_ZOOM_LEVELS {
        let tiles = load_mlt_tiles(zoom);
        if tiles.is_empty() {
            continue;
        }

        let total_bytes: usize = tiles.iter().map(|(_, d)| d.len()).sum();
        group.throughput(Throughput::Bytes(total_bytes as u64));

        group.bench_with_input(BenchmarkId::new("zoom", zoom), &tiles, |b, tiles| {
            b.iter(|| {
                for (_, data) in tiles {
                    let _ = parse_layers(black_box(data)).expect("mlt parse failed");
                }
            });
        });
    }

    group.finish();
}

fn bench_mlt_decode_all(c: &mut Criterion) {
    let mut group = c.benchmark_group("mlt decode_all (no parse)");

    for zoom in BENCHMARKED_ZOOM_LEVELS {
        let tiles = load_mlt_tiles(zoom);
        if tiles.is_empty() {
            continue;
        }

        let total_bytes: usize = tiles.iter().map(|(_, d)| d.len()).sum();
        group.throughput(Throughput::Bytes(total_bytes as u64));

        group.bench_with_input(BenchmarkId::new("zoom", zoom), &tiles, |b, tiles| {
            for (_, data) in tiles {
                let mut layers = parse_layers(black_box(data)).expect("mlt parse failed");
                b.iter(|| {
                    for layer in &mut layers {
                        layer.decode_all().expect("mlt decode_all failed");
                    }
                });
                black_box(layers);
            }
        });
    }

    group.finish();
}

fn bench_mvt_parse(c: &mut Criterion) {
    let mut group = c.benchmark_group("mvt parse");

    for zoom in BENCHMARKED_ZOOM_LEVELS {
        let tiles = load_mvt_tiles(zoom);

        let total_bytes: usize = tiles.iter().map(|(_, d)| d.len()).sum();
        group.throughput(Throughput::Bytes(total_bytes as u64));

        group.bench_with_input(BenchmarkId::new("zoom", zoom), &tiles, |b, tiles| {
            for (_id, tile) in tiles {
                b.iter_batched(
                    || tile.clone(),
                    |data| {
                        let reader = mvt_reader::Reader::new(black_box(data))
                            .expect("mvt reader construction failed");
                        let _ = black_box(reader);
                    },
                    BatchSize::LargeInput,
                );
            }
        });
    }

    group.finish();
}

fn bench_mvt_decode_all(c: &mut Criterion) {
    let mut group = c.benchmark_group("mvt decode_all (no parse)");

    for zoom in BENCHMARKED_ZOOM_LEVELS {
        let tiles = load_mvt_tiles(zoom);

        let total_bytes: usize = tiles.iter().map(|(_, d)| d.len()).sum();
        group.throughput(Throughput::Bytes(total_bytes as u64));

        group.bench_with_input(BenchmarkId::new("zoom", zoom), &tiles, |b, tiles| {
            for (_id, tile) in tiles {
                b.iter_batched(
                    || {
                        mvt_reader::Reader::new(tile.clone())
                            .expect("mvt reader construction failed")
                    },
                    |reader| {
                        let layers = reader
                            .get_layer_metadata()
                            .expect("mvt layer metadata failed");
                        for layer in &layers {
                            let features = reader
                                .get_features(layer.layer_index)
                                .expect("mvt get_features failed");
                            let _ = black_box(features);
                        }
                        let _ = black_box(reader);
                    },
                    BatchSize::LargeInput,
                );
            }
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_mlt_parse,
    bench_mlt_decode_all,
    bench_mvt_parse,
    bench_mvt_decode_all,
);
criterion_main!(benches);
