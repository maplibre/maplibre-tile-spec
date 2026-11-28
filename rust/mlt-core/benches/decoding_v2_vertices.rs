//! v2 decode without gunzip against the gunzip + MVT decode it has to beat.

use std::hint::black_box;

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use mlt_core::__private::{dec, parser};
use mlt_core::TileLayer;
use mlt_core::encoder::{EncoderConfig, Network, PredictedVertices, Profile, WireVersion};
use mlt_core::mvt::mvt_to_tile_layers;
use usize_cast::FromUsize as _;

#[path = "bench_utils.rs"]
mod bench_utils;
use crate::bench_utils::{compress_gzip, decompress_gzip, load_all_tiles, mvt_decode, total_bytes};

const CORPORA: [&str; 4] = ["omt", "amazon", "amazon_here", "bing"];

/// The v2 encoding `mlt convert --mlt-version 2` writes by default.
fn cfg_v2() -> EncoderConfig {
    EncoderConfig::from_profile(Profile::Native(Network::Cellular4G))
        .with_wire_version(WireVersion::V02)
        .with_float_alp(true)
}

/// Every decode config, in the order its ids appear in the report.
fn configs() -> Vec<(&'static str, EncoderConfig)> {
    vec![
        ("mlt-v2", cfg_v2()),
        (
            "mlt-v2-componentwise-delta",
            cfg_v2().with_predicted_vertices(PredictedVertices::Off),
        ),
        ("mlt-v2-vertex-sort", cfg_v2().with_vertex_sort(true)),
        (
            "mlt-v2-vertex-sort-chain",
            cfg_v2().with_vertex_sort(true).with_chain_sort(true),
        ),
        ("mlt-v2-chain", cfg_v2().with_chain_sort(true)),
    ]
}

fn parse_mvt(path: &str, data: &[u8]) -> Vec<TileLayer> {
    mvt_to_tile_layers(data).unwrap_or_else(|e| panic!("{path}: {e}"))
}

fn vertex_count(mvt: &[(String, Vec<u8>)]) -> u64 {
    let mut vertices = 0;
    for (path, data) in mvt {
        for layer in &parse_mvt(path, data) {
            vertices += layer
                .features()
                .iter()
                .map(|f| u64::from_usize(f.vertex_count()))
                .sum::<u64>();
        }
    }
    vertices
}

fn encode(mvt: &[(String, Vec<u8>)], cfg: EncoderConfig) -> Vec<(String, Vec<u8>)> {
    mvt.iter()
        .map(|(path, data)| {
            let tile = parse_mvt(path, data)
                .into_iter()
                .flat_map(|layer| layer.encode(cfg).expect("encode failed"))
                .collect();
            (path.clone(), tile)
        })
        .collect()
}

fn mlt_decode(tiles: &[(String, Vec<u8>)]) {
    let mut d = dec();
    for (_, tile) in tiles {
        for layer in parser().parse_layers(tile).expect("mlt parse failed") {
            d.reset_budget();
            black_box(layer.decode_all(&mut d).expect("mlt decode failed"));
        }
    }
}

fn bench_decode(c: &mut Criterion) {
    let mut group = c.benchmark_group("decode_v2_vertices");
    for corpus in CORPORA {
        let mvt = load_all_tiles(&format!("fixtures/{corpus}"), ".mvt");
        let vertices = vertex_count(&mvt);
        let gz: Vec<(String, Vec<u8>)> = mvt
            .iter()
            .map(|(path, data)| (path.clone(), compress_gzip(data)))
            .collect();
        eprintln!(
            "{corpus}: {vertices} vertices, mvt+gzip {} B",
            total_bytes(&gz)
        );
        group.throughput(Throughput::Elements(vertices));
        group.bench_with_input(BenchmarkId::new("mvt+gzip", corpus), &gz, |b, t| {
            b.iter(|| {
                for (_, tile) in t {
                    mvt_decode(&decompress_gzip(tile));
                }
            });
        });
        for (id, cfg) in configs() {
            let tiles = encode(&mvt, cfg);
            eprintln!("{corpus}: {id} {} B", total_bytes(&tiles));
            group.bench_with_input(BenchmarkId::new(id, corpus), &tiles, |b, t| {
                b.iter(|| mlt_decode(t));
            });
        }
    }
    group.finish();
}

criterion_group!(benches, bench_decode);
criterion_main!(benches);
