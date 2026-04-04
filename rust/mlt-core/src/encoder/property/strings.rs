use std::borrow::Cow;
use std::collections::HashMap;

use fsst::Compressor;
use integer_encoding::VarIntWriter as _;

use super::model::{PresenceKind, StagedSharedDict, StagedSharedDictItem, StagedStrings};
use crate::MltError::DictIndexOutOfBounds;
use crate::decoder::strings::{
    checked_string_end, decode_shared_dict_range, encode_null_end, resolve_dict_spans,
    shared_dict_spans,
};
use crate::decoder::{
    ColumnType, DictionaryType, LengthType, OffsetType, ParsedSharedDict, ParsedSharedDictItem,
    RawSharedDict, RawSharedDictEncoding, RawSharedDictItem, StreamType,
};
use crate::encoder::stream::{DataProfile, FsstStrEncoder, IntEncoder, dedup_strings};
use crate::encoder::{
    EncodedFsstData, EncodedPlainData, EncodedStream, EncodedStringsEncoding, Encoder, StrEncoding,
};

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

/// Encode a string column and write it to `enc`.
///
/// When [`Encoder::get_str_encoding`] returns [`None`], tries Plain and (if viable) FSST, keeping the shorter.
/// When [`Some`], uses the caller-specified string encoding and [`Encoder::get_int_encoder`] for sub-streams.
///
/// The `col_type` + `name` are written to `enc.meta` once; the stream count + streams
/// go into alternatives in `enc.data`.
pub(crate) fn write_str_col(
    v: &StagedStrings,
    presence_stream: Option<&EncodedStream>,
    enc: &mut Encoder,
) -> MltResult<()> {
    let dense_values = v.dense_values();
    let non_null: Vec<&str> = dense_values.iter().map(String::as_str).collect();

    enc.start_alternatives();
    if let Some(str_enc) = enc.get_str_encoding("prop", &v.name) {
        // Explicit path: one deterministic encoding chosen by the caller.
        let encoding = match str_enc {
            StrEncoding::Plain => EncodedStream::encode_strings_with_type(
                &dense_values,
                enc.get_int_encoder("prop", &v.name, Some("lengths"))
                    .expect("explicit string column int encoders"),
                LengthType::VarBinary,
                DictionaryType::None,
            )?,
            StrEncoding::Dict => EncodedStream::encode_strings_dict(
                &dense_values,
                enc.get_int_encoder("prop", &v.name, Some("lengths"))
                    .expect("explicit string column int encoders"),
                enc.get_int_encoder("prop", &v.name, Some("offsets"))
                    .expect("explicit string column int encoders"),
            )?,
            StrEncoding::Fsst => EncodedStream::encode_strings_fsst_with_type(
                &dense_values,
                FsstStrEncoder {
                    symbol_lengths: enc
                        .get_int_encoder("prop", &v.name, Some("sym_lengths"))
                        .expect("explicit string column int encoders"),
                    dict_lengths: enc
                        .get_int_encoder("prop", &v.name, Some("dict_lengths"))
                        .expect("explicit string column int encoders"),
                },
                DictionaryType::Single,
            )?,
            StrEncoding::FsstDict => EncodedStream::encode_strings_fsst_dict(
                &dense_values,
                FsstStrEncoder {
                    symbol_lengths: enc
                        .get_int_encoder("prop", &v.name, Some("sym_lengths"))
                        .expect("explicit string column int encoders"),
                    dict_lengths: enc
                        .get_int_encoder("prop", &v.name, Some("dict_lengths"))
                        .expect("explicit string column int encoders"),
                },
                enc.get_int_encoder("prop", &v.name, Some("offsets"))
                    .expect("explicit string column int encoders"),
            )?,
        };
        write_str_streams(encoding.streams(), presence_stream, enc)?;
        enc.finish_alternative();
    } else {
        // Auto path: two-level competition.
        //
        //  Outer level  — which string-encoding type wins overall?
        //    Plain / Dict / FSST (if viable) / FsstDict (if viable)
        //
        //  Inner level (Plain only) — which int encoder for the lengths stream?
        //    Candidates from DataProfile::prune_candidates, written as actual
        //    bytes so the winner is the real smallest encoding.
        //
        // All encodings are compared by their total written bytes, so the outer
        // competition automatically accounts for the inner winner's size.

        // Pre-compute data needed outside the alternatives loops.
        let plain_lengths = strings_to_lengths(&non_null)?;
        let plain_len_cands = DataProfile::prune_candidates::<i32>(&plain_lengths);

        // Dedup once; reused by Dict and FsstDict candidates.
        let (unique, offset_indices) = dedup_strings(&non_null)?;
        let dict_lengths = strings_to_lengths(&unique)?;
        let dict_len_cands = DataProfile::prune_candidates::<i32>(&dict_lengths);
        let offset_enc = if offset_indices.is_empty() {
            IntEncoder::plain()
        } else {
            IntEncoder::auto_u32(&offset_indices)
        };

        let use_fsst = fsst_is_viable(&non_null);

        // ── Plain (inner: length-encoder competition) ─────────────────────
        enc.start_alternatives();
        for lenc in plain_len_cands {
            let encoding = EncodedStream::encode_strings_with_type(
                &non_null,
                lenc,
                LengthType::VarBinary,
                DictionaryType::None,
            )?;
            write_str_streams(encoding.streams(), presence_stream, enc)?;
            enc.finish_alternative();
        }
        enc.finish_alternatives();
        enc.finish_alternative(); // commit best-Plain outer candidate

        // ── Dict (inner: length-encoder competition, offset_enc is fixed) ─
        enc.start_alternatives();
        for lenc in dict_len_cands {
            let encoding = EncodedStream::encode_strings_dict(&non_null, lenc, offset_enc)?;
            write_str_streams(encoding.streams(), presence_stream, enc)?;
            enc.finish_alternative();
        }
        enc.finish_alternatives();
        enc.finish_alternative(); // commit best-Dict outer candidate

        // ── FSST / FsstDict — flat candidates (re-training FSST is expensive)
        if use_fsst {
            let fsst_enc = FsstStrEncoder {
                symbol_lengths: IntEncoder::varint(),
                dict_lengths: IntEncoder::varint(),
            };
            let encoding = EncodedStream::encode_strings_fsst_with_type(
                &non_null,
                fsst_enc,
                DictionaryType::Single,
            )?;
            write_str_streams(encoding.streams(), presence_stream, enc)?;
            enc.finish_alternative(); // FSST outer candidate

            let encoding =
                EncodedStream::encode_strings_fsst_dict(&non_null, fsst_enc, offset_enc)?;
            write_str_streams(encoding.streams(), presence_stream, enc)?;
            enc.finish_alternative(); // FsstDict outer candidate
        }
    }
    enc.finish_alternatives();
    Ok(())
}

