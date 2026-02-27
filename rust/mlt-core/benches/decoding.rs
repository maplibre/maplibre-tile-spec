use std::hint::black_box;

use criterion::{BatchSize, BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use mlt_core::parse_layers;

#[path = "bench_utils.rs"]
mod bench_utils;
use bench_utils::{BENCHMARKED_ZOOM_LEVELS, load_mlt_tiles, load_tiles};

fn load_mvt_tiles(zoom: u8) -> Vec<(String, Vec<u8>)> {
    load_tiles(zoom, "fixtures/omt", ".mvt")
}

fn bench_mlt_parse(c: &mut Criterion) {
    let mut group = c.benchmark_group("mlt parse (no decode_all)");

    for zoom in BENCHMARKED_ZOOM_LEVELS {
        let tiles = load_mlt_tiles(zoom);
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
        let total_bytes: usize = tiles.iter().map(|(_, d)| d.len()).sum();
        group.throughput(Throughput::Bytes(total_bytes as u64));

        group.bench_with_input(BenchmarkId::new("zoom", zoom), &tiles, |b, tiles| {
            b.iter_batched(
                || {
                    tiles
                        .iter()
                        .map(|(_, v)| parse_layers(black_box(v)).expect("mlt parse failed"))
                        .collect::<Vec<_>>()
                },
                |mut mlt| {
                    for layers in &mut mlt {
                        for layer in layers.iter_mut() {
                            layer.decode_all().expect("mlt decode_all failed");
                        }
                    }
                    black_box(mlt);
                },
                BatchSize::SmallInput,
            );
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
            b.iter_batched(
                || {
                    tiles
                        .iter()
                        .map(|(_, tile)| black_box(tile))
                        .cloned()
                        .collect::<Vec<_>>()
                },
                |mvt| {
                    for data in mvt {
                        let reader = mvt_reader::Reader::new(black_box(data))
                            .expect("mvt reader construction failed");
                        let _ = black_box(reader);
                    }
                },
                BatchSize::LargeInput,
            );
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
            b.iter_batched(
                || {
                    tiles
                        .iter()
                        .map(|(_, tile)| {
                            mvt_reader::Reader::new(tile.clone())
                                .expect("mvt reader construction failed")
                        })
                        .collect::<Vec<_>>()
                },
                |readers| {
                    for reader in readers {
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
                    }
                },
                BatchSize::LargeInput,
            );
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
