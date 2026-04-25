use std::borrow::Cow;

use crate::MltError::{BufferUnderflow, DictIndexOutOfBounds};
use crate::codecs::fsst::decode_fsst;
use crate::decoder::{
    DictionaryType, LengthType, OffsetType, ParsedSharedDict, ParsedSharedDictItem, ParsedStrings,
    RawFsstData, RawPlainData, RawPresence, RawSharedDictEncoding, RawStream, RawStrings,
    RawStringsEncoding, StreamType,
};
use crate::errors::AsMltError as _;
use crate::utils::AsUsize as _;
use crate::{Decoder, DictRange, MltError, MltResult, RawSharedDict, RawSharedDictItem};

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
    pub fn presence_bools(&self) -> Vec<bool> {
        self.lengths.iter().map(|&end| end >= 0).collect()
    }

    #[must_use]
    pub fn get(&self, idx: u32) -> Option<&str> {
        let idx = idx.as_usize();

        let end = *self.lengths.get(idx)?;
        if end < 0 {
            return None;
        }
        let end = decode_end(end).as_usize();

        let start = idx
            .checked_sub(1)
            .and_then(|prev| self.lengths.get(prev).copied())
            .map_or(0, decode_end)
            .as_usize();

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
        Ok((
            str::from_utf8(self.data.data)?,
            self.lengths.decode_u32s(dec)?,
        ))
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
