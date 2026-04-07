use fsst::Compressor;
use integer_encoding::VarIntWriter as _;

use super::model::{PresenceKind, StagedStrings};
use crate::MltResult;
use crate::codecs::fsst::{FsstRawData, compress_fsst, compress_fsst_with};
use crate::decoder::strings::{checked_string_end, encode_null_end};
use crate::decoder::{DictionaryType, IntEncoding, LengthType, OffsetType, StreamMeta, StreamType};
use crate::encoder::model::{StrEncoding, StreamCtx};
use crate::encoder::stream::{dedup_strings, write_u32_stream};
use crate::encoder::{EncodedStream, Encoder};
use crate::utils::{AsUsize as _, BinarySerializer as _, strings_to_lengths};

/// Minimum total raw byte size of a column before attempting FSST compression.
const FSST_OVERHEAD_THRESHOLD: usize = 2_048;
/// Maximum number of strings sampled for the FSST viability probe.
const FSST_SAMPLE_STRINGS: usize = 256;

/// Train an FSST compressor and return it when compression is likely to save space.
///
/// Returns `None` when the column is empty, too small for FSST overhead to pay off,
/// or when trial compression shows no benefit. The returned [`Compressor`] is reused
/// by `write_str_fsst_with` / `write_str_fsst_dict_with` to avoid a redundant
/// training pass.
#[cfg_attr(feature = "__hotpath", hotpath::measure)]
pub(crate) fn fsst_try_train(strings: &[&str]) -> Option<Compressor> {
    if strings.is_empty() {
        return None;
    }
    let sample = if strings.len() <= FSST_SAMPLE_STRINGS {
        strings
    } else {
        &strings[..FSST_SAMPLE_STRINGS]
    };
    let plain_size: usize = sample.iter().map(|s| s.len()).sum();
    if plain_size < FSST_OVERHEAD_THRESHOLD {
        return None;
    }
    let byte_slices: Vec<&[u8]> = sample.iter().map(|s| s.as_bytes()).collect();
    let compressor = Compressor::train(&byte_slices);
    let symbols = compressor.symbol_table();
    let symbol_lengths = compressor.symbol_lengths();
    let symbol_overhead: usize = symbol_lengths
        .iter()
        .take(symbols.len())
        .map(|&l| usize::from(l))
        .sum();
    let compressed_size: usize = sample
        .iter()
        .map(|s| compressor.compress(s.as_bytes()).len())
        .sum();
    if symbol_overhead + compressed_size < plain_size {
        Some(compressor)
    } else {
        None
    }
}

/// Encode a string column, following the same explicit-or-auto pattern as numeric columns.
///
/// If [`Encoder::override_str_enc`] returns `Some`, only that type is encoded.
/// Otherwise Plain, Dict, and (when viable) FSST variants are competed via the alternatives
/// machinery, mirroring the `write_int_prop_*` pattern one level up.
#[cfg_attr(feature = "__hotpath", hotpath::measure)]
pub(crate) fn write_str_col(
    v: &StagedStrings,
    presence: Option<&EncodedStream>,
    enc: &mut Encoder,
) -> MltResult<()> {
    let non_null = v.dense_values();
    let name = &v.name;
    if let Some(str_enc) = enc.override_str_enc(name) {
        match str_enc {
            StrEncoding::Plain => write_str_plain(&non_null, presence, name, enc)?,
            StrEncoding::Dict => write_str_dict(&non_null, presence, name, enc)?,
            StrEncoding::Fsst => write_str_fsst(&non_null, presence, name, enc)?,
            StrEncoding::FsstDict => write_str_fsst_dict(&non_null, presence, name, enc)?,
        }
    } else {
        let mut alt = enc.try_alternatives();
        alt.with(|enc| write_str_plain(&non_null, presence, name, enc))?;
        alt.with(|enc| write_str_dict(&non_null, presence, name, enc))?;

        if let Some(compressor) = fsst_try_train(&non_null) {
            alt.with(|enc| write_str_fsst_with(&non_null, &compressor, presence, name, enc))?;
            alt.with(|enc| write_str_fsst_dict_with(&non_null, &compressor, presence, name, enc))?;
        }
    }
    Ok(())
}

