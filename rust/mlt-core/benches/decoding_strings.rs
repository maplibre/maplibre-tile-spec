use std::hint::black_box;

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use mlt_core::__private::{dec, roundtrip_stream};
use mlt_core::encoder::{
    EncodedProperty, EncodedSharedDict, EncodedSharedDictEncoding, EncodedStream,
    EncodedStringsEncoding, IntEncoder, PhysicalEncoder, SharedDictEncoder, SharedDictItemEncoder,
    StagedSharedDict, StagedStrings, StrEncoder, encode_shared_dict_prop,
};
use mlt_core::v01::{
    DictionaryType, LengthType, LogicalEncoder, RawFsstData, RawPlainData, RawPresence,
    RawSharedDict, RawSharedDictEncoding, RawSharedDictItem, RawStrings, RawStringsEncoding,
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

/// One `Vec<u8>` per stream so each `roundtrip_stream` borrow is disjoint (see `test_helpers`).
enum StringsBenchBufs {
    Plain {
        lengths: Vec<u8>,
        data: Vec<u8>,
    },
    Dictionary {
        lengths: Vec<u8>,
        data: Vec<u8>,
        offsets: Vec<u8>,
    },
    FsstPlain {
        symbol_lengths: Vec<u8>,
        symbol_table: Vec<u8>,
        lengths: Vec<u8>,
        corpus: Vec<u8>,
    },
    FsstDictionary {
        symbol_lengths: Vec<u8>,
        symbol_table: Vec<u8>,
        lengths: Vec<u8>,
        corpus: Vec<u8>,
        offsets: Vec<u8>,
    },
}

fn strings_bench_bufs_for(enc: &EncodedStringsEncoding) -> StringsBenchBufs {
    match enc {
        EncodedStringsEncoding::Plain(_) => StringsBenchBufs::Plain {
            lengths: Vec::new(),
            data: Vec::new(),
        },
        EncodedStringsEncoding::Dictionary { .. } => StringsBenchBufs::Dictionary {
            lengths: Vec::new(),
            data: Vec::new(),
            offsets: Vec::new(),
        },
        EncodedStringsEncoding::FsstPlain(_) => StringsBenchBufs::FsstPlain {
            symbol_lengths: Vec::new(),
            symbol_table: Vec::new(),
            lengths: Vec::new(),
            corpus: Vec::new(),
        },
        EncodedStringsEncoding::FsstDictionary { .. } => StringsBenchBufs::FsstDictionary {
            symbol_lengths: Vec::new(),
            symbol_table: Vec::new(),
            lengths: Vec::new(),
            corpus: Vec::new(),
            offsets: Vec::new(),
        },
    }
}

/// Serialize each stream to its buffer, parse with `RawStream::from_bytes` (`roundtrip_stream`).
fn materialize_raw_strings<'a>(
    enc: &'a EncodedStringsEncoding,
    name: &'a str,
    presence_enc: Option<&EncodedStream>,
    bufs: &'a mut StringsBenchBufs,
    pres_buf: &'a mut Vec<u8>,
) -> RawStrings<'a> {
    let presence = RawPresence(presence_enc.map(|s| roundtrip_stream(pres_buf, s)));
    let encoding = match (enc, bufs) {
        (EncodedStringsEncoding::Plain(d), StringsBenchBufs::Plain { lengths, data }) => {
            RawStringsEncoding::Plain(RawPlainData {
                lengths: roundtrip_stream(lengths, &d.lengths),
                data: roundtrip_stream(data, &d.data),
            })
        }
        (
            EncodedStringsEncoding::Dictionary {
                plain_data,
                offsets,
            },
            StringsBenchBufs::Dictionary {
                lengths,
                data,
                offsets: off,
            },
        ) => RawStringsEncoding::Dictionary {
            plain_data: RawPlainData {
                lengths: roundtrip_stream(lengths, &plain_data.lengths),
                data: roundtrip_stream(data, &plain_data.data),
            },
            offsets: roundtrip_stream(off, offsets),
        },
        (
            EncodedStringsEncoding::FsstPlain(d),
            StringsBenchBufs::FsstPlain {
                symbol_lengths,
                symbol_table,
                lengths,
                corpus,
            },
        ) => RawStringsEncoding::FsstPlain(
            RawFsstData::new(
                roundtrip_stream(symbol_lengths, &d.symbol_lengths),
                roundtrip_stream(symbol_table, &d.symbol_table),
                roundtrip_stream(lengths, &d.lengths),
                roundtrip_stream(corpus, &d.corpus),
            )
            .expect("RawFsstData::new"),
        ),
        (
            EncodedStringsEncoding::FsstDictionary { fsst_data, offsets },
            StringsBenchBufs::FsstDictionary {
                symbol_lengths,
                symbol_table,
                lengths,
                corpus,
                offsets: off,
            },
        ) => RawStringsEncoding::FsstDictionary {
            fsst_data: RawFsstData::new(
                roundtrip_stream(symbol_lengths, &fsst_data.symbol_lengths),
                roundtrip_stream(symbol_table, &fsst_data.symbol_table),
                roundtrip_stream(lengths, &fsst_data.lengths),
                roundtrip_stream(corpus, &fsst_data.corpus),
            )
            .expect("RawFsstData::new"),
            offsets: roundtrip_stream(off, offsets),
        },
        _ => panic!("StringsBenchBufs variant does not match EncodedStringsEncoding"),
    };
    RawStrings {
        name,
        presence,
        encoding,
    }
}

