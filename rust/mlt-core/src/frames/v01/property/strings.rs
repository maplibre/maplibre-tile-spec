use std::borrow::Cow;
use std::collections::HashMap;
use std::io::Write;

use crate::MltError::{
    BufferUnderflow, DictIndexOutOfBounds, NotImplemented, UnexpectedStreamType2,
};
use crate::codecs::fsst::decode_fsst;
use crate::errors::AsMltError as _;
use crate::utils::AsUsize as _;
use crate::v01::{
    ColumnRef, ColumnType, DictionaryType, EncodedFsstData, EncodedName, EncodedPlainData,
    EncodedPresence, EncodedProperty, EncodedSharedDict, EncodedSharedDictEncoding,
    EncodedSharedDictItem, EncodedStream, EncodedStrings, EncodedStringsEncoding, FsstStrEncoder,
    IntEncoder, LengthType, OffsetType, ParsedSharedDict, ParsedSharedDictItem, ParsedStrings,
    PresenceKind, PropValueRef, PropertyEncoder, RawFsstData, RawPlainData, RawPresence,
    RawSharedDict, RawSharedDictEncoding, RawSharedDictItem, RawStream, RawStrings,
    RawStringsEncoding, SharedDictEncoder, SharedDictItemEncoder, StagedSharedDict,
    StagedSharedDictItem, StagedStrings, StrEncoder, StreamType,
};
use crate::{Decoder, MltError, MltResult};

impl StrEncoder {
    #[must_use]
    pub fn plain(string_lengths: IntEncoder) -> Self {
        Self::Plain { string_lengths }
    }
    #[must_use]
    pub fn dict(string_lengths: IntEncoder, offsets: IntEncoder) -> Self {
        Self::Dict {
            string_lengths,
            offsets,
        }
    }
    #[must_use]
    pub fn fsst(symbol_lengths: IntEncoder, dict_lengths: IntEncoder) -> Self {
        Self::Fsst(FsstStrEncoder {
            symbol_lengths,
            dict_lengths,
        })
    }
    #[must_use]
    pub fn fsst_dict(
        symbol_lengths: IntEncoder,
        dict_lengths: IntEncoder,
        offsets: IntEncoder,
    ) -> Self {
        Self::FsstDict {
            fsst: FsstStrEncoder {
                symbol_lengths,
                dict_lengths,
            },
            offsets,
        }
    }
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

impl SharedDictItemEncoder {
    /// Create a new encoder for a shared-dictionary child column.
    #[must_use]
    pub fn new(offsets: IntEncoder) -> Self {
        Self {
            offsets,
            #[cfg(feature = "__private")]
            forced_presence: false,
        }
    }

    /// Force a presence stream to be emitted even when the column has no nulls.
    /// Intended only for generating intentionally edge-case tiles in synthetics/tests.
    #[cfg(feature = "__private")]
    #[must_use]
    pub fn forced_presence(mut self, present: bool) -> Self {
        self.forced_presence = present;
        self
    }
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

// ── ParsedStrings ─────────────────────────────────────────────────────────────

impl<'a> ParsedStrings<'a> {
    #[must_use]
    pub fn new(name: &'a str, lengths: Vec<i32>, data: Cow<'a, str>) -> Self {
        ParsedStrings {
            name,
            lengths,
            data,
        }
    }