/// Encode with plain (`VarBinary` lengths) layout.
///
/// Stream count varint is written first, then presence, then the lengths stream
/// (via [`write_u32_stream`] which handles the explicit/auto dispatch internally),
/// then the raw string bytes as a plain unencoded data stream.
#[cfg_attr(feature = "__hotpath", hotpath::measure)]
fn write_str_plain(
    non_null: &[&str],
    presence: Option<&EncodedStream>,
    name: &str,
    enc: &mut Encoder,
) -> MltResult<()> {
    let lengths = strings_to_lengths(non_null)?;
    enc.write_varint(2u32 + u32::from(presence.is_some()))?;
    enc.write_optional_stream(presence)?;
    let ctx = StreamCtx::prop(StreamType::Length(LengthType::VarBinary), name);
    write_u32_stream(&lengths, &ctx, enc)?;
    write_raw_str_data(non_null, DictionaryType::None, enc)
}

/// Encode with dictionary (deduped corpus + offset indices) layout.
#[cfg_attr(feature = "__hotpath", hotpath::measure)]
fn write_str_dict(
    non_null: &[&str],
    presence: Option<&EncodedStream>,
    name: &str,
    enc: &mut Encoder,
) -> MltResult<()> {
    let (unique, offset_indices) = dedup_strings(non_null)?;
    let lengths = strings_to_lengths(&unique)?;
    enc.write_varint(3u32 + u32::from(presence.is_some()))?;
    enc.write_optional_stream(presence)?;

    let ctx = StreamCtx::prop(StreamType::Length(LengthType::Dictionary), name);
    write_u32_stream(&lengths, &ctx, enc)?;

    let ctx = StreamCtx::prop(StreamType::Offset(OffsetType::String), name);
    write_u32_stream(&offset_indices, &ctx, enc)?;
    write_raw_str_data(&unique, DictionaryType::Single, enc)
}

/// Encode with FSST compression, training a fresh compressor.
///
/// Used by the explicit-encoder path. The auto path uses [`write_str_fsst_with`]
/// to reuse the compressor from the viability probe.
#[cfg_attr(feature = "__hotpath", hotpath::measure)]
fn write_str_fsst(
    non_null: &[&str],
    presence: Option<&EncodedStream>,
    name: &str,
    enc: &mut Encoder,
) -> MltResult<()> {
    let raw = compress_fsst(non_null);
    write_str_fsst_raw(&raw, non_null.len(), presence, name, enc)
}

/// Encode with FSST compression, reusing an already-trained compressor.
#[cfg_attr(feature = "__hotpath", hotpath::measure)]
fn write_str_fsst_with(
    non_null: &[&str],
    compressor: &Compressor,
    presence: Option<&EncodedStream>,
    name: &str,
    enc: &mut Encoder,
) -> MltResult<()> {
    let raw = compress_fsst_with(non_null, compressor);
    write_str_fsst_raw(&raw, non_null.len(), presence, name, enc)
}

/// Shared FSST write logic for both fresh and reused compressor paths.
fn write_str_fsst_raw(
    raw: &FsstRawData,
    count: usize,
    presence: Option<&EncodedStream>,
    name: &str,
    enc: &mut Encoder,
) -> MltResult<()> {
    let offsets: Vec<u32> = (0..u32::try_from(count)?).collect();
    enc.write_varint(5u32 + u32::from(presence.is_some()))?;
    enc.write_optional_stream(presence)?;
    write_fsst_data(raw, DictionaryType::Single, name, enc)?;
    let ctx = StreamCtx::prop(StreamType::Offset(OffsetType::String), name);
    write_u32_stream(&offsets, &ctx, enc)
}

/// Encode with FSST + dictionary layout, training a fresh compressor.
///
/// Used by the explicit-encoder path. The auto path uses [`write_str_fsst_dict_with`]
/// to reuse the compressor from the viability probe.
#[cfg_attr(feature = "__hotpath", hotpath::measure)]
fn write_str_fsst_dict(
    non_null: &[&str],
    presence: Option<&EncodedStream>,
    name: &str,
    enc: &mut Encoder,
) -> MltResult<()> {
    let (unique, offset_indices) = dedup_strings(non_null)?;
    let raw = compress_fsst(&unique);
    write_str_fsst_dict_raw(&raw, &offset_indices, presence, name, enc)
}

