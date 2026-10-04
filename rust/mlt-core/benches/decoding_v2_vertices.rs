//! v2 decode with and without rANS vertices, against the gunzip + MVT decode it has to beat.

use std::hint::black_box;

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use mlt_core::__private::{dec, parser};
use mlt_core::encoder::{EncoderConfig, WireVersion};
use mlt_core::mvt::mvt_to_tile_layers;

#[path = "bench_utils.rs"]
mod bench_utils;
use bench_utils::{compress_gzip, decompress_gzip, load_all_tiles, mvt_decode, total_bytes};

const CORPORA: [&str; 4] = ["omt", "amazon", "amazon_here", "bing"];

/// The v2 encoding `mlt convert --mlt-version 2` writes by default.
fn cfg_v2() -> EncoderConfig {
    EncoderConfig::default()
        .with_wire_version(WireVersion::V02)
        .with_spatial_hilbert_sort(false)
        .with_id_sort(false)
        .with_float_alp(true)
        .with_float_dict(true)
}

fn encode(mvt: &[(String, Vec<u8>)], cfg: EncoderConfig) -> Vec<(String, Vec<u8>)> {
    mvt.iter()
        .map(|(path, data)| {
            let layers = mvt_to_tile_layers(data.clone()).unwrap_or_else(|e| panic!("{path}: {e}"));
            let tile = layers
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
        let gz: Vec<(String, Vec<u8>)> = mvt
            .iter()
            .map(|(path, data)| (path.clone(), compress_gzip(data)))
            .collect();
        let plain = encode(&mvt, cfg_v2());
        let rans = encode(&mvt, cfg_v2().with_rans_vertices(true));
        eprintln!(
            "{corpus}: mvt+gzip {} B, v2 {} B, v2+rans {} B",
            total_bytes(&gz),
            total_bytes(&plain),
            total_bytes(&rans)
        );
        group.bench_with_input(BenchmarkId::new("mvt+gzip", corpus), &gz, |b, t| {
            b.iter(|| {
                for (_, tile) in t {
                    mvt_decode(&decompress_gzip(tile));
                }
            });
        });
        group.bench_with_input(BenchmarkId::new("mlt-v2", corpus), &plain, |b, t| {
            b.iter(|| mlt_decode(t));
        });
        group.bench_with_input(BenchmarkId::new("mlt-v2+rans", corpus), &rans, |b, t| {
            b.iter(|| mlt_decode(t));
        });
    }
    group.finish();
}

criterion_group!(benches, bench_decode);
criterion_main!(benches);
