use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::io::Write;

use crate::MltError;
use crate::MltError::{
    BufferUnderflow, DictIndexOutOfBounds, NotImplemented, UnexpectedStreamType2,
};
use crate::utils::{AsUsize as _, BinarySerializer as _, apply_present};
use crate::v01::{
    ColumnType, DecodedProperty, DictionaryType, EncodedSharedDict, EncodedStrings, EncodedValues,
    FsstStrEncoder, IntEncoder, LengthType, OffsetType, OwnedEncodedPropValue,
    OwnedEncodedProperty, OwnedEncodedSharedDict, OwnedEncodedStrings, OwnedEncodedValues,
    OwnedStream, PresenceStream, PropValue, SharedDictItem, Stream, StreamData, StreamType,
};

impl<'a> EncodedValues<'a> {
    #[must_use]
    pub fn new(name: &'a str, presence: Option<Stream<'a>>, data: Stream<'a>) -> Self {
        Self {
            name,
            presence,
            data,
        }
    }
}

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
    pub fn plain(
        name: &'a str,
        presence: Option<Stream<'a>>,
        lengths: Stream<'a>,
        data: Stream<'a>,
    ) -> Result<Self, MltError> {
        validate_stream!(lengths, StreamType::Length(LengthType::VarBinary));
        validate_stream!(
            data,
            StreamType::Data(DictionaryType::None | DictionaryType::Single)
        );
        Ok(Self::Plain {
            name,
            presence,
            lengths,
            data,
        })
    }

    pub fn dictionary(
        name: &'a str,
        presence: Option<Stream<'a>>,
        lengths: Stream<'a>,
        offsets: Stream<'a>,
        data: Stream<'a>,
    ) -> Result<Self, MltError> {
        validate_stream!(lengths, StreamType::Length(LengthType::Dictionary));
        validate_stream!(offsets, StreamType::Offset(OffsetType::String));
        validate_stream!(data, StreamType::Data(DictionaryType::Single));
        Ok(Self::Dictionary {
            name,
            presence,
            lengths,
            offsets,
            data,
        })
    }

    pub fn fsst_plain(
        name: &'a str,
        presence: Option<Stream<'a>>,
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
            name,
            presence,
            symbol_lengths,
            symbol_table,
            lengths,
            corpus,
        })
    }

    pub fn fsst_dictionary(
        name: &'a str,
        presence: Option<Stream<'a>>,
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
            name,
            presence,
            symbol_lengths,
            symbol_table,
            lengths,
            corpus,
            offsets,
        })
    }

    /// Returns the presence stream if this column is optional.
    #[must_use]
    pub fn presence(&self) -> Option<&Stream<'_>> {
        match self {
            Self::Plain { presence, .. }
            | Self::Dictionary { presence, .. }
            | Self::FsstPlain { presence, .. }
            | Self::FsstDictionary { presence, .. } => presence.as_ref(),
        }
    }

    /// Streams in wire order: presence (if any) then content streams.
    #[must_use]
    pub fn streams(&self) -> Vec<&Stream<'_>> {
        let mut v: Vec<&Stream<'_>> = self.presence().into_iter().collect();
        match self {
            Self::Plain {
                lengths,
                data,
                name: _,
                presence: _,
            } => v.extend([lengths, data]),
            Self::Dictionary {
                lengths,
                offsets,
                data,
                name: _,
                presence: _,
            } => v.extend([lengths, offsets, data]),
            Self::FsstPlain {
                symbol_lengths,
                symbol_table,
                lengths,
                corpus,
                name: _,
                presence: _,
            } => v.extend([symbol_lengths, symbol_table, lengths, corpus]),
            Self::FsstDictionary {
                symbol_lengths,
                symbol_table,
                lengths,
                corpus,
                offsets,
                name: _,
                presence: _,
            } => v.extend([symbol_lengths, symbol_table, lengths, corpus, offsets]),
        }
        v
    }
}

impl OwnedEncodedStrings {
    /// Returns the presence stream if this column is optional.
    #[must_use]
    pub fn presence(&self) -> Option<&OwnedStream> {
        match self {
            Self::Plain { presence, .. }
            | Self::Dictionary { presence, .. }
            | Self::FsstPlain { presence, .. }
            | Self::FsstDictionary { presence, .. } => presence.as_ref(),
        }
    }

