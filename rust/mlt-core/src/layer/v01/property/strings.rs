use std::borrow::Cow;
use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::io::Write;

use borrowme::{Borrow as BorrowmeBorrow, ToOwned as BorrowmeToOwned};

use crate::MltError::{
    BufferUnderflow, DictIndexOutOfBounds, NotImplemented, UnexpectedStreamType2,
};
use crate::utils::AsUsize as _;
use crate::v01::{
    ColumnType, DecodedProperty, DecodedSharedDict, DecodedSharedDictItem, DecodedStrings,
    DictionaryType, EncodedPresence, EncodedSharedDict, EncodedSharedDictChild, EncodedStrings,
    FsstStrEncoder, IntEncoder, LengthType, OffsetType, OwnedEncodedProperty,
    OwnedEncodedSharedDict, OwnedEncodedSharedDictChild, OwnedEncodedStrings, OwnedName,
    OwnedStream, PresenceStream, Stream, StreamData, StreamType,
};
use crate::{Analyze, MltError, StatType};

/// Encoder for an individual sub-property within a shared dictionary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SharedDictItemEncoder {
    /// If a stream for optional values should be attached.
    pub presence: PresenceStream,
    /// Encoder used for the offset-index stream of this child.
    pub offsets: IntEncoder,
}

/// Encoder for a shared dictionary property with multiple string sub-properties.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SharedDictEncoder {
    /// Encoder for the shared dictionary strings (plain vs FSST).
    pub dict_encoder: StrEncoder,
    /// Encoders for individual sub-properties.
    pub items: Vec<SharedDictItemEncoder>,
}

#[derive(Debug, Eq, PartialEq, Clone, Copy)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
#[cfg_attr(all(not(test), feature = "arbitrary"), derive(arbitrary::Arbitrary))]
pub enum StrEncoder {
    Plain { string_lengths: IntEncoder },
    Fsst(FsstStrEncoder),
}

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

impl From<Vec<Option<String>>> for DecodedStrings<'static> {
    fn from(values: Vec<Option<String>>) -> Self {
        Self::from_optional_strings(values)
    }
}

impl From<Vec<String>> for DecodedStrings<'static> {
    fn from(values: Vec<String>) -> Self {
        Self::from_optional_strings(values.into_iter().map(Some).collect())
    }
}

impl DecodedStrings<'static> {
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
            lengths,
            data: Cow::Owned(data),
        }
    }
}

impl BorrowmeToOwned for DecodedStrings<'_> {
    type Owned = DecodedStrings<'static>;

    fn to_owned(&self) -> Self::Owned {
        DecodedStrings {
            lengths: self.lengths.clone(),
            data: Cow::Owned(self.data.as_ref().to_string()),
        }
    }
}

impl BorrowmeBorrow for DecodedStrings<'static> {
    type Target<'a>
        = DecodedStrings<'a>
    where
        Self: 'a;

    fn borrow(&self) -> Self::Target<'_> {
        DecodedStrings {
            lengths: self.lengths.clone(),
            data: Cow::Borrowed(self.data.as_ref()),
        }
    }
}

#[cfg(all(not(test), feature = "arbitrary"))]
impl<'a> arbitrary::Arbitrary<'a> for DecodedStrings<'static> {
    fn arbitrary(u: &mut arbitrary::Unstructured<'a>) -> arbitrary::Result<Self> {
        Ok(Self::from(u.arbitrary::<Vec<Option<String>>>()?))
    }
}

impl BorrowmeToOwned for DecodedSharedDict<'_> {
    type Owned = DecodedSharedDict<'static>;

    fn to_owned(&self) -> Self::Owned {
        DecodedSharedDict {
            prefix: self.prefix.clone(),
            data: Cow::Owned(self.data.as_ref().to_string()),
            items: self.items.clone(),
        }
    }
}

impl BorrowmeBorrow for DecodedSharedDict<'static> {
    type Target<'a>
        = DecodedSharedDict<'a>
    where
        Self: 'a;

    fn borrow(&self) -> Self::Target<'_> {
        DecodedSharedDict {
            prefix: self.prefix.clone(),
            data: Cow::Borrowed(self.data.as_ref()),
            items: self.items.clone(),
        }
    }
}