enum SharedDictBenchBufs {
    Plain {
        dict_lengths: Vec<u8>,
        dict_data: Vec<u8>,
        children: Vec<(Option<Vec<u8>>, Vec<u8>)>,
    },
    FsstPlain {
        sl: Vec<u8>,
        st: Vec<u8>,
        len: Vec<u8>,
        corpus: Vec<u8>,
        children: Vec<(Option<Vec<u8>>, Vec<u8>)>,
    },
}

fn shared_dict_bench_bufs_for(sd: &EncodedSharedDict) -> SharedDictBenchBufs {
    let children = sd
        .children
        .iter()
        .map(|c| (c.presence.0.as_ref().map(|_| Vec::new()), Vec::new()))
        .collect();
    match &sd.encoding {
        EncodedSharedDictEncoding::Plain(_) => SharedDictBenchBufs::Plain {
            dict_lengths: Vec::new(),
            dict_data: Vec::new(),
            children,
        },
        EncodedSharedDictEncoding::FsstPlain(_) => SharedDictBenchBufs::FsstPlain {
            sl: Vec::new(),
            st: Vec::new(),
            len: Vec::new(),
            corpus: Vec::new(),
            children,
        },
    }
}

fn materialize_raw_children<'a>(
    sd: &'a EncodedSharedDict,
    child_bufs: &'a mut [(Option<Vec<u8>>, Vec<u8>)],
) -> Vec<RawSharedDictItem<'a>> {
    sd.children
        .iter()
        .zip(child_bufs.iter_mut())
        .map(|(c, (pres_buf, data_buf))| {
            let presence = match (&c.presence.0, pres_buf.as_mut()) {
                (Some(es), Some(buf)) => Some(roundtrip_stream(buf, es)),
                (None, None) => None,
                _ => panic!("presence buffer layout mismatch"),
            };
            RawSharedDictItem {
                name: c.name.0.as_str(),
                presence: RawPresence(presence),
                data: roundtrip_stream(data_buf, &c.data),
            }
        })
        .collect()
}

