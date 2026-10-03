//! Typed decode of a single data blob, for callers that render the values themselves.

use std::borrow::Cow;

use serde::Serialize;
use usize_cast::IntoUsize as _;

use super::model::{BlobInfo, DecodeHint, DumpTree, Region};
use crate::codecs::fsst::decode_fsst_bytes;
use crate::decoder::strings::rebuild_dictionary;
use crate::decoder::{DictLayout, DictionaryType, LengthType, RawFsstData, RawStream, StreamType};
use crate::{Decoder, GeometryType, MltError, MltResult};

/// One data blob decoded for display, capped at the caller's value count.
///
/// [`render`](super::render()) formats the same payloads as text.
/// This is the structured form, for the wasm binding and anything else that lays out its own view.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum DecodedBlob {
    /// Values that survive an `f64` round-trip.
    Numbers {
        values: Vec<f64>,
        /// Value count before the cap, or `None` when nothing was dropped.
        truncated_from: Option<u32>,
    },
    /// 64-bit values, which an `f64` would round.
    #[serde(rename = "bigints")]
    BigInts {
        values: Vec<i128>,
        truncated_from: Option<u32>,
    },
    Bools {
        values: Vec<bool>,
        truncated_from: Option<u32>,
    },
    /// Values that name something: each `names` entry is what the `values` entry stands for.
    Enum {
        values: Vec<u32>,
        names: Vec<Cow<'static, str>>,
        truncated_from: Option<u32>,
    },
    /// The strings an FSST corpus stands for, which its own bytes do not say.
    Strings {
        values: Vec<String>,
        truncated_from: Option<u32>,
    },
    /// A byte payload that is valid UTF-8.
    Text { value: String },
    /// A byte payload that is not.
    Binary { len: u32 },
    /// The payload did not decode.
    Error { message: String },
}

/// Decode `data` the way `info`'s hint says to, keeping at most `max_values` values.
///
/// `max_values` of `0` keeps all of them.
/// A bytes payload is one value rather than a list, so it crosses whole.
/// Never panics: a decode failure comes back as [`DecodedBlob::Error`].
pub fn decode_blob(
    info: BlobInfo,
    data: &[u8],
    max_values: usize,
    dec: &mut Decoder,
) -> DecodedBlob {
    // Bound memory/time per blob, as `render` does.
    dec.reset_budget();
    let meta = info.meta;
    match info.hint {
        DecodeHint::Presence => match RawStream::new(meta, data).decode_bitvec(dec) {
            Ok(bits) => bools(bits.iter().by_vals().collect(), max_values),
            Err(e) => error(&e),
        },
        #[cfg(feature = "unstable-v2")]
        DecodeHint::PresenceCoded(coding) => {
            use crate::codecs::presence_coding::{Unmetered, read};
            match read(data, meta.num_values, coding, &mut Unmetered) {
                Ok((_, bits)) => bools(bits.iter().by_vals().collect(), max_values),
                Err(e) => DecodedBlob::Error {
                    message: e.to_string(),
                },
            }
        }
        #[cfg(feature = "unstable-v2")]
        DecodeHint::PackedBits => {
            let n = meta.num_values.into_usize();
            let available = data.len() * 8;
            if available < n {
                return DecodedBlob::Error {
                    message: format!("needs {n} bits, got {available}"),
                };
            }
            bools(
                (0..n).map(|i| data[i / 8] >> (i % 8) & 1 == 1).collect(),
                max_values,
            )
        }
        DecodeHint::Bool => match RawStream::new(meta, data).decode_bools(dec) {
            Ok(v) => bools(v, max_values),
            Err(e) => error(&e),
        },
        DecodeHint::GeometryType => geometry_types(
            RawStream::new(meta, data).decode_ints::<u32>(dec),
            max_values,
        ),
        DecodeHint::I32 => numbers(
            RawStream::new(meta, data).decode_ints::<i32>(dec),
            max_values,
        ),
        DecodeHint::U32 => numbers(
            RawStream::new(meta, data).decode_ints::<u32>(dec),
            max_values,
        ),
        DecodeHint::F32 => numbers(
            RawStream::new(meta, data).decode_floats::<f32>(dec),
            max_values,
        ),
        DecodeHint::F64 => numbers(
            RawStream::new(meta, data).decode_floats::<f64>(dec),
            max_values,
        ),
        DecodeHint::I64 => bigints(
            RawStream::new(meta, data).decode_ints::<i64>(dec),
            max_values,
        ),
        DecodeHint::U64 => bigints(
            RawStream::new(meta, data).decode_ints::<u64>(dec),
            max_values,
        ),
        #[cfg(feature = "unstable-v2")]
        DecodeHint::Alp(params) => bigints(
            RawStream::new(meta, data).decode_alp_codes(params, dec),
            max_values,
        ),
        DecodeHint::Bytes => match std::str::from_utf8(data) {
            Ok(s) => DecodedBlob::Text {
                value: s.to_string(),
            },
            Err(_) => DecodedBlob::Binary {
                len: count(data.len()),
            },
        },
    }
}

