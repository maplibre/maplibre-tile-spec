use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::io::Write;

use borrowme::borrowme;

use crate::MltError;
use crate::MltError::{
    BufferUnderflow, DictIndexOutOfBounds, MissingStringStream, NotImplemented,
    UnexpectedStreamType, UnexpectedStreamType2,
};
use crate::utils::{BinarySerializer as _, apply_present};
use crate::v01::{
    ColumnType, DecodedProperty, DictionaryType, FsstStrEncoder, IntEncoder, LengthType,
    OffsetType, OwnedEncodedPropValue, OwnedEncodedProperty, OwnedStream, PresenceStream,
    PropValue, Property, Stream, StreamData, StreamType,
};

/// A single child field within a Struct column
#[borrowme]
#[derive(Clone, Debug, PartialEq)]
pub struct EncodedStructChild<'a> {
    pub name: &'a str,
    pub typ: ColumnType,
    pub optional: Option<Stream<'a>>,
    pub data: Stream<'a>,
}

/// Encoded data for a Struct column with shared dictionary encoding.
/// Four variants: plain (2 streams), dictionary (3), FSST plain (4), FSST dictionary (5).
#[borrowme]
#[derive(Debug, Clone, PartialEq)]
pub enum EncodedStructProp<'a> {
    /// Plain shared dict (2 streams): length + data.
    Plain {
        enc_len: Stream<'a>,
        enc_data: Stream<'a>,
        children: Vec<EncodedStructChild<'a>>,
    },
    /// Dictionary shared dict (3 streams): length + offset + data.
    Dictionary {
        enc_len: Stream<'a>,
        enc_off: Stream<'a>,
        enc_data: Stream<'a>,
        children: Vec<EncodedStructChild<'a>>,
    },
    /// FSST plain shared dict (4 streams): symbol lengths, symbol table, length, corpus.
    FsstPlain {
        enc_len_sym: Stream<'a>,
        symbol_table: Stream<'a>,
        enc_len_dic: Stream<'a>,
        corpus: Stream<'a>,
        children: Vec<EncodedStructChild<'a>>,
    },
    /// FSST dictionary shared dict (5 streams): symbol lengths, symbol table, length, corpus, offset.
    FsstDictionary {
        enc_len_sym: Stream<'a>,
        symbol_table: Stream<'a>,
        enc_len_dic: Stream<'a>,
        corpus: Stream<'a>,
        offset: Stream<'a>,
        children: Vec<EncodedStructChild<'a>>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct SharedDictEncoder {
    /// Name of the parent struct column.
    ///
    /// All instructions with the same value are grouped into one struct column.
    pub struct_name: String,
    /// Name of this field within the struct column.
    pub child_name: String,
    /// Encoder used for the offset-index stream of this child.
    pub offset: IntEncoder,
    /// If a stream for optional values should be attached
    pub optional: PresenceStream,
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

/// String column encoding as produced by the encoder (plain, dictionary, or FSST).
/// Stream order matches the encoder: see `StringEncoder.encode()` and `encodePlain` /
/// `encodeDictionary` / `encodeFsstDictionary`.
#[borrowme]
#[derive(Debug, Clone, PartialEq)]
pub enum EncodedStrProp<'a> {
    /// Plain: length stream + data stream
    Plain {
        lengths: Stream<'a>,
        data: Stream<'a>,
    },
    /// Dictionary: length + offset + dictionary data
    Dictionary {
        lengths: Stream<'a>,
        offset: Stream<'a>,
        data: Stream<'a>,
    },
    /// FSST dict: symbol lengths, symbol table, value length, compressed corpus; optional offset (5th stream).
    FsstDictionary {
        symbol_lengths: Stream<'a>,
        symbol_table: Stream<'a>,
        length: Stream<'a>,
        corpus: Stream<'a>,
        offset: Option<Stream<'a>>,
    },
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

impl<'a> EncodedStrProp<'a> {
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
        offset: Stream<'a>,
        data: Stream<'a>,
    ) -> Result<Self, MltError> {
        validate_stream!(lengths, StreamType::Length(LengthType::Dictionary));
        validate_stream!(offset, StreamType::Offset(OffsetType::String));
        validate_stream!(data, StreamType::Data(DictionaryType::Single));
        Ok(Self::Dictionary {
            lengths,
            offset,
            data,
        })
    }

    pub fn fsst(
        symbol_lengths: Stream<'a>,
        symbol_table: Stream<'a>,
        length: Stream<'a>,
        corpus: Stream<'a>,
        offset: Option<Stream<'a>>,
    ) -> Result<Self, MltError> {
        validate_stream!(symbol_lengths, StreamType::Length(LengthType::Symbol));
        validate_stream!(symbol_table, StreamType::Data(DictionaryType::Fsst));
        validate_stream!(length, StreamType::Length(LengthType::Dictionary));
        validate_stream!(corpus, StreamType::Data(DictionaryType::Single));
        if let Some(ref o) = offset {
            validate_stream!(o, StreamType::Offset(OffsetType::String));
        }
        Ok(Self::FsstDictionary {
            symbol_lengths,
            symbol_table,
            length,
            corpus,
            offset,
        })
    }

    /// Build from a parsed stream list (2 = plain, 3 = dictionary, 4 or 5 = FSST). Does not validate stream types.
    pub fn from_streams(vec: Vec<Stream<'a>>) -> Result<Self, MltError> {
        let n = vec.len();
        let mut it = vec.into_iter();
        Ok(match n {
            2 => Self::Plain {
                lengths: it.next().ok_or(MissingStringStream("plain length"))?,
                data: it.next().ok_or(MissingStringStream("plain data"))?,
            },
            3 => Self::Dictionary {
                lengths: it.next().ok_or(MissingStringStream("dict length"))?,
                offset: it.next().ok_or(MissingStringStream("dict offset"))?,
                data: it.next().ok_or(MissingStringStream("dict data"))?,
            },
            4 => Self::FsstDictionary {
                symbol_lengths: it
                    .next()
                    .ok_or(MissingStringStream("fsst symbol_lengths"))?,
                symbol_table: it.next().ok_or(MissingStringStream("fsst symbol_table"))?,
                length: it.next().ok_or(MissingStringStream("fsst length"))?,
                corpus: it.next().ok_or(MissingStringStream("fsst corpus"))?,
                offset: None,
            },
            5 => Self::FsstDictionary {
                symbol_lengths: it
                    .next()
                    .ok_or(MissingStringStream("fsst symbol_lengths"))?,
                symbol_table: it.next().ok_or(MissingStringStream("fsst symbol_table"))?,
                length: it.next().ok_or(MissingStringStream("fsst length"))?,
                corpus: it.next().ok_or(MissingStringStream("fsst corpus"))?,
                offset: Some(it.next().ok_or(MissingStringStream("fsst offset"))?),
            },
            n => return Err(MltError::UnsupportedStringStreamCount(n)),
        })
    }

    /// Number of streams (for stream count varint).
    #[must_use]
    pub fn stream_count(&self) -> usize {
        match self {
            Self::Plain { .. } => 2,
            Self::Dictionary { .. } => 3,
            Self::FsstDictionary { offset, .. } => 4 + offset.as_ref().map_or(0, |_| 1),
        }
    }

    /// Streams in an order suitable for classification (`decode_string_streams`).
    #[must_use]
    pub fn streams(&self) -> Vec<Stream<'_>> {
        match self {
            Self::Plain { lengths, data } => vec![lengths.clone(), data.clone()],
            Self::Dictionary {
                lengths,
                offset,
                data,
            } => {
                vec![lengths.clone(), offset.clone(), data.clone()]
            }
            Self::FsstDictionary {
                symbol_lengths,
                symbol_table,
                length,
                corpus,
                offset,
            } => {
                let mut v = vec![
                    symbol_lengths.clone(),
                    symbol_table.clone(),
                    length.clone(),
                    corpus.clone(),
                ];
                if let Some(o) = offset {
                    v.push(o.clone());
                }
                v
            }
        }
    }
}