#[cfg(all(not(test), feature = "arbitrary"))]
impl<'a> arbitrary::Arbitrary<'a> for DecodedSharedDict<'static> {
    fn arbitrary(u: &mut arbitrary::Unstructured<'a>) -> arbitrary::Result<Self> {
        let prefix: String = u.arbitrary()?;
        let values: Vec<String> = u.arbitrary()?;
        let mut data = String::new();
        for value in values {
            data.push_str(&value);
        }
        Ok(Self {
            prefix,
            data: Cow::Owned(data),
            items: u.arbitrary()?,
        })
    }
}

impl DecodedStrings<'_> {
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

impl Analyze for DecodedStrings<'_> {
    fn collect_statistic(&self, stat: StatType) -> usize {
        self.dense_values().collect_statistic(stat)
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

impl DecodedSharedDict<'_> {
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

pub(crate) fn collect_shared_dict_spans(items: &[DecodedSharedDictItem]) -> Vec<(u32, u32)> {
    let mut spans = items
        .iter()
        .flat_map(DecodedSharedDictItem::dense_spans)
        .collect::<Vec<_>>();
    spans.sort_unstable();
    spans.dedup();
    spans
}

impl DecodedSharedDictItem {
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
    pub fn materialize(&self, shared_dict: &DecodedSharedDict<'_>) -> Vec<Option<String>> {
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
    pub fn get<'a>(&self, shared_dict: &'a DecodedSharedDict<'_>, i: usize) -> Option<&'a str> {
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

impl<'a> EncodedStrings<'a> {
    pub fn plain(lengths: Stream<'a>, data: Stream<'a>) -> Result<Self, MltError> {
        validate_stream!(lengths, StreamType::Length(LengthType::VarBinary));
        validate_stream!(
            data,
            StreamType::Data(DictionaryType::None | DictionaryType::Single)
        );
        Ok(Self::Plain { lengths, data })
    }

    pub fn dictionary(
        lengths: Stream<'a>,
        offsets: Stream<'a>,
        data: Stream<'a>,
    ) -> Result<Self, MltError> {
        validate_stream!(lengths, StreamType::Length(LengthType::Dictionary));
        validate_stream!(offsets, StreamType::Offset(OffsetType::String));
        validate_stream!(data, StreamType::Data(DictionaryType::Single));
        Ok(Self::Dictionary {
            lengths,
            offsets,
            data,
        })
    }

    pub fn fsst_plain(
        symbol_lengths: Stream<'a>,
        symbol_table: Stream<'a>,
        lengths: Stream<'a>,
        corpus: Stream<'a>,
    ) -> Result<Self, MltError> {
        validate_stream!(symbol_lengths, StreamType::Length(LengthType::Symbol));
        validate_stream!(symbol_table, StreamType::Data(DictionaryType::Fsst));
        validate_stream!(lengths, StreamType::Length(LengthType::Dictionary));
        validate_stream!(corpus, StreamType::Data(DictionaryType::Single));
        Ok(Self::FsstPlain {
            symbol_lengths,
            symbol_table,
            lengths,
            corpus,
        })
    }

    pub fn fsst_dictionary(
        symbol_lengths: Stream<'a>,
        symbol_table: Stream<'a>,
        lengths: Stream<'a>,
        corpus: Stream<'a>,
        offsets: Stream<'a>,
    ) -> Result<Self, MltError> {
        validate_stream!(symbol_lengths, StreamType::Length(LengthType::Symbol));
        validate_stream!(symbol_table, StreamType::Data(DictionaryType::Fsst));
        validate_stream!(lengths, StreamType::Length(LengthType::Dictionary));
        validate_stream!(corpus, StreamType::Data(DictionaryType::Single));
        validate_stream!(offsets, StreamType::Offset(OffsetType::String));
        Ok(Self::FsstDictionary {
            symbol_lengths,
            symbol_table,
            lengths,
            corpus,
            offsets,
        })
    }

    /// Content streams in wire order.
    #[must_use]
    pub fn streams(&self) -> Vec<&Stream<'_>> {
        match self {
            Self::Plain { lengths, data } => vec![lengths, data],
            Self::Dictionary {
                lengths,
                offsets,
                data,
            } => vec![lengths, offsets, data],
            Self::FsstPlain {
                symbol_lengths,
                symbol_table,
                lengths,
                corpus,
            } => vec![symbol_lengths, symbol_table, lengths, corpus],
            Self::FsstDictionary {
                symbol_lengths,
                symbol_table,
                lengths,
                corpus,
                offsets,
            } => vec![symbol_lengths, symbol_table, lengths, corpus, offsets],
        }
    }
}

impl OwnedEncodedStrings {
    /// Content streams only.
    #[must_use]
    pub fn content_streams(&self) -> Vec<&OwnedStream> {
        match self {
            Self::Plain { lengths, data } => vec![lengths, data],
            Self::Dictionary {
                lengths,
                offsets,
                data,
            } => vec![lengths, offsets, data],
            Self::FsstPlain {
                symbol_lengths,
                symbol_table,
                lengths,
                corpus,
            } => {
                vec![symbol_lengths, symbol_table, lengths, corpus]
            }
            Self::FsstDictionary {
                symbol_lengths,
                symbol_table,
                lengths,
                corpus,
                offsets,
            } => vec![symbol_lengths, symbol_table, lengths, corpus, offsets],
        }
    }

    /// Streams in wire order.
    #[must_use]
    pub fn streams(&self) -> Vec<&OwnedStream> {
        self.content_streams()
    }
}

impl<'a> EncodedSharedDict<'a> {
    /// Plain shared dict (2 streams): lengths + data.
    pub fn plain(lengths: Stream<'a>, data: Stream<'a>) -> Result<Self, MltError> {
        validate_stream!(lengths, StreamType::Length(LengthType::Dictionary));
        validate_stream!(data, StreamType::Data(DictionaryType::Shared));
        Ok(Self::Plain { lengths, data })
    }

    /// FSST plain shared dict (4 streams): symbol lengths, symbol table, lengths, corpus.
    pub fn fsst_plain(
        symbol_lengths: Stream<'a>,
        symbol_table: Stream<'a>,
        lengths: Stream<'a>,
        corpus: Stream<'a>,
    ) -> Result<Self, MltError> {
        validate_stream!(symbol_lengths, StreamType::Length(LengthType::Symbol));
        validate_stream!(symbol_table, StreamType::Data(DictionaryType::Fsst));
        validate_stream!(lengths, StreamType::Length(LengthType::Dictionary));
        validate_stream!(
            corpus,
            StreamType::Data(DictionaryType::Single | DictionaryType::Shared)
        );
        Ok(Self::FsstPlain {
            symbol_lengths,
            symbol_table,
            lengths,
            corpus,
        })
    }

    /// Decode the shared dictionary entries from this encoding.
    pub fn decode_dictionary(&self) -> Result<Vec<String>, MltError> {
        match self {
            Self::Plain { lengths, data, .. } => {
                let lens = lengths.clone().decode_bits_u32()?.decode_u32()?;
                let data_bytes = raw_bytes(data.clone());
                split_to_strings(&lens, &data_bytes)
            }
            Self::FsstPlain {
                symbol_lengths,
                symbol_table,
                lengths,
                corpus,
                ..
            } => {
                let sym_lens = symbol_lengths.clone().decode_bits_u32()?.decode_u32()?;
                let sym_data = raw_bytes(symbol_table.clone());
                let dict_lens = lengths.clone().decode_bits_u32()?.decode_u32()?;
                let compressed = raw_bytes(corpus.clone());
                let decompressed = decode_fsst(&sym_data, &sym_lens, &compressed);
                split_to_strings(&dict_lens, &decompressed)
            }
        }
    }

    /// Dict streams in wire order (for serialization).
    #[must_use]
    pub fn dict_streams(&self) -> Vec<&Stream<'_>> {
        match self {
            Self::Plain { lengths, data, .. } => vec![lengths, data],
            Self::FsstPlain {
                symbol_lengths,
                symbol_table,
                lengths,
                corpus,
                ..
            } => vec![symbol_lengths, symbol_table, lengths, corpus],
        }
    }
}

impl OwnedEncodedSharedDict {
    #[must_use]
    pub fn dict_streams(&self) -> Vec<&OwnedStream> {
        match self {
            Self::Plain { lengths, data, .. } => vec![lengths, data],
            Self::FsstPlain {
                symbol_lengths,
                symbol_table,
                lengths,
                corpus,
                ..
            } => vec![symbol_lengths, symbol_table, lengths, corpus],
        }
    }
}

pub struct SharedDictionaryGroup<'a> {
    pub shared: StrEncoder,
    pub children: Vec<SharedDictChild<'a>>,
}

pub struct SharedDictChild<'a> {
    pub prop_value: &'a DecodedProperty<'a>,
    pub prop_name: String,
    pub presence: PresenceStream,
    pub offsets: IntEncoder,
}