    #[must_use]
    pub fn feature_count(&self) -> usize {
        self.lengths.len()
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

    fn bounds(&self, i: u32) -> Option<(u32, u32)> {
        let idx = i.as_usize();
        let end = *self.lengths.get(idx)?;
        if end < 0 {
            return None;
        }
        let start = idx
            .checked_sub(1)
            .and_then(|prev| self.lengths.get(prev).copied())
            .map_or(0, decode_end);
        Some((start, decode_end(end)))
    }

    #[must_use]
    pub fn get(&self, i: u32) -> Option<&str> {
        let (start, end) = self.bounds(i)?;
        let start = start.as_usize();
        let end = end.as_usize();
        self.data.get(start..end)
    }

    #[must_use]
    pub fn dense_values(&self) -> Vec<String> {
        let mut values = Vec::new();
        let mut start = 0_u32;
        for &end in &self.lengths {
            let end_u32 = decode_end(end);
            let start_idx = start.as_usize();
            let end_idx = end_u32.as_usize();
            if end >= 0
                && let Some(value) = self.data.get(start_idx..end_idx)
            {
                values.push(value.to_string());
            }
            start = end_u32;
        }
        values
    }

    #[must_use]
    pub fn materialize(&self) -> Vec<Option<String>> {
        (0..u32::try_from(self.feature_count()).unwrap_or(u32::MAX))
            .map(|i| self.get(i).map(str::to_string))
            .collect()
    }

    #[inline]
    pub(crate) fn value_at(&'a self, feat_idx: usize) -> Option<PropValueRef<'a>> {
        let idx = u32::try_from(feat_idx).expect("feat_idx fits u32");
        self.get(idx).map(PropValueRef::Str)
    }

    #[inline]
    pub(crate) fn column_at(&'a self, idx: usize) -> Option<ColumnRef<'a>> {
        self.value_at(idx).map(|v| ColumnRef::new(self.name, v))
    }
}

fn encode_shared_dict_range(start: u32, end: u32) -> MltResult<(i32, i32)> {
    Ok((i32::try_from(start)?, i32::try_from(end)?))
}

fn decode_shared_dict_range(range: (i32, i32)) -> Option<(u32, u32)> {
    if let (Ok(start), Ok(end)) = (u32::try_from(range.0), u32::try_from(range.1)) {
        Some((start, end))
    } else {
        None
    }
}

fn shared_dict_spans(lengths: &[u32], dec: &mut Decoder) -> MltResult<Vec<(u32, u32)>> {
    let mut spans = dec.alloc(lengths.len())?;
    let mut offset = 0_u32;
    for &len in lengths {
        let start = offset;
        offset = offset.saturating_add(len);
        spans.push((start, offset));
    }
    Ok(spans)
}

fn resolve_dict_spans(
    offsets: &[u32],
    presence: Option<&[bool]>,
    dict_spans: &[(u32, u32)],
    dec: &mut Decoder,
) -> MltResult<Vec<Option<(u32, u32)>>> {
    let present_count = presence.map_or(offsets.len(), <[bool]>::len);
    let mut resolved = dec.alloc(present_count)?;
    let mut next = offsets.iter().copied();

    if let Some(presence) = presence {
        let fail = || {
            MltError::PresenceValueCountMismatch(
                presence.iter().filter(|&&v| v).count(),
                offsets.len(),
            )
        };
        for &present in presence {
            if !present {
                resolved.push(None);
                continue;
            }
            let idx = next.next().ok_or_else(fail)?;
            let span = dict_spans
                .get(idx as usize)
                .copied()
                .ok_or(DictIndexOutOfBounds(idx, dict_spans.len()))?;
            resolved.push(Some(span));
        }

        if next.next().is_some() {
            return Err(fail());
        }
    } else {
        for &idx in offsets {
            let span = dict_spans
                .get(idx as usize)
                .copied()
                .ok_or(DictIndexOutOfBounds(idx, dict_spans.len()))?;
            resolved.push(Some(span));
        }
    }

    Ok(resolved)
}

fn dict_span_str(dict_data: &str, span: (u32, u32)) -> MltResult<&str> {
    let start = span.0.as_usize();
    let end = span.1.as_usize();
    let bytes = dict_data.as_bytes();
    let Some(value) = bytes.get(start..end) else {
        let len = span.1.saturating_sub(span.0);
        return Err(BufferUnderflow(len, bytes.len().saturating_sub(start)));
    };
    Ok(str::from_utf8(value)?)
}

impl<'a> ParsedSharedDict<'a> {
    #[must_use]
    pub fn corpus(&self) -> &str {
        &self.data
    }

    #[must_use]
    pub fn get(&self, span: (u32, u32)) -> Option<&str> {
        let start = span.0.as_usize();
        let end = span.1.as_usize();
        self.corpus().get(start..end)
    }

    #[inline]
    pub(crate) fn value_at(
        &'a self,
        feat_idx: usize,
        dict_idx: &mut usize,
    ) -> Option<(&'a str, PropValueRef<'a>)> {
        if *dict_idx < self.items.len() {
            let idx = *dict_idx;
            *dict_idx += 1;
            let item = &self.items[idx];
            item.get(self, feat_idx)
                .map(|v| (item.suffix, PropValueRef::Str(v)))
        } else {
            *dict_idx = 0;
            None
        }
    }

    #[inline]
    pub(crate) fn column_at(
        &'a self,
        feat_idx: usize,
        dict_idx: &mut usize,
    ) -> Option<ColumnRef<'a>> {
        self.value_at(feat_idx, dict_idx)
            .map(|v| ColumnRef::new_sub(self.prefix, v.0, v.1))
    }
}

impl ParsedSharedDictItem<'_> {
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
    pub fn materialize(&self, shared_dict: &ParsedSharedDict<'_>) -> Vec<Option<String>> {
        self.ranges
            .iter()
            .map(|&range| {
                decode_shared_dict_range(range)
                    .and_then(|span| shared_dict.get(span))
                    .map(str::to_string)
            })
            .collect()
    }

    #[must_use]
    pub fn get<'a>(&self, shared_dict: &'a ParsedSharedDict<'_>, i: usize) -> Option<&'a str> {
        self.ranges
            .get(i)
            .copied()
            .and_then(decode_shared_dict_range)
            .and_then(|span| shared_dict.get(span))
    }
}

/// a helper to validate stream type to match expectation using matches! syntax
macro_rules! validate_stream {
    ($stream:expr, $expected:pat $(,)?) => {
        if !matches!($stream.meta.stream_type, $expected) {
            return Err(UnexpectedStreamType2(
                $stream.meta.stream_type,
                stringify!($expected),
                stringify!($stream),
            ));
        }
    };
}

impl<'a> RawPlainData<'a> {
    pub fn new(lengths: RawStream<'a>, data: RawStream<'a>) -> MltResult<Self> {
        validate_stream!(
            lengths,
            StreamType::Length(LengthType::VarBinary | LengthType::Dictionary)
        );
        validate_stream!(
            data,
            StreamType::Data(
                DictionaryType::None | DictionaryType::Single | DictionaryType::Shared
            )
        );
        Ok(Self { lengths, data })
    }

