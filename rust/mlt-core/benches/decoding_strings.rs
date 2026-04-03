use std::hint::black_box;

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use geo_types::Point;
use mlt_core::encoder::{
    Encoder, EncoderConfig, ExplicitEncoder, IntEncoder, PhysicalEncoder, StagedLayer01,
    StagedProperty, StagedSharedDict, StrEncoding,
};
use mlt_core::geojson::Geom32;
use mlt_core::test_helpers::{dec, parser};
use mlt_core::{GeometryValues, Layer01, LogicalEncoder, ParsedLayer01, PropValueRef};
use strum::IntoEnumIterator as _;

// This code runs in CI because of --all-targets, so make it run really fast.
#[cfg(debug_assertions)]
pub const BENCHMARKED_LENGTHS: [usize; 1] = [1];
#[cfg(not(debug_assertions))]
pub const BENCHMARKED_LENGTHS: [usize; 6] = [1, 20, 64, 256, 1024, 2048];

fn limit<T>(values: impl Iterator<Item = T>) -> impl Iterator<Item = T> {
    if cfg!(debug_assertions) {
        values.take(1)
    } else {
        values.take(usize::MAX)
    }
}

/// Generate a mix of strings with repetition so dictionary encoding has something to compress.
/// The vocabulary is small relative to N, so values repeat often.
fn make_strings(n: usize) -> Vec<String> {
    const VOCAB: &[&str] = &[
        "highway",
        "residential",
        "motorway",
        "primary",
        "secondary",
        "tertiary",
        "water",
        "forest",
        "park",
        "building",
        "amenity",
        "shop",
        "landuse",
        "natural",
        "place",
        "boundary",
    ];
    black_box(
        (0..n)
            .map(|i| {
                let idx = i % VOCAB.len();
                if i.is_multiple_of(4) {
                    VOCAB[idx].to_string()
                } else {
                    format!("{}_{}", VOCAB[idx], i % 32)
                }
            })
            .collect(),
    )
}

/// Same pool as `make_strings`, but every 5th entry is `None` so the presence
/// stream has real work to do.
fn make_nullable_strings(n: usize) -> Vec<Option<String>> {
    black_box(
        make_strings(n)
            .into_iter()
            .enumerate()
            .map(|(i, s)| if i.is_multiple_of(5) { None } else { Some(s) })
            .collect(),
    )
}

/// Build `n` degenerate point features at the origin for use as layer geometry.
fn make_geometry(n: usize) -> GeometryValues {
    let mut g = GeometryValues::default();
    for _ in 0..n {
        g.push_geom(&Geom32::Point(Point::new(0, 0)));
    }
    g
}

/// Encode `props` into a single-layer tile with `n` point features and return wire bytes.
fn encode_layer(n: usize, props: Vec<StagedProperty>, cfg: ExplicitEncoder) -> Vec<u8> {
    let mut enc = Encoder::with_explicit(EncoderConfig::default(), cfg);
    StagedLayer01 {
        name: "bench".into(),
        extent: 4096,
        id: None,
        geometry: make_geometry(n),
        properties: props,
    }
    .encode_explicit(&mut enc)
    .expect("encode_layer failed");
    enc.into_raw_bytes()
}

/// Sum the byte lengths of all non-null string property values across all features.
///
/// Used as the benchmark measurement: the return value prevents the compiler from
/// optimizing away the iteration, and its magnitude is proportional to work done.
fn sum_str_lens(parsed: &ParsedLayer01<'_>) -> usize {
    parsed
        .iter_features()
        .map(|feat_res| {
            feat_res
                .unwrap()
                .iter_all_properties()
                .map(|v| {
                    if let Some(PropValueRef::Str(s)) = v {
                        s.len()
                    } else {
                        0
                    }
                })
                .sum::<usize>()
        })
        .sum()
}

/// plain strings: vary the `IntEncoder` used for the length stream
fn bench_plain_length_encoding(c: &mut Criterion) {
    let mut group = c.benchmark_group("strings/plain/length_enc");

    for n in BENCHMARKED_LENGTHS {
        let col: Vec<Option<String>> = make_strings(n).into_iter().map(Some).collect();
        group.throughput(Throughput::Elements(n as u64));

        for logical in limit(LogicalEncoder::iter()) {
            for physical in limit(PhysicalEncoder::iter()) {
                let int_enc = IntEncoder::new(logical, physical);
                let bytes = encode_layer(
                    n,
                    vec![StagedProperty::str("name", col.clone())],
                    ExplicitEncoder::all(int_enc),
                );

                group.bench_with_input(
                    BenchmarkId::new(format!("{logical:?}-{physical:?}"), n),
                    &bytes,
                    |b, bytes| {
                        b.iter(|| {
                            let layer = Layer01::from_bytes(bytes, &mut parser()).expect("parse");
                            let parsed = layer.decode_all(&mut dec()).expect("decode_all");
                            black_box(sum_str_lens(&parsed))
                        });
                    },
                );
            }
        }
    }

    group.finish();
}