/// Encode a group of decoded string properties into a single struct column with a shared
/// dictionary. Children are ordered as provided.
pub fn encode_shared_dictionary(
    name: &str,
    group: &SharedDictionaryGroup,
) -> Result<OwnedEncodedProperty, MltError> {
    // Build shared dictionary: unique strings in first-occurrence insertion order.
    let mut dict = Vec::<String>::new();
    let mut dict_index = HashMap::<String, u32>::new();

    for child in &group.children {
        match child.prop_value {
            DecodedProperty::Str(_, values) => {
                for value in values.dense_values() {
                    if let Entry::Vacant(e) = dict_index.entry(value.clone()) {
                        let idx = u32::try_from(dict.len())?;
                        e.insert(idx);
                        dict.push(value);
                    }
                }
            }
            _ => return Err(NotImplemented("generic prop_child encoding")),
        }
    }

    let dict_encoded = match group.shared {
        StrEncoder::Plain { string_lengths } => OwnedStream::encode_strings_with_type(
            &dict,
            string_lengths,
            LengthType::Dictionary,
            DictionaryType::Shared,
        )?,
        StrEncoder::Fsst(enc) => {
            OwnedStream::encode_strings_fsst_plain_with_type(&dict, enc, DictionaryType::Single)?
        }
    };

    // Encode each child column.
    let mut children = Vec::with_capacity(group.children.len());
    for child in &group.children {
        let DecodedProperty::Str(_, values) = child.prop_value else {
            return Err(NotImplemented("generic struct child encoding"));
        };

        // Presence stream
        let presence = if child.presence == PresenceStream::Present {
            let present_bools = values.presence_bools();
            Some(OwnedStream::encode_presence(&present_bools)?)
        } else {
            None
        };

        // Offset indices for non-null values only.
        let offsets: Vec<u32> = values
            .dense_values()
            .iter()
            .map(|s| dict_index[s.as_str()])
            .collect();

        let data = OwnedStream::encode_u32s_of_type(
            &offsets,
            child.offsets,
            StreamType::Offset(OffsetType::String),
        )?;

        children.push(OwnedEncodedSharedDictChild {
            name: OwnedName(child.prop_name.clone()),
            presence: crate::v01::OwnedEncodedPresence(presence),
            data,
        });
    }

    let struct_prop = match dict_encoded {
        OwnedEncodedStrings::Plain { lengths, data } => {
            OwnedEncodedSharedDict::Plain { lengths, data }
        }
        OwnedEncodedStrings::FsstPlain {
            symbol_lengths,
            symbol_table,
            lengths,
            corpus,
        } => OwnedEncodedSharedDict::FsstPlain {
            symbol_lengths,
            symbol_table,
            lengths,
            corpus,
        },
        OwnedEncodedStrings::Dictionary { .. } | OwnedEncodedStrings::FsstDictionary { .. } => {
            return Err(NotImplemented(
                "SharedDict only supports Plain or FsstPlain encoding",
            ));
        }
    };

    Ok(OwnedEncodedProperty::SharedDict(
        OwnedName(name.to_string()),
        struct_prop,
        children,
    ))
}