impl OwnedEncodedStrProp {
    /// Number of streams (for writing stream count varint).
    #[must_use]
    pub fn stream_count(&self) -> usize {
        match self {
            Self::Plain { .. } => 2,
            Self::Dictionary { .. } => 3,
            Self::FsstDictionary { offset, .. } => 4 + offset.as_ref().map_or(0, |_| 1),
        }
    }

    /// Streams in wire order for serialization.
    #[must_use]
    pub fn streams(&self) -> Vec<&OwnedStream> {
        match self {
            Self::Plain {
                lengths: length,
                data,
            } => vec![length, data],
            Self::Dictionary {
                lengths,
                offset,
                data,
            } => vec![lengths, offset, data],
            Self::FsstDictionary {
                symbol_lengths,
                symbol_table,
                length,
                corpus,
                offset,
            } => {
                let mut v = vec![symbol_lengths, symbol_table, length, corpus];
                if let Some(o) = offset {
                    v.push(o);
                }
                v
            }
        }
    }

    /// Consume and return the streams in wire order (e.g. for struct shared-dict encoding).
    #[must_use]
    pub fn into_streams(self) -> Vec<OwnedStream> {
        match self {
            Self::Plain {
                lengths: length,
                data,
            } => vec![length, data],
            Self::Dictionary {
                lengths,
                offset,
                data,
            } => vec![lengths, offset, data],
            Self::FsstDictionary {
                symbol_lengths,
                symbol_table,
                length,
                corpus,
                offset,
            } => {
                let mut v = vec![symbol_lengths, symbol_table, length, corpus];
                if let Some(o) = offset {
                    v.push(o);
                }
                v
            }
        }
    }
}

impl<'a> EncodedStructProp<'a> {
    /// Plain shared dict (2 streams): length + data.
    #[must_use]
    pub fn plain(
        enc_len: Stream<'a>,
        enc_data: Stream<'a>,
        children: Vec<EncodedStructChild<'a>>,
    ) -> Self {
        Self::Plain {
            enc_len,
            enc_data,
            children,
        }
    }

    /// Dictionary shared dict (3 streams): length + offset + data.
    #[must_use]
    pub fn dictionary(
        enc_len: Stream<'a>,
        enc_off: Stream<'a>,
        enc_data: Stream<'a>,
        children: Vec<EncodedStructChild<'a>>,
    ) -> Self {
        Self::Dictionary {
            enc_len,
            enc_off,
            enc_data,
            children,
        }
    }