/// FSST strings: vary the `IntEncoder` used for the symbol-length and value-length streams
fn bench_fsst_length_encoding(c: &mut Criterion) {
    let mut group = c.benchmark_group("strings/fsst/length_enc");

    for n in BENCHMARKED_LENGTHS {
        let col: Vec<Option<String>> = make_strings(n).into_iter().map(Some).collect();
        group.throughput(Throughput::Elements(n as u64));

        for logical in limit(LogicalEncoder::iter()) {
            for physical in limit(PhysicalEncoder::iter()) {
                let int_enc = IntEncoder::new(logical, physical);
                let bytes = encode_layer(
                    n,
                    vec![StagedProperty::str("name", col.clone())],
                    ExplicitEncoder::all_with_str(int_enc, StrEncoding::Fsst),
                );

                group.bench_with_input(
                    BenchmarkId::new(format!("{logical:?}-{physical:?}"), n),
                    &bytes,
                    |b, bytes| {
                        b.iter(|| {
                            let layer = Layer01::from_bytes(bytes, &mut parser()).expect("parse");
                            let parsed = layer.decode_all(&mut dec()).expect("decode_all");
                            black_box(sum_str_lens(&parsed))
                        });
                    },
                );
            }
        }
    }

    group.finish();
}

/// Benchmark 3 – encoding type: plain vs FSST, fixed `IntEncoder`
fn bench_encoding_type(c: &mut Criterion) {
    let mut group = c.benchmark_group("strings/encoding_type");
    let int_enc = IntEncoder::plain();

    for n in BENCHMARKED_LENGTHS {
        let col: Vec<Option<String>> = make_strings(n).into_iter().map(Some).collect();
        group.throughput(Throughput::Elements(n as u64));

        let plain_bytes = encode_layer(
            n,
            vec![StagedProperty::str("name", col.clone())],
            ExplicitEncoder::all(int_enc),
        );
        group.bench_with_input(BenchmarkId::new("plain", n), &plain_bytes, |b, bytes| {
            b.iter(|| {
                let layer = Layer01::from_bytes(bytes, &mut parser()).expect("parse");
                let parsed = layer.decode_all(&mut dec()).expect("decode_all");
                black_box(sum_str_lens(&parsed))
            });
        });

        let fsst_bytes = encode_layer(
            n,
            vec![StagedProperty::str("name", col)],
            ExplicitEncoder::all_with_str(int_enc, StrEncoding::Fsst),
        );
        group.bench_with_input(BenchmarkId::new("fsst", n), &fsst_bytes, |b, bytes| {
            b.iter(|| {
                let layer = Layer01::from_bytes(bytes, &mut parser()).expect("parse");
                let parsed = layer.decode_all(&mut dec()).expect("decode_all");
                black_box(sum_str_lens(&parsed))
            });
        });
    }

    group.finish();
}

/// Benchmark 4 – presence stream overhead: non-nullable vs nullable column
fn bench_presence(c: &mut Criterion) {
    let mut group = c.benchmark_group("strings/presence");
    let int_enc = IntEncoder::plain();

    for n in BENCHMARKED_LENGTHS {
        group.throughput(Throughput::Elements(n as u64));

        // Non-nullable: no presence stream emitted.
        let no_null_bytes = encode_layer(
            n,
            vec![StagedProperty::str(
                "name",
                make_strings(n).into_iter().map(Some).collect(),
            )],
            ExplicitEncoder::all(int_enc),
        );
        group.bench_with_input(
            BenchmarkId::new("no_nulls", n),
            &no_null_bytes,
            |b, bytes| {
                b.iter(|| {
                    let layer = Layer01::from_bytes(bytes, &mut parser()).expect("parse");
                    let parsed = layer.decode_all(&mut dec()).expect("decode_all");
                    black_box(sum_str_lens(&parsed))
                });
            },
        );

        // Nullable: presence stream present, every 5th entry is None.
        let null_bytes = encode_layer(
            n,
            vec![StagedProperty::str("name", make_nullable_strings(n))],
            ExplicitEncoder::all(int_enc),
        );
        group.bench_with_input(
            BenchmarkId::new("with_nulls", n),
            &null_bytes,
            |b, bytes| {
                b.iter(|| {
                    let layer = Layer01::from_bytes(bytes, &mut parser()).expect("parse");
                    let parsed = layer.decode_all(&mut dec()).expect("decode_all");
                    black_box(sum_str_lens(&parsed))
                });
            },
        );
    }

    group.finish();
}