/// As [`decode_blob`] for the payload at `index` of `tree`, which may need the streams beside it.
///
/// An FSST corpus is only codes into the symbol table next to it, so it decodes to the strings
/// those codes expand to. Any other payload decodes exactly as [`decode_blob`] does.
pub fn decode_region(
    tree: &DumpTree,
    buf: &[u8],
    index: usize,
    max_values: usize,
    dec: &mut Decoder,
) -> DecodedBlob {
    let Some(region) = tree.regions.get(index) else {
        return DecodedBlob::Error {
            message: format!("the tree has no region {index}"),
        };
    };
    let Some(info) = region.blob else {
        return DecodedBlob::Error {
            message: format!("region {index} carries no stream metadata"),
        };
    };
    if let Some(streams) = fsst_streams(&tree.regions, index) {
        dec.reset_budget();
        let layout = corpus_layout(&tree.regions, buf, index);
        return fsst_strings(streams, layout, buf, max_values, dec).unwrap_or_else(|e| error(&e));
    }
    match buf.get(region.offset..region.offset + region.len) {
        Some(bytes) => decode_blob(info, bytes, max_values, dec),
        None => DecodedBlob::Error {
            message: format!("region {index} lies outside the tile buffer"),
        },
    }
}

/// The four streams an FSST corpus is read against, in the order `RawFsstData::new` takes them.
///
/// They are the streams of the corpus's own column, so a second of any kind there would make
/// a pick a guess, and the corpus is then left to decode as bytes.
fn fsst_streams(regions: &[Region], at: usize) -> Option<[&Region; 4]> {
    let corpus = &regions[at];
    if !matches!(
        corpus.blob?.meta.stream_type,
        StreamType::Data(DictionaryType::Single | DictionaryType::Shared)
    ) {
        return None;
    }
    // The corpus sits in its stream, which sits in the column.
    let column = parent(regions, parent(regions, at)?)?;
    let depth = regions[column].depth;
    let after = &regions[column + 1..];
    let column_streams = &after[..after
        .iter()
        .position(|r| r.depth <= depth)
        .unwrap_or(after.len())];
    let only = |want: StreamType| {
        let mut hits = column_streams
            .iter()
            .filter(|r| r.blob.is_some_and(|b| b.meta.stream_type == want));
        let first = hits.next()?;
        hits.next().is_none().then_some(first)
    };
    Some([
        only(StreamType::Length(LengthType::Symbol))?,
        only(StreamType::Data(DictionaryType::Fsst))?,
        only(StreamType::Length(LengthType::Dictionary))?,
        corpus,
    ])
}

/// The region `at` sits in.
fn parent(regions: &[Region], at: usize) -> Option<usize> {
    regions[..at]
        .iter()
        .rposition(|r| r.depth < regions[at].depth)
}

/// How the corpus at `at` lays its entries out, as its stream's encoding byte says.
///
/// Only a v2 layer has that byte, so a v1 corpus is always plain.
#[cfg(feature = "unstable-v2")]
fn corpus_layout(regions: &[Region], buf: &[u8], at: usize) -> DictLayout {
    let is_v2 = regions[..at]
        .iter()
        .rposition(|r| r.depth == 0)
        .and_then(|layer| {
            regions[layer + 1..at]
                .iter()
                .find(|r| r.depth == 1 && r.label == "tag")
        })
        .and_then(|tag| buf.get(tag.offset))
        == Some(&2);
    let enc_byte = parent(regions, at).and_then(|stream| buf.get(regions[stream].offset));
    match enc_byte {
        Some(&b) if is_v2 => DictLayout::from_bits(b).unwrap_or(DictLayout::Plain),
        _ => DictLayout::Plain,
    }
}

