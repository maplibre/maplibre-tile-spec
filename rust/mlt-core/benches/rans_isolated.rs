//! rANS vertex decode alone, over the streams the v2 encoder writes for each corpus.

use std::hint::black_box;

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use mlt_core::__private::rans::{decode_vertices, encode_vertices};
use mlt_core::__private::{Layer, ParsedLayer, dec, parser};
use mlt_core::encoder::{EncoderConfig, WireVersion};
use mlt_core::mvt::mvt_to_tile_layers;
use mlt_core::wire::{LogicalEncoding, VertexLogical};

#[path = "bench_utils.rs"]
mod bench_utils;
use bench_utils::load_all_tiles;

const CORPORA: [&str; 4] = ["omt", "amazon", "amazon_here", "bing"];

/// A rANS payload with the run offsets and word count it decodes against.
struct Stream {
    data: Vec<u8>,
    offsets: Option<Vec<u32>>,
    num_values: u32,
}

/// The v2 encoding `mlt convert --rans-vertices` writes.
fn cfg_rans() -> EncoderConfig {
    EncoderConfig::default()
        .with_wire_version(WireVersion::V02)
        .with_float_alp(true)
        .with_float_dict(true)
        .with_rans_vertices(true)
}

/// Re-encode the vertices of every layer the encoder wrote with rANS, which reproduces its payloads.
fn rans_streams(corpus: &str) -> Vec<Stream> {
    let mut d = dec();
    let mut streams = Vec::new();
    for (path, mvt) in load_all_tiles(&format!("fixtures/{corpus}"), ".mvt") {
        let tile: Vec<u8> = mvt_to_tile_layers(mvt)
            .unwrap_or_else(|e| panic!("{path}: {e}"))
            .into_iter()
            .flat_map(|layer| layer.encode(cfg_rans()).expect("encode failed"))
            .collect();
        for layer in parser().parse_layers(&tile).expect("mlt parse failed") {
            let Layer::Tag02(lazy) = &layer else { continue };
            let mut rans = false;
            lazy.for_each_stream(&mut |meta| {
                rans |= meta.encoding.logical == LogicalEncoding::Vertex(VertexLogical::Rans);
            });
            if !rans {
                continue;
            }
            d.reset_budget();
            let ParsedLayer::Tag02(parsed) = layer.decode_all(&mut d).expect("mlt decode failed")
            else {
                unreachable!("a v2 layer decodes to v2");
            };
            let geometry = parsed.layer().geometry_values();
            let vertices = geometry.vertices().expect("a rANS layer has vertices");
            let offsets = geometry
                .ring_offsets()
                .or(geometry.part_offsets())
                .or(geometry.geometry_offsets());
            streams.push(Stream {
                data: encode_vertices(vertices, offsets).expect("rANS encode failed"),
                offsets: offsets.map(<[u32]>::to_vec),
                num_values: u32::try_from(vertices.len()).expect("fewer than 2^32 words"),
            });
        }
    }
    streams
}

fn decode_all(streams: &[Stream]) {
    let mut d = dec();
    for s in streams {
        d.reset_budget();
        black_box(
            decode_vertices(&s.data, s.offsets.as_deref(), s.num_values, &mut d)
                .expect("rANS decode failed"),
        );
    }
}

fn bench_decode(c: &mut Criterion) {
    let mut group = c.benchmark_group("rans_isolated");
    for corpus in CORPORA {
        let streams = rans_streams(corpus);
        let vertices: u64 = streams.iter().map(|s| u64::from(s.num_values / 2)).sum();
        let bytes: usize = streams.iter().map(|s| s.data.len()).sum();
        eprintln!(
            "{corpus}: {} streams, {vertices} vertices, {bytes} B",
            streams.len()
        );
        group.throughput(Throughput::Elements(vertices));
        group.bench_with_input(BenchmarkId::from_parameter(corpus), &streams, |b, s| {
            b.iter(|| decode_all(s));
        });
    }
    group.finish();
}

criterion_group!(benches, bench_decode);
criterion_main!(benches);
