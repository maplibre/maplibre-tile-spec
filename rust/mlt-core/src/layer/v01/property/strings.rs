use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::io::Write;

use borrowme::borrowme;

use crate::MltError;
use crate::MltError::{
    BufferUnderflow, DictIndexOutOfBounds, NotImplemented, UnexpectedStreamType2,
};
use crate::utils::{BinarySerializer as _, apply_present};
use crate::v01::{
    ColumnType, DecodedProperty, DictionaryType, FsstStrEncoder, IntEncoder, LengthType,
    OffsetType, OwnedEncodedPropValue, OwnedEncodedProperty, OwnedStream, PresenceStream,
    PropValue, Property, Stream, StreamData, StreamType,
};

/// A single child field within a `SharedDict` column
#[borrowme]
#[derive(Clone, Debug, PartialEq)]
pub struct EncodedStructChild<'a> {
    pub name: &'a str,
    pub typ: ColumnType,
    pub optional: Option<Stream<'a>>,
    pub data: Stream<'a>,
}

/// Encoded data for a `SharedDict` column with shared dictionary encoding.
#[borrowme]
#[derive(Debug, Clone, PartialEq)]
pub enum EncodedSharedDictProp<'a> {
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

#[derive(Debug, Clone, PartialEq, Eq)]
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
    /// FSST plain (4 streams): symbol lengths, symbol table, value length, compressed corpus. No offset.
    FsstPlain {
        symbol_lengths: Stream<'a>,
        symbol_table: Stream<'a>,
        length: Stream<'a>,
        corpus: Stream<'a>,
    },
    /// FSST dictionary (5 streams): symbol lengths, symbol table, value length, compressed corpus, offset.
    FsstDictionary {
        symbol_lengths: Stream<'a>,
        symbol_table: Stream<'a>,
        length: Stream<'a>,
        corpus: Stream<'a>,
        offset: Stream<'a>,
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

    pub fn fsst_plain(
        symbol_lengths: Stream<'a>,
        symbol_table: Stream<'a>,
        length: Stream<'a>,
        corpus: Stream<'a>,
    ) -> Result<Self, MltError> {
        validate_stream!(symbol_lengths, StreamType::Length(LengthType::Symbol));
        validate_stream!(symbol_table, StreamType::Data(DictionaryType::Fsst));
        validate_stream!(length, StreamType::Length(LengthType::Dictionary));
        validate_stream!(corpus, StreamType::Data(DictionaryType::Single));
        Ok(Self::FsstPlain {
            symbol_lengths,
            symbol_table,
            length,
            corpus,
        })
    }

    pub fn fsst_dictionary(
        symbol_lengths: Stream<'a>,
        symbol_table: Stream<'a>,
        length: Stream<'a>,
        corpus: Stream<'a>,
        offset: Stream<'a>,
    ) -> Result<Self, MltError> {
        validate_stream!(symbol_lengths, StreamType::Length(LengthType::Symbol));
        validate_stream!(symbol_table, StreamType::Data(DictionaryType::Fsst));
        validate_stream!(length, StreamType::Length(LengthType::Dictionary));
        validate_stream!(corpus, StreamType::Data(DictionaryType::Single));
        validate_stream!(offset, StreamType::Offset(OffsetType::String));
        Ok(Self::FsstDictionary {
            symbol_lengths,
            symbol_table,
            length,
            corpus,
            offset,
        })
    }

    /// Streams in an order suitable for classification [`decode_strings`]
    #[must_use]
    pub fn streams(&self) -> Vec<&Stream<'_>> {
        match self {
            Self::Plain { lengths, data } => vec![lengths, data],
            Self::Dictionary {
                lengths,
                offset,
                data,
            } => vec![lengths, offset, data],
            Self::FsstPlain {
                symbol_lengths,
                symbol_table,
                length,
                corpus,
            } => vec![symbol_lengths, symbol_table, length, corpus],
            Self::FsstDictionary {
                symbol_lengths,
                symbol_table,
                length,
                corpus,
                offset,
            } => vec![symbol_lengths, symbol_table, length, corpus, offset],
        }
    }
}

impl OwnedEncodedStrProp {
    /// Streams in wire order for serialization.
    #[must_use]
    pub fn streams(&self) -> Vec<&OwnedStream> {
        match self {
            Self::Plain { lengths, data } => vec![lengths, data],
            Self::Dictionary {
                lengths,
                offset,
                data,
            } => vec![lengths, offset, data],
            Self::FsstPlain {
                symbol_lengths,
                symbol_table,
                length,
                corpus,
            } => vec![symbol_lengths, symbol_table, length, corpus],
            Self::FsstDictionary {
                symbol_lengths,
                symbol_table,
                length,
                corpus,
                offset,
            } => vec![symbol_lengths, symbol_table, length, corpus, offset],
        }
    }
}

