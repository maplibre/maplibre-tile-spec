use std::borrow::Cow;
use std::collections::HashMap;

use fsst::Compressor;
use integer_encoding::VarIntWriter as _;

use super::model::{PresenceKind, StagedSharedDict, StagedSharedDictItem, StagedStrings};
use crate::MltError::DictIndexOutOfBounds;
use crate::codecs::fsst::{FsstRawData, compress_fsst};
use crate::decoder::strings::{
    checked_string_end, decode_shared_dict_range, encode_null_end, resolve_dict_spans,
    shared_dict_spans,
};
use crate::decoder::{
    ColumnType, DictionaryType, IntEncoding, LengthType, OffsetType, ParsedSharedDict,
    ParsedSharedDictItem, RawSharedDict, RawSharedDictEncoding, RawSharedDictItem, StreamMeta,
    StreamType,
};
use crate::encoder::model::StrEncoding;
use crate::encoder::stream::{
    FsstStrEncoder, IntEncoder, dedup_strings, do_write_u32, write_u32_stream,
};
use crate::encoder::{EncodedStream, Encoder};
use crate::errors::AsMltError as _;
use crate::utils::{AsUsize as _, BinarySerializer as _, checked_sum3, strings_to_lengths};
use crate::{Decoder, MltResult};

/// Minimum total raw byte size of a column before attempting FSST compression.
const FSST_OVERHEAD_THRESHOLD: usize = 4_096;
/// Maximum number of strings sampled for the FSST viability probe.
const FSST_SAMPLE_STRINGS: usize = 512;

/// Returns `true` when FSST compression is likely to save space on `strings`.
pub(crate) fn fsst_is_viable(strings: &[&str]) -> bool {
    if strings.is_empty() {
        return false;
    }
    let sample = if strings.len() <= FSST_SAMPLE_STRINGS {
        strings
    } else {
        &strings[..FSST_SAMPLE_STRINGS]
    };
    let plain_size: usize = sample.iter().map(|s| s.len()).sum();
    if plain_size < FSST_OVERHEAD_THRESHOLD {
        return false;
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
    symbol_overhead + compressed_size < plain_size
}

/// Encode a string column, following the same explicit-or-auto pattern as numeric columns.
///
/// If [`Encoder::get_str_encoding`] returns `Some`, only that type is encoded.
/// Otherwise Plain, Dict, and (when viable) FSST variants are competed via the alternatives
/// machinery, mirroring the `write_int_prop_*` pattern one level up.
pub(crate) fn write_str_col(
    v: &StagedStrings,
    presence: Option<&EncodedStream>,
    enc: &mut Encoder,
) -> MltResult<()> {
    let dense = v.dense_values();
    let non_null: Vec<&str> = dense.iter().map(String::as_str).collect();
    let name = &v.name;
    if let Some(str_enc) = enc.get_str_encoding(name) {
        match str_enc {
            StrEncoding::Plain => write_str_plain(&non_null, presence, name, enc)?,
            StrEncoding::Dict => write_str_dict(&non_null, presence, name, enc)?,
            StrEncoding::Fsst => write_str_fsst(&non_null, presence, name, enc)?,
            StrEncoding::FsstDict => write_str_fsst_dict(&non_null, presence, name, enc)?,
        }
    } else {
        enc.start_alternatives();
        write_str_plain(&non_null, presence, name, enc)?;
        enc.end_alternative();
        write_str_dict(&non_null, presence, name, enc)?;
        enc.end_alternative();

        if fsst_is_viable(&non_null) {
            // Re-training FSST per candidate is expensive; use one flat candidate per type.
            write_str_fsst(&non_null, presence, name, enc)?;
            enc.end_alternative();
            write_str_fsst_dict(&non_null, presence, name, enc)?;
            enc.end_alternative();
        }
        enc.finish_alternatives();
    }
    Ok(())
}

/// Encode with plain (`VarBinary` lengths) layout.
///
/// Stream count varint is written first, then presence, then the lengths stream
/// (via [`write_u32_stream`] which handles the explicit/auto dispatch internally),
/// then the raw string bytes as a plain unencoded data stream.
fn write_str_plain(
    non_null: &[&str],
    presence: Option<&EncodedStream>,
    name: &str,
    enc: &mut Encoder,
) -> MltResult<()> {
    let lengths = strings_to_lengths(non_null)?;
    enc.write_varint(2u32 + u32::from(presence.is_some()))?;
    enc.write_optional_stream(presence)?;
    let typ = StreamType::Length(LengthType::VarBinary);
    write_u32_stream(&lengths, typ, "prop", name, Some("lengths"), enc)?;
    write_raw_str_data(non_null, DictionaryType::None, enc)
}

/// Encode with dictionary (deduped corpus + offset indices) layout.
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
    let typ = StreamType::Length(LengthType::Dictionary);
    write_u32_stream(&lengths, typ, "prop", name, Some("lengths"), enc)?;
    let typ = StreamType::Offset(OffsetType::String);
    write_u32_stream(&offset_indices, typ, "prop", name, Some("offsets"), enc)?;
    write_raw_str_data(&unique, DictionaryType::Single, enc)
}

