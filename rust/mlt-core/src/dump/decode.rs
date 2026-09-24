//! Typed decode of a single data blob, for callers that render the values themselves.

use serde::Serialize;
#[cfg(feature = "unstable-v2")]
use usize_cast::IntoUsize as _;

use super::model::{BlobInfo, DecodeHint};
use crate::Decoder;
use crate::decoder::RawStream;

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

fn numbers<T: Into<f64>>(res: crate::MltResult<Vec<T>>, max_values: usize) -> DecodedBlob {
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

fn bigints<T: Into<i128>>(res: crate::MltResult<Vec<T>>, max_values: usize) -> DecodedBlob {
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

fn error(e: &crate::MltError) -> DecodedBlob {
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