    pub fn decode(self, dec: &mut Decoder) -> MltResult<(&'a str, Vec<u32>)> {
        Ok((
            str::from_utf8(self.data.as_bytes())?,
            self.lengths.decode_u32s(dec)?,
        ))
    }
}

impl EncodedPlainData {
    pub fn new(lengths: EncodedStream, data: EncodedStream) -> MltResult<Self> {
        validate_stream!(
            lengths,
            StreamType::Length(LengthType::VarBinary | LengthType::Dictionary)
        );
        validate_stream!(
            data,
            StreamType::Data(
                DictionaryType::None | DictionaryType::Single | DictionaryType::Shared
            )
        );
        Ok(Self { lengths, data })
    }

    #[must_use]
    pub fn streams(&self) -> Vec<&EncodedStream> {
        vec![&self.lengths, &self.data]
    }
}

impl<'a> RawFsstData<'a> {
    pub fn new(
        symbol_lengths: RawStream<'a>,
        symbol_table: RawStream<'a>,
        lengths: RawStream<'a>,
        corpus: RawStream<'a>,
    ) -> MltResult<Self> {
        validate_stream!(symbol_lengths, StreamType::Length(LengthType::Symbol));
        validate_stream!(symbol_table, StreamType::Data(DictionaryType::Fsst));
        validate_stream!(lengths, StreamType::Length(LengthType::Dictionary));
        validate_stream!(
            corpus,
            StreamType::Data(DictionaryType::Single | DictionaryType::Shared)
        );
        Ok(Self {
            symbol_lengths,
            symbol_table,
            lengths,
            corpus,
        })
    }