/// Encode with FSST + dictionary layout, reusing an already-trained compressor.
#[cfg_attr(feature = "__hotpath", hotpath::measure)]
fn write_str_fsst_dict_with(
    non_null: &[&str],
    compressor: &Compressor,
    presence: Option<&EncodedStream>,
    name: &str,
    enc: &mut Encoder,
) -> MltResult<()> {
    let (unique, offset_indices) = dedup_strings(non_null)?;
    let raw = compress_fsst_with(&unique, compressor);
    write_str_fsst_dict_raw(&raw, &offset_indices, presence, name, enc)
}

/// Shared FSST+dict write logic for both fresh and reused compressor paths.
fn write_str_fsst_dict_raw(
    raw: &FsstRawData,
    offset_indices: &[u32],
    presence: Option<&EncodedStream>,
    name: &str,
    enc: &mut Encoder,
) -> MltResult<()> {
    enc.write_varint(5u32 + u32::from(presence.is_some()))?;
    enc.write_optional_stream(presence)?;
    write_fsst_data(raw, DictionaryType::Single, name, enc)?;
    let ctx = StreamCtx::prop(StreamType::Offset(OffsetType::String), name);
    write_u32_stream(offset_indices, &ctx, enc)
}

/// Write 4 FSST sub-streams directly to `enc.data`.
///
/// The two integer sub-streams (`symbol_lengths`, `value_lengths`) use [`write_u32_stream`]
/// so explicit encoder overrides are honored and all candidates are competed automatically.
/// The two raw-byte sub-streams (`symbol_table`, `corpus`) are written without integer encoding.
///
/// Stream order: `symbol_lengths`, `symbol_table`, `value_lengths`, `corpus`.
#[cfg_attr(feature = "__hotpath", hotpath::measure)]
pub fn write_fsst_data(
    raw: &FsstRawData,
    dict_type: DictionaryType,
    name: &str,
    enc: &mut Encoder,
) -> MltResult<()> {
    let ctx = StreamCtx::prop(StreamType::Length(LengthType::Symbol), name);
    write_u32_stream(&raw.symbol_lengths, &ctx, enc)?;
    let num_syms = u32::try_from(raw.symbol_lengths.len())?;
    let sym_bytes_len = u32::try_from(raw.symbol_bytes.len())?;
    let typ = StreamType::Data(DictionaryType::Fsst);
    StreamMeta::new(typ, IntEncoding::none(), num_syms).write_to(enc, false, sym_bytes_len)?;
    enc.data.extend_from_slice(&raw.symbol_bytes);
    let ctx = StreamCtx::prop(StreamType::Length(LengthType::Dictionary), name);
    write_u32_stream(&raw.value_lengths, &ctx, enc)?;
    let num_vals = u32::try_from(raw.value_lengths.len())?;
    let corpus_len = u32::try_from(raw.corpus.len())?;
    StreamMeta::new(StreamType::Data(dict_type), IntEncoding::none(), num_vals)
        .write_to(enc, false, corpus_len)?;
    enc.data.extend_from_slice(&raw.corpus);
    Ok(())
}

/// Write raw string bytes as an unencoded data stream directly to `enc.data`.
#[cfg_attr(feature = "__hotpath", hotpath::measure)]
pub fn write_raw_str_data(
    strings: &[&str],
    dict_type: DictionaryType,
    enc: &mut Encoder,
) -> MltResult<()> {
    let total_len: usize = strings.iter().map(|s| s.len()).sum();
    let num_values = u32::try_from(strings.len())?;
    let byte_length = u32::try_from(total_len)?;
    let typ = StreamType::Data(dict_type);
    StreamMeta::new(typ, IntEncoding::none(), num_values).write_to(enc, false, byte_length)?;
    enc.data.reserve(total_len);
    for s in strings {
        enc.data.extend_from_slice(s.as_bytes());
    }
    Ok(())
}