    /// Content streams only (excluding presence).
    #[must_use]
    pub fn content_streams(&self) -> Vec<&OwnedStream> {
        match self {
            Self::Plain {
                lengths,
                data,
                name: _,
                presence: _,
            } => vec![lengths, data],
            Self::Dictionary {
                lengths,
                offsets,
                data,
                name: _,
                presence: _,
            } => vec![lengths, offsets, data],
            Self::FsstPlain {
                symbol_lengths,
                symbol_table,
                lengths,
                corpus,
                name: _,
                presence: _,
            } => {
                vec![symbol_lengths, symbol_table, lengths, corpus]
            }
            Self::FsstDictionary {
                symbol_lengths,
                symbol_table,
                lengths,
                corpus,
                offsets,
                name: _,
                presence: _,
            } => vec![symbol_lengths, symbol_table, lengths, corpus, offsets],
        }
    }

    /// Streams in wire order: presence (if any) then content streams.
    #[must_use]
    pub fn streams(&self) -> Vec<&OwnedStream> {
        let mut v: Vec<&OwnedStream> = self.presence().into_iter().collect();
        v.extend(self.content_streams());
        v
    }
}

impl OwnedEncodedStrings {
    pub fn set_name(&mut self, n: String) {
        match self {
            Self::Plain { name, .. }
            | Self::Dictionary { name, .. }
            | Self::FsstPlain { name, .. }
            | Self::FsstDictionary { name, .. } => *name = n,
        }
    }

    pub fn set_presence(&mut self, p: Option<OwnedStream>) {
        match self {
            Self::Plain { presence, .. }
            | Self::Dictionary { presence, .. }
            | Self::FsstPlain { presence, .. }
            | Self::FsstDictionary { presence, .. } => *presence = p,
        }
    }
}

impl<'a> EncodedSharedDict<'a> {
    /// Plain shared dict (2 streams): lengths + data.
    pub fn plain(
        prefix: &'a str,
        lengths: Stream<'a>,
        data: Stream<'a>,
        children: Vec<EncodedValues<'a>>,
    ) -> Result<Self, MltError> {
        validate_stream!(lengths, StreamType::Length(LengthType::Dictionary));
        validate_stream!(data, StreamType::Data(DictionaryType::Shared));
        Ok(Self::Plain {
            prefix,
            lengths,
            data,
            children,
        })
    }