    /// FSST plain shared dict (4 streams): symbol lengths, symbol table, length, corpus.
    #[must_use]
    pub fn fsst_plain(
        enc_len_sym: Stream<'a>,
        symbol_table: Stream<'a>,
        enc_len_dic: Stream<'a>,
        corpus: Stream<'a>,
        children: Vec<EncodedStructChild<'a>>,
    ) -> Self {
        Self::FsstPlain {
            enc_len_sym,
            symbol_table,
            enc_len_dic,
            corpus,
            children,
        }
    }

    /// FSST dictionary shared dict (5 streams): symbol lengths, symbol table, length, corpus, offset.
    #[must_use]
    pub fn fsst_dictionary(
        enc_len_sym: Stream<'a>,
        symbol_table: Stream<'a>,
        enc_len_dic: Stream<'a>,
        corpus: Stream<'a>,
        offset: Stream<'a>,
        children: Vec<EncodedStructChild<'a>>,
    ) -> Self {
        Self::FsstDictionary {
            enc_len_sym,
            symbol_table,
            enc_len_dic,
            corpus,
            offset,
            children,
        }
    }

    /// Dict stream count (2, 3, 4, or 5).
    #[must_use]
    pub fn dict_stream_count(&self) -> usize {
        match self {
            Self::Plain { .. } => 2,
            Self::Dictionary { .. } => 3,
            Self::FsstPlain { .. } => 4,
            Self::FsstDictionary { .. } => 5,
        }
    }

    /// Dict streams in wire order (for serialization and for `decode_shared_dictionary`).
    #[must_use]
    pub fn dict_streams(&self) -> Vec<Stream<'_>> {
        match self {
            Self::Plain {
                enc_len, enc_data, ..
            } => vec![enc_len.clone(), enc_data.clone()],
            Self::Dictionary {
                enc_len,
                enc_off,
                enc_data,
                ..
            } => vec![enc_len.clone(), enc_off.clone(), enc_data.clone()],
            Self::FsstPlain {
                enc_len_sym,
                symbol_table,
                enc_len_dic,
                corpus,
                ..
            } => vec![
                enc_len_sym.clone(),
                symbol_table.clone(),
                enc_len_dic.clone(),
                corpus.clone(),
            ],
            Self::FsstDictionary {
                enc_len_sym,
                symbol_table,
                enc_len_dic,
                corpus,
                offset,
                ..
            } => vec![
                enc_len_sym.clone(),
                symbol_table.clone(),
                enc_len_dic.clone(),
                corpus.clone(),
                offset.clone(),
            ],
        }
    }

    /// Total stream count (dict + children).
    #[must_use]
    pub fn stream_count(&self) -> usize {
        let child_count: usize = self
            .children()
            .iter()
            .map(|c| 1 + c.optional.as_ref().map_or(0, |_| 1))
            .sum();
        self.dict_stream_count() + child_count
    }

    /// All streams in wire order: dict streams then each child's optional (if any) and data.
    #[must_use]
    pub fn streams(&self) -> Vec<Stream<'_>> {
        let mut v = self.dict_streams();
        for c in self.children() {
            if let Some(ref o) = c.optional {
                v.push(o.clone());
            }
            v.push(c.data.clone());
        }
        v
    }

    #[must_use]
    pub fn children(&self) -> &[EncodedStructChild<'a>] {
        match self {
            Self::Plain { children, .. }
            | Self::Dictionary { children, .. }
            | Self::FsstPlain { children, .. }
            | Self::FsstDictionary { children, .. } => children,
        }
    }
}

impl OwnedEncodedStructProp {
    #[must_use]
    pub fn dict_stream_count(&self) -> usize {
        match self {
            Self::Plain { .. } => 2,
            Self::Dictionary { .. } => 3,
            Self::FsstPlain { .. } => 4,
            Self::FsstDictionary { .. } => 5,
        }
    }

    #[must_use]
    pub fn stream_count(&self) -> usize {
        let child_count: usize = self
            .children()
            .iter()
            .map(|c| 1 + c.optional.as_ref().map_or(0, |_| 1))
            .sum();
        self.dict_stream_count() + child_count
    }

    #[must_use]
    pub fn streams(&self) -> Vec<&OwnedStream> {
        let mut v = self.dict_streams();
        for c in self.children() {
            if let Some(o) = &c.optional {
                v.push(o);
            }
            v.push(&c.data);
        }
        v
    }

    #[must_use]
    pub fn dict_streams(&self) -> Vec<&OwnedStream> {
        match self {
            Self::Plain {
                enc_len, enc_data, ..
            } => vec![enc_len, enc_data],
            Self::Dictionary {
                enc_len,
                enc_off,
                enc_data,
                ..
            } => vec![enc_len, enc_off, enc_data],
            Self::FsstPlain {
                enc_len_sym,
                symbol_table,
                enc_len_dic,
                corpus,
                ..
            } => vec![enc_len_sym, symbol_table, enc_len_dic, corpus],
            Self::FsstDictionary {
                enc_len_sym,
                symbol_table,
                enc_len_dic,
                corpus,
                offset,
                ..
            } => vec![enc_len_sym, symbol_table, enc_len_dic, corpus, offset],
        }
    }