impl<'a> EncodedSharedDictProp<'a> {
    /// Plain shared dict (2 streams): length + data.
    pub fn plain(
        enc_len: Stream<'a>,
        enc_data: Stream<'a>,
        children: Vec<EncodedStructChild<'a>>,
    ) -> Result<Self, MltError> {
        validate_stream!(enc_len, StreamType::Length(LengthType::Dictionary));
        validate_stream!(enc_data, StreamType::Data(DictionaryType::Shared));
        Ok(Self::Plain {
            enc_len,
            enc_data,
            children,
        })
    }

    /// Dictionary shared dict (3 streams): length + offset + data.
    pub fn dictionary(
        enc_len: Stream<'a>,
        enc_off: Stream<'a>,
        enc_data: Stream<'a>,
        children: Vec<EncodedStructChild<'a>>,
    ) -> Result<Self, MltError> {
        validate_stream!(enc_len, StreamType::Length(LengthType::Dictionary));
        validate_stream!(enc_off, StreamType::Offset(OffsetType::String));
        validate_stream!(enc_data, StreamType::Data(DictionaryType::Shared));
        Ok(Self::Dictionary {
            enc_len,
            enc_off,
            enc_data,
            children,
        })
    }

    /// FSST plain shared dict (4 streams): symbol lengths, symbol table, length, corpus.
    pub fn fsst_plain(
        enc_len_sym: Stream<'a>,
        symbol_table: Stream<'a>,
        enc_len_dic: Stream<'a>,
        corpus: Stream<'a>,
        children: Vec<EncodedStructChild<'a>>,
    ) -> Result<Self, MltError> {
        validate_stream!(enc_len_sym, StreamType::Length(LengthType::Symbol));
        validate_stream!(symbol_table, StreamType::Data(DictionaryType::Fsst));
        validate_stream!(enc_len_dic, StreamType::Length(LengthType::Dictionary));
        validate_stream!(
            corpus,
            StreamType::Data(DictionaryType::Single | DictionaryType::Shared)
        );
        Ok(Self::FsstPlain {
            enc_len_sym,
            symbol_table,
            enc_len_dic,
            corpus,
            children,
        })
    }

    /// FSST dictionary shared dict (5 streams): symbol lengths, symbol table, length, corpus, offset.
    pub fn fsst_dictionary(
        enc_len_sym: Stream<'a>,
        symbol_table: Stream<'a>,
        enc_len_dic: Stream<'a>,
        corpus: Stream<'a>,
        offset: Stream<'a>,
        children: Vec<EncodedStructChild<'a>>,
    ) -> Result<Self, MltError> {
        validate_stream!(enc_len_sym, StreamType::Length(LengthType::Symbol));
        validate_stream!(symbol_table, StreamType::Data(DictionaryType::Fsst));
        validate_stream!(enc_len_dic, StreamType::Length(LengthType::Dictionary));
        validate_stream!(
            corpus,
            StreamType::Data(DictionaryType::Single | DictionaryType::Shared)
        );
        validate_stream!(offset, StreamType::Offset(OffsetType::String));
        Ok(Self::FsstDictionary {
            enc_len_sym,
            symbol_table,
            enc_len_dic,
            corpus,
            offset,
            children,
        })
    }

