use std::borrow::Cow;
use std::collections::HashMap;
use std::io::Write;

use super::model::{
    EncodedName, EncodedPresence, EncodedProperty, EncodedSharedDict, EncodedSharedDictItem,
    EncodedStringsEncoding, PresenceKind, PropertyEncoder, SharedDictEncoder,
    SharedDictItemEncoder, StagedSharedDict, StagedSharedDictItem, StagedStrings, StrEncoder,
};
use crate::MltError::{DictIndexOutOfBounds, NotImplemented};
use crate::encoder::stream::{FsstStrEncoder, IntEncoder};
use crate::encoder::{
    EncodedFsstData, EncodedPlainData, EncodedSharedDictEncoding, EncodedStream, EncodedStrings,
};
use crate::errors::AsMltError as _;
use crate::frames::v01::strings::{
    checked_string_end, decode_shared_dict_range, encode_null_end, resolve_dict_spans,
    shared_dict_spans,
};
use crate::utils::AsUsize as _;
use crate::v01::{
    ColumnType, DictionaryType, LengthType, OffsetType, ParsedSharedDict, ParsedSharedDictItem,
    RawSharedDict, RawSharedDictEncoding, RawSharedDictItem, StreamType,
};
use crate::{Decoder, MltResult};

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

fn encode_shared_dict_range(start: u32, end: u32) -> MltResult<(i32, i32)> {
    Ok((i32::try_from(start)?, i32::try_from(end)?))
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

impl EncodedPlainData {
    pub fn new(lengths: EncodedStream, data: EncodedStream) -> MltResult<Self> {
        crate::validate_stream!(
            lengths,
            StreamType::Length(LengthType::VarBinary | LengthType::Dictionary)
        );
        crate::validate_stream!(
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

impl EncodedSharedDictEncoding {
    #[must_use]
    pub fn dict_streams(&self) -> Vec<&EncodedStream> {
        match self {
            Self::Plain(plain_data) => plain_data.streams(),
            Self::FsstPlain(fsst_data) => fsst_data.streams(),
        }
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
