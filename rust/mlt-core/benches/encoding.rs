use std::hint::black_box;

use criterion::{BatchSize, BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use mlt_core::v01::{
    AproxPropertyType, GeometryEncoder, IdEncoder, IdWidth, IntegerEncoder, LogicalEncoder,
    PhysicalEncoder, PresenceStream, ScalarEncoder,
};
use mlt_core::{Encodable as _, OwnedLayer, parse_layers};
use strum::IntoEnumIterator as _;

#[path = "bench_utils.rs"]
mod bench_utils;
use bench_utils::{BENCHMARKED_ZOOM_LEVELS, load_mlt_tiles};

fn decode_to_owned(tiles: &[(String, Vec<u8>)]) -> Vec<OwnedLayer> {
    tiles
        .iter()
        .flat_map(|(_, data)| {
            let mut layers = parse_layers(data).expect("mlt parse failed");
            for layer in &mut layers {
                layer.decode_all().expect("mlt decode_all failed");
            }
            layers
                .iter()
                .map(borrowme::ToOwned::to_owned)
                .collect::<Vec<_>>()
        })
        .collect()
}

fn bench_encode_geometry(c: &mut Criterion) {
    let mut group = c.benchmark_group("mlt encode geometry");

    for zoom in BENCHMARKED_ZOOM_LEVELS {
        let tiles = load_mlt_tiles(zoom);
        let total_bytes: usize = tiles.iter().map(|(_, d)| d.len()).sum();
        group.throughput(Throughput::Bytes(total_bytes as u64));

        for physical in PhysicalEncoder::iter() {
            for logical in LogicalEncoder::iter() {
                let geometry_encoder = GeometryEncoder::all(IntegerEncoder::new(logical, physical));
                group.bench_with_input(
                    BenchmarkId::new(format!("{logical:?}-{physical:?}"), zoom),
                    &tiles,
                    |b, tiles| {
                        b.iter_batched(
                            || decode_to_owned(tiles),
                            |mut layers| {
                                for layer in &mut layers {
                                    if let OwnedLayer::Tag01(l) = layer {
                                        l.geometry
                                            .encode_with(geometry_encoder)
                                            .expect("geometry encode failed");
                                    }
                                }
                                black_box(layers);
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

        for fmt in IdWidth::iter() {
            for logical in LogicalEncoder::iter() {
                let id_encoder = IdEncoder::new(logical, fmt);
                group.bench_with_input(
                    BenchmarkId::new(format!("{logical:?}-{fmt:?}"), zoom),
                    &tiles,
                    |b, tiles| {
                        b.iter_batched(
                            || decode_to_owned(tiles),
                            |mut layers| {
                                for layer in &mut layers {
                                    if let OwnedLayer::Tag01(l) = layer {
                                        l.id.encode_with(id_encoder).expect("id encode failed");
                                    }
                                }
                                black_box(layers);
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

        for presence in PresenceStream::iter() {
            for physical in PhysicalEncoder::iter() {
                for logical in LogicalEncoder::iter() {
                    group.bench_with_input(
                        BenchmarkId::new(format!("{presence:?}-{logical:?}-{physical:?}"), zoom),
                        &tiles,
                        |b, tiles| {
                            b.iter_batched(
                                || decode_to_owned(tiles),
                                |mut layers| {
                                    for layer in &mut layers {
                                        if let OwnedLayer::Tag01(l) = layer {
                                            for prop in &mut l.properties {
                                                let int_enc =
                                                    IntegerEncoder::new(logical, physical);
                                                let enc = match prop.approx_type() {
                                                    AproxPropertyType::Bool => {
                                                        ScalarEncoder::bool(presence)
                                                    }
                                                    AproxPropertyType::Integer => {
                                                        ScalarEncoder::int(presence, int_enc)
                                                    }
                                                    AproxPropertyType::Float => {
                                                        ScalarEncoder::float(presence)
                                                    }
                                                    AproxPropertyType::String => {
                                                        ScalarEncoder::str_fsst(
                                                            presence, int_enc, int_enc,
                                                        )
                                                    }
                                                    AproxPropertyType::Struct => {
                                                        unreachable!("unimplemented")
                                                    }
                                                };

                                                prop.encode_with(enc).expect("prop encode failed");
                                            }
                                        }
                                    }
                                    black_box(layers);
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
    bench_encode_properties,
);
criterion_main!(benches);