/// Encode a shared dictionary property directly from `PropValue::SharedDict` and `SharedDictEncoder`.
pub fn encode_shared_dict_prop(
    shared_dict: &DecodedSharedDict<'_>,
    encoder: &SharedDictEncoder,
) -> Result<OwnedEncodedProperty, MltError> {
    if shared_dict.items.len() != encoder.items.len() {
        return Err(NotImplemented(
            "SharedDict items count must match encoder items count",
        ));
    }

    let dict_spans = collect_shared_dict_spans(&shared_dict.items);
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
        StrEncoder::Plain { string_lengths } => OwnedStream::encode_strings_with_type(
            &dict,
            string_lengths,
            LengthType::Dictionary,
            DictionaryType::Shared,
        )?,
        StrEncoder::Fsst(enc) => {
            OwnedStream::encode_strings_fsst_plain_with_type(&dict, enc, DictionaryType::Single)?
        }
    };

    // Encode each child column.
    let mut children = Vec::with_capacity(shared_dict.items.len());
    for (item, item_enc) in shared_dict.items.iter().zip(&encoder.items) {
        // Presence stream
        let presence = if item_enc.presence == PresenceStream::Present {
            let present_bools = item.presence_bools();
            Some(OwnedStream::encode_presence(&present_bools)?)
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

        let data = OwnedStream::encode_u32s_of_type(
            &offsets,
            item_enc.offsets,
            StreamType::Offset(OffsetType::String),
        )?;

        children.push(OwnedEncodedSharedDictChild {
            name: OwnedName(item.suffix.clone()),
            presence: crate::v01::OwnedEncodedPresence(presence),
            data,
        });
    }

    let struct_prop = match dict_encoded {
        OwnedEncodedStrings::Plain { lengths, data } => {
            OwnedEncodedSharedDict::Plain { lengths, data }
        }
        OwnedEncodedStrings::FsstPlain {
            symbol_lengths,
            symbol_table,
            lengths,
            corpus,
        } => OwnedEncodedSharedDict::FsstPlain {
            symbol_lengths,
            symbol_table,
            lengths,
            corpus,
        },
        OwnedEncodedStrings::Dictionary { .. } | OwnedEncodedStrings::FsstDictionary { .. } => {
            return Err(NotImplemented(
                "SharedDict only supports Plain or FsstPlain encoding",
            ));
        }
    };

    Ok(OwnedEncodedProperty::SharedDict(
        OwnedName(shared_dict.prefix.clone()),
        struct_prop,
        children,
    ))
}