    /// Decode the shared dictionary entries from this encoding.
    pub fn decode_dictionary(&self) -> Result<Vec<String>, MltError> {
        match self {
            Self::Plain {
                enc_len, enc_data, ..
            }
            | Self::Dictionary {
                enc_len, enc_data, ..
            } => {
                let lens = enc_len.clone().decode_bits_u32()?.decode_u32()?;
                let data = raw_bytes(enc_data.clone());
                split_to_strings(&lens, &data)
            }
            Self::FsstPlain {
                enc_len_sym,
                symbol_table,
                enc_len_dic,
                corpus,
                ..
            }
            | Self::FsstDictionary {
                enc_len_sym,
                symbol_table,
                enc_len_dic,
                corpus,
                ..
            } => {
                let sym_lens = enc_len_sym.clone().decode_bits_u32()?.decode_u32()?;
                let sym_data = raw_bytes(symbol_table.clone());
                let dict_lens = enc_len_dic.clone().decode_bits_u32()?.decode_u32()?;
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

    /// All streams in wire order: dict streams then each child's optional (if any) and data.
    #[must_use]
    pub fn streams(&self) -> Vec<&Stream<'_>> {
        let mut v = self.dict_streams();
        for c in self.children() {
            if let Some(ref o) = c.optional {
                v.push(o);
            }
            v.push(&c.data);
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

impl OwnedEncodedSharedDictProp {
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
        StrEncoder::Fsst(enc) => {
            OwnedStream::encode_strings_fsst_plain_with_type(&dict, enc, DictionaryType::Single)?
        }
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
        } => OwnedEncodedSharedDictProp::Plain {
            enc_len,
            enc_data,
            children,
        },
        OwnedEncodedStrProp::Dictionary {
            lengths: enc_len,
            offset: enc_off,
            data: enc_data,
        } => OwnedEncodedSharedDictProp::Dictionary {
            enc_len,
            enc_off,
            enc_data,
            children,
        },
        OwnedEncodedStrProp::FsstPlain {
            symbol_lengths: enc_len_sym,
            symbol_table,
            length: enc_len_dic,
            corpus,
        } => OwnedEncodedSharedDictProp::FsstPlain {
            enc_len_sym,
            symbol_table,
            enc_len_dic,
            corpus,
            children,
        },
        OwnedEncodedStrProp::FsstDictionary {
            symbol_lengths: enc_len_sym,
            symbol_table,
            length: enc_len_dic,
            corpus,
            offset,
        } => OwnedEncodedSharedDictProp::FsstDictionary {
            enc_len_sym,
            symbol_table,
            enc_len_dic,
            corpus,
            offset,
            children,
        },
    };

    Ok(OwnedEncodedProperty {
        name: name.to_owned(),
        value: OwnedEncodedPropValue::SharedDict(struct_prop),
    })
}

impl OwnedEncodedStructChild {
    pub(crate) fn write_columns_meta_to<W: Write>(&self, writer: &mut W) -> Result<(), MltError> {
        self.typ.write_to(writer)?;
        writer.write_string(&self.name)?;
        Ok(())
    }
}

/// Decode string property from its encoded stream encoding.
pub fn decode_strings(encoding: EncodedStrProp<'_>) -> Result<Vec<String>, MltError> {
    match encoding {
        EncodedStrProp::Plain { lengths, data } => {
            let lens = lengths.decode_bits_u32()?.decode_u32()?;
            let data_bytes = raw_bytes(data);
            split_to_strings(&lens, &data_bytes)
        }
        EncodedStrProp::Dictionary {
            lengths,
            offset,
            data,
        } => {
            let dict_lens = lengths.decode_bits_u32()?.decode_u32()?;
            let dict_bytes = raw_bytes(data);
            let dict = split_to_strings(&dict_lens, &dict_bytes)?;
            let offsets = offset.decode_bits_u32()?.decode_u32()?;
            resolve_offsets(&dict, &offsets)
        }
        EncodedStrProp::FsstPlain {
            symbol_lengths,
            symbol_table,
            length,
            corpus,
        } => {
            let sym_lens = symbol_lengths.decode_bits_u32()?.decode_u32()?;
            let sym_data = raw_bytes(symbol_table);
            let dict_lens = length.decode_bits_u32()?.decode_u32()?;
            let compressed = raw_bytes(corpus);
            let decompressed = decode_fsst(&sym_data, &sym_lens, &compressed);
            split_to_strings(&dict_lens, &decompressed)
        }
        EncodedStrProp::FsstDictionary {
            symbol_lengths,
            symbol_table,
            length,
            corpus,
            offset,
        } => {
            let sym_lens = symbol_lengths.decode_bits_u32()?.decode_u32()?;
            let sym_data = raw_bytes(symbol_table);
            let dict_lens = length.decode_bits_u32()?.decode_u32()?;
            let compressed = raw_bytes(corpus);
            let decompressed = decode_fsst(&sym_data, &sym_lens, &compressed);
            let dict = split_to_strings(&dict_lens, &decompressed)?;
            let offsets = offset.decode_bits_u32()?.decode_u32()?;
            resolve_offsets(&dict, &offsets)
        }
    }
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
    struct_data: &EncodedSharedDictProp<'_>,
) -> Result<Vec<Property<'a>>, MltError> {
    let dict = struct_data.decode_dictionary()?;
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