#[cfg(not(feature = "unstable-v2"))]
fn corpus_layout(_: &[Region], _: &[u8], _: usize) -> DictLayout {
    DictLayout::Plain
}

fn fsst_strings(
    [symbol_lengths, symbol_table, lengths, corpus]: [&Region; 4],
    layout: DictLayout,
    buf: &[u8],
    max_values: usize,
    dec: &mut Decoder,
) -> MltResult<DecodedBlob> {
    let stream = |r: &Region| -> MltResult<RawStream<'_>> {
        let meta = r
            .blob
            .ok_or(MltError::MalformedFsst("a stream without metadata"))?
            .meta;
        let bytes = buf
            .get(r.offset..r.offset + r.len)
            .ok_or(MltError::MalformedFsst("a stream outside the tile"))?;
        Ok(RawStream::new(meta, bytes))
    };
    let raw = RawFsstData::new(
        stream(symbol_lengths)?,
        stream(symbol_table)?,
        stream(lengths)?,
        stream(corpus)?,
    )?;
    let (bytes, lengths) = decode_fsst_bytes(raw, dec)?;
    let (text, lengths) = rebuild_dictionary(layout, &lengths, &bytes)?;
    if let Cow::Owned(entries) = &text {
        dec.consume_items::<u8>(entries.len())?;
    }
    let (kept, truncated_from) = cap(lengths.into_owned(), max_values);

    let mut rest = &*text;
    let mut values = dec.alloc(kept.len())?;
    for len in kept {
        let (string, tail) = rest
            .split_at_checked(len.into_usize())
            .ok_or(MltError::MalformedFsst("string lengths overrun the corpus"))?;
        dec.consume_items::<u8>(string.len())?;
        values.push(string.to_owned());
        rest = tail;
    }
    Ok(DecodedBlob::Strings {
        values,
        truncated_from,
    })
}

fn numbers<T: Into<f64>>(res: MltResult<Vec<T>>, max_values: usize) -> DecodedBlob {
    match res {
        Ok(v) => {
            let (v, truncated_from) = cap(v, max_values);
            DecodedBlob::Numbers {
                values: v.into_iter().map(Into::into).collect(),
                truncated_from,
            }
        }
        Err(e) => error(&e),
    }
}

fn geometry_types(res: MltResult<Vec<u32>>, max_values: usize) -> DecodedBlob {
    match res {
        Ok(v) => {
            let (values, truncated_from) = cap(v, max_values);
            let names = values
                .iter()
                .map(|&v| {
                    u8::try_from(v)
                        .ok()
                        .and_then(|b| GeometryType::try_from(b).ok())
                        .map_or_else(
                            || format!("unknown({v})").into(),
                            |t| <&str>::from(t).into(),
                        )
                })
                .collect();
            DecodedBlob::Enum {
                values,
                names,
                truncated_from,
            }
        }
        Err(e) => error(&e),
    }
}

fn bigints<T: Into<i128>>(res: MltResult<Vec<T>>, max_values: usize) -> DecodedBlob {
    match res {
        Ok(v) => {
            let (v, truncated_from) = cap(v, max_values);
            DecodedBlob::BigInts {
                values: v.into_iter().map(Into::into).collect(),
                truncated_from,
            }
        }
        Err(e) => error(&e),
    }
}

fn bools(v: Vec<bool>, max_values: usize) -> DecodedBlob {
    let (values, truncated_from) = cap(v, max_values);
    DecodedBlob::Bools {
        values,
        truncated_from,
    }
}

fn error(e: &MltError) -> DecodedBlob {
    DecodedBlob::Error {
        message: e.to_string(),
    }
}

/// Keep the first `max_values`, reporting the original count when that drops any.
fn cap<T>(mut values: Vec<T>, max_values: usize) -> (Vec<T>, Option<u32>) {
    let total = values.len();
    if max_values == 0 || total <= max_values {
        return (values, None);
    }
    values.truncate(max_values);
    (values, Some(count(total)))
}

fn count(n: usize) -> u32 {
    u32::try_from(n).unwrap_or(u32::MAX)
}