impl StagedStrings {
    /// Stages a string column where every row has a value (no nulls).
    ///
    /// `name` is the column key (e.g. shared-dict suffix or top-level property name).
    ///
    /// `values` can be any iterator of string fragments, for example `["a", "b"]`,
    /// `vec!["x".into(), "y".into()]`, or `some_vec.iter().map(|s| s.as_str())`.
    #[must_use]
    pub fn from_strings(
        name: impl Into<String>,
        values: impl IntoIterator<Item = impl AsRef<str>>,
    ) -> Self {
        let name = name.into();
        let iter = values.into_iter();
        let (lower, _) = iter.size_hint();
        let mut lengths = Vec::with_capacity(lower);
        let mut data = String::new();
        let mut end = 0_i32;
        for value in iter {
            let value = value.as_ref();
            end = checked_string_end(end, value.len())
                .expect("staged string corpus exceeds supported i32 range");
            lengths.push(end);
            data.push_str(value);
        }
        Self {
            name,
            lengths,
            data,
        }
    }

    /// Stages a string column with optional values (nulls encoded in the length stream).
    ///
    /// `name` is the column key (e.g. shared-dict suffix or top-level property name).
    ///
    /// `values` can be any iterator of optional string fragments, for example
    /// `vec![Some("a"), None]` or a `Vec<Option<String>>`.
    #[must_use]
    pub fn from_optional(
        name: impl Into<String>,
        values: impl IntoIterator<Item = Option<impl AsRef<str>>>,
    ) -> Self {
        let name = name.into();
        let iter = values.into_iter();
        let (lower, _) = iter.size_hint();
        let mut lengths = Vec::with_capacity(lower);
        let mut data = String::new();
        let mut end = 0_i32;
        for value in iter {
            match value {
                Some(value) => {
                    let value = value.as_ref();
                    end = checked_string_end(end, value.len())
                        .expect("staged string corpus exceeds supported i32 range");
                    lengths.push(end);
                    data.push_str(value);
                }
                None => lengths.push(encode_null_end(end)),
            }
        }
        Self {
            name,
            lengths,
            data,
        }
    }

    #[must_use]
    pub fn feature_count(&self) -> usize {
        self.lengths.len()
    }

    fn bounds(&self, i: u32) -> Option<(u32, u32)> {
        let i = i.as_usize();
        let end = *self.lengths.get(i)?;
        if end < 0 {
            return None;
        }
        let end = end.cast_unsigned();
        let start = if i == 0 {
            0
        } else {
            let prev = self.lengths[i - 1];
            if prev < 0 {
                (!prev).cast_unsigned()
            } else {
                prev.cast_unsigned()
            }
        };
        Some((start, end))
    }

    #[must_use]
    pub fn presence(&self) -> PresenceKind {
        let mut has_null = false;
        let mut has_present = false;
        for &end in &self.lengths {
            if end < 0 {
                has_null = true;
            } else {
                has_present = true;
            }
            if has_null && has_present {
                return PresenceKind::Mixed;
            }
        }
        match (has_null, has_present) {
            (false, false) => PresenceKind::Empty,
            (false, true) => PresenceKind::AllPresent,
            (true, false) => PresenceKind::AllNull,
            (true, true) => unreachable!("early return handles Mixed"),
        }
    }

    #[must_use]
    pub fn presence_bools(&self) -> Vec<bool> {
        self.lengths.iter().map(|&end| end >= 0).collect()
    }

    #[must_use]
    pub fn get(&self, i: u32) -> Option<&str> {
        let (start, end) = self.bounds(i)?;
        self.data.get(start.as_usize()..end.as_usize())
    }

    #[must_use]
    pub fn dense_values(&self) -> Vec<&str> {
        let mut values = Vec::new();
        let mut start = 0_u32;
        for &end in &self.lengths {
            if end >= 0 {
                let end = end.cast_unsigned();
                values.push(&self.data[start.as_usize()..end.as_usize()]);
                start = end;
            } else {
                start = (!end).cast_unsigned();
            }
        }
        values
    }
}
