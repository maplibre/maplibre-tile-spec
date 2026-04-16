use std::hint::black_box;

use criterion::{BatchSize, BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use mlt_core::__private::{dec, parser};
use mlt_core::Layer;
use mlt_core::encoder::{LogicalEncoder, SortStrategy};
use strum::IntoEnumIterator as _;

#[path = "bench_utils.rs"]
mod bench_utils;
use bench_utils::{BENCHMARKED_ZOOM_LEVELS, load_mlt_tiles};
use mlt_core::encoder::{
    Encoder, ExplicitEncoder, IntEncoder, PhysicalEncoder, StagedLayer, StagedLayer01,
};

fn limit<T>(values: impl Iterator<Item = T>) -> impl Iterator<Item = T> {
    if cfg!(debug_assertions) {
        values.take(1)
    } else {
        values.take(usize::MAX)
    }
}

/// Build `StagedLayer01` values from decoded tiles for encode benchmarks.
///
/// Goes through `Layer01 → TileLayer01 → StagedLayer01`, which is the correct
/// encode-pipeline entry point per CONTRIBUTING.md.
fn decode_to_owned(tiles: &[(String, Vec<u8>)]) -> Vec<StagedLayer> {
    tiles
        .iter()
        .flat_map(|(_, data)| {
            let mut d = dec();
            let layers = parser().parse_layers(data).expect("mlt parse failed");
            layers
                .into_iter()
                .filter_map(|layer| {
                    let Layer::Tag01(layer01) = layer else {
                        return None;
                    };
                    let tile = layer01.into_tile(&mut d).ok()?;
                    Some(StagedLayer::Tag01(StagedLayer01::from_tile(
                        tile,
                        SortStrategy::Unsorted,
                        &[],
                    )))
                })
                .collect::<Vec<_>>()
        })
        .collect()
}

fn bench_encode(c: &mut Criterion) {
    let mut group = c.benchmark_group("mlt encode");

    for zoom in BENCHMARKED_ZOOM_LEVELS {
        let tiles = load_mlt_tiles(zoom);
        let total_bytes: usize = tiles.iter().map(|(_, d)| d.len()).sum();
        group.throughput(Throughput::Bytes(total_bytes as u64));

        for physical in limit(PhysicalEncoder::iter()) {
            for logical in limit(LogicalEncoder::iter()) {
                let int_enc = IntEncoder::new(logical, physical);
                group.bench_with_input(
                    BenchmarkId::new(format!("{logical:?}-{physical:?}"), zoom),
                    &tiles,
                    |b, tiles| {
                        b.iter_batched(
                            || decode_to_owned(tiles),
                            |layers| {
                                for layer in layers {
                                    if let StagedLayer::Tag01(l) = layer {
                                        let enc = Encoder::with_explicit(
                                            Encoder::default().cfg,
                                            ExplicitEncoder::all(int_enc),
                                        );
                                        black_box(l.encode_into(enc).expect("encode failed"));
                                    }
                                }
                            },
                            BatchSize::SmallInput,
                        );
                    },
                );
            }
        }
    }

    group.finish();
}

criterion_group!(benches, bench_encode);
criterion_main!(benches);