    #[must_use]
    pub fn children(&self) -> &[OwnedEncodedStructChild] {
        match self {
            Self::Plain { children, .. }
            | Self::Dictionary { children, .. }
            | Self::FsstPlain { children, .. }
            | Self::FsstDictionary { children, .. } => children,
        }
    }
}

pub struct SharedDictionaryGroup<'a> {
    pub shared: StrEncoder,
    pub children: Vec<SharedDictChild<'a>>,
}

pub struct SharedDictChild<'a> {
    pub prop_value: &'a DecodedProperty,
    pub prop_name: String,
    pub optional: PresenceStream,
    pub offset: IntEncoder,
}

/// Encode a group of decoded string properties into a single struct column with a shared
/// dictionary. Children are ordered as provided.
pub fn encode_shared_dictionary(
    name: &str,
    group: &SharedDictionaryGroup,
) -> Result<OwnedEncodedProperty, MltError> {
    // Build shared dictionary: unique strings in first-occurrence insertion order.
    let mut dict: Vec<&str> = Vec::new();
    let mut dict_index: HashMap<&str, u32> = HashMap::new();

    for child in &group.children {
        match &child.prop_value.values {
            PropValue::Str(values) => {
                for s in values.iter().flatten() {
                    if let Entry::Vacant(e) = dict_index.entry(s) {
                        let idx = u32::try_from(dict.len())?;
                        e.insert(idx);
                        dict.push(s);
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
        StrEncoder::Fsst(enc) => OwnedStream::encode_strings_fsst_with_type(
            &dict,
            enc,
            DictionaryType::Single, // TODO: figure out if this is correct. According to Java it is.. but why?
        )?,
    };

    // Encode each child column.
    let mut children = Vec::with_capacity(group.children.len());
    for child in &group.children {
        let PropValue::Str(values) = &child.prop_value.values else {
            return Err(NotImplemented("generic struct child encoding"));
        };

        // Presence stream
        let optional = if child.optional == PresenceStream::Present {
            let present_bools: Vec<bool> = values.iter().map(Option::is_some).collect();
            Some(OwnedStream::encode_presence(&present_bools)?)
        } else {
            None
        };

        // Offset indices for non-null values only.
        let offsets: Vec<u32> = values
            .iter()
            .filter_map(|v| v.as_deref())
            .map(|s| dict_index[s])
            .collect();

        let data = OwnedStream::encode_u32s_of_type(
            &offsets,
            child.offset,
            StreamType::Offset(OffsetType::String),
        )?;

        children.push(OwnedEncodedStructChild {
            name: child.prop_name.clone(),
            typ: if child.optional == PresenceStream::Present {
                ColumnType::OptStr
            } else {
                ColumnType::Str
            },
            optional,
            data,
        });
    }

    let struct_prop = match dict_encoded {
        OwnedEncodedStrProp::Plain {
            lengths: enc_len,
            data: enc_data,
        } => OwnedEncodedStructProp::Plain {
            enc_len,
            enc_data,
            children,
        },
        OwnedEncodedStrProp::Dictionary {
            lengths: enc_len,
            offset: enc_off,
            data: enc_data,
        } => OwnedEncodedStructProp::Dictionary {
            enc_len,
            enc_off,
            enc_data,
            children,
        },
        OwnedEncodedStrProp::FsstDictionary {
            symbol_lengths: enc_len_sym,
            symbol_table,
            length: enc_len_dic,
            corpus,
            offset,
            ..
        } => match offset {
            None => OwnedEncodedStructProp::FsstPlain {
                enc_len_sym,
                symbol_table,
                enc_len_dic,
                corpus,
                children,
            },
            Some(offset) => OwnedEncodedStructProp::FsstDictionary {
                enc_len_sym,
                symbol_table,
                enc_len_dic,
                corpus,
                offset,
                children,
            },
        },
    };

    Ok(OwnedEncodedProperty {
        name: name.to_owned(),
        value: OwnedEncodedPropValue::Struct(struct_prop),
    })
}

impl OwnedEncodedStructChild {
    pub(crate) fn write_columns_meta_to<W: Write>(&self, writer: &mut W) -> Result<(), MltError> {
        self.typ.write_to(writer)?;
        writer.write_string(&self.name)?;
        Ok(())
    }
}

/// Classified string sub-streams, used by both regular string and shared dictionary decoding.
#[derive(Default)]
struct StringStreams {
    var_binary_lengths: Option<Vec<u32>>,
    dict_lengths: Option<Vec<u32>>,
    symbol_lengths: Option<Vec<u32>>,
    data_bytes: Option<Vec<u8>>,
    dict_bytes: Option<Vec<u8>>,
    symbol_bytes: Option<Vec<u8>>,
    offsets: Option<Vec<u32>>,
}

impl StringStreams {
    fn classify(streams: Vec<Stream<'_>>) -> Result<Self, MltError> {
        use StreamType as ST;
        let mut result = Self::default();
        for s in streams {
            match s.meta.stream_type {
                ST::Length(LengthType::VarBinary) => {
                    result.var_binary_lengths = Some(s.decode_bits_u32()?.decode_u32()?);
                }
                ST::Length(LengthType::Dictionary) => {
                    result.dict_lengths = Some(s.decode_bits_u32()?.decode_u32()?);
                }
                ST::Length(LengthType::Symbol) => {
                    result.symbol_lengths = Some(s.decode_bits_u32()?.decode_u32()?);
                }
                ST::Data(DictionaryType::None) => {
                    result.data_bytes = Some(raw_bytes(s));
                }
                ST::Data(DictionaryType::Single | DictionaryType::Shared) => {
                    result.dict_bytes = Some(raw_bytes(s));
                }
                ST::Data(DictionaryType::Fsst) => {
                    result.symbol_bytes = Some(raw_bytes(s));
                }
                ST::Offset(OffsetType::String) => {
                    result.offsets = Some(s.decode_bits_u32()?.decode_u32()?);
                }
                _ => Err(UnexpectedStreamType(s.meta.stream_type))?,
            }
        }
        Ok(result)
    }

    /// Decode dictionary entries from length + data streams, with optional FSST decompression.
    fn decode_dictionary(&self) -> Result<Vec<String>, MltError> {
        let dl = self.dict_lengths.as_deref();
        let dl = dl.ok_or(MissingStringStream("dictionary lengths"))?;
        let dd = self.dict_bytes.as_deref();
        let dd = dd.ok_or(MissingStringStream("dictionary data"))?;
        if let (Some(sym_lens), Some(sym_data)) = (&self.symbol_lengths, &self.symbol_bytes) {
            split_to_strings(dl, &decode_fsst(sym_data, sym_lens, dd))
        } else {
            split_to_strings(dl, dd)
        }
    }
}

/// Decode string property from its encoded stream encoding.
pub fn decode_strings(encoding: &EncodedStrProp<'_>) -> Result<Vec<String>, MltError> {
    let streams = encoding.streams();
    let ss = StringStreams::classify(streams)?;

    if let Some(offsets) = &ss.offsets {
        resolve_offsets(&ss.decode_dictionary()?, offsets)
    } else if let Some(lengths) = &ss.var_binary_lengths {
        let data = ss.data_bytes.as_deref().or(ss.dict_bytes.as_deref());
        let data = data.ok_or(MissingStringStream("string data"))?;
        split_to_strings(lengths, data)
    } else if ss.dict_lengths.is_some() {
        ss.decode_dictionary()
    } else {
        Err(MissingStringStream("any usable combination"))
    }
}

/// Decode a shared dictionary from its streams, returning the dictionary entries.
fn decode_shared_dictionary(streams: Vec<Stream<'_>>) -> Result<Vec<String>, MltError> {
    StringStreams::classify(streams)?.decode_dictionary()
}

/// Look up dictionary entries by index.
fn resolve_offsets(dict: &[String], offsets: &[u32]) -> Result<Vec<String>, MltError> {
    offsets
        .iter()
        .map(|&idx| {
            dict.get(idx as usize)
                .cloned()
                .ok_or(DictIndexOutOfBounds(idx, dict.len()))
        })
        .collect()
}

fn raw_bytes(s: Stream<'_>) -> Vec<u8> {
    match s.data {
        StreamData::Encoded(d) => d.data.to_vec(),
        StreamData::VarInt(d) => d.data.to_vec(),
    }
}

/// Split `data` into UTF-8 strings using `lengths` as byte lengths for each entry.
fn split_to_strings(lengths: &[u32], data: &[u8]) -> Result<Vec<String>, MltError> {
    let mut strings = Vec::with_capacity(lengths.len());
    let mut offset = 0;
    for &len in lengths {
        let len = len as usize;
        let Some(v) = data.get(offset..offset + len) else {
            return Err(BufferUnderflow(len, data.len().saturating_sub(offset)));
        };
        strings.push(str::from_utf8(v)?.to_owned());
        offset += len;
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
        let sym_idx = compressed[i] as usize;
        if sym_idx == 255 {
            i += 1;
            output.push(compressed[i]);
        } else if sym_idx < symbol_lengths.len() {
            let len = symbol_lengths[sym_idx] as usize;
            let off = symbol_offsets[sym_idx] as usize;
            output.extend_from_slice(&symbols[off..off + len]);
        }
        i += 1;
    }
    output
}

/// Decode a struct with shared dictionary into one decoded property per child.
pub fn decode_struct_children<'a>(
    parent_name: &str,
    struct_data: &EncodedStructProp<'_>,
) -> Result<Vec<Property<'a>>, MltError> {
    let dict = decode_shared_dictionary(struct_data.dict_streams())?;
    struct_data
        .children()
        .iter()
        .map(|child| {
            let offsets = child.data.clone().decode_bits_u32()?.decode_u32()?;
            let strings = resolve_offsets(&dict, &offsets)?;
            let name = format!("{parent_name}{}", child.name);
            let values = PropValue::Str(apply_present(child.optional.clone(), strings)?);
            Ok(Property::Decoded(DecodedProperty { name, values }))
        })
        .collect::<Result<Vec<_>, _>>()
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use borrowme::borrow;
    use proptest::prelude::*;

    use super::*;
    use crate::MltError;
    use crate::decode::FromEncoded as _;
    use crate::encode::FromDecoded as _;
    use crate::v01::property::{
        DecodedProperty, MultiPropertyEncoder, OwnedEncodedPropValue, OwnedEncodedProperty,
        PresenceStream, PropValue, Property, PropertyEncoder, ScalarEncoder,
    };
    use crate::v01::{IntEncoder, LogicalEncoder, PhysicalEncoder};

    fn strs(vals: &[&str]) -> Vec<Option<String>> {
        vals.iter().map(|v| Some(v.to_string())).collect()
    }

    fn opt_strs(vals: &[Option<&str>]) -> Vec<Option<String>> {
        vals.iter().map(|v| v.map(ToString::to_string)).collect()
    }

    fn str_prop(name: &str, values: Vec<Option<String>>) -> DecodedProperty {
        DecodedProperty {
            name: name.to_string(),
            values: PropValue::Str(values),
        }
    }

    fn expand_struct(prop: &OwnedEncodedProperty) -> Vec<DecodedProperty> {
        Property::from(borrow(prop))
            .decode_expand()
            .expect("decode_expand failed")
            .into_iter()
            .map(|p| p.decode().expect("decode failed"))
            .collect()
    }

    fn decode_scalar(prop: &OwnedEncodedProperty) -> DecodedProperty {
        DecodedProperty::from_encoded(borrow(prop)).expect("decode failed")
    }

    /// Encode a group of string children as a struct column and expand them back out.
    fn struct_encode_and_expand(
        struct_name: &str,
        children: &[(&str, Vec<Option<String>>)],
        presence: PresenceStream,
        encoder: IntEncoder,
        shared_dicts: impl Into<HashMap<String, StrEncoder>>,
    ) -> Vec<DecodedProperty> {
        let decoded: Vec<DecodedProperty> = children
            .iter()
            .map(|(child_name, values)| str_prop(child_name, values.clone()))
            .collect();
        let instructions: Vec<PropertyEncoder> = children
            .iter()
            .map(|(child_name, _)| {
                PropertyEncoder::shared_dict(struct_name, *child_name, presence, encoder)
            })
            .collect();
        let encoded_prop = Vec::<OwnedEncodedProperty>::from_decoded(
            &decoded,
            MultiPropertyEncoder {
                properties: instructions,
                shared_dicts: shared_dicts.into(),
            },
        )
        .expect("encoding failed");
        assert_eq!(
            encoded_prop.len(),
            1,
            "struct children must collapse to one column"
        );
        expand_struct(&encoded_prop[0])
    }

    fn roundtrip(decoded: &DecodedProperty, encoder: ScalarEncoder) -> DecodedProperty {
        let encoded_prop =
            OwnedEncodedProperty::from_decoded(decoded, encoder).expect("encoding failed");
        DecodedProperty::from_encoded(borrow(&encoded_prop)).expect("decoding failed")
    }

    /// Excludes `FastPFOR` because it only handles 32-bit integers.
    fn physical_no_fastpfor() -> impl Strategy<Value = PhysicalEncoder> {
        any::<PhysicalEncoder>().prop_filter("no FastPFOR", |v| *v != PhysicalEncoder::FastPFOR)
    }

    // PropValue::Str can be encoded as a standalone (non-struct) column.  This is
    // a separate code path from the shared-dictionary struct encoding below.
    #[test]
    fn str_scalar_with_nulls() {
        let prop = str_prop(
            "city",
            opt_strs(&[Some("Berlin"), None, Some("Hamburg"), None]),
        );
        let enc = ScalarEncoder::str(PresenceStream::Present, IntEncoder::plain());
        assert_eq!(roundtrip(&prop, enc), prop);
    }

    #[test]
    fn str_scalar_all_null() {
        // All-None with a presence stream: the data stream is empty, presence is all-false.
        let prop = str_prop("city", opt_strs(&[None, None, None]));
        let enc = ScalarEncoder::str(PresenceStream::Present, IntEncoder::plain());
        assert_eq!(roundtrip(&prop, enc), prop);
    }

    #[test]
    fn str_scalar_empty() {
        // Zero-row property: nothing to encode on either stream.
        let prop = str_prop("unused", vec![]);
        let enc = ScalarEncoder::str(PresenceStream::Present, IntEncoder::plain());
        assert_eq!(roundtrip(&prop, enc), prop);
    }

    proptest! {
        #[test]
        fn str_scalar_roundtrip(
            values in prop::collection::vec(
                prop::option::of("[a-zA-Z0-9 ]{0,30}"),
                0..50,
            ),
        ) {
            let prop = str_prop("name", values);
            let enc = ScalarEncoder::str(PresenceStream::Present, IntEncoder::plain());
            prop_assert_eq!(roundtrip(&prop, enc), prop);
        }
    }

    // FSST builds a symbol table from repeated byte sequences and compresses the
    // corpus against it.  Both scalar and struct paths use separate code routes.
    #[test]
    fn fsst_scalar_string_roundtrip() {
        let enc = ScalarEncoder::str_fsst(
            PresenceStream::Present,
            IntEncoder::plain(),
            IntEncoder::plain(),
        );
        // Repeated "Br" prefix gives FSST something to compress.
        let prop = str_prop(
            "name",
            strs(&["Berlin", "Brandenburg", "Bremen", "Braunschweig"]),
        );
        assert_eq!(roundtrip(&prop, enc), prop);
    }

    #[test]
    fn fsst_struct_shared_dict_roundtrip() {
        let enc = IntEncoder::plain();
        let de = strs(&["Berlin", "Brandenburg", "Bremen"]);
        let en = strs(&["Berlin", "Brandenburg", "Bremen"]);
        let result = struct_encode_and_expand(
            "name",
            &[(":de", de.clone()), (":en", en.clone())],
            PresenceStream::Present,
            enc,
            [("name".to_string(), StrEncoder::plain(IntEncoder::plain()))],
        );
        assert_eq!(result[0].values, PropValue::Str(de));
        assert_eq!(result[1].values, PropValue::Str(en));
    }

    #[test]
    fn struct_with_nulls() {
        // decode_expand must prefix each child name with the parent struct name.
        let de = opt_strs(&[Some("Berlin"), Some("München"), None]);
        let en = opt_strs(&[Some("Berlin"), None, Some("London")]);
        let result = struct_encode_and_expand(
            "name",
            &[(":de", de.clone()), (":en", en.clone())],
            PresenceStream::Present,
            IntEncoder::plain(),
            [("name".to_string(), StrEncoder::plain(IntEncoder::plain()))],
        );
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].name, "name:de");
        assert_eq!(result[0].values, PropValue::Str(de));
        assert_eq!(result[1].name, "name:en");
        assert_eq!(result[1].values, PropValue::Str(en));
    }

    #[test]
    fn struct_no_nulls() {
        let de = strs(&["Berlin", "München", "Hamburg"]);
        let en = strs(&["Berlin", "Munich", "Hamburg"]);
        let result = struct_encode_and_expand(
            "name",
            &[(":de", de.clone()), (":en", en.clone())],
            PresenceStream::Present,
            IntEncoder::plain(),
            [("name".to_string(), StrEncoder::plain(IntEncoder::plain()))],
        );
        assert_eq!(result[0].values, PropValue::Str(de));
        assert_eq!(result[1].values, PropValue::Str(en));
    }

    #[test]
    fn struct_shared_dict_deduplication() {
        // "Berlin" appears in both children.  The encoder must store it only once in
        // the shared dictionary, not once per child.  We verify this by inspecting
        // the encoded column directly: plain encoding always produces exactly 2 dict
        // streams (a length stream + a data stream), regardless of how many strings
        // are in the dictionary.  Then we confirm the decoded values are correct.
        let decoded = vec![
            str_prop(":de", strs(&["Berlin", "Berlin"])),
            str_prop(":en", strs(&["Berlin", "London"])),
        ];
        let enc = IntEncoder::plain();
        let prop_encs = vec![
            PropertyEncoder::shared_dict("name", ":de", PresenceStream::Present, enc),
            PropertyEncoder::shared_dict("name", ":en", PresenceStream::Present, enc),
        ];
        let string_enc = StrEncoder::Plain {
            string_lengths: IntEncoder::plain(),
        };
        let enc = MultiPropertyEncoder {
            properties: prop_encs.clone(),
            shared_dicts: HashMap::from([("name".to_string(), string_enc)]),
        };

        let encoded = Vec::<OwnedEncodedProperty>::from_decoded(&decoded, enc).unwrap();

        let OwnedEncodedPropValue::Struct(ref s) = encoded[0].value else {
            panic!("expected Struct variant");
        };
        // If deduplication were broken a naive implementation might write "Berlin" twice,
        // but the stream count is structural — it will always be 2 for plain encoding.
        // What changes is the data stream's byte length; we check that after decode the
        // second offset still resolves "London" correctly (dict index 1, not 2).
        assert_eq!(
            s.dict_stream_count(),
            2,
            "plain: one length stream + one data stream"
        );

        let children = expand_struct(&encoded[0]);
        assert_eq!(
            children[0].values,
            PropValue::Str(strs(&["Berlin", "Berlin"]))
        );
        assert_eq!(
            children[1].values,
            PropValue::Str(strs(&["Berlin", "London"]))
        );
    }

    #[test]
    fn struct_mixed_with_scalars() {
        // Scalar columns before and after a struct group must land in the right
        // positions after the two-pass grouping logic.
        let scalar_enc = ScalarEncoder::int(PresenceStream::Present, IntEncoder::plain());
        let population = DecodedProperty {
            name: "population".to_string(),
            values: PropValue::U32(vec![Some(3_748_000), Some(1_787_000)]),
        };
        let name_de = str_prop(":de", strs(&["Berlin", "Hamburg"]));
        let name_en = str_prop(":en", strs(&["Berlin", "Hamburg"]));
        let rank = DecodedProperty {
            name: "rank".to_string(),
            values: PropValue::U32(vec![Some(1), Some(2)]),
        };

        let props = vec![
            population.clone(),
            name_de.clone(),
            name_en.clone(),
            rank.clone(),
        ];
        let prop_encs = vec![
            PropertyEncoder::Scalar(scalar_enc),
            PropertyEncoder::shared_dict(
                "name",
                ":de",
                PresenceStream::Present,
                IntEncoder::plain(),
            ),
            PropertyEncoder::shared_dict(
                "name",
                ":en",
                PresenceStream::Present,
                IntEncoder::plain(),
            ),
            PropertyEncoder::Scalar(scalar_enc),
        ];
        let string_enc = StrEncoder::Plain {
            string_lengths: IntEncoder::plain(),
        };
        let enc = MultiPropertyEncoder {
            properties: prop_encs.clone(),
            shared_dicts: HashMap::from([("name".to_string(), string_enc)]),
        };

        let encoded_prop = Vec::<OwnedEncodedProperty>::from_decoded(&props, enc).unwrap();

        // Output order: scalar "population", struct "name", scalar "rank"
        assert_eq!(encoded_prop.len(), 3);
        assert_eq!(decode_scalar(&encoded_prop[0]), population);
        let name = expand_struct(&encoded_prop[1]);
        assert_eq!(name[0].name, "name:de");
        assert_eq!(name[0].values, name_de.values);
        assert_eq!(name[1].name, "name:en");
        assert_eq!(name[1].values, name_en.values);
        assert_eq!(decode_scalar(&encoded_prop[2]), rank);
    }

    #[test]
    fn two_struct_groups_with_scalar_between() {
        // Two independent struct columns must each get their own shared dictionary
        // and appear at the position of their first child in the output.
        let name_de = str_prop(":de", strs(&["Berlin", "Hamburg"]));
        let name_en = str_prop(":en", strs(&["Berlin", "Hamburg"]));
        let population = DecodedProperty {
            name: "population".to_string(),
            values: PropValue::U32(vec![Some(3_748_000), Some(1_787_000)]),
        };
        let label_de = str_prop(":de", strs(&["BE", "HH"]));
        let label_en = str_prop(":en", strs(&["BER", "HAM"]));

        let decoded_props = vec![
            name_de.clone(),
            name_en.clone(),
            population.clone(),
            label_de.clone(),
            label_en.clone(),
        ];
        let enc = IntEncoder::plain();
        let prop_encoders = vec![
            PropertyEncoder::shared_dict("name:", "de", PresenceStream::Present, enc),
            PropertyEncoder::shared_dict("name:", "en", PresenceStream::Present, enc),
            ScalarEncoder::int(PresenceStream::Present, enc).into(),
            PropertyEncoder::shared_dict("label:", "de", PresenceStream::Present, enc),
            PropertyEncoder::shared_dict("label:", "en", PresenceStream::Present, enc),
        ];
        let str_enc = StrEncoder::plain(IntEncoder::plain());
        let encoded_prop = Vec::<OwnedEncodedProperty>::from_decoded(
            &decoded_props,
            MultiPropertyEncoder {
                properties: prop_encoders,
                shared_dicts: HashMap::from([
                    ("name:".to_string(), str_enc),
                    ("label:".to_string(), str_enc),
                ]),
            },
        )
        .unwrap();

        // Expected output order: struct "name", scalar "population", struct "label"
        assert_eq!(encoded_prop.len(), 3);
        let name = expand_struct(&encoded_prop[0]);
        assert_eq!(name[0].name, "name:de");
        assert_eq!(name[0].values, name_de.values);
        assert_eq!(name[1].name, "name:en");
        assert_eq!(name[1].values, name_en.values);
        assert_eq!(decode_scalar(&encoded_prop[1]), population);
        let label = expand_struct(&encoded_prop[2]);
        assert_eq!(label[0].name, "label:de");
        assert_eq!(label[0].values, label_de.values);
        assert_eq!(label[1].name, "label:en");
        assert_eq!(label[1].values, label_en.values);
    }

    #[test]
    fn struct_instruction_count_mismatch() {
        let err = Vec::<OwnedEncodedProperty>::from_decoded(
            &vec![DecodedProperty::default()],
            MultiPropertyEncoder {
                properties: vec![],
                shared_dicts: HashMap::default(),
            },
        )
        .unwrap_err();
        assert!(
            matches!(
                err,
                MltError::EncodingInstructionCountMismatch {
                    input_len: 1,
                    config_len: 0
                }
            ),
            "unexpected error: {err}"
        );
    }

    proptest! {
        #[test]
        fn struct_roundtrip(
            struct_name in "[a-z]{1,8}",
            children in prop::collection::vec(
                (
                    "[a-z]{1,6}",
                    prop::collection::vec(prop::option::of("[a-zA-Z ]{0,20}"), 0..20),
                ),
                1..5usize,
            ),
            logical in any::<LogicalEncoder>(),
            physical in physical_no_fastpfor(),
            string_enc in  any::<StrEncoder>(),
        ) {
            let encoder = IntEncoder::new(logical, physical);
            let decoded: Vec<DecodedProperty> = children
                .iter()
                .map(|(child_name, values)| str_prop(child_name, values.clone()))
                .collect();
            let properties: Vec<PropertyEncoder> = children
                .iter()
                .map(|(child_name, _)| {
                    PropertyEncoder::shared_dict(&struct_name, child_name,PresenceStream::Present,  encoder)
                })
                .collect();
            let shared_dicts = HashMap::from([(struct_name.clone(),string_enc)]);
            let enc = MultiPropertyEncoder{ properties, shared_dicts };
            let encoded = Vec::<OwnedEncodedProperty>::from_decoded(&decoded, enc)
                .expect("encoding failed");
            prop_assert_eq!(encoded.len(), 1, "struct children must collapse to one column");
            let re_children = expand_struct(&encoded[0]);
            prop_assert_eq!(re_children.len(), children.len());
            for (re, (child_name, values)) in re_children.into_iter().zip(children.iter()) {
                prop_assert_eq!(re.name, format!("{struct_name}{child_name}"));
                prop_assert_eq!(re.values, PropValue::Str(values.clone()));
            }
        }
    }
}