/// Benchmark 5 – shared dict vs plain
///
/// Compares decoding two plain string columns against a shared-dictionary struct
/// column (plain and FSST flavours) that carries the same string data spread
/// across two child sub-properties.
/// Throughput is reported per *logical* string entry so all variants are comparable.
fn bench_vs_shared_dict(c: &mut Criterion) {
    let mut group = c.benchmark_group("strings/vs_shared_dict");
    let int_enc = IntEncoder::plain();

    for n in BENCHMARKED_LENGTHS {
        let total_entries = n * 2;
        group.throughput(Throughput::Elements(total_entries as u64));

        let strings = make_strings(n);
        let col: Vec<Option<String>> = strings.iter().map(|s| Some(s.clone())).collect();

        // --- plain: two independent string columns ---
        let plain_x2_bytes = encode_layer(
            n,
            vec![
                StagedProperty::str("col1", col.clone()),
                StagedProperty::str("col2", col.clone()),
            ],
            ExplicitEncoder::all(int_enc),
        );
        group.bench_with_input(
            BenchmarkId::new("plain_x2", n),
            &plain_x2_bytes,
            |b, bytes| {
                b.iter(|| {
                    let layer = Layer01::from_bytes(bytes, &mut parser()).expect("parse");
                    let parsed = layer.decode_all(&mut dec()).expect("decode_all");
                    black_box(sum_str_lens(&parsed))
                });
            },
        );

        // --- shared dict (plain) ---
        //
        // Two sub-properties; the second child has every 3rd entry as NULL so
        // the child presence path is exercised.
        let col2: Vec<Option<String>> = strings
            .iter()
            .enumerate()
            .map(|(i, s)| if i % 3 == 0 { None } else { Some(s.clone()) })
            .collect();
        let make_sd = || {
            StagedSharedDict::new("place:", [("type", col.clone()), ("subtype", col2.clone())])
                .expect("StagedSharedDict::new failed")
        };

        let sd_plain_bytes = encode_layer(
            n,
            vec![StagedProperty::SharedDict(make_sd())],
            ExplicitEncoder::all(int_enc),
        );
        group.bench_with_input(
            BenchmarkId::new("shared_dict_plain", n),
            &sd_plain_bytes,
            |b, bytes| {
                b.iter(|| {
                    let layer = Layer01::from_bytes(bytes, &mut parser()).expect("parse");
                    let parsed = layer.decode_all(&mut dec()).expect("decode_all");
                    black_box(sum_str_lens(&parsed))
                });
            },
        );

        // --- shared dict (FSST) ---
        let sd_fsst_bytes = encode_layer(
            n,
            vec![StagedProperty::SharedDict(make_sd())],
            ExplicitEncoder::all_with_str(int_enc, StrEncoding::Fsst),
        );
        group.bench_with_input(
            BenchmarkId::new("shared_dict_fsst", n),
            &sd_fsst_bytes,
            |b, bytes| {
                b.iter(|| {
                    let layer = Layer01::from_bytes(bytes, &mut parser()).expect("parse");
                    let parsed = layer.decode_all(&mut dec()).expect("decode_all");
                    black_box(sum_str_lens(&parsed))
                });
            },
        );

        // --- FSST plain: two independent FSST-encoded columns ---
        let fsst_x2_bytes = encode_layer(
            n,
            vec![
                StagedProperty::str("col1", col.clone()),
                StagedProperty::str("col2", col),
            ],
            ExplicitEncoder::all_with_str(int_enc, StrEncoding::Fsst),
        );
        group.bench_with_input(
            BenchmarkId::new("fsst_x2", n),
            &fsst_x2_bytes,
            |b, bytes| {
                b.iter(|| {
                    let layer = Layer01::from_bytes(bytes, &mut parser()).expect("parse");
                    let parsed = layer.decode_all(&mut dec()).expect("decode_all");
                    black_box(sum_str_lens(&parsed))
                });
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_plain_length_encoding,
    bench_fsst_length_encoding,
    bench_encoding_type,
    bench_presence,
    bench_vs_shared_dict,
);
criterion_main!(benches);