    pub fn decode(self, dec: &mut Decoder) -> MltResult<(String, Vec<u32>)> {
        decode_fsst(self, dec)
    }
}

impl EncodedFsstData {
    #[must_use]
    pub fn streams(&self) -> Vec<&EncodedStream> {
        vec![
            &self.symbol_lengths,
            &self.symbol_table,
            &self.lengths,
            &self.corpus,
        ]
    }
}

impl<'a> RawStringsEncoding<'a> {
    #[must_use]
    pub fn plain(plain_data: RawPlainData<'a>) -> Self {
        Self::Plain(plain_data)
    }

    pub fn dictionary(plain_data: RawPlainData<'a>, offsets: RawStream<'a>) -> MltResult<Self> {
        validate_stream!(offsets, StreamType::Offset(OffsetType::String));
        Ok(Self::Dictionary {
            plain_data,
            offsets,
        })
    }

    #[must_use]
    pub fn fsst_plain(fsst_data: RawFsstData<'a>) -> Self {
        Self::FsstPlain(fsst_data)
    }

    pub fn fsst_dictionary(fsst_data: RawFsstData<'a>, offsets: RawStream<'a>) -> MltResult<Self> {
        validate_stream!(offsets, StreamType::Offset(OffsetType::String));
        Ok(Self::FsstDictionary { fsst_data, offsets })
    }
}

impl EncodedStringsEncoding {
    /// Content streams only.
    #[must_use]
    pub fn streams(&self) -> Vec<&EncodedStream> {
        match self {
            Self::Plain(plain_data) => plain_data.streams(),
            Self::Dictionary {
                plain_data,
                offsets,
            } => {
                let mut streams = plain_data.streams();
                streams.insert(1, offsets); // Offset stays here to preserve the current wire order.
                streams
            }
            Self::FsstPlain(fsst_data) => fsst_data.streams(),
            Self::FsstDictionary { fsst_data, offsets } => {
                let mut streams = fsst_data.streams();
                streams.push(offsets);
                streams
            }
        }
    }
}

impl EncodedStrings {
    /// Streams in wire order.
    #[must_use]
    pub fn streams(&self) -> Vec<&EncodedStream> {
        self.encoding.streams()
    }
}

impl<'a> RawSharedDictEncoding<'a> {
    /// Plain shared dict (2 streams): lengths + data.
    #[must_use]
    pub fn plain(plain_data: RawPlainData<'a>) -> Self {
        Self::Plain(plain_data)
    }

