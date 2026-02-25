use crate::MltError;
use crate::MltError::{
    BufferUnderflow, DictIndexOutOfBounds, MissingStringStream, UnexpectedStreamType,
};
use crate::utils::apply_present;
use crate::v01::{
    DecodedProperty, DictionaryType, EncodedStructProp, LengthType, OffsetType, PropValue,
    Property, Stream, StreamData, StreamType,
};

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
        use StreamType as PST;
        let mut result = Self::default();
        for s in streams {
            match s.meta.stream_type {
                PST::Length(LengthType::VarBinary) => {
                    result.var_binary_lengths = Some(s.decode_bits_u32()?.decode_u32()?);
                }
                PST::Length(LengthType::Dictionary) => {
                    result.dict_lengths = Some(s.decode_bits_u32()?.decode_u32()?);
                }
                PST::Length(LengthType::Symbol) => {
                    result.symbol_lengths = Some(s.decode_bits_u32()?.decode_u32()?);
                }
                PST::Data(DictionaryType::None) => {
                    result.data_bytes = Some(raw_bytes(s));
                }
                PST::Data(DictionaryType::Single | DictionaryType::Shared) => {
                    result.dict_bytes = Some(raw_bytes(s));
                }
                PST::Data(DictionaryType::Fsst) => {
                    result.symbol_bytes = Some(raw_bytes(s));
                }
                PST::Offset(OffsetType::String) => {
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

/// Decode string property from its sub-streams.
pub fn decode_string_streams(streams: Vec<Stream<'_>>) -> Result<Vec<String>, MltError> {
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
    struct_data: EncodedStructProp<'_>,
) -> Result<Vec<Property<'a>>, MltError> {
    let dict = decode_shared_dictionary(struct_data.dict_streams)?;
    struct_data
        .children
        .into_iter()
        .map(|child| {
            let present = if let Some(c) = child.optional {
                Some(c.decode_bools()?)
            } else {
                None
            };
            let offsets = child.data.decode_bits_u32()?.decode_u32()?;
            let strings = resolve_offsets(&dict, &offsets)?;
            let name = format!("{parent_name}{}", child.name);
            let values = PropValue::Str(apply_present(present, strings)?);
            Ok(Property::Decoded(DecodedProperty { name, values }))
        })
        .collect()
}