/// Encode with FSST compression (sub-streams use varint or explicit; no int-encoder competition).
///
/// The FSST sub-streams are written individually since their encoding is determined by
/// the FSST algorithm, not by the alternatives machinery.
/// The offset stream uses [`write_u32_stream`] for explicit/auto dispatch.
fn write_str_fsst(
    non_null: &[&str],
    presence: Option<&EncodedStream>,
    name: &str,
    enc: &mut Encoder,
) -> MltResult<()> {
    let raw = compress_fsst(non_null);
    let offsets: Vec<u32> = (0..u32::try_from(non_null.len())?).collect();
    enc.write_varint(5u32 + u32::from(presence.is_some()))?;
    enc.write_optional_stream(presence)?;
    write_fsst_data(&raw, fsst_enc(enc, name), DictionaryType::Single, enc)?;
    let typ = StreamType::Offset(OffsetType::String);
    write_u32_stream(&offsets, typ, "prop", name, Some("offsets"), enc)
}

/// Encode with FSST + dictionary layout (deduped unique strings, per-feature offset indices).
fn write_str_fsst_dict(
    non_null: &[&str],
    presence: Option<&EncodedStream>,
    name: &str,
    enc: &mut Encoder,
) -> MltResult<()> {
    let (unique, offset_indices) = dedup_strings(non_null)?;
    let raw = compress_fsst(&unique);
    enc.write_varint(5u32 + u32::from(presence.is_some()))?;
    enc.write_optional_stream(presence)?;
    write_fsst_data(&raw, fsst_enc(enc, name), DictionaryType::Single, enc)?;
    let typ = StreamType::Offset(OffsetType::String);
    write_u32_stream(&offset_indices, typ, "prop", name, Some("offsets"), enc)
}

/// Builds an FSST sub-stream encoder, using explicit overrides when configured or varint otherwise.
fn fsst_enc(enc: &Encoder, name: &str) -> FsstStrEncoder {
    FsstStrEncoder {
        symbol_lengths: enc
            .get_int_encoder("prop", name, Some("sym_lengths"))
            .unwrap_or(IntEncoder::varint()),
        dict_lengths: enc
            .get_int_encoder("prop", name, Some("dict_lengths"))
            .unwrap_or(IntEncoder::varint()),
    }
}

/// Write 4 FSST sub-streams directly to `enc.data`.
///
/// Stream order: `symbol_lengths`, `symbol_table`, `value_lengths`, `corpus`.
fn write_fsst_data(
    raw: &FsstRawData,
    encoding: FsstStrEncoder,
    dict_type: DictionaryType,
    enc: &mut Encoder,
) -> MltResult<()> {
    let typ = StreamType::Length(LengthType::Symbol);
    do_write_u32(&raw.symbol_lengths, typ, encoding.symbol_lengths, enc)?;
    let num_syms = u32::try_from(raw.symbol_lengths.len())?;
    let sym_bytes_len = u32::try_from(raw.symbol_bytes.len())?;
    let typ = StreamType::Data(DictionaryType::Fsst);
    StreamMeta::new(typ, IntEncoding::none(), num_syms).write_to(enc, false, sym_bytes_len)?;
    enc.data.extend_from_slice(&raw.symbol_bytes);
    let typ = StreamType::Length(LengthType::Dictionary);
    do_write_u32(&raw.value_lengths, typ, encoding.dict_lengths, enc)?;
    let num_vals = u32::try_from(raw.value_lengths.len())?;
    let corpus_len = u32::try_from(raw.corpus.len())?;
    StreamMeta::new(StreamType::Data(dict_type), IntEncoding::none(), num_vals)
        .write_to(enc, false, corpus_len)?;
    enc.data.extend_from_slice(&raw.corpus);
    Ok(())
}

