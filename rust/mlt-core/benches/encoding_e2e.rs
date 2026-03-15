use std::hint::black_box;

use criterion::{BatchSize, BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use mlt_core::v01::{
    EncodeProperties as _, GeometryEncoder, IdEncoder, IdWidth, IntEncoder, LogicalEncoder,
    PhysicalEncoder, PresenceStream, PropertyEncoder, PropertyKind, ScalarEncoder, StagedLayer01,
};
use mlt_core::{Decoder, Layer, MemBudget, StagedLayer, parse_layers};
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

/// Build `StagedLayer01` values from decoded tiles for encode benchmarks.
///
/// Goes through `Layer01 → TileLayer01 → StagedLayer01`, which is the correct
/// encode-pipeline entry point per CONTRIBUTING.md.
fn decode_to_owned(tiles: &[(String, Vec<u8>)]) -> Vec<StagedLayer> {
    tiles
        .iter()
        .flat_map(|(_, data)| {
            let mut dec = Decoder::default();
            let layers = parse_layers(data, &mut MemBudget::default()).expect("mlt parse failed");
            layers
                .into_iter()
                .filter_map(|layer| {
                    let Layer::Tag01(layer01) = layer else {
                        return None;
                    };
                    let tile = layer01.into_tile(&mut dec).ok()?;
                    Some(StagedLayer::Tag01(StagedLayer01::from(tile)))
                })
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

        for physical in limit(PhysicalEncoder::iter()) {
            for logical in limit(LogicalEncoder::iter()) {
                let geometry_encoder = GeometryEncoder::all(IntEncoder::new(logical, physical));
                group.bench_with_input(
                    BenchmarkId::new(format!("{logical:?}-{physical:?}"), zoom),
                    &tiles,
                    |b, tiles| {
                        b.iter_batched(
                            || decode_to_owned(tiles),
                            |layers| {
                                for layer in layers {
                                    if let StagedLayer::Tag01(l) = layer {
                                        black_box(
                                            l.geometry
                                                .encode(geometry_encoder)
                                                .expect("geometry encode failed"),
                                        );
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
                            || decode_to_owned(tiles),
                            |layers| {
                                for layer in layers {
                                    if let StagedLayer::Tag01(l) = layer {
                                        let Some(id) = l.id else { continue };
                                        black_box(id.encode(id_encoder).expect("id encode failed"));
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
                                || decode_to_owned(tiles),
                                |mut layers| {
                                    for layer in &mut layers {
                                        if let StagedLayer::Tag01(l) = layer {
                                            // Skip layers that contain SharedDict properties;
                                            // this bench focuses on scalar encoding only.
                                            if l.properties.iter().any(|p| {
                                                matches!(p.kind(), PropertyKind::SharedDict)
                                            }) {
                                                continue;
                                            }
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
                                                    PropertyKind::String => {
                                                        ScalarEncoder::str_fsst(
                                                            presence, int_enc, int_enc,
                                                        )
                                                        .into()
                                                    }
                                                    PropertyKind::SharedDict => unreachable!(),
                                                })
                                                .collect();
                                            let props = std::mem::take(&mut l.properties);
                                            let _ =
                                                props.encode(encoders).expect("prop encode failed");
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