fn materialize_raw_shared_dict<'a>(
    sd: &'a EncodedSharedDict,
    bufs: &'a mut SharedDictBenchBufs,
) -> RawSharedDict<'a> {
    match bufs {
        SharedDictBenchBufs::Plain {
            dict_lengths,
            dict_data,
            children: child_bufs,
        } => {
            let EncodedSharedDictEncoding::Plain(d) = &sd.encoding else {
                panic!("SharedDictBenchBufs::Plain vs encoding mismatch");
            };
            let encoding = RawSharedDictEncoding::Plain(RawPlainData {
                lengths: roundtrip_stream(dict_lengths, &d.lengths),
                data: roundtrip_stream(dict_data, &d.data),
            });
            let children = materialize_raw_children(sd, child_bufs);
            RawSharedDict {
                name: sd.name.0.as_str(),
                encoding,
                children,
            }
        }
        SharedDictBenchBufs::FsstPlain {
            sl,
            st,
            len,
            corpus,
            children: child_bufs,
        } => {
            let EncodedSharedDictEncoding::FsstPlain(d) = &sd.encoding else {
                panic!("SharedDictBenchBufs::FsstPlain vs encoding mismatch");
            };
            let encoding = RawSharedDictEncoding::FsstPlain(
                RawFsstData::new(
                    roundtrip_stream(sl, &d.symbol_lengths),
                    roundtrip_stream(st, &d.symbol_table),
                    roundtrip_stream(len, &d.lengths),
                    roundtrip_stream(corpus, &d.corpus),
                )
                .expect("RawFsstData::new"),
            );
            let children = materialize_raw_children(sd, child_bufs);
            RawSharedDict {
                name: sd.name.0.as_str(),
                encoding,
                children,
            }
        }
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
                        let mut bufs = strings_bench_bufs_for(encoded);
                        let mut pres_buf = Vec::new();
                        b.iter(|| {
                            black_box(
                                materialize_raw_strings(
                                    encoded,
                                    "",
                                    None,
                                    &mut bufs,
                                    &mut pres_buf,
                                )
                                .decode(&mut dec())
                                .unwrap()
                                .feature_count(),
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
                        let mut bufs = strings_bench_bufs_for(encoded);
                        let mut pres_buf = Vec::new();
                        b.iter(|| {
                            black_box(
                                materialize_raw_strings(
                                    encoded,
                                    "",
                                    None,
                                    &mut bufs,
                                    &mut pres_buf,
                                )
                                .decode(&mut dec())
                                .unwrap()
                                .feature_count(),
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
    let int_enc = IntEncoder::plain();

    for n in BENCHMARKED_LENGTHS {
        let strings = make_strings(n);
        group.throughput(Throughput::Elements(n as u64));

        let plain = encode_plain(&strings, int_enc);
        group.bench_with_input(BenchmarkId::new("plain", n), &plain, |b, encoded| {
            let mut bufs = strings_bench_bufs_for(encoded);
            let mut pres_buf = Vec::new();
            b.iter(|| {
                black_box(
                    materialize_raw_strings(encoded, "", None, &mut bufs, &mut pres_buf)
                        .decode(&mut dec())
                        .unwrap()
                        .feature_count(),
                )
            });
        });

        let fsst = encode_fsst(&strings, int_enc);
        group.bench_with_input(BenchmarkId::new("fsst", n), &fsst, |b, encoded| {
            let mut bufs = strings_bench_bufs_for(encoded);
            let mut pres_buf = Vec::new();
            b.iter(|| {
                black_box(
                    materialize_raw_strings(encoded, "", None, &mut bufs, &mut pres_buf)
                        .decode(&mut dec())
                        .unwrap()
                        .feature_count(),
                )
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

        // Non-nullable: no presence stream at all.
        let strings = make_strings(n);
        let enc_no_nulls = encode_plain(&strings, int_enc);

        group.bench_with_input(
            BenchmarkId::new("no_nulls", n),
            &enc_no_nulls,
            |b, encoded| {
                let mut bufs = strings_bench_bufs_for(encoded);
                let mut pres_buf = Vec::new();
                b.iter(|| {
                    black_box(
                        materialize_raw_strings(encoded, "", None, &mut bufs, &mut pres_buf)
                            .decode(&mut dec())
                            .unwrap()
                            .feature_count(),
                    )
                });
            },
        );

        // Nullable: attach a presence bitmap and only encode the non-null values.
        let nullable = StagedStrings::from_optional("", make_nullable_strings(n));
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
                let mut bufs = strings_bench_bufs_for(enc);
                let mut pres_buf = Vec::new();
                b.iter(|| {
                    black_box(
                        materialize_raw_strings(enc, "", Some(pres), &mut bufs, &mut pres_buf)
                            .decode(&mut dec())
                            .unwrap()
                            .feature_count(),
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
    let int_enc = IntEncoder::plain();

    for n in BENCHMARKED_LENGTHS {
        // Two logical string columns of N entries each.
        let total_entries = n * 2;
        group.throughput(Throughput::Elements(total_entries as u64));

        let strings = make_strings(n);

        // --- plain: two independent decode_strings calls ---
        let enc_plain = encode_plain(&strings, int_enc);
        group.bench_with_input(BenchmarkId::new("plain_x2", n), &enc_plain, |b, encoded| {
            let mut bufs = strings_bench_bufs_for(encoded);
            let mut pres_buf = Vec::new();
            b.iter(|| {
                for _ in 0..2 {
                    black_box(
                        materialize_raw_strings(encoded, "", None, &mut bufs, &mut pres_buf)
                            .decode(&mut dec())
                            .unwrap()
                            .feature_count(),
                    );
                }
            });
        });

        // --- shared dict (plain) ---
        //
        // Build a decoded shared dict from two sub-properties; the second child
        // has every 3rd entry as NULL so the child presence path is exercised.
        let col1: Vec<Option<&str>> = strings.iter().map(|s| Some(s.as_str())).collect();
        let col2: Vec<Option<&str>> = strings
            .iter()
            .enumerate()
            .map(|(i, s)| if i % 3 == 0 { None } else { Some(s.as_str()) })
            .collect();

        let decoded_shared = StagedSharedDict::new("place:", [("type", col1), ("subtype", col2)])
            .expect("StagedSharedDict::try_new failed");

        let item_enc = SharedDictItemEncoder::new(int_enc);
        let encoder_plain = SharedDictEncoder {
            dict_encoder: StrEncoder::plain(int_enc),
            items: vec![item_enc, item_enc],
        };
        let encoded_prop_plain = encode_shared_dict_prop(&decoded_shared, &encoder_plain)
            .expect("encode_shared_dict_prop failed")
            .expect("expected non-empty SharedDict");
        let EncodedProperty::SharedDict(ref sd_plain) = encoded_prop_plain else {
            panic!("expected SharedDict property");
        };

        group.bench_with_input(
            BenchmarkId::new("shared_dict_plain", n),
            sd_plain,
            |b, sd| {
                let mut bufs = shared_dict_bench_bufs_for(sd);
                b.iter(|| {
                    black_box(
                        materialize_raw_shared_dict(sd, &mut bufs)
                            .decode(&mut dec())
                            .unwrap()
                            .corpus()
                            .len(),
                    )
                });
            },
        );

        // --- shared dict (FSST) ---
        let item_enc_fsst = SharedDictItemEncoder::new(int_enc);
        let encoder_fsst = SharedDictEncoder {
            dict_encoder: StrEncoder::fsst(int_enc, int_enc),
            items: vec![item_enc_fsst, item_enc_fsst],
        };
        let encoded_prop_fsst = encode_shared_dict_prop(&decoded_shared, &encoder_fsst)
            .expect("encode_shared_dict_prop (fsst) failed")
            .expect("expected non-empty SharedDict (fsst)");
        let EncodedProperty::SharedDict(ref sd_fsst) = encoded_prop_fsst else {
            panic!("expected SharedDict property");
        };

        group.bench_with_input(BenchmarkId::new("shared_dict_fsst", n), sd_fsst, |b, sd| {
            let mut bufs = shared_dict_bench_bufs_for(sd);
            b.iter(|| {
                black_box(
                    materialize_raw_shared_dict(sd, &mut bufs)
                        .decode(&mut dec())
                        .unwrap()
                        .corpus()
                        .len(),
                )
            });
        });

        // --- FSST plain (two independent columns) for a fair FSST comparison ---
        let enc_fsst = encode_fsst(&strings, int_enc);
        group.bench_with_input(BenchmarkId::new("fsst_x2", n), &enc_fsst, |b, encoded| {
            let mut bufs = strings_bench_bufs_for(encoded);
            let mut pres_buf = Vec::new();
            b.iter(|| {
                for _ in 0..2 {
                    black_box(
                        materialize_raw_strings(encoded, "", None, &mut bufs, &mut pres_buf)
                            .decode(&mut dec())
                            .unwrap()
                            .feature_count(),
                    );
                }
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