/// Write the stream-count varint + optional presence stream + content streams to `enc.data`.
fn write_str_streams(
    content_streams: Vec<&EncodedStream>,
    presence_stream: Option<&EncodedStream>,
    enc: &mut Encoder,
) -> MltResult<()> {
    let stream_count =
        u32::try_from(content_streams.len() + usize::from(presence_stream.is_some()))?;
    enc.write_varint(stream_count)?;
    enc.write_optional_stream(presence_stream)?;
    for s in content_streams {
        enc.write_stream(s)?;
    }
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

    let dict_enc = if let Some(str_enc) = enc.get_str_encoding("prop", &shared_dict.prefix) {
        // Explicit path: use caller-specified encoding.
        match str_enc {
            StrEncoding::Fsst | StrEncoding::FsstDict => {
                let fsst_enc = FsstStrEncoder {
                    symbol_lengths: enc
                        .get_int_encoder("prop", &shared_dict.prefix, Some("sym_lengths"))
                        .expect("explicit shared-dict int encoders"),
                    dict_lengths: enc
                        .get_int_encoder("prop", &shared_dict.prefix, Some("dict_lengths"))
                        .expect("explicit shared-dict int encoders"),
                };
                let encoded = EncodedStream::encode_strings_fsst_plain_with_type(
                    &dict,
                    fsst_enc,
                    DictionaryType::Single,
                )?;
                match encoded {
                    EncodedStringsEncoding::FsstPlain(fsst_data) => {
                        EncodedSharedDictEncoding::FsstPlain(fsst_data)
                    }
                    _ => {
                        unreachable!("encode_strings_fsst_plain_with_type always returns FsstPlain")
                    }
                }
            }
            StrEncoding::Plain | StrEncoding::Dict => {
                let lengths_enc = enc
                    .get_int_encoder("prop", &shared_dict.prefix, Some("lengths"))
                    .expect("explicit shared-dict int encoders");
                let encoded = EncodedStream::encode_strings_with_type(
                    &dict,
                    lengths_enc,
                    LengthType::Dictionary,
                    DictionaryType::Shared,
                )?;
                match encoded {
                    EncodedStringsEncoding::Plain(plain_data) => {
                        EncodedSharedDictEncoding::Plain(plain_data)
                    }
                    _ => unreachable!("encode_strings_with_type always returns Plain"),
                }
            }
        }
    } else {
        // Auto path: FSST if viable, else plain with auto-selected length encoder.
        if fsst_is_viable(&dict) {
            let encoded = EncodedStream::encode_strings_fsst_plain_with_type(
                &dict,
                FsstStrEncoder {
                    symbol_lengths: IntEncoder::varint(),
                    dict_lengths: IntEncoder::varint(),
                },
                DictionaryType::Single,
            )?;
            match encoded {
                EncodedStringsEncoding::FsstPlain(fsst_data) => {
                    EncodedSharedDictEncoding::FsstPlain(fsst_data)
                }
                _ => unreachable!("encode_strings_fsst_plain_with_type always returns FsstPlain"),
            }
        } else {
            let lengths: Vec<u32> = dict
                .iter()
                .map(|s| u32::try_from(s.len()).unwrap_or(u32::MAX))
                .collect();
            let length_enc = IntEncoder::auto_u32(&lengths);
            let encoded = EncodedStream::encode_strings_with_type(
                &dict,
                length_enc,
                LengthType::Dictionary,
                DictionaryType::Shared,
            )?;
            match encoded {
                EncodedStringsEncoding::Plain(plain_data) => {
                    EncodedSharedDictEncoding::Plain(plain_data)
                }
                _ => unreachable!("encode_strings_with_type always returns Plain"),
            }
        }
    };

    // Encode each child column.
    // `force_presence` is only consulted for all-present items (same rule as scalar properties).
    let force_presence = enc.override_presence("prop", &shared_dict.prefix, None);
    let mut children: Vec<EncodedChild> = Vec::with_capacity(shared_dict.items.len());
    for item in &shared_dict.items {
        let has_presence = item.presence().needs_presence_stream()
            || (force_presence && matches!(item.presence(), PresenceKind::AllPresent));
        let presence = if has_presence {
            Some(EncodedStream::encode_presence(&item.presence_bools())?)
        } else {
            None
        };

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

        let offsets_enc = enc
            .get_int_encoder("prop", &shared_dict.prefix, Some(&item.suffix))
            .unwrap_or_else(|| {
                if offsets.is_empty() {
                    IntEncoder::plain()
                } else {
                    IntEncoder::auto_u32(&offsets)
                }
            });

        let data = EncodedStream::encode_u32s_of_type(
            &offsets,
            offsets_enc,
            StreamType::Offset(OffsetType::String),
        )?;

        children.push(EncodedChild { presence, data });
    }

    // Write column metadata.
    ColumnType::SharedDict.write_to(&mut enc.meta)?;
    enc.meta.write_string(&shared_dict.prefix)?;
    enc.meta.write_varint(u32::try_from(children.len())?)?;
    for (item, child) in shared_dict.items.iter().zip(&children) {
        let child_type = if child.presence.is_some() {
            ColumnType::OptStr
        } else {
            ColumnType::Str
        };
        child_type.write_to(&mut enc.meta)?;
        enc.meta.write_string(&item.suffix)?;
    }

    // Write stream data.
    let dict_streams = dict_enc.dict_streams();
    let dict_stream_len = u32::try_from(dict_streams.len())?;
    let children_len = u32::try_from(children.len())?;
    let optional_children_count = children.iter().filter(|c| c.presence.is_some()).count();
    let optional_children_len = u32::try_from(optional_children_count)?;
    let stream_len = checked_sum3(dict_stream_len, children_len, optional_children_len)?;
    enc.write_varint(stream_len)?;
    for stream in dict_streams {
        enc.write_stream(stream)?;
    }
    for child in &children {
        enc.write_varint(1 + u32::from(child.presence.is_some()))?;
        enc.write_optional_stream(child.presence.as_ref())?;
        enc.write_stream(&child.data)?;
    }

    enc.increment_column_count();
    Ok(true)
}

/// Internal encoding of a shared-dict corpus (Plain or FSST-compressed).
enum EncodedSharedDictEncoding {
    Plain(EncodedPlainData),
    FsstPlain(EncodedFsstData),
}

impl EncodedSharedDictEncoding {
    fn dict_streams(&self) -> Vec<&EncodedStream> {
        match self {
            Self::Plain(plain_data) => plain_data.streams(),
            Self::FsstPlain(fsst_data) => fsst_data.streams(),
        }
    }
}
use crate::errors::AsMltError as _;
use crate::utils::{AsUsize as _, BinarySerializer as _, checked_sum3, strings_to_lengths};
use crate::{Decoder, MltResult};

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

struct EncodedChild {
    presence: Option<EncodedStream>,
    data: EncodedStream,
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