/// Write raw string bytes as an unencoded data stream directly to `enc.data`.
fn write_raw_str_data(
    strings: &[&str],
    dict_type: DictionaryType,
    enc: &mut Encoder,
) -> MltResult<()> {
    let bytes: Vec<u8> = strings
        .iter()
        .flat_map(|s| s.as_bytes().iter().copied())
        .collect();
    let num_values = u32::try_from(strings.len())?;
    let byte_length = u32::try_from(bytes.len())?;
    StreamMeta::new(StreamType::Data(dict_type), IntEncoding::none(), num_values).write_to(
        enc,
        false,
        byte_length,
    )?;
    enc.data.extend_from_slice(&bytes);
    Ok(())
}

/// Encode a shared-dictionary property and write it to `enc`.
///
/// When [`Encoder::get_str_encoding`] returns [`None`], auto-selects the corpus encoding (FSST if viable, else plain)
/// and uses automatic offset encoders.
/// When [`Some`], uses the caller-specified encoding and [`Encoder::get_int_encoder`] for offsets.
///
/// Returns `false` when every child column is [`PresenceKind::Empty`] or
/// [`PresenceKind::AllNull`] — the whole shared-dict property is skipped.
pub(crate) fn write_shared_dict(
    shared_dict: &StagedSharedDict,
    enc: &mut Encoder,
) -> MltResult<bool> {
    if shared_dict
        .items
        .iter()
        .all(|item| matches!(item.presence(), PresenceKind::Empty | PresenceKind::AllNull))
    {
        return Ok(false);
    }

    let dict_spans = collect_staged_shared_dict_spans(&shared_dict.items);
    let dict: Vec<&str> = dict_spans
        .iter()
        .map(|&span| {
            shared_dict
                .get(span)
                .ok_or(DictIndexOutOfBounds(span.0, dict_spans.len()))
        })
        .collect::<Result<_, _>>()?;
    let dict_index: HashMap<(u32, u32), u32> = dict_spans.iter().copied().zip(0_u32..).collect();

    // Decide corpus encoding upfront to determine the stream count for the varint header.
    // FSST uses 4 streams; plain uses 2.
    let use_fsst = match enc.get_str_encoding(&shared_dict.prefix) {
        Some(StrEncoding::Fsst | StrEncoding::FsstDict) => true,
        Some(StrEncoding::Plain | StrEncoding::Dict) => false,
        None => fsst_is_viable(&dict),
    };
    let dict_stream_count = if use_fsst { 4u32 } else { 2u32 };

    // Determine per-child presence upfront (needed before writing meta + stream count).
    let force_presence = enc.override_presence("prop", &shared_dict.prefix, None);
    let child_has_presence: Vec<bool> = shared_dict
        .items
        .iter()
        .map(|item| {
            item.presence().needs_presence_stream()
                || (force_presence && matches!(item.presence(), PresenceKind::AllPresent))
        })
        .collect();

    // Write column metadata.
    let children_count = u32::try_from(shared_dict.items.len())?;
    let optional_count = u32::try_from(child_has_presence.iter().filter(|&&x| x).count())?;
    let stream_len = checked_sum3(dict_stream_count, children_count, optional_count)?;

    ColumnType::SharedDict.write_to(&mut enc.meta)?;
    enc.meta.write_string(&shared_dict.prefix)?;
    enc.meta.write_varint(children_count)?;
    for (item, &has_presence) in shared_dict.items.iter().zip(&child_has_presence) {
        use ColumnType as CT;
        CT::write_one_of(has_presence, CT::OptStr, CT::Str, &mut enc.meta)?;
        enc.meta.write_string(&item.suffix)?;
    }

    // Write stream data: total count, corpus streams, then per-child streams.
    enc.write_varint(stream_len)?;
    if use_fsst {
        let raw = compress_fsst(&dict);
        let enc_type = if let Some(str_enc) = enc.get_str_encoding(&shared_dict.prefix) {
            debug_assert!(matches!(str_enc, StrEncoding::Fsst | StrEncoding::FsstDict));
            FsstStrEncoder {
                symbol_lengths: enc
                    .get_int_encoder("prop", &shared_dict.prefix, Some("sym_lengths"))
                    .unwrap_or(IntEncoder::varint()),
                dict_lengths: enc
                    .get_int_encoder("prop", &shared_dict.prefix, Some("dict_lengths"))
                    .unwrap_or(IntEncoder::varint()),
            }
        } else {
            FsstStrEncoder {
                symbol_lengths: IntEncoder::varint(),
                dict_lengths: IntEncoder::varint(),
            }
        };
        write_fsst_data(&raw, enc_type, DictionaryType::Single, enc)?;
    } else {
        let lengths = strings_to_lengths(&dict)?;
        write_u32_stream(
            &lengths,
            StreamType::Length(LengthType::Dictionary),
            "prop",
            &shared_dict.prefix,
            Some("lengths"),
            enc,
        )?;
        write_raw_str_data(&dict, DictionaryType::Shared, enc)?;
    }

    for (item, &has_presence) in shared_dict.items.iter().zip(&child_has_presence) {
        let offsets: Vec<u32> = item
            .dense_spans()
            .iter()
            .map(|span| {
                dict_index
                    .get(span)
                    .copied()
                    .ok_or(DictIndexOutOfBounds(span.0, dict_spans.len()))
            })
            .collect::<Result<_, _>>()?;

        enc.write_varint(1 + u32::from(has_presence))?;
        if has_presence {
            let presence = EncodedStream::encode_presence(&item.presence_bools())?;
            enc.write_boolean_stream(&presence)?;
        }
        let typ = StreamType::Offset(OffsetType::String);
        write_u32_stream(
            &offsets,
            typ,
            "prop",
            &shared_dict.prefix,
            Some(&item.suffix),
            enc,
        )?;
    }

    enc.increment_column_count();
    Ok(true)
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
    pub fn dense_values(&self) -> Vec<String> {
        let mut values = Vec::new();
        let mut start = 0_u32;
        for &end in &self.lengths {
            if end >= 0 {
                let end = end.cast_unsigned();
                values.push(self.data[start.as_usize()..end.as_usize()].to_string());
                start = end;
            } else {
                start = (!end).cast_unsigned();
            }
        }
        values
    }
}