    /// FSST plain shared dict (4 streams): symbol lengths, symbol table, lengths, corpus.
    pub fn fsst_plain(
        prefix: &'a str,
        symbol_lengths: Stream<'a>,
        symbol_table: Stream<'a>,
        lengths: Stream<'a>,
        corpus: Stream<'a>,
        children: Vec<EncodedValues<'a>>,
    ) -> Result<Self, MltError> {
        validate_stream!(symbol_lengths, StreamType::Length(LengthType::Symbol));
        validate_stream!(symbol_table, StreamType::Data(DictionaryType::Fsst));
        validate_stream!(lengths, StreamType::Length(LengthType::Dictionary));
        validate_stream!(
            corpus,
            StreamType::Data(DictionaryType::Single | DictionaryType::Shared)
        );
        Ok(Self::FsstPlain {
            prefix,
            symbol_lengths,
            symbol_table,
            lengths,
            corpus,
            children,
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

    /// All streams in wire order: dict streams then each child's presence (if any) and data.
    #[must_use]
    pub fn streams(&self) -> Vec<&Stream<'_>> {
        let mut v = self.dict_streams();
        for c in self.children() {
            if let Some(ref o) = c.presence {
                v.push(o);
            }
            v.push(&c.data);
        }
        v
    }

    #[must_use]
    pub fn children(&self) -> &[EncodedValues<'a>] {
        match self {
            Self::Plain { children, .. } | Self::FsstPlain { children, .. } => children,
        }
    }
}

impl OwnedEncodedSharedDict {
    #[must_use]
    pub fn streams(&self) -> Vec<&OwnedStream> {
        let mut v = self.dict_streams();
        for c in self.children() {
            if let Some(o) = &c.presence {
                v.push(o);
            }
            v.push(&c.data);
        }
        v
    }

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

    #[must_use]
    pub fn children(&self) -> &[OwnedEncodedValues] {
        match self {
            Self::Plain { children, .. } | Self::FsstPlain { children, .. } => children,
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
        let presence = if child.presence == PresenceStream::Present {
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
            child.offsets,
            StreamType::Offset(OffsetType::String),
        )?;

        children.push(OwnedEncodedValues {
            name: child.prop_name.clone(),
            presence,
            data,
        });
    }

    let struct_prop = match dict_encoded {
        OwnedEncodedStrings::Plain {
            lengths,
            data,
            name: _,
            presence: _,
        } => OwnedEncodedSharedDict::Plain {
            prefix: name.to_owned(),
            lengths,
            data,
            children,
        },
        OwnedEncodedStrings::FsstPlain {
            symbol_lengths,
            symbol_table,
            lengths,
            corpus,
            name: _,
            presence: _,
        } => OwnedEncodedSharedDict::FsstPlain {
            prefix: name.to_owned(),
            symbol_lengths,
            symbol_table,
            lengths,
            corpus,
            children,
        },
        OwnedEncodedStrings::Dictionary { .. } | OwnedEncodedStrings::FsstDictionary { .. } => {
            return Err(NotImplemented(
                "SharedDict only supports Plain or FsstPlain encoding",
            ));
        }
    };

    Ok(OwnedEncodedProperty {
        name: name.to_owned(),
        value: OwnedEncodedPropValue::SharedDict(struct_prop),
    })
}

/// Encode a shared dictionary property directly from `PropValue::SharedDict` and `SharedDictEncoder`.
pub fn encode_shared_dict_prop(
    prefix: &str,
    items: &[SharedDictItem],
    encoder: &SharedDictEncoder,
) -> Result<OwnedEncodedPropValue, MltError> {
    if items.len() != encoder.items.len() {
        return Err(NotImplemented(
            "SharedDict items count must match encoder items count",
        ));
    }

    // Build shared dictionary: unique strings in first-occurrence insertion order.
    let mut dict: Vec<&str> = Vec::new();
    let mut dict_index: HashMap<&str, u32> = HashMap::new();

    for item in items {
        for s in item.values.iter().flatten() {
            if let Entry::Vacant(e) = dict_index.entry(s) {
                let idx = u32::try_from(dict.len())?;
                e.insert(idx);
                dict.push(s);
            }
        }
    }

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
    let mut children = Vec::with_capacity(items.len());
    for (item, item_enc) in items.iter().zip(&encoder.items) {
        // Presence stream
        let presence = if item_enc.presence == PresenceStream::Present {
            let present_bools: Vec<bool> = item.values.iter().map(Option::is_some).collect();
            Some(OwnedStream::encode_presence(&present_bools)?)
        } else {
            None
        };

        // Offset indices for non-null values only.
        let offsets: Vec<u32> = item
            .values
            .iter()
            .filter_map(|v| v.as_deref())
            .map(|s| dict_index[s])
            .collect();

        let data = OwnedStream::encode_u32s_of_type(
            &offsets,
            item_enc.offsets,
            StreamType::Offset(OffsetType::String),
        )?;

        children.push(OwnedEncodedValues {
            name: item.suffix.clone(),
            presence,
            data,
        });
    }

    let struct_prop = match dict_encoded {
        OwnedEncodedStrings::Plain {
            lengths,
            data,
            name: _,
            presence: _,
        } => OwnedEncodedSharedDict::Plain {
            prefix: prefix.to_owned(),
            lengths,
            data,
            children,
        },
        OwnedEncodedStrings::FsstPlain {
            symbol_lengths,
            symbol_table,
            lengths,
            corpus,
            name: _,
            presence: _,
        } => OwnedEncodedSharedDict::FsstPlain {
            prefix: prefix.to_owned(),
            symbol_lengths,
            symbol_table,
            lengths,
            corpus,
            children,
        },
        OwnedEncodedStrings::Dictionary { .. } | OwnedEncodedStrings::FsstDictionary { .. } => {
            return Err(NotImplemented(
                "SharedDict only supports Plain or FsstPlain encoding",
            ));
        }
    };

    Ok(OwnedEncodedPropValue::SharedDict(struct_prop))
}

impl OwnedEncodedValues {
    pub(crate) fn write_columns_meta_to<W: Write>(&self, writer: &mut W) -> Result<(), MltError> {
        let typ = if self.presence.is_some() {
            ColumnType::OptStr
        } else {
            ColumnType::Str
        };
        typ.write_to(writer)?;
        writer.write_string(&self.name)?;
        Ok(())
    }
}

/// Decode string property, returning the presence stream and decoded string values.
pub fn decode_strings_with_presence(
    encoding: EncodedStrings<'_>,
) -> Result<(Option<Stream<'_>>, Vec<String>), MltError> {
    let presence = match &encoding {
        EncodedStrings::Plain { presence, .. }
        | EncodedStrings::Dictionary { presence, .. }
        | EncodedStrings::FsstPlain { presence, .. }
        | EncodedStrings::FsstDictionary { presence, .. } => presence.clone(),
    };
    let strings = decode_strings(encoding)?;
    Ok((presence, strings))
}

/// Decode string property from its encoded stream encoding.
pub fn decode_strings(encoding: EncodedStrings<'_>) -> Result<Vec<String>, MltError> {
    match encoding {
        EncodedStrings::Plain {
            lengths,
            data,
            name: _,
            presence: _,
        } => {
            let lens = lengths.decode_bits_u32()?.decode_u32()?;
            let data_bytes = raw_bytes(data);
            split_to_strings(&lens, &data_bytes)
        }
        EncodedStrings::Dictionary {
            lengths,
            offsets,
            data,
            name: _,
            presence: _,
        } => {
            let dict_lens = lengths.decode_bits_u32()?.decode_u32()?;
            let dict_bytes = raw_bytes(data);
            let dict = split_to_strings(&dict_lens, &dict_bytes)?;
            let offset_vals = offsets.decode_bits_u32()?.decode_u32()?;
            resolve_offsets(&dict, &offset_vals)
        }
        EncodedStrings::FsstPlain {
            symbol_lengths,
            symbol_table,
            lengths,
            corpus,
            name: _,
            presence: _,
        } => {
            let sym_lens = symbol_lengths.decode_bits_u32()?.decode_u32()?;
            let sym_data = raw_bytes(symbol_table);
            let dict_lens = lengths.decode_bits_u32()?.decode_u32()?;
            let compressed = raw_bytes(corpus);
            let decompressed = decode_fsst(&sym_data, &sym_lens, &compressed);
            split_to_strings(&dict_lens, &decompressed)
        }
        EncodedStrings::FsstDictionary {
            symbol_lengths,
            symbol_table,
            lengths,
            corpus,
            offsets,
            name: _,
            presence: _,
        } => {
            let sym_lens = symbol_lengths.decode_bits_u32()?.decode_u32()?;
            let sym_data = raw_bytes(symbol_table);
            let dict_lens = lengths.decode_bits_u32()?.decode_u32()?;
            let compressed = raw_bytes(corpus);
            let decompressed = decode_fsst(&sym_data, &sym_lens, &compressed);
            let dict = split_to_strings(&dict_lens, &decompressed)?;
            let offset_vals = offsets.decode_bits_u32()?.decode_u32()?;
            resolve_offsets(&dict, &offset_vals)
        }
    }
}

/// Look up dictionary entries by index.
fn resolve_offsets(dict: &[String], offsets: &[u32]) -> Result<Vec<String>, MltError> {
    offsets
        .iter()
        .map(|&idx| {
            dict.get(idx.as_usize())
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
    let mut offset = 0_usize;
    for &len in lengths {
        let len_usize = len.as_usize();
        let Some(v) = data.get(offset..offset + len_usize) else {
            return Err(BufferUnderflow(len, data.len().saturating_sub(offset)));
        };
        strings.push(str::from_utf8(v)?.to_owned());
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
pub fn decode_shared_dict(struct_data: &EncodedSharedDict<'_>) -> Result<PropValue, MltError> {
    let dict = struct_data.decode_dictionary()?;
    Ok(PropValue::SharedDict(
        struct_data
            .children()
            .iter()
            .map(|child| -> Result<SharedDictItem, MltError> {
                let offsets = child.data.clone().decode_bits_u32()?.decode_u32()?;
                let strings = resolve_offsets(&dict, &offsets)?;
                let values = apply_present(child.presence.clone(), strings)?;
                Ok(SharedDictItem {
                    suffix: child.name.to_string(),
                    values,
                })
            })
            .collect::<Result<Vec<_>, _>>()?,
    ))
}
