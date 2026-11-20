use std::borrow::Cow;

use bitvec::order::Lsb0;
use bitvec::slice::BitSlice;
use usize_cast::IntoUsize as _;

use crate::MltError::{BufferUnderflow, DictIndexOutOfBounds};
#[cfg(feature = "unstable-v2")]
use crate::codecs::front_coding::{FrontLengths, front_decode};
use crate::codecs::fsst::{decode_fsst, decode_fsst_bytes};
use crate::decoder::{
    ColumnStorage, DictLayout, DictionaryType, LengthType, OffsetType, ParsedSharedDict,
    ParsedSharedDictItem, ParsedStrings, RawFsstData, RawPlainData, RawPresence,
    RawSharedDictEncoding, RawStream, RawStrings, RawStringsEncoding, StreamType, StringLayout,
};
use crate::errors::AsMltError as _;
use crate::{Decoder, DictRange, MltError, MltResult, RawSharedDict, RawSharedDictItem};

impl<'a> ParsedStrings<'a> {
    /// The code of a null value in [`Self::dictionary`].
    pub const NULL_CODE: u32 = u32::MAX;

    #[must_use]
    pub fn new(name: &'a str, lengths: Vec<i32>, data: Cow<'a, str>) -> Self {
        ParsedStrings {
            name,
            lengths,
            data,
            codes: None,
        }
    }

    #[must_use]
    pub fn feature_count(&self) -> usize {
        self.codes.as_ref().map_or(self.lengths.len(), Vec::len)
    }

    #[must_use]
    pub fn presence_bools(&self) -> Vec<bool> {
        match &self.codes {
            Some(codes) => codes.iter().map(|&code| code != Self::NULL_CODE).collect(),
            None => self.lengths.iter().map(|&end| end >= 0).collect(),
        }
    }

    #[must_use]
    pub fn get(&self, idx: u32) -> Option<&str> {
        match &self.codes {
            Some(codes) => self.entry(*codes.get(idx.into_usize())?),
            None => self.entry(idx),
        }
    }

    /// A dictionary-coded column as its entries and the entry each value names, or
    /// [`Self::NULL_CODE`] for a null; [`None`] for a column that stores each value.
    #[must_use]
    pub fn dictionary(&self) -> Option<(impl ExactSizeIterator<Item = &str>, &[u32])> {
        let codes = self.codes.as_deref()?;
        let entries = (0..self.lengths.len()).map(|i| {
            u32::try_from(i)
                .ok()
                .and_then(|i| self.entry(i))
                .unwrap_or_default()
        });
        Some((entries, codes))
    }

    /// The string `lengths` and `data` hold at `idx`, or [`None`] for a null.
    fn entry(&self, idx: u32) -> Option<&str> {
        let idx = idx.into_usize();

        let end = *self.lengths.get(idx)?;
        if end < 0 {
            return None;
        }
        let end = decode_end(end).into_usize();

        let start = idx
            .checked_sub(1)
            .and_then(|prev| self.lengths.get(prev).copied())
            .map_or(0, decode_end)
            .into_usize();

        self.data.get(start..end)
    }

    #[must_use]
    pub fn dense_values(&self) -> Vec<String> {
        self.values().flatten().map(str::to_string).collect()
    }

    #[must_use]
    pub fn materialize(&self) -> Vec<Option<String>> {
        self.values().map(|v| v.map(str::to_string)).collect()
    }

    /// Each value in turn, [`None`] for a null.
    pub(crate) fn values(&self) -> impl Iterator<Item = Option<&str>> {
        (0..u32::try_from(self.feature_count()).unwrap_or(u32::MAX)).map(|i| self.get(i))
    }
}

impl PartialEq for ParsedStrings<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name && self.values().eq(other.values())
    }
}

impl Eq for ParsedStrings<'_> {}

pub(crate) fn decode_shared_dict_range(range: DictRange) -> Option<(u32, u32)> {
    if let (Ok(start), Ok(end)) = (u32::try_from(range.start), u32::try_from(range.end)) {
        Some((start, end))
    } else {
        None
    }
}