// ── StagedSharedDict ──────────────────────────────────────────────────────────

impl StagedSharedDict {
    #[must_use]
    pub fn corpus(&self) -> &str {
        &self.data
    }

    #[must_use]
    pub fn get(&self, span: (u32, u32)) -> Option<&str> {
        self.corpus().get(span.0.as_usize()..span.1.as_usize())
    }
}

pub fn collect_staged_shared_dict_spans(items: &[StagedSharedDictItem]) -> Vec<(u32, u32)> {
    let mut spans = items
        .iter()
        .flat_map(StagedSharedDictItem::dense_spans)
        .collect::<Vec<_>>();
    spans.sort_unstable();
    spans.dedup();
    spans
}

impl StagedSharedDictItem {
    #[must_use]
    pub fn feature_count(&self) -> usize {
        self.ranges.len()
    }

    #[must_use]
    pub fn presence(&self) -> PresenceKind {
        let mut has_null = false;
        let mut has_present = false;
        for &range in &self.ranges {
            if decode_shared_dict_range(range).is_none() {
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
        self.ranges
            .iter()
            .map(|&range| decode_shared_dict_range(range).is_some())
            .collect()
    }

    #[must_use]
    pub fn dense_spans(&self) -> Vec<(u32, u32)> {
        self.ranges
            .iter()
            .filter_map(|&range| decode_shared_dict_range(range))
            .collect()
    }

    #[must_use]
    pub fn get<'a>(&self, shared_dict: &'a StagedSharedDict, i: usize) -> Option<&'a str> {
        self.ranges
            .get(i)
            .copied()
            .and_then(decode_shared_dict_range)
            .and_then(|span| shared_dict.get(span))
    }

    #[must_use]
    pub fn materialize(&self, shared_dict: &StagedSharedDict) -> Vec<Option<String>> {
        self.ranges
            .iter()
            .map(|&range| {
                decode_shared_dict_range(range)
                    .and_then(|span| shared_dict.get(span))
                    .map(str::to_string)
            })
            .collect()
    }
}

fn encode_shared_dict_range(start: u32, end: u32) -> MltResult<(i32, i32)> {
    Ok((i32::try_from(start)?, i32::try_from(end)?))
}

