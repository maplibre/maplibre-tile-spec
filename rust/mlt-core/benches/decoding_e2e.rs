use std::hint::black_box;
use std::io::{Read as _, Write as _};

use criterion::{BatchSize, BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use mlt_core::__private::{dec, parser};

#[path = "bench_utils.rs"]
mod bench_utils;
use bench_utils::{BENCHMARKED_ZOOM_LEVELS, load_mlt_tiles, load_tiles, total_bytes};

fn load_proto_tiles(zoom: u8) -> Vec<(String, Vec<u8>)> {
    load_tiles(zoom, "fixtures/omt", ".mvt")
}

fn compress_gzip(data: &[u8]) -> Vec<u8> {
    let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    encoder.write_all(data).expect("gzip compress failed");
    encoder.finish().expect("gzip finish failed")
}

fn decompress_gzip(data: &[u8]) -> Vec<u8> {
    let mut decoder = flate2::read::GzDecoder::new(data);
    let mut out = Vec::new();
    decoder
        .read_to_end(&mut out)
        .expect("gzip decompress failed");
    out
}

fn compress_zstd(data: &[u8]) -> Vec<u8> {
    zstd::encode_all(data, 3).expect("zstd compress failed")
}

fn decompress_zstd(data: &[u8]) -> Vec<u8> {
    zstd::decode_all(data).expect("zstd decompress failed")
}

fn compress_brotli(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    // quality 6, lgwin 22 (brotli defaults)
    let mut encoder = brotli::CompressorWriter::new(&mut out, 4096, 6, 22);
    encoder.write_all(data).expect("brotli compress failed");
    drop(encoder);
    out
}

fn decompress_brotli(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut decoder = brotli::Decompressor::new(data, 4096);
    decoder
        .read_to_end(&mut out)
        .expect("brotli decompress failed");
    out
}

fn compress_tiles(
    tiles: &[(String, Vec<u8>)],
    compress: fn(&[u8]) -> Vec<u8>,
) -> Vec<(String, Vec<u8>)> {
    tiles
        .iter()
        .map(|(name, data)| (name.clone(), compress(data)))
        .collect()
}

fn mvt_parse(data: Vec<u8>) {
    let reader = mvt_reader::Reader::new(black_box(data)).expect("mvt reader construction failed");
    let _ = black_box(reader);
}

fn mvt_decode(data: Vec<u8>) {
    let reader = mvt_reader::Reader::new(black_box(data)).expect("mvt reader construction failed");
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

type Codec = (&'static str, fn(&[u8]) -> Vec<u8>, fn(&[u8]) -> Vec<u8>);

fn identity(data: &[u8]) -> Vec<u8> {
    data.to_vec()
}

const CODECS: &[Codec] = &[
    ("none", identity, identity),
    ("gzip", compress_gzip, decompress_gzip),
    ("zstd", compress_zstd, decompress_zstd),
    ("brotli", compress_brotli, decompress_brotli),
];

fn bench_parse(c: &mut Criterion) {
    let mut group = c.benchmark_group("parse");

    for zoom in BENCHMARKED_ZOOM_LEVELS {
        let mlt_tiles = load_mlt_tiles(zoom);
        let proto_tiles = load_proto_tiles(zoom);

        // mlt parse
        group.throughput(Throughput::Bytes(total_bytes(&mlt_tiles) as u64));
        group.bench_with_input(BenchmarkId::new("mlt", zoom), &mlt_tiles, |b, tiles| {
            b.iter(|| {
                for (_, data) in tiles {
                    black_box(
                        parser()
                            .parse_layers(black_box(data))
                            .expect("mlt parse failed"),
                    );
                }
            });
        });

        // mvt parse (per codec)
        for &(codec_name, compress, decompress) in CODECS {
            let compressed = compress_tiles(&proto_tiles, compress);
            group.throughput(Throughput::Bytes(total_bytes(&compressed) as u64));
            group.bench_with_input(
                BenchmarkId::new(format!("mvt+{codec_name}"), zoom),
                &compressed,
                |b, tiles| {
                    b.iter_batched(
                        || tiles.iter().map(|(_, d)| d.clone()).collect::<Vec<_>>(),
                        |compressed_data| {
                            for data in compressed_data {
                                mvt_parse(decompress(black_box(&data)));
                            }
                        },
                        BatchSize::LargeInput,
                    );
                },
            );
        }
    }

    group.finish();
}

fn bench_decode_all(c: &mut Criterion) {
    let mut group = c.benchmark_group("decode_all");

    for zoom in BENCHMARKED_ZOOM_LEVELS {
        let mlt_tiles = load_mlt_tiles(zoom);
        let proto_tiles = load_proto_tiles(zoom);

        // mlt decode_all
        group.throughput(Throughput::Bytes(total_bytes(&mlt_tiles) as u64));
        group.bench_with_input(BenchmarkId::new("mlt", zoom), &mlt_tiles, |b, tiles| {
            b.iter_batched(
                || {
                    tiles
                        .iter()
                        .map(|(_, v)| {
                            parser()
                                .parse_layers(black_box(v))
                                .expect("mlt parse failed")
                        })
                        .collect::<Vec<_>>()
                },
                |mlt| {
                    let mut d = dec();
                    let decoded: Vec<Vec<_>> = mlt
                        .into_iter()
                        .map(|layers| {
                            d.reset_budget();
                            let dec_tile = d.decode_all(layers).expect("mlt decode_all failed");
                            black_box(dec_tile)
                        })
                        .collect();
                    black_box(decoded);
                },
                BatchSize::SmallInput,
            );
        });

        // mvt decode_all (per codec)
        for &(codec_name, compress, decompress) in CODECS {
            let compressed = compress_tiles(&proto_tiles, compress);
            group.throughput(Throughput::Bytes(total_bytes(&compressed) as u64));
            group.bench_with_input(
                BenchmarkId::new(format!("mvt+{codec_name}"), zoom),
                &compressed,
                |b, tiles| {
                    b.iter_batched(
                        || tiles.iter().map(|(_, d)| d.clone()).collect::<Vec<_>>(),
                        |compressed_data| {
                            for data in compressed_data {
                                mvt_decode(decompress(black_box(&data)));
                            }
                        },
                        BatchSize::LargeInput,
                    );
                },
            );
        }
    }

    group.finish();
}

criterion_group!(benches, bench_parse, bench_decode_all);
criterion_main!(benches);
