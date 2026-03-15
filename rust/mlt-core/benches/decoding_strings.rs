use std::hint::black_box;

use criterion::{BatchSize, BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use mlt_core::Decoder;

fn dec() -> Decoder {
    Decoder::default()
}

use mlt_core::v01::{
    DictionaryType, EncodedProperty, EncodedSharedDict, EncodedSharedDictEncoding, EncodedStream,
    EncodedStringsEncoding, IntEncoder, LengthType, LogicalEncoder, PhysicalEncoder,
    PresenceStream, RawFsstData, RawPlainData, RawPresence, RawSharedDict, RawSharedDictEncoding,
    RawSharedDictItem, RawStream, RawStrings, RawStringsEncoding, SharedDictEncoder,
    SharedDictItemEncoder, StagedStrings, StrEncoder, build_staged_shared_dict,
    encode_shared_dict_prop,
};
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

fn encode_plain(strings: &[String], int_enc: IntEncoder) -> EncodedStringsEncoding {
    EncodedStream::encode_strings_with_type(
        strings,
        int_enc,
        LengthType::VarBinary,
        DictionaryType::None,
    )
    .expect("encode_plain failed")
}

fn encode_fsst(strings: &[String], int_enc: IntEncoder) -> EncodedStringsEncoding {
    // StrEncoder::fsst builds the FsstStrEncoder internally; its fields are private.
    let StrEncoder::Fsst(enc) = StrEncoder::fsst(int_enc, int_enc) else {
        unreachable!()
    };
    EncodedStream::encode_strings_fsst_with_type(strings, enc, DictionaryType::Single)
        .expect("encode_fsst failed")
}

/// Borrow an [`EncodedStringsEncoding`] as [`RawStrings<'_>`] for decoding.
/// This is the benchmark-only helper: owned data stays in the outer scope,
/// and only leaf-level `EncodedStream::as_borrowed()` calls are made.
fn borrow_enc_strings<'a>(
    enc: &'a EncodedStringsEncoding,
    name: &'a str,
    presence: Option<RawStream<'a>>,
) -> RawStrings<'a> {
    let encoding = match enc {
        EncodedStringsEncoding::Plain(d) => RawStringsEncoding::Plain(RawPlainData {
            lengths: d.lengths.as_borrowed(),
            data: d.data.as_borrowed(),
        }),
        EncodedStringsEncoding::Dictionary {
            plain_data,
            offsets,
        } => RawStringsEncoding::Dictionary {
            plain_data: RawPlainData {
                lengths: plain_data.lengths.as_borrowed(),
                data: plain_data.data.as_borrowed(),
            },
            offsets: offsets.as_borrowed(),
        },
        EncodedStringsEncoding::FsstPlain(d) => RawStringsEncoding::FsstPlain(RawFsstData {
            symbol_lengths: d.symbol_lengths.as_borrowed(),
            symbol_table: d.symbol_table.as_borrowed(),
            lengths: d.lengths.as_borrowed(),
            corpus: d.corpus.as_borrowed(),
        }),
        EncodedStringsEncoding::FsstDictionary { fsst_data, offsets } => {
            RawStringsEncoding::FsstDictionary {
                fsst_data: RawFsstData {
                    symbol_lengths: fsst_data.symbol_lengths.as_borrowed(),
                    symbol_table: fsst_data.symbol_table.as_borrowed(),
                    lengths: fsst_data.lengths.as_borrowed(),
                    corpus: fsst_data.corpus.as_borrowed(),
                },
                offsets: offsets.as_borrowed(),
            }
        }
    };
    RawStrings {
        name,
        presence: RawPresence(presence),
        encoding,
    }
}

/// Borrow an [`EncodedSharedDict`] as [`RawSharedDict<'_>`] for decoding.
fn borrow_owned_shared_dict(sd: &EncodedSharedDict) -> RawSharedDict<'_> {
    let encoding = match &sd.encoding {
        EncodedSharedDictEncoding::Plain(d) => RawSharedDictEncoding::Plain(RawPlainData {
            lengths: d.lengths.as_borrowed(),
            data: d.data.as_borrowed(),
        }),
        EncodedSharedDictEncoding::FsstPlain(d) => RawSharedDictEncoding::FsstPlain(
            RawFsstData::new(
                d.symbol_lengths.as_borrowed(),
                d.symbol_table.as_borrowed(),
                d.lengths.as_borrowed(),
                d.corpus.as_borrowed(),
            )
            .expect("RawFsstData::new failed"),
        ),
    };
    let children = sd
        .children
        .iter()
        .map(|c| RawSharedDictItem {
            name: &c.name.0,
            presence: RawPresence(c.presence.0.as_ref().map(|s| s.as_borrowed())),
            data: c.data.as_borrowed(),
        })
        .collect();
    RawSharedDict {
        name: &sd.name.0,
        encoding,
        children,
    }
}