pub fn build_decoded_shared_dict(
    prefix: impl Into<String>,
    items: impl IntoIterator<Item = (String, DecodedStrings<'static>)>,
) -> Result<DecodedSharedDict<'static>, MltError> {
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
            |(suffix, values)| -> Result<DecodedSharedDictItem, MltError> {
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
                Ok(DecodedSharedDictItem { suffix, ranges })
            },
        )
        .collect::<Result<Vec<_>, _>>()?;

    Ok(DecodedSharedDict {
        prefix,
        data: data.into(),
        items,
    })
}

impl OwnedEncodedSharedDictChild {
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

/// Decode string property, returning the presence stream and decoded string values.
pub fn decode_strings_with_presence<'a>(
    presence: EncodedPresence<'a>,
    encoding: EncodedStrings<'a>,
) -> Result<DecodedStrings<'static>, MltError> {
    decode_strings(presence, encoding)
}

/// Decode string property from its encoded stream encoding.
pub fn decode_strings(
    presence: EncodedPresence<'_>,
    encoding: EncodedStrings<'_>,
) -> Result<DecodedStrings<'static>, MltError> {
    let presence = presence.0.map(Stream::decode_bools).transpose()?;
    Ok(match encoding {
        EncodedStrings::Plain { lengths, data } => DecodedStrings {
            lengths: to_absolute_lengths(
                &lengths.decode_bits_u32()?.decode_u32()?,
                presence.as_deref(),
            )?,
            data: str::from_utf8(&raw_bytes(data))?.to_string().into(),
        },
        EncodedStrings::Dictionary {
            lengths,
            offsets,
            data,
        } => decode_dictionary_strings(
            &lengths.decode_bits_u32()?.decode_u32()?,
            &offsets.decode_bits_u32()?.decode_u32()?,
            presence.as_deref(),
            str::from_utf8(&raw_bytes(data))?,
        )?,
        EncodedStrings::FsstPlain {
            symbol_lengths,
            symbol_table,
            lengths,
            corpus,
        } => {
            let sym_lens = symbol_lengths.decode_bits_u32()?.decode_u32()?;
            let sym_data = raw_bytes(symbol_table);
            let value_lens = lengths.decode_bits_u32()?.decode_u32()?;
            let compressed = raw_bytes(corpus);
            let decompressed = decode_fsst(&sym_data, &sym_lens, &compressed);
            DecodedStrings {
                lengths: to_absolute_lengths(&value_lens, presence.as_deref())?,
                data: str::from_utf8(&decompressed)?.to_string().into(),
            }
        }
        EncodedStrings::FsstDictionary {
            symbol_lengths,
            symbol_table,
            lengths,
            corpus,
            offsets,
        } => {
            let sym_lens = symbol_lengths.decode_bits_u32()?.decode_u32()?;
            let sym_data = raw_bytes(symbol_table);
            let dict_lens = lengths.decode_bits_u32()?.decode_u32()?;
            let compressed = raw_bytes(corpus);
            let decompressed = decode_fsst(&sym_data, &sym_lens, &compressed);
            decode_dictionary_strings(
                &dict_lens,
                &offsets.decode_bits_u32()?.decode_u32()?,
                presence.as_deref(),
                str::from_utf8(&decompressed)?,
            )?
        }
    })
}

