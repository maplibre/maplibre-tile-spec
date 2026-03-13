use std::hint::black_box;

use criterion::{BatchSize, BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use mlt_core::parse_layers;
use mlt_core::v01::property::optimizer::encode_properties;
use mlt_core::v01::tile::TileLayer01;
use mlt_core::v01::{
    GeometryEncoder, IdEncoder, IdWidth, IntEncoder, LogicalEncoder, PhysicalEncoder,
    PresenceStream, PropertyEncoder, PropertyKind, ScalarEncoder, StagedLayer01,
};
use strum::IntoEnumIterator as _;

#[path = "bench_utils.rs"]
mod bench_utils;
use bench_utils::{BENCHMARKED_ZOOM_LEVELS, load_mlt_tiles};

fn limit<T>(values: impl Iterator<Item = T>) -> impl Iterator<Item = T> {
    if cfg!(debug_assertions) {
        values.take(1)
    } else {
        values.take(usize::MAX)
    }
}

fn decode_to_staged(tiles: &[(String, Vec<u8>)]) -> Vec<StagedLayer01> {
    tiles
        .iter()
        .flat_map(|(_, data)| {
            let mut layers = parse_layers(data).expect("mlt parse failed");
            let mut staged = Vec::new();
            for layer in layers.drain(..) {
                let mlt_core::Layer::Tag01(mut layer01) = layer else {
                    continue;
                };
                layer01.decode_all().expect("mlt decode_all failed");
                let tile_layer = TileLayer01::from_layer01(layer01).expect("to tile layer failed");
                staged.push(StagedLayer01::from(tile_layer));
            }
            staged
        })
        .collect()
}

fn bench_encode_geometry(c: &mut Criterion) {
    let mut group = c.benchmark_group("mlt encode geometry");

    for zoom in BENCHMARKED_ZOOM_LEVELS {
        let tiles = load_mlt_tiles(zoom);
        let total_bytes: usize = tiles.iter().map(|(_, d)| d.len()).sum();
        group.throughput(Throughput::Bytes(total_bytes as u64));

        for physical in limit(PhysicalEncoder::iter()) {
            for logical in limit(LogicalEncoder::iter()) {
                let geometry_encoder = GeometryEncoder::all(IntEncoder::new(logical, physical));
                group.bench_with_input(
                    BenchmarkId::new(format!("{logical:?}-{physical:?}"), zoom),
                    &tiles,
                    |b, tiles| {
                        b.iter_batched(
                            || decode_to_staged(tiles),
                            |layers| {
                                for l in layers {
                                    l.geometry
                                        .encode(geometry_encoder)
                                        .expect("geometry encode failed");
                                }
                                black_box(());
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

fn bench_encode_ids(c: &mut Criterion) {
    let mut group = c.benchmark_group("mlt encode ids");

    for zoom in BENCHMARKED_ZOOM_LEVELS {
        let tiles = load_mlt_tiles(zoom);
        let total_bytes: usize = tiles.iter().map(|(_, d)| d.len()).sum();
        group.throughput(Throughput::Bytes(total_bytes as u64));

        for fmt in limit(IdWidth::iter()) {
            for logical in limit(LogicalEncoder::iter()) {
                let id_encoder = IdEncoder::new(logical, fmt);
                group.bench_with_input(
                    BenchmarkId::new(format!("{logical:?}-{fmt:?}"), zoom),
                    &tiles,
                    |b, tiles| {
                        b.iter_batched(
                            || decode_to_staged(tiles),
                            |layers| {
                                for l in layers {
                                    if let Some(id) = &l.id {
                                        id.encode(id_encoder).expect("id encode failed");
                                    }
                                }
                                black_box(());
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

fn bench_encode_properties(c: &mut Criterion) {
    let mut group = c.benchmark_group("mlt encode properties");

    for zoom in BENCHMARKED_ZOOM_LEVELS {
        let tiles = load_mlt_tiles(zoom);
        let total_bytes: usize = tiles.iter().map(|(_, d)| d.len()).sum();
        group.throughput(Throughput::Bytes(total_bytes as u64));

        for presence in limit(PresenceStream::iter()) {
            for physical in limit(PhysicalEncoder::iter()) {
                for logical in limit(LogicalEncoder::iter()) {
                    group.bench_with_input(
                        BenchmarkId::new(format!("{presence:?}-{logical:?}-{physical:?}"), zoom),
                        &tiles,
                        |b, tiles| {
                            b.iter_batched(
                                || decode_to_staged(tiles),
                                |layers| {
                                    for l in layers {
                                        let int_enc = IntEncoder::new(logical, physical);
                                        let encoders: Vec<PropertyEncoder> = l
                                            .properties
                                            .iter()
                                            .map(|prop| match prop.kind() {
                                                PropertyKind::Bool => {
                                                    ScalarEncoder::bool(presence).into()
                                                }
                                                PropertyKind::Integer => {
                                                    ScalarEncoder::int(presence, int_enc).into()
                                                }
                                                PropertyKind::Float => {
                                                    ScalarEncoder::float(presence).into()
                                                }
                                                PropertyKind::String => ScalarEncoder::str_fsst(
                                                    presence, int_enc, int_enc,
                                                )
                                                .into(),
                                                PropertyKind::SharedDict => {
                                                    unreachable!("unimplemented")
                                                }
                                            })
                                            .collect();
                                        encode_properties(&l.properties, encoders)
                                            .expect("prop encode failed");
                                    }
                                    black_box(());
                                },
                                BatchSize::SmallInput,
                            );
                        },
                    );
                }
            }
        }
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_encode_geometry,
    bench_encode_ids,
    bench_encode_properties
);
criterion_main!(benches);