/// plain strings: vary the `IntEncoder` used for the length stream
fn bench_plain_length_encoding(c: &mut Criterion) {
    let mut group = c.benchmark_group("strings/plain/length_enc");

    for n in BENCHMARKED_LENGTHS {
        let strings = make_strings(n);
        group.throughput(Throughput::Elements(n as u64));

        for logical in limit(LogicalEncoder::iter()) {
            for physical in limit(PhysicalEncoder::iter()) {
                let int_enc = IntEncoder::new(logical, physical);
                let encoded = encode_plain(&strings, int_enc);

                group.bench_with_input(
                    BenchmarkId::new(format!("{logical:?}-{physical:?}"), n),
                    &encoded,
                    |b, encoded| {
                        b.iter(|| {
                            black_box(
                                borrow_enc_strings(encoded, "", None)
                                    .decode(&mut dec())
                                    .unwrap(),
                            )
                        });
                    },
                );
            }
        }
    }

    group.finish();
}

/// FSST strings: vary the `IntEncoder` used for the symbol-length and value-length streams inside the FSST block.
fn bench_fsst_length_encoding(c: &mut Criterion) {
    let mut group = c.benchmark_group("strings/fsst/length_enc");

    for n in BENCHMARKED_LENGTHS {
        let strings = make_strings(n);
        group.throughput(Throughput::Elements(n as u64));

        for logical in limit(LogicalEncoder::iter()) {
            for physical in limit(PhysicalEncoder::iter()) {
                let int_enc = IntEncoder::new(logical, physical);
                let encoded = encode_fsst(&strings, int_enc);

                group.bench_with_input(
                    BenchmarkId::new(format!("{logical:?}-{physical:?}"), n),
                    &encoded,
                    |b, encoded| {
                        b.iter(|| {
                            black_box(
                                borrow_enc_strings(encoded, "", None)
                                    .decode(&mut dec())
                                    .unwrap(),
                            )
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
    let int_enc = IntEncoder::new(LogicalEncoder::None, PhysicalEncoder::None);

    for n in BENCHMARKED_LENGTHS {
        let strings = make_strings(n);
        group.throughput(Throughput::Elements(n as u64));

        let plain = encode_plain(&strings, int_enc);
        group.bench_with_input(BenchmarkId::new("plain", n), &plain, |b, encoded| {
            b.iter(|| {
                black_box(
                    borrow_enc_strings(encoded, "", None)
                        .decode(&mut dec())
                        .unwrap(),
                )
            });
        });

        let fsst = encode_fsst(&strings, int_enc);
        group.bench_with_input(BenchmarkId::new("fsst", n), &fsst, |b, encoded| {
            b.iter(|| {
                black_box(
                    borrow_enc_strings(encoded, "", None)
                        .decode(&mut dec())
                        .unwrap(),
                )
            });
        });
    }

    group.finish();
}

/// Benchmark 4 – presence stream overhead: non-nullable vs nullable column
fn bench_presence(c: &mut Criterion) {
    let mut group = c.benchmark_group("strings/presence");
    let int_enc = IntEncoder::new(LogicalEncoder::None, PhysicalEncoder::None);

    for n in BENCHMARKED_LENGTHS {
        group.throughput(Throughput::Elements(n as u64));

        // Non-nullable: no presence stream at all.
        let strings = make_strings(n);
        let enc_no_nulls = encode_plain(&strings, int_enc);

        group.bench_with_input(
            BenchmarkId::new("no_nulls", n),
            &enc_no_nulls,
            |b, encoded| {
                b.iter(|| {
                    black_box(
                        borrow_enc_strings(encoded, "", None)
                            .decode(&mut dec())
                            .unwrap(),
                    )
                });
            },
        );

        // Nullable: attach a presence bitmap and only encode the non-null values.
        let nullable = StagedStrings::from(make_nullable_strings(n));
        let presence_bools = nullable.presence_bools();
        let presence_stream =
            EncodedStream::encode_presence(&presence_bools).expect("encode_presence failed");
        let dense = nullable.dense_values();
        let enc_values = encode_plain(&dense, int_enc);

        let with_nulls = (presence_stream, enc_values);
        group.bench_with_input(
            BenchmarkId::new("with_nulls", n),
            &with_nulls,
            |b, (pres, enc)| {
                b.iter(|| {
                    let p = pres.as_borrowed();
                    black_box(
                        borrow_enc_strings(enc, "", Some(p))
                            .decode(&mut dec())
                            .unwrap(),
                    )
                });
            },
        );
    }

    group.finish();
}

/// Benchmark 5 – shared dict vs plain
///
/// Compares decoding a plain string column against a shared-dictionary struct
/// column (plain and FSST flavours) that carries the same string data spread
/// across two child sub-properties.
/// Throughput is reported per *logical* string entry so both variants are directly comparable.
fn bench_vs_shared_dict(c: &mut Criterion) {
    let mut group = c.benchmark_group("strings/vs_shared_dict");
    let int_enc = IntEncoder::new(LogicalEncoder::None, PhysicalEncoder::None);

    for n in BENCHMARKED_LENGTHS {
        // Two logical string columns of N entries each.
        let total_entries = n * 2;
        group.throughput(Throughput::Elements(total_entries as u64));

        let strings = make_strings(n);

        // --- plain: two independent decode_strings calls ---
        let enc_plain = encode_plain(&strings, int_enc);
        group.bench_with_input(BenchmarkId::new("plain_x2", n), &enc_plain, |b, encoded| {
            b.iter(|| {
                black_box(
                    borrow_enc_strings(encoded, "", None)
                        .decode(&mut dec())
                        .unwrap(),
                );
                black_box(
                    borrow_enc_strings(encoded, "", None)
                        .decode(&mut dec())
                        .unwrap(),
                );
            });
        });

        // --- shared dict (plain) ---
        //
        // Build a decoded shared dict from two sub-properties; the second child
        // has every 3rd entry as NULL so the child presence path is exercised.
        let child1: StagedStrings = strings
            .iter()
            .map(|s| Some(s.clone()))
            .collect::<Vec<_>>()
            .into();
        let child2: StagedStrings = strings
            .iter()
            .enumerate()
            .map(|(i, s)| if i % 3 == 0 { None } else { Some(s.clone()) })
            .collect::<Vec<_>>()
            .into();

        let decoded_shared = build_staged_shared_dict(
            "place:",
            [
                ("type".to_string(), child1),
                ("subtype".to_string(), child2),
            ],
        )
        .expect("build_staged_shared_dict failed");

        let item_enc = SharedDictItemEncoder {
            presence: PresenceStream::Absent,
            offsets: int_enc,
        };
        let encoder_plain = SharedDictEncoder {
            dict_encoder: StrEncoder::plain(int_enc),
            items: vec![item_enc.clone(), item_enc],
        };
        let encoded_prop_plain = encode_shared_dict_prop(&decoded_shared, &encoder_plain)
            .expect("encode_shared_dict_prop failed");
        let EncodedProperty::SharedDict(ref sd_plain) = encoded_prop_plain else {
            panic!("expected SharedDict property");
        };

        group.bench_with_input(
            BenchmarkId::new("shared_dict_plain", n),
            sd_plain,
            |b, sd| {
                b.iter_batched(
                    || borrow_owned_shared_dict(sd),
                    |sd_ref: RawSharedDict<'_>| {
                        black_box(sd_ref.decode(&mut dec()).unwrap())
                    },
                    BatchSize::SmallInput,
                );
            },
        );

        // --- shared dict (FSST) ---
        let item_enc_fsst = SharedDictItemEncoder {
            presence: PresenceStream::Absent,
            offsets: int_enc,
        };
        let encoder_fsst = SharedDictEncoder {
            dict_encoder: StrEncoder::fsst(int_enc, int_enc),
            items: vec![item_enc_fsst.clone(), item_enc_fsst],
        };
        let encoded_prop_fsst = encode_shared_dict_prop(&decoded_shared, &encoder_fsst)
            .expect("encode_shared_dict_prop (fsst) failed");
        let EncodedProperty::SharedDict(ref sd_fsst) = encoded_prop_fsst else {
            panic!("expected SharedDict property");
        };

        group.bench_with_input(BenchmarkId::new("shared_dict_fsst", n), sd_fsst, |b, sd| {
            b.iter_batched(
                || borrow_owned_shared_dict(sd),
                |sd_ref: RawSharedDict<'_>| {
                    black_box(sd_ref.decode(&mut dec()).unwrap())
                },
                BatchSize::SmallInput,
            );
        });

        // --- FSST plain (two independent columns) for a fair FSST comparison ---
        let enc_fsst = encode_fsst(&strings, int_enc);
        group.bench_with_input(BenchmarkId::new("fsst_x2", n), &enc_fsst, |b, encoded| {
            b.iter(|| {
                black_box(
                    borrow_enc_strings(encoded, "", None)
                        .decode(&mut dec())
                        .unwrap(),
                );
                black_box(
                    borrow_enc_strings(encoded, "", None)
                        .decode(&mut dec())
                        .unwrap(),
                );
            });
        });
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