pub(crate) fn shared_dict_spans(lengths: &[u32], dec: &mut Decoder) -> MltResult<Vec<(u32, u32)>> {
    let mut spans = dec.alloc(lengths.len())?;
    let mut offset = 0_u32;
    for &len in lengths {
        let start = offset;
        offset = offset.saturating_add(len);
        spans.push((start, offset));
    }
    Ok(spans)
}

pub(crate) fn resolve_dict_spans(
    offsets: &[u32],
    presence: Option<&BitSlice<u8, Lsb0>>,
    dict_spans: &[(u32, u32)],
    dec: &mut Decoder,
) -> MltResult<Vec<Option<(u32, u32)>>> {
    let present_count = presence.map_or(offsets.len(), BitSlice::len);
    let mut resolved = dec.alloc(present_count)?;
    let mut next = offsets.iter().copied();

    if let Some(presence) = presence {
        let fail = || MltError::PresenceValueCountMismatch(presence.count_ones(), offsets.len());
        for present in presence.iter().by_vals() {
            if !present {
                resolved.push(None);
                continue;
            }
            let idx = next.next().ok_or_else(fail)?;
            #[expect(
                clippy::unnecessary_lazy_evaluations,
                reason = "an eager error is dropped on every success"
            )]
            let span = dict_spans
                .get(idx.into_usize())
                .copied()
                .ok_or_else(|| DictIndexOutOfBounds(idx, dict_spans.len()))?;
            resolved.push(Some(span));
        }

        if next.next().is_some() {
            return Err(fail());
        }
    } else {
        for &idx in offsets {
            #[expect(
                clippy::unnecessary_lazy_evaluations,
                reason = "an eager error is dropped on every success"
            )]
            let span = dict_spans
                .get(idx.into_usize())
                .copied()
                .ok_or_else(|| DictIndexOutOfBounds(idx, dict_spans.len()))?;
            resolved.push(Some(span));
        }
    }

    Ok(resolved)
}

fn dict_span_str(dict_data: &str, span: (u32, u32)) -> MltResult<&str> {
    let start = span.0.into_usize();
    let end = span.1.into_usize();
    // A slice of valid UTF-8 is valid itself wherever both ends fall on character boundaries.
    if let Some(value) = dict_data.get(start..end) {
        return Ok(value);
    }
    let bytes = dict_data.as_bytes();
    let Some(value) = bytes.get(start..end) else {
        let len = span.1.saturating_sub(span.0);
        return Err(BufferUnderflow(len, bytes.len().saturating_sub(start)));
    };
    Ok(str::from_utf8(value)?)
}

impl ParsedSharedDict<'_> {
    /// The child columns reading this dictionary, in wire order.
    #[must_use]
    pub fn items(&self) -> &[ParsedSharedDictItem<'_>] {
        &self.items
    }

    #[must_use]
    pub fn corpus(&self) -> &str {
        &self.data
    }

    #[must_use]
    pub fn get(&self, span: (u32, u32)) -> Option<&str> {
        let start = span.0.into_usize();
        let end = span.1.into_usize();
        self.corpus().get(start..end)
    }
}