impl StagedSharedDict {
    /// Build a shared-dictionary column directly from raw per-column string data.
    ///
    /// Each column is a `(suffix, values)` pair where `values` is an iterator of
    /// optional strings (one per feature).  All unique non-null strings across every
    /// column are deduplicated into a shared byte corpus; per-feature byte-range offsets
    /// into that corpus are recorded in each [`StagedSharedDictItem`].
    pub fn new<S, I, T>(
        prefix: impl Into<String>,
        columns: impl IntoIterator<Item = (S, I)>,
    ) -> MltResult<Self>
    where
        S: Into<String>,
        I: IntoIterator<Item = Option<T>>,
        T: AsRef<str>,
    {
        let prefix = prefix.into();
        // Collect all columns so we can make two passes (dedup then assign ranges).
        let columns: Vec<(String, Vec<Option<T>>)> = columns
            .into_iter()
            .map(|(s, i)| (s.into(), i.into_iter().collect()))
            .collect();

        // First pass: build deduplicated corpus in insertion order.
        let mut dict_index = HashMap::<String, u32>::new();
        let mut dict_ranges = Vec::<(u32, u32)>::new();
        let mut data = String::new();

        for (_, values) in &columns {
            for value in values.iter().filter_map(Option::as_ref) {
                let s = value.as_ref();
                if !dict_index.contains_key(s) {
                    let idx = u32::try_from(dict_ranges.len()).or_overflow()?;
                    let offset = u32::try_from(data.len()).or_overflow()?;
                    let end = offset
                        .checked_add(u32::try_from(s.len()).or_overflow()?)
                        .or_overflow()?;
                    dict_index.insert(s.to_owned(), idx);
                    dict_ranges.push((offset, end));
                    data.push_str(s);
                }
            }
        }

        // Second pass: emit per-feature ranges for each column.
        let items = columns
            .into_iter()
            .map(|(suffix, values)| -> MltResult<StagedSharedDictItem> {
                let mut ranges = Vec::with_capacity(values.len());
                for opt_val in values {
                    match opt_val {
                        Some(value) => {
                            let idx = *dict_index
                                .get(value.as_ref())
                                .expect("StagedSharedDict::new: value must be present");
                            let (start, end) = dict_ranges[idx as usize];
                            ranges.push(encode_shared_dict_range(start, end)?);
                        }
                        None => ranges.push((-1, -1)),
                    }
                }
                Ok(StagedSharedDictItem { suffix, ranges })
            })
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Self {
            prefix,
            data,
            items,
        })
    }
}

impl<'a> RawSharedDict<'a> {
    #[must_use]
    pub fn new(
        name: &'a str,
        encoding: RawSharedDictEncoding<'a>,
        children: Vec<RawSharedDictItem<'a>>,
    ) -> Self {
        Self {
            name,
            encoding,
            children,
        }
    }

    /// Decode a shared-dictionary column into its decoded form.
    pub fn decode(self, dec: &mut Decoder) -> MltResult<ParsedSharedDict<'a>> {
        let prefix = self.name;
        let (data, dict_spans) = match self.encoding {
            RawSharedDictEncoding::Plain(plain_data) => {
                let (decoded, lengths) = plain_data.decode(dec)?;
                let dict_spans = shared_dict_spans(&lengths, dec)?;
                (Cow::Borrowed(decoded), dict_spans)
            }
            RawSharedDictEncoding::FsstPlain(fsst_data) => {
                let (decoded, lengths) = fsst_data.decode(dec)?;
                let dict_spans = shared_dict_spans(&lengths, dec)?;
                (decoded.into(), dict_spans)
            }
        };
        let mut items = Vec::with_capacity(self.children.len());
        for child in self.children {
            let offsets: Vec<u32> = child.data.decode_u32s(dec)?;
            let presence = match child.presence.0 {
                Some(s) => Some(s.decode_bools(dec)?),
                None => None,
            };
            let ranges = resolve_dict_spans(&offsets, presence.as_deref(), &dict_spans, dec)?
                .into_iter()
                .map(|span| match span {
                    Some(span) => encode_shared_dict_range(span.0, span.1),
                    None => Ok((-1, -1)),
                })
                .collect::<Result<Vec<_>, _>>()?;
            items.push(ParsedSharedDictItem {
                suffix: child.name,
                ranges,
            });
        }

        let parsed = ParsedSharedDict {
            prefix,
            data,
            items,
        };
        // Corpus size is only known after decompression; charge after.
        let bytes = parsed.items.iter().try_fold(
            u32::try_from(parsed.data.len()).or_overflow()?,
            |acc, item| {
                let n = u32::try_from(item.ranges.len() * size_of::<(i32, i32)>()).or_overflow()?;
                acc.checked_add(n).or_overflow()
            },
        )?;
        dec.consume(bytes)?;
        Ok(parsed)
    }
}