fn raw_bytes(s: Stream<'_>) -> Vec<u8> {
    match s.data {
        StreamData::Encoded(d) => d.data.to_vec(),
        StreamData::VarInt(d) => d.data.to_vec(),
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

fn decode_dictionary_strings(
    dict_lengths: &[u32],
    offsets: &[u32],
    presence: Option<&[bool]>,
    dict_data: &str,
) -> Result<DecodedStrings<'static>, MltError> {
    let dictionary = split_to_strings(dict_lengths, dict_data.as_bytes())?;
    let mut lengths = Vec::with_capacity(presence.map_or(offsets.len(), <[bool]>::len));
    let mut data = String::new();
    let mut next_offset = offsets.iter().copied();
    let mut end = 0_i32;
    if let Some(presence) = presence {
        for &present in presence {
            if !present {
                lengths.push(encode_null_end(end));
                continue;
            }
            let offset = next_offset
                .next()
                .ok_or(MltError::PresenceValueCountMismatch(
                    presence.len(),
                    offsets.len(),
                ))?;
            let value = dictionary
                .get(offset.as_usize())
                .ok_or(DictIndexOutOfBounds(offset, dictionary.len()))?;
            data.push_str(value);
            end = checked_string_end(end, value.len())?;
            lengths.push(end);
        }
        if next_offset.next().is_some() {
            return Err(MltError::PresenceValueCountMismatch(
                presence.iter().filter(|v| **v).count(),
                offsets.len(),
            ));
        }
    } else {
        for &offset in offsets {
            let value = dictionary
                .get(offset.as_usize())
                .ok_or(DictIndexOutOfBounds(offset, dictionary.len()))?;
            data.push_str(value);
            end = checked_string_end(end, value.len())?;
            lengths.push(end);
        }
    }
    Ok(DecodedStrings {
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

/// Split `data` into UTF-8 strings using `lengths` as byte lengths for each entry.
fn split_to_strings(lengths: &[u32], data: &[u8]) -> Result<Vec<String>, MltError> {
    let mut strings = Vec::with_capacity(lengths.len());
    let mut offset = 0_usize;
    for &len in lengths {
        let len_usize = len.as_usize();
        let Some(v) = data.get(offset..offset + len_usize) else {
            return Err(BufferUnderflow(len, data.len().saturating_sub(offset)));
        };
        strings.push(str::from_utf8(v)?.to_string());
        offset += len_usize;
    }
    Ok(strings)
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

/// Decode a struct with shared dictionary into a single decoded property with all children.
pub fn decode_shared_dict(
    prefix: impl Into<String>,
    struct_data: &EncodedSharedDict<'_>,
    children: &[EncodedSharedDictChild<'_>],
) -> Result<DecodedSharedDict<'static>, MltError> {
    let prefix = prefix.into();
    let (shared_dict, dict_spans) = match struct_data {
        EncodedSharedDict::Plain { lengths, data } => {
            let lengths = lengths.clone().decode_bits_u32()?.decode_u32()?;
            let dict_spans = lengths
                .iter()
                .scan(0_u32, |offset, len| {
                    let start = *offset;
                    *offset = offset.saturating_add(*len);
                    Some((start, *offset))
                })
                .collect::<Vec<_>>();
            (
                DecodedSharedDict {
                    prefix: String::new(),
                    data: str::from_utf8(&raw_bytes(data.clone()))?.to_string().into(),
                    items: Vec::new(),
                },
                dict_spans,
            )
        }
        EncodedSharedDict::FsstPlain {
            symbol_lengths,
            symbol_table,
            lengths,
            corpus,
        } => {
            let symbol_lengths = symbol_lengths.clone().decode_bits_u32()?.decode_u32()?;
            let symbol_table = raw_bytes(symbol_table.clone());
            let lengths = lengths.clone().decode_bits_u32()?.decode_u32()?;
            let compressed = raw_bytes(corpus.clone());
            let decoded = decode_fsst(&symbol_table, &symbol_lengths, &compressed);
            let dict_spans = lengths
                .iter()
                .scan(0_u32, |offset, len| {
                    let start = *offset;
                    *offset = offset.saturating_add(*len);
                    Some((start, *offset))
                })
                .collect::<Vec<_>>();
            (
                DecodedSharedDict {
                    prefix: String::new(),
                    data: str::from_utf8(&decoded)?.to_string().into(),
                    items: Vec::new(),
                },
                dict_spans,
            )
        }
    };
    let items = children
        .iter()
        .map(|child| -> Result<DecodedSharedDictItem, MltError> {
            let offsets = child.data.clone().decode_bits_u32()?.decode_u32()?;
            let presence = child
                .presence
                .0
                .clone()
                .map(Stream::decode_bools)
                .transpose()?;
            let mut next = offsets.into_iter();
            let ranges = if let Some(presence) = presence {
                let mut ranges = Vec::with_capacity(presence.len());
                for present in presence {
                    if present {
                        let idx = next.next().ok_or(MltError::PresenceValueCountMismatch(
                            ranges.iter().filter(|&&(s, e)| (s, e) != (-1, -1)).count(),
                            ranges.len() + 1,
                        ))?;
                        let span = dict_spans
                            .get(idx as usize)
                            .copied()
                            .ok_or(DictIndexOutOfBounds(idx, dict_spans.len()))?;
                        ranges.push(encode_shared_dict_range(span.0, span.1)?);
                    } else {
                        ranges.push((-1, -1));
                    }
                }
                if next.next().is_some() {
                    return Err(MltError::PresenceValueCountMismatch(
                        ranges.iter().filter(|&&(s, e)| (s, e) != (-1, -1)).count() + 1,
                        ranges.len(),
                    ));
                }
                ranges
            } else {
                next.map(|idx| {
                    let span = dict_spans
                        .get(idx as usize)
                        .copied()
                        .ok_or(DictIndexOutOfBounds(idx, dict_spans.len()))?;
                    encode_shared_dict_range(span.0, span.1)
                })
                .collect::<Result<Vec<_>, _>>()?
            };
            Ok(DecodedSharedDictItem {
                suffix: child.name.0.to_string(),
                ranges,
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(DecodedSharedDict {
        prefix,
        data: shared_dict.data,
        items,
    })
}