impl ParsedSharedDictItem<'_> {
    /// Number of features this child spans, present or not.
    #[must_use]
    pub fn feature_count(&self) -> usize {
        self.ranges.len()
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
        let (data, lengths) = self.decode_bytes(dec)?;
        Ok((str::from_utf8(data)?, lengths))
    }

    /// As [`Self::decode`], for a blob that is only valid UTF-8 once its caller reassembles it.
    pub fn decode_bytes(self, dec: &mut Decoder) -> MltResult<(&'a [u8], Vec<u32>)> {
        Ok((self.data.data, self.lengths.decode_ints(dec)?))
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

    /// As [`Self::decode`], for a corpus that is only valid UTF-8 once its caller reassembles it.
    pub fn decode_bytes(self, dec: &mut Decoder) -> MltResult<(Vec<u8>, Vec<u32>)> {
        decode_fsst_bytes(self, dec)
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
        dict: DictLayout,
    ) -> MltResult<Self> {
        validate_stream!(offsets, StreamType::Offset(OffsetType::String));
        Ok(Self::Dictionary {
            plain_data,
            offsets,
            dict,
        })
    }

    #[must_use]
    pub fn fsst_plain(fsst_data: RawFsstData<'a>) -> Self {
        Self::FsstPlain(fsst_data)
    }

    pub fn fsst_dictionary(
        fsst_data: RawFsstData<'a>,
        offsets: RawStream<'a>,
        dict: DictLayout,
    ) -> MltResult<Self> {
        validate_stream!(offsets, StreamType::Offset(OffsetType::String));
        Ok(Self::FsstDictionary {
            fsst_data,
            offsets,
            dict,
        })
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

impl<'a> RawStrings<'a> {
    #[must_use]
    pub fn new(name: &'a str, presence: RawPresence<'a>, encoding: RawStringsEncoding<'a>) -> Self {
        Self {
            name,
            presence,
            encoding,
        }
    }

    /// How this column stores its strings.
    #[must_use]
    pub(crate) fn storage(&self) -> ColumnStorage {
        let (string, dictionary) = match &self.encoding {
            RawStringsEncoding::Plain(_) => (StringLayout::Plain, None),
            RawStringsEncoding::Dictionary { dict, .. } => (StringLayout::Dict, Some(*dict)),
            RawStringsEncoding::FsstPlain(_) => (StringLayout::Fsst, None),
            RawStringsEncoding::FsstDictionary { dict, .. } => {
                (StringLayout::FsstDict, Some(*dict))
            }
        };
        ColumnStorage {
            string: Some(string),
            dictionary,
        }
    }

    /// How many values this column holds, off whichever of its streams counts them.
    ///
    /// The leading stream is one value per string in every layout: its lengths
    /// where the strings are stored one each, its codes where they are pooled.
    #[cfg(feature = "unstable-v2")]
    #[must_use]
    pub(crate) fn value_count(&self) -> u32 {
        match &self.encoding {
            RawStringsEncoding::Plain(plain) => plain.lengths.meta.num_values,
            RawStringsEncoding::FsstPlain(fsst) => fsst.lengths.meta.num_values,
            RawStringsEncoding::Dictionary { offsets, .. }
            | RawStringsEncoding::FsstDictionary { offsets, .. } => offsets.meta.num_values,
        }
    }

    /// Decode string property from its encoded column.
    pub fn decode(self, dec: &mut Decoder) -> MltResult<ParsedStrings<'a>> {
        let name = self.name;
        let presence = self.presence.decode_bits(dec)?;

        let parsed = match self.encoding {
            RawStringsEncoding::Plain(plain_data) => {
                let (data, lengths) = plain_data.decode(dec)?;
                ParsedStrings {
                    name,
                    lengths: to_absolute_lengths(&lengths, presence.as_deref(), dec)?,
                    data: data.into(),
                    codes: None,
                }
            }
            RawStringsEncoding::Dictionary {
                plain_data,
                offsets,
                dict,
            } => {
                let (data, lengths) = plain_data.decode_bytes(dec)?;
                let offsets: Vec<u32> = offsets.decode_ints(dec)?;
                let (dictionary, lengths) = rebuild_dictionary(dict, &lengths, data, dec)?;
                decode_dictionary_strings(
                    name,
                    &lengths,
                    offsets,
                    presence.as_deref(),
                    dictionary,
                    dec,
                )?
            }
            RawStringsEncoding::FsstPlain(fsst_data) => {
                let (data, dict_lens) = fsst_data.decode(dec)?;
                ParsedStrings {
                    name,
                    lengths: to_absolute_lengths(&dict_lens, presence.as_deref(), dec)?,
                    data: data.into(),
                    codes: None,
                }
            }
            RawStringsEncoding::FsstDictionary {
                fsst_data,
                offsets,
                dict,
            } => {
                let (data, lengths) = fsst_data.decode_bytes(dec)?;
                let offsets: Vec<u32> = offsets.decode_ints(dec)?;
                // A plain dictionary is the decompressed corpus itself, so it is kept, not copied.
                let (dictionary, lengths) = if dict == DictLayout::Plain {
                    (String::from_utf8(data)?, Cow::Borrowed(&*lengths))
                } else {
                    let (dictionary, lengths) = rebuild_dictionary(dict, &lengths, &data, dec)?;
                    (dictionary.into_owned(), lengths)
                };
                decode_dictionary_strings(
                    name,
                    &lengths,
                    offsets,
                    presence.as_deref(),
                    Cow::Owned(dictionary),
                    dec,
                )?
            }
        };
        Ok(parsed)
    }
}

fn to_absolute_lengths(
    lengths: &[u32],
    presence: Option<&BitSlice<u8, Lsb0>>,
    dec: &mut Decoder,
) -> MltResult<Vec<i32>> {
    let capacity = presence.map_or(lengths.len(), BitSlice::len);
    let mut absolute = dec.alloc(capacity)?;
    let mut iter = lengths.iter().copied();
    let mut end = 0_i32;
    if let Some(presence) = presence {
        for present in presence.iter().by_vals() {
            if present {
                let len = iter.next().ok_or_else(|| {
                    MltError::PresenceValueCountMismatch(presence.len(), lengths.len())
                })?;
                end = checked_absolute_end(end, len)?;
                absolute.push(end);
            } else {
                absolute.push(encode_null_end(end));
            }
        }
        if iter.next().is_some() {
            return Err(MltError::PresenceValueCountMismatch(
                presence.count_ones(),
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

/// Turn a dictionary blob into its entries back to back with one length each, whatever its layout.
///
/// A front-coded blob holds only suffixes, which are not valid UTF-8 on their own, so the
/// conversion happens here rather than when the blob is read.
#[cfg_attr(
    not(feature = "unstable-v2"),
    expect(unused_variables, reason = "only front coding allocates")
)]
pub(crate) fn rebuild_dictionary<'d, 'l>(
    dict: DictLayout,
    lengths: &'l [u32],
    data: &'d [u8],
    dec: &mut Decoder,
) -> MltResult<(Cow<'d, str>, Cow<'l, [u32]>)> {
    match dict {
        DictLayout::Plain => Ok((Cow::Borrowed(str::from_utf8(data)?), Cow::Borrowed(lengths))),
        #[cfg(feature = "unstable-v2")]
        DictLayout::FrontCoded => {
            let (entries, entry_lengths) = front_decode(FrontLengths::split(lengths)?, data, dec)?;
            Ok((Cow::Owned(entries), Cow::Owned(entry_lengths)))
        }
    }
}

/// A dictionary-coded column as its dictionary and one code per value, so that a few entries
/// repeated over many values are never copied out.
fn decode_dictionary_strings<'a>(
    name: &'a str,
    dict_lengths: &[u32],
    offsets: Vec<u32>,
    presence: Option<&BitSlice<u8, Lsb0>>,
    dict_data: Cow<'a, str>,
    dec: &mut Decoder,
) -> MltResult<ParsedStrings<'a>> {
    let mut lengths = dec.alloc(dict_lengths.len())?;
    let mut end = 0_i32;
    for &len in dict_lengths {
        let start = end;
        end = checked_absolute_end(end, len)?;
        dict_span_str(&dict_data, (start.unsigned_abs(), end.unsigned_abs()))?;
        lengths.push(end);
    }
    let entries = u32::try_from(dict_lengths.len()).unwrap_or(u32::MAX);
    let entry = |idx: u32| {
        if idx < entries {
            Ok(idx)
        } else {
            Err(DictIndexOutOfBounds(idx, dict_lengths.len()))
        }
    };
    let codes = if let Some(presence) = presence {
        let fail = || MltError::PresenceValueCountMismatch(presence.count_ones(), offsets.len());
        let mut codes = dec.alloc(presence.len())?;
        let mut next = offsets.iter().copied();
        for present in presence.iter().by_vals() {
            codes.push(if present {
                entry(next.next().ok_or_else(fail)?)?
            } else {
                ParsedStrings::NULL_CODE
            });
        }
        if next.next().is_some() {
            return Err(fail());
        }
        codes
    } else {
        for &idx in &offsets {
            entry(idx)?;
        }
        offsets
    };
    Ok(ParsedStrings {
        name,
        lengths,
        data: dict_data,
        codes: Some(codes),
    })
}

pub(crate) fn encode_null_end(end: i32) -> i32 {
    -end - 1
}

fn decode_end(end: i32) -> u32 {
    if end >= 0 {
        u32::try_from(end).expect("non-negative decoded string end must fit in u32")
    } else {
        u32::try_from(-i64::from(end) - 1).expect("encoded null boundary must fit in u32")
    }
}

pub(crate) fn checked_string_end(current_end: i32, byte_len: usize) -> MltResult<i32> {
    let byte_len = u32::try_from(byte_len)?;
    checked_absolute_end(current_end, byte_len)
}

pub(crate) fn checked_absolute_end(current_end: i32, delta: u32) -> MltResult<i32> {
    let delta = i32::try_from(delta)?;
    current_end.checked_add(delta).or_overflow()
}

impl<'a> RawSharedDict<'a> {
    #[must_use]
    pub fn new(
        name: &'a str,
        encoding: RawSharedDictEncoding<'a>,
        dict: DictLayout,
        children: Vec<RawSharedDictItem<'a>>,
    ) -> Self {
        Self {
            name,
            encoding,
            dict,
            children,
        }
    }

    /// Decode a shared-dictionary column into its decoded form.
    pub fn decode(self, dec: &mut Decoder) -> MltResult<ParsedSharedDict<'a>> {
        let prefix = self.name;
        let (data, dict_spans) = match self.encoding {
            RawSharedDictEncoding::Plain(plain_data) => {
                let (blob, lengths) = plain_data.decode_bytes(dec)?;
                let (entries, lengths) = rebuild_dictionary(self.dict, &lengths, blob, dec)?;
                let dict_spans = shared_dict_spans(&lengths, dec)?;
                (entries, dict_spans)
            }
            RawSharedDictEncoding::FsstPlain(fsst_data) => {
                let (blob, lengths) = fsst_data.decode_bytes(dec)?;
                // `entries` borrows the decompressed corpus, which is local, so it is taken owned.
                let (entries, lengths) = rebuild_dictionary(self.dict, &lengths, &blob, dec)?;
                let (entries, lengths) = (entries.into_owned(), lengths.into_owned());
                let dict_spans = shared_dict_spans(&lengths, dec)?;
                (Cow::Owned(entries), dict_spans)
            }
        };
        let mut items = Vec::with_capacity(self.children.len());
        for child in self.children {
            let offsets: Vec<u32> = child.data.decode_ints(dec)?;
            let presence = child.presence.decode_bits(dec)?;
            let ranges = resolve_dict_spans(&offsets, presence.as_deref(), &dict_spans, dec)?
                .into_iter()
                .map(|span| match span {
                    Some(span) => encode_shared_dict_range(span.0, span.1),
                    None => Ok(DictRange::NULL),
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
                let n = u32::try_from(item.ranges.len() * size_of::<DictRange>()).or_overflow()?;
                acc.checked_add(n).or_overflow()
            },
        )?;
        dec.consume(bytes)?;
        Ok(parsed)
    }
}

pub(crate) fn encode_shared_dict_range(start: u32, end: u32) -> MltResult<DictRange> {
    Ok(DictRange {
        start: i32::try_from(start)?,
        end: i32::try_from(end)?,
    })
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::test_helpers::dec;

    #[rstest]
    #[case::empty(&[], "", &[])]
    #[case::all_present(&[1, 3, 6], "abbccc", &[Some("a"), Some("bb"), Some("ccc")])]
    #[case::leading_null(&[-1, 1], "a", &[None, Some("a")])]
    #[case::trailing_null(&[1, -2], "a", &[Some("a"), None])]
    #[case::consecutive_nulls(&[1, -2, -2, 2], "ab", &[Some("a"), None, None, Some("b")])]
    #[case::only_nulls(&[-1, -1], "", &[None, None])]
    #[case::empty_string_after_a_value(&[1, 1, -2], "a", &[Some("a"), Some(""), None])]
    #[case::empty_string_after_a_null(&[1, -2, 1], "a", &[Some("a"), None, Some("")])]
    #[case::multibyte(&[2, 7], "µ€µ", &[Some("µ"), Some("€µ")])]
    fn rows_read_back_from_the_length_stream(
        #[case] lengths: &[i32],
        #[case] data: &str,
        #[case] expected: &[Option<&str>],
    ) {
        let strings = ParsedStrings::new("col", lengths.to_vec(), Cow::Borrowed(data));

        assert_eq!(strings.feature_count(), expected.len());
        assert_eq!(
            strings.materialize(),
            expected
                .iter()
                .map(|v| v.map(str::to_string))
                .collect::<Vec<_>>()
        );
        assert_eq!(
            strings.dense_values(),
            expected
                .iter()
                .flatten()
                .map(|v| (*v).to_string())
                .collect::<Vec<_>>()
        );
        assert_eq!(
            strings.presence_bools(),
            expected.iter().map(Option::is_some).collect::<Vec<_>>()
        );
        for (idx, want) in expected.iter().enumerate() {
            assert_eq!(strings.get(u32::try_from(idx).expect("small")), *want);
        }
        assert_eq!(
            strings.get(u32::try_from(expected.len()).expect("small")),
            None
        );
    }

    #[rstest]
    #[case::end_past_the_corpus(&[9], "ab")]
    #[case::end_before_the_start(&[2, 1], "ab")]
    #[case::split_multibyte(&[1], "µ")]
    fn a_row_outside_the_corpus_reads_as_absent(#[case] lengths: &[i32], #[case] data: &str) {
        let strings = ParsedStrings::new("col", lengths.to_vec(), Cow::Borrowed(data));
        let last = u32::try_from(lengths.len() - 1).expect("small");

        assert_eq!(strings.get(last), None);
        assert!(strings.presence_bools().iter().all(|&p| p));
        assert_eq!(strings.materialize().last(), Some(&None));
    }

    fn dictionary_rows(
        dict_lengths: &[u32],
        dict: &str,
        offsets: &[u32],
    ) -> MltResult<Vec<Option<String>>> {
        decode_dictionary_strings(
            "col",
            dict_lengths,
            offsets.to_vec(),
            None,
            Cow::Borrowed(dict),
            &mut dec(),
        )
        .map(|strings| strings.materialize())
    }

    #[test]
    fn dictionary_rows_keep_their_codes_and_nulls() {
        let presence = bitvec::bitvec![u8, Lsb0; 1, 0, 1];
        let strings = decode_dictionary_strings(
            "col",
            &[1, 2],
            vec![1, 0],
            Some(&presence),
            Cow::Borrowed("abc"),
            &mut dec(),
        )
        .expect("valid dictionary");
        let (entries, codes) = strings.dictionary().expect("coded");
        assert_eq!(entries.collect::<Vec<_>>(), ["a", "bc"]);
        assert_eq!(codes, [1, ParsedStrings::NULL_CODE, 0]);
        assert_eq!(
            strings.materialize(),
            [Some("bc".into()), None, Some("a".into())]
        );
        assert_eq!(
            strings,
            ParsedStrings::new("col", vec![2, -3, 3], Cow::Borrowed("bca"))
        );
    }

    #[test]
    fn dictionary_code_past_the_dictionary() {
        let err = dictionary_rows(&[1], "a", &[0, 1]).expect_err("code past the dictionary");
        assert!(matches!(err, DictIndexOutOfBounds(1, 1)), "{err:?}");
    }

    #[test]
    fn dictionary_rows_repeat_multibyte_entries() {
        let rows = dictionary_rows(&[2, 3], "µ€", &[1, 0, 1]).expect("valid dictionary");
        assert_eq!(
            rows,
            [
                Some("€".to_string()),
                Some("µ".to_string()),
                Some("€".to_string())
            ]
        );
    }

    #[test]
    fn dictionary_entry_past_the_dictionary() {
        let err = dictionary_rows(&[5], "ab", &[0]).expect_err("entry runs past the dictionary");
        assert!(matches!(err, BufferUnderflow(5, 2)), "{err:?}");
    }

    #[test]
    fn dictionary_entry_splitting_a_character() {
        let err = dictionary_rows(&[1, 1], "µ", &[0]).expect_err("entry splits a character");
        assert!(matches!(err, MltError::Utf8(_)), "{err:?}");
    }

    #[test]
    fn dictionary_rows_are_not_copied_out_of_the_dictionary() {
        let consumed = |dict: &str| {
            let mut dec = dec();
            let len = u32::try_from(dict.len()).expect("small");
            decode_dictionary_strings("col", &[len], vec![0; 100], None, dict.into(), &mut dec)
                .expect("valid dictionary");
            dec.consumed()
        };
        assert_eq!(consumed("abc"), consumed("a"));
    }
}
