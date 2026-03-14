use std::borrow::Cow;
use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::io::Write;

use crate::MltError::{
    BufferUnderflow, DictIndexOutOfBounds, NotImplemented, UnexpectedStreamType2,
};
use crate::utils::AsUsize as _;
use crate::v01::{
    ColumnType, DictionaryType, EncodedFsstData, EncodedName, EncodedPlainData, EncodedProperty,
    EncodedSharedDict, EncodedSharedDictChild, EncodedSharedDictEncoding, EncodedStream,
    EncodedStrings, EncodedStringsEncoding, FsstStrEncoder, IntEncoder, LengthType, OffsetType,
    ParsedSharedDict, ParsedSharedDictItem, ParsedStrings, PresenceStream, PropertyEncoder,
    RawFsstData, RawPlainData, RawPresence, RawSharedDict, RawSharedDictChild,
    RawSharedDictEncoding, RawStream, RawStrings, RawStringsEncoding, SharedDictEncoder,
    StagedSharedDict, StagedSharedDictItem, StagedStrings, StrEncoder, StreamType,
};
use crate::{Analyze, DecodeInto, MltError, StatType};

impl StrEncoder {
    #[must_use]
    pub fn plain(string_lengths: IntEncoder) -> Self {
        Self::Plain { string_lengths }
    }
    #[must_use]
    pub fn fsst(symbol_lengths: IntEncoder, dict_lengths: IntEncoder) -> Self {
        Self::Fsst(FsstStrEncoder {
            symbol_lengths,
            dict_lengths,
        })
    }
}

impl From<Vec<Option<String>>> for ParsedStrings<'static> {
    fn from(values: Vec<Option<String>>) -> Self {
        Self::from_optional_strings(values)
    }
}

impl From<Vec<String>> for ParsedStrings<'static> {
    fn from(values: Vec<String>) -> Self {
        Self::from_optional_strings(values.into_iter().map(Some).collect())
    }
}

impl ParsedStrings<'static> {
    fn from_optional_strings(values: Vec<Option<String>>) -> Self {
        let mut lengths = Vec::with_capacity(values.len());
        let mut data = String::new();
        let mut end = 0_i32;
        for value in values {
            match value {
                Some(value) => {
                    end = checked_string_end(end, value.len())
                        .expect("decoded string corpus exceeds supported i32 range");
                    lengths.push(end);
                    data.push_str(&value);
                }
                None => lengths.push(encode_null_end(end)),
            }
        }
        Self {
            name: Cow::Borrowed(""),
            lengths,
            data: Cow::Owned(data),
        }
    }
}

#[cfg(all(not(test), feature = "arbitrary"))]
impl<'a> arbitrary::Arbitrary<'a> for ParsedStrings<'static> {
    fn arbitrary(u: &mut arbitrary::Unstructured<'a>) -> arbitrary::Result<Self> {
        Ok(Self::from(u.arbitrary::<Vec<Option<String>>>()?))
    }
}

impl ParsedSharedDict<'_> {}

#[cfg(all(not(test), feature = "arbitrary"))]
impl<'a> arbitrary::Arbitrary<'a> for ParsedSharedDict<'static> {
    fn arbitrary(u: &mut arbitrary::Unstructured<'a>) -> arbitrary::Result<Self> {
        let prefix = Cow::Owned(u.arbitrary()?);
        let mut data = String::new();
        for value in u.arbitrary::<Vec<String>>()? {
            data.push_str(&value);
        }
        let items: Vec<(String, Vec<(i32, i32)>)> = u.arbitrary()?;
        let items = items
            .into_iter()
            .map(|(suffix, ranges)| ParsedSharedDictItem {
                suffix: Cow::Owned(suffix),
                ranges,
            })
            .collect();
        Ok(Self {
            prefix,
            data: Cow::Owned(data),
            items,
        })
    }
}

// ── StagedStrings ─────────────────────────────────────────────────────────────

impl From<Vec<Option<String>>> for StagedStrings {
    fn from(values: Vec<Option<String>>) -> Self {
        Self::from_optional_strings(values)
    }
}