    /// FSST plain shared dict (4 streams): symbol lengths, symbol table, lengths, corpus.
    #[must_use]
    pub fn fsst_plain(fsst_data: RawFsstData<'a>) -> Self {
        Self::FsstPlain(fsst_data)
    }
}

impl EncodedSharedDictEncoding {
    #[must_use]
    pub fn dict_streams(&self) -> Vec<&EncodedStream> {
        match self {
            Self::Plain(plain_data) => plain_data.streams(),
            Self::FsstPlain(fsst_data) => fsst_data.streams(),
        }
    }
}

impl EncodedSharedDict {
    #[must_use]
    pub fn dict_streams(&self) -> Vec<&EncodedStream> {
        self.encoding.dict_streams()
    }
}

/// Encode a staged shared dictionary property using `SharedDictEncoder`.
///
/// Returns `Ok(None)` when every child column is [`PresenceKind::Empty`] or
/// [`PresenceKind::AllNull`] — the whole shared-dict property is skipped.
pub fn encode_shared_dict_prop(
    shared_dict: &StagedSharedDict,
    encoder: &SharedDictEncoder,
) -> MltResult<Option<EncodedProperty>> {
    if shared_dict.items.len() != encoder.items.len() {
        return Err(NotImplemented(
            "SharedDict items count must match encoder items count",
        ));
    }

    // Skip the entire shared-dict property when every item column carries no
    // data worth encoding (empty or all-null).
    if shared_dict
        .items
        .iter()
        .all(|item| matches!(item.presence(), PresenceKind::Empty | PresenceKind::AllNull))
    {
        return Ok(None);
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

    let dict_encoded = match encoder.dict_encoder {
        StrEncoder::Plain { string_lengths } => EncodedStream::encode_strings_with_type(
            &dict,
            string_lengths,
            LengthType::Dictionary,
            DictionaryType::Shared,
        )?,
        StrEncoder::Fsst(enc) => {
            EncodedStream::encode_strings_fsst_plain_with_type(&dict, enc, DictionaryType::Single)?
        }
        StrEncoder::Dict { .. } | StrEncoder::FsstDict { .. } => {
            return Err(NotImplemented(
                "Dict/FsstDict encoder cannot be used as a shared-dict struct encoder",
            ));
        }
    };

    // Encode each child column.
    let mut children = Vec::with_capacity(shared_dict.items.len());
    for (item, item_enc) in shared_dict.items.iter().zip(&encoder.items) {
        #[cfg(feature = "__private")]
        let forced_presence = item_enc.forced_presence;
        #[cfg(not(feature = "__private"))]
        let forced_presence = false;

        let presence = if forced_presence || item.presence().needs_presence_stream() {
            let present_bools = item.presence_bools();
            Some(EncodedStream::encode_presence(&present_bools)?)
        } else {
            None
        };

        // Offset indices for non-null values only.
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

        let data = EncodedStream::encode_u32s_of_type(
            &offsets,
            item_enc.offsets,
            StreamType::Offset(OffsetType::String),
        )?;

        children.push(EncodedSharedDictItem {
            name: EncodedName(item.suffix.clone()),
            presence: EncodedPresence(presence),
            data,
        });
    }

    let encoding = match dict_encoded {
        EncodedStringsEncoding::Plain(plain_data) => EncodedSharedDictEncoding::Plain(plain_data),
        EncodedStringsEncoding::FsstPlain(fsst_data) => {
            EncodedSharedDictEncoding::FsstPlain(fsst_data)
        }
        EncodedStringsEncoding::Dictionary { .. }
        | EncodedStringsEncoding::FsstDictionary { .. } => {
            return Err(NotImplemented(
                "SharedDict only supports Plain or FsstPlain encoding",
            ));
        }
    };

    Ok(Some(EncodedProperty::SharedDict(EncodedSharedDict {
        name: EncodedName(shared_dict.prefix.clone()),
        encoding,
        children,
    })))
}

impl StagedSharedDict {
    /// Build a shared-dictionary column directly from raw per-column string data.
    ///
    /// Each column is a `(suffix, values)` pair where `values` is an iterator of
    /// optional strings (one per feature).  All unique non-null strings across every
    /// column are deduplicated into a shared byte corpus; per-feature byte-range offsets
    /// into that corpus are recorded in each [`StagedSharedDictItem`].
    ///
    /// Unlike the old constructor that took pre-encoded [`StagedStrings`], this works
    /// directly with raw string data and skips the wasteful encode-then-decode round-trip.
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

impl EncodedSharedDictItem {
    pub(crate) fn write_columns_meta_to<W: Write>(&self, writer: &mut W) -> MltResult<()> {
        let typ = if self.presence.0.is_some() {
            ColumnType::OptStr
        } else {
            ColumnType::Str
        };
        typ.write_to(writer)?;
        Ok(())
    }
}

impl<'a> RawStrings<'a> {
    #[must_use]
    pub fn new(name: &'a str, presence: RawPresence<'a>, encoding: RawStringsEncoding<'a>) -> Self {
        Self {
            name,
            presence,
            encoding,
        }
    }

    /// Decode string property from its encoded column.
    pub fn decode(self, dec: &mut Decoder) -> MltResult<ParsedStrings<'a>> {
        let name = self.name;
        let presence = match self.presence.0 {
            Some(s) => Some(s.decode_bools(dec)?),
            None => None,
        };

        let parsed = match self.encoding {
            RawStringsEncoding::Plain(plain_data) => {
                let (data, lengths) = plain_data.decode(dec)?;
                ParsedStrings {
                    name,
                    lengths: to_absolute_lengths(&lengths, presence.as_deref(), dec)?,
                    data: data.into(),
                }
            }
            RawStringsEncoding::Dictionary {
                plain_data,
                offsets,
            } => {
                let (data, lengths) = plain_data.decode(dec)?;
                let offsets: Vec<u32> = offsets.decode_u32s(dec)?;
                decode_dictionary_strings(name, &lengths, &offsets, presence.as_deref(), data, dec)?
            }
            RawStringsEncoding::FsstPlain(fsst_data) => {
                let (data, dict_lens) = fsst_data.decode(dec)?;
                ParsedStrings {
                    name,
                    lengths: to_absolute_lengths(&dict_lens, presence.as_deref(), dec)?,
                    data: data.into(),
                }
            }
            RawStringsEncoding::FsstDictionary { fsst_data, offsets } => {
                let (data, lengths) = fsst_data.decode(dec)?;
                let offsets: Vec<u32> = offsets.decode_u32s(dec)?;
                decode_dictionary_strings(
                    name,
                    &lengths,
                    &offsets,
                    presence.as_deref(),
                    &data,
                    dec,
                )?
            }
        };
        Ok(parsed)
    }
}

fn to_absolute_lengths(
    lengths: &[u32],
    presence: Option<&[bool]>,
    dec: &mut Decoder,
) -> MltResult<Vec<i32>> {
    let capacity = presence.map_or(lengths.len(), <[bool]>::len);
    let mut absolute = dec.alloc(capacity)?;
    let mut iter = lengths.iter().copied();
    let mut end = 0_i32;
    if let Some(presence) = presence {
        for &present in presence {
            if present {
                let len = iter.next().ok_or(MltError::PresenceValueCountMismatch(
                    presence.len(),
                    lengths.len(),
                ))?;
                end = checked_absolute_end(end, len)?;
                absolute.push(end);
            } else {
                absolute.push(encode_null_end(end));
            }
        }
        if iter.next().is_some() {
            return Err(MltError::PresenceValueCountMismatch(
                presence.iter().filter(|v| **v).count(),
                lengths.len(),
            ));
        }
    } else {
        for &len in lengths {
            end = checked_absolute_end(end, len)?;
            absolute.push(end);
        }
    }
    Ok(absolute)
}

fn decode_dictionary_strings<'a>(
    name: &'a str,
    dict_lengths: &[u32],
    offsets: &[u32],
    presence: Option<&[bool]>,
    dict_data: &str,
    dec: &mut Decoder,
) -> MltResult<ParsedStrings<'a>> {
    let dict_spans = shared_dict_spans(dict_lengths, dec)?;
    let resolved_spans = resolve_dict_spans(offsets, presence, &dict_spans, dec)?;
    let mut lengths = dec.alloc(resolved_spans.len())?;
    let mut data = String::new();
    let mut end = 0_i32;
    for span in resolved_spans {
        if let Some(span) = span {
            let value = dict_span_str(dict_data, span)?;
            data.push_str(value);
            end = checked_string_end(end, value.len())?;
            lengths.push(end);
        } else {
            lengths.push(encode_null_end(end));
        }
    }
    Ok(ParsedStrings {
        name,
        lengths,
        data: Cow::Owned(data),
    })
}

fn encode_null_end(end: i32) -> i32 {
    -end - 1
}

fn decode_end(end: i32) -> u32 {
    if end >= 0 {
        u32::try_from(end).expect("non-negative decoded string end must fit in u32")
    } else {
        u32::try_from(-i64::from(end) - 1).expect("encoded null boundary must fit in u32")
    }
}

fn checked_string_end(current_end: i32, byte_len: usize) -> MltResult<i32> {
    let byte_len = u32::try_from(byte_len)?;
    checked_absolute_end(current_end, byte_len)
}

fn checked_absolute_end(current_end: i32, delta: u32) -> MltResult<i32> {
    let delta = i32::try_from(delta)?;
    current_end.checked_add(delta).or_overflow()
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

/// FIXME:  uncertain why we need this, delete?
impl From<SharedDictEncoder> for PropertyEncoder {
    fn from(encoder: SharedDictEncoder) -> Self {
        Self::SharedDict(encoder)
    }
}