impl From<Vec<String>> for StagedStrings {
    fn from(values: Vec<String>) -> Self {
        Self::from_optional_strings(values.into_iter().map(Some).collect())
    }
}

impl StagedStrings {
    fn from_optional_strings(values: Vec<Option<String>>) -> Self {
        let mut lengths = Vec::with_capacity(values.len());
        let mut data = String::new();
        let mut end = 0_i32;
        for value in values {
            match value {
                Some(value) => {
                    end = checked_string_end(end, value.len())
                        .expect("staged string corpus exceeds supported i32 range");
                    lengths.push(end);
                    data.push_str(&value);
                }
                None => lengths.push(encode_null_end(end)),
            }
        }
        Self {
            name: String::new(),
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
    pub fn has_nulls(&self) -> bool {
        self.lengths.iter().any(|&end| end < 0)
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

    #[must_use]
    pub fn materialize(&self) -> Vec<Option<String>> {
        (0..u32::try_from(self.feature_count()).unwrap_or(u32::MAX))
            .map(|i| self.get(i).map(str::to_string))
            .collect()
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

pub(crate) fn collect_staged_shared_dict_spans(items: &[StagedSharedDictItem]) -> Vec<(u32, u32)> {
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
    pub fn has_nulls(&self) -> bool {
        self.ranges
            .iter()
            .any(|&range| decode_shared_dict_range(range).is_none())
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

impl ParsedStrings<'_> {
    #[must_use]
    pub fn feature_count(&self) -> usize {
        self.lengths.len()
    }

    #[must_use]
    pub fn has_nulls(&self) -> bool {
        self.lengths.iter().any(|end| *end < 0)
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
}

impl Analyze for ParsedStrings<'_> {
    fn collect_statistic(&self, stat: StatType) -> usize {
        let meta = if stat == StatType::DecodedMetaSize {
            self.name.len()
        } else {
            0
        };
        meta + self.dense_values().collect_statistic(stat)
    }
}

fn encode_shared_dict_range(start: u32, end: u32) -> Result<(i32, i32), MltError> {
    Ok((i32::try_from(start)?, i32::try_from(end)?))
}

fn decode_shared_dict_range(range: (i32, i32)) -> Option<(u32, u32)> {
    if let (Ok(start), Ok(end)) = (u32::try_from(range.0), u32::try_from(range.1)) {
        Some((start, end))
    } else {
        None
    }
}

fn shared_dict_spans(lengths: &[u32]) -> Vec<(u32, u32)> {
    lengths
        .iter()
        .scan(0_u32, |offset, len| {
            let start = *offset;
            *offset = offset.saturating_add(*len);
            Some((start, *offset))
        })
        .collect()
}

fn resolve_dict_spans(
    offsets: &[u32],
    presence: Option<&[bool]>,
    dict_spans: &[(u32, u32)],
) -> Result<Vec<Option<(u32, u32)>>, MltError> {
    let present_count = presence.map_or(offsets.len(), <[bool]>::len);
    let mut resolved = Vec::with_capacity(present_count);
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

fn dict_span_str(dict_data: &str, span: (u32, u32)) -> Result<&str, MltError> {
    let start = span.0.as_usize();
    let end = span.1.as_usize();
    let bytes = dict_data.as_bytes();
    let Some(value) = bytes.get(start..end) else {
        let len = span.1.saturating_sub(span.0);
        return Err(BufferUnderflow(len, bytes.len().saturating_sub(start)));
    };
    Ok(str::from_utf8(value)?)
}

impl ParsedSharedDict<'_> {
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
}

impl ParsedSharedDictItem<'_> {
    #[must_use]
    pub fn feature_count(&self) -> usize {
        self.ranges.len()
    }

    #[must_use]
    pub fn has_nulls(&self) -> bool {
        self.ranges
            .iter()
            .any(|&range| decode_shared_dict_range(range).is_none())
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
    pub fn new(lengths: RawStream<'a>, data: RawStream<'a>) -> Result<Self, MltError> {
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

    pub fn decode(&self) -> Result<(&'a str, Vec<u32>), MltError> {
        Ok((
            str::from_utf8(self.data.as_bytes())?,
            self.lengths.decode_bits_u32()?.decode_u32()?,
        ))
    }

    #[must_use]
    pub fn streams(&self) -> Vec<&RawStream<'_>> {
        vec![&self.lengths, &self.data]
    }
}

impl EncodedPlainData {
    pub fn new(lengths: EncodedStream, data: EncodedStream) -> Result<Self, MltError> {
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
    ) -> Result<Self, MltError> {
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

    pub fn decode(&self) -> Result<(String, Vec<u32>), MltError> {
        let sym_lens = self.symbol_lengths.decode_bits_u32()?.decode_u32()?;
        let sym_data = self.symbol_table.as_bytes();
        let compressed = self.corpus.as_bytes();
        let decompressed = decode_fsst(sym_data, &sym_lens, compressed);
        Ok((
            String::from_utf8(decompressed)?,
            self.lengths.decode_bits_u32()?.decode_u32()?,
        ))
    }

    #[must_use]
    pub fn streams(&self) -> Vec<&RawStream<'_>> {
        vec![
            &self.symbol_lengths,
            &self.symbol_table,
            &self.lengths,
            &self.corpus,
        ]
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

    pub fn dictionary(
        plain_data: RawPlainData<'a>,
        offsets: RawStream<'a>,
    ) -> Result<Self, MltError> {
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

    pub fn fsst_dictionary(
        fsst_data: RawFsstData<'a>,
        offsets: RawStream<'a>,
    ) -> Result<Self, MltError> {
        validate_stream!(offsets, StreamType::Offset(OffsetType::String));
        Ok(Self::FsstDictionary { fsst_data, offsets })
    }

    /// Content streams in wire order.
    #[must_use]
    pub fn streams(&self) -> Vec<&RawStream<'_>> {
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

impl EncodedStringsEncoding {
    /// Content streams only.
    #[must_use]
    pub fn content_streams(&self) -> Vec<&EncodedStream> {
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

    /// Streams in wire order.
    #[must_use]
    pub fn streams(&self) -> Vec<&EncodedStream> {
        self.content_streams()
    }
}

impl RawStrings<'_> {
    /// Content streams in wire order.
    #[must_use]
    pub fn streams(&self) -> Vec<&RawStream<'_>> {
        self.encoding.streams()
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

    /// Dict streams in wire order (for serialization).
    #[must_use]
    pub fn dict_streams(&self) -> Vec<&RawStream<'_>> {
        match self {
            Self::Plain(plain_data) => plain_data.streams(),
            Self::FsstPlain(fsst_data) => fsst_data.streams(),
        }
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

impl RawSharedDict<'_> {
    /// Dict streams in wire order (for serialization).
    #[must_use]
    pub fn dict_streams(&self) -> Vec<&RawStream<'_>> {
        self.encoding.dict_streams()
    }
}

impl EncodedSharedDict {
    #[must_use]
    pub fn dict_streams(&self) -> Vec<&EncodedStream> {
        self.encoding.dict_streams()
    }
}

/// Encode a staged shared dictionary property using `SharedDictEncoder`.
pub fn encode_shared_dict_prop(
    shared_dict: &StagedSharedDict,
    encoder: &SharedDictEncoder,
) -> Result<EncodedProperty, MltError> {
    if shared_dict.items.len() != encoder.items.len() {
        return Err(NotImplemented(
            "SharedDict items count must match encoder items count",
        ));
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
    };

    // Encode each child column.
    let mut children = Vec::with_capacity(shared_dict.items.len());
    for (item, item_enc) in shared_dict.items.iter().zip(&encoder.items) {
        // Presence stream
        let presence = if item_enc.presence == PresenceStream::Present {
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

        children.push(EncodedSharedDictChild {
            name: EncodedName(item.suffix.clone()),
            presence: crate::v01::EncodedPresence(presence),
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

    Ok(EncodedProperty::SharedDict(EncodedSharedDict {
        name: EncodedName(shared_dict.prefix.clone()),
        encoding,
        children,
    }))
}

/// Build a [`StagedSharedDict`] from a list of `(suffix, values)` pairs.
///
/// Deduplicates string values into a shared corpus and records per-feature
/// byte-range offsets into it.
pub fn build_staged_shared_dict(
    prefix: impl Into<String>,
    items: impl IntoIterator<Item = (String, StagedStrings)>,
) -> Result<StagedSharedDict, MltError> {
    let prefix = prefix.into();
    let items = items.into_iter().collect::<Vec<_>>();
    let mut dict_entries = Vec::<String>::new();
    let mut dict_index = HashMap::<String, u32>::new();

    for (_, values) in &items {
        for value in values.dense_values() {
            if let Entry::Vacant(entry) = dict_index.entry(value.clone()) {
                let idx = u32::try_from(dict_entries.len())?;
                entry.insert(idx);
                dict_entries.push(value);
            }
        }
    }

    let mut dict_ranges = Vec::with_capacity(dict_entries.len());
    let mut data = String::new();
    for value in &dict_entries {
        let offset = u32::try_from(data.len())?;
        let len = u32::try_from(value.len())?;
        let end = offset.saturating_add(len);
        dict_ranges.push((offset, end));
        data.push_str(value);
    }

    let items = items
        .into_iter()
        .map(
            |(suffix, values)| -> Result<StagedSharedDictItem, MltError> {
                let mut ranges = Vec::with_capacity(values.feature_count());
                for i in 0..u32::try_from(values.feature_count())? {
                    if let Some(value) = values.get(i) {
                        let idx = dict_index
                            .get(value)
                            .copied()
                            .ok_or(DictIndexOutOfBounds(0, dict_entries.len()))?;
                        let span = dict_ranges[idx as usize];
                        ranges.push(encode_shared_dict_range(span.0, span.1)?);
                    } else {
                        ranges.push((-1, -1));
                    }
                }
                Ok(StagedSharedDictItem { suffix, ranges })
            },
        )
        .collect::<Result<Vec<_>, _>>()?;

    Ok(StagedSharedDict {
        prefix,
        data,
        items,
    })
}

impl EncodedSharedDictChild {
    pub(crate) fn write_columns_meta_to<W: Write>(&self, writer: &mut W) -> Result<(), MltError> {
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
    pub fn into_decoded(self) -> Result<ParsedStrings<'a>, MltError> {
        let name = self.name;
        let presence = self
            .presence
            .0
            .map(DecodeInto::<Vec<bool>>::decode_into)
            .transpose()?;
        match self.encoding {
            RawStringsEncoding::Plain(plain_data) => {
                let (data, lengths) = plain_data.decode()?;
                Ok(ParsedStrings {
                    name: Cow::Borrowed(name),
                    lengths: to_absolute_lengths(&lengths, presence.as_deref())?,
                    data: data.into(),
                })
            }
            RawStringsEncoding::Dictionary {
                plain_data,
                offsets,
            } => {
                let (data, lengths) = plain_data.decode()?;
                let offsets: Vec<u32> = offsets.decode_into()?;
                decode_dictionary_strings(name, &lengths, &offsets, presence.as_deref(), data)
            }
            RawStringsEncoding::FsstPlain(fsst_data) => {
                let (data, dict_lens) = fsst_data.decode()?;
                Ok(ParsedStrings {
                    name: Cow::Borrowed(name),
                    lengths: to_absolute_lengths(&dict_lens, presence.as_deref())?,
                    data: data.into(),
                })
            }
            RawStringsEncoding::FsstDictionary { fsst_data, offsets } => {
                let (data, lengths) = fsst_data.decode()?;
                let offsets: Vec<u32> = offsets.decode_into()?;
                decode_dictionary_strings(name, &lengths, &offsets, presence.as_deref(), &data)
            }
        }
    }
}

fn to_absolute_lengths(lengths: &[u32], presence: Option<&[bool]>) -> Result<Vec<i32>, MltError> {
    let mut absolute = Vec::with_capacity(presence.map_or(lengths.len(), <[bool]>::len));
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
) -> Result<ParsedStrings<'a>, MltError> {
    let dict_spans = shared_dict_spans(dict_lengths);
    let resolved_spans = resolve_dict_spans(offsets, presence, &dict_spans)?;
    let mut lengths = Vec::with_capacity(resolved_spans.len());
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
        name: Cow::Borrowed(name),
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

fn checked_string_end(current_end: i32, byte_len: usize) -> Result<i32, MltError> {
    let byte_len = u32::try_from(byte_len)?;
    checked_absolute_end(current_end, byte_len)
}

fn checked_absolute_end(current_end: i32, delta: u32) -> Result<i32, MltError> {
    let delta = i32::try_from(delta)?;
    current_end
        .checked_add(delta)
        .ok_or(MltError::IntegerOverflow)
}

fn decode_fsst(symbols: &[u8], symbol_lengths: &[u32], compressed: &[u8]) -> Vec<u8> {
    // Build symbol offset table
    let mut symbol_offsets = vec![0u32; symbol_lengths.len()];
    for i in 1..symbol_lengths.len() {
        symbol_offsets[i] = symbol_offsets[i - 1] + symbol_lengths[i - 1];
    }
    let mut output = Vec::new();
    let mut i = 0;
    while i < compressed.len() {
        let sym_idx = usize::from(compressed[i]);
        if sym_idx == 255 {
            i += 1;
            output.push(compressed[i]);
        } else if sym_idx < symbol_lengths.len() {
            let len = symbol_lengths[sym_idx].as_usize();
            let off = symbol_offsets[sym_idx].as_usize();
            output.extend_from_slice(&symbols[off..off + len]);
        }
        i += 1;
    }
    output
}

impl<'a> RawSharedDict<'a> {
    #[must_use]
    pub fn new(
        name: &'a str,
        encoding: RawSharedDictEncoding<'a>,
        children: Vec<RawSharedDictChild<'a>>,
    ) -> Self {
        Self {
            name,
            encoding,
            children,
        }
    }

    /// Decode a shared-dictionary column into its decoded form.
    pub fn into_decoded(self) -> Result<ParsedSharedDict<'a>, MltError> {
        let prefix = Cow::Borrowed(self.name);
        let (data, dict_spans) = match self.encoding {
            RawSharedDictEncoding::Plain(plain_data) => {
                let (decoded, lengths) = plain_data.decode()?;
                let dict_spans = shared_dict_spans(&lengths);
                (Cow::Borrowed(decoded), dict_spans)
            }
            RawSharedDictEncoding::FsstPlain(fsst_data) => {
                let (decoded, lengths) = fsst_data.decode()?;
                let dict_spans = shared_dict_spans(&lengths);
                (decoded.into(), dict_spans)
            }
        };
        let items = self
            .children
            .into_iter()
            .map(|child| -> Result<ParsedSharedDictItem, MltError> {
                let offsets: Vec<u32> = child.data.decode_into()?;
                let presence = child
                    .presence
                    .0
                    .map(DecodeInto::<Vec<bool>>::decode_into)
                    .transpose()?;
                let ranges = resolve_dict_spans(&offsets, presence.as_deref(), &dict_spans)?
                    .into_iter()
                    .map(|span| match span {
                        Some(span) => encode_shared_dict_range(span.0, span.1),
                        None => Ok((-1, -1)),
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(ParsedSharedDictItem {
                    suffix: Cow::Borrowed(child.name),
                    ranges,
                })
            })
            .collect::<Result<Vec<_>, _>>()?;

        Ok(ParsedSharedDict {
            prefix,
            data,
            items,
        })
    }
}

impl From<SharedDictEncoder> for PropertyEncoder {
    fn from(encoder: SharedDictEncoder) -> Self {
        Self::SharedDict(encoder)
    }
}
