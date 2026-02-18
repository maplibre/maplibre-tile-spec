use crate::MltError;
use crate::v01::{DictionaryType, LengthType, OffsetType, PhysicalStreamType, Stream, StreamData};

/// Decode string property from its sub-streams
pub fn decode_string_streams(streams: Vec<Stream<'_>>) -> Result<Vec<String>, MltError> {
    let mut var_binary_lengths: Option<Vec<u32>> = None;
    let mut dict_lengths: Option<Vec<u32>> = None;
    let mut symbol_lengths: Option<Vec<u32>> = None;
    let mut data_bytes: Option<Vec<u8>> = None;
    let mut dict_bytes: Option<Vec<u8>> = None;
    let mut symbol_bytes: Option<Vec<u8>> = None;
    let mut offsets: Option<Vec<u32>> = None;

    for s in streams {
        match s.meta.physical_type {
            PhysicalStreamType::Length(LengthType::VarBinary) => {
                var_binary_lengths = Some(s.decode_bits_u32()?.decode_u32()?);
            }
            PhysicalStreamType::Length(LengthType::Dictionary) => {
                dict_lengths = Some(s.decode_bits_u32()?.decode_u32()?);
            }
            PhysicalStreamType::Length(LengthType::Symbol) => {
                symbol_lengths = Some(s.decode_bits_u32()?.decode_u32()?);
            }
            PhysicalStreamType::Data(DictionaryType::None) => {
                data_bytes = Some(raw_bytes(s));
            }
            PhysicalStreamType::Data(DictionaryType::Single) => {
                dict_bytes = Some(raw_bytes(s));
            }
            PhysicalStreamType::Data(DictionaryType::Fsst) => {
                symbol_bytes = Some(raw_bytes(s));
            }
            PhysicalStreamType::Offset(OffsetType::String) => {
                offsets = Some(s.decode_bits_u32()?.decode_u32()?);
            }
            other => {
                return Err(MltError::DecodeError(format!(
                    "Unexpected stream type in string property: {other:?}"
                )));
            }
        }
    }

    if let (Some(sym_lens), Some(sym_data), Some(dl), Some(dd), Some(offs)) = (
        &symbol_lengths,
        &symbol_bytes,
        &dict_lengths,
        &dict_bytes,
        &offsets,
    ) {
        // FSST dictionary
        decode_dict_strings(dl, &decode_fsst(sym_data, sym_lens, dd), offs)
    } else if let (Some(dl), Some(dd), Some(offs)) = (&dict_lengths, &dict_bytes, &offsets) {
        // Dictionary
        decode_dict_strings(dl, dd, offs)
    } else if let Some(lengths) = &var_binary_lengths {
        // Plain (VarBinary lengths + raw data)
        let data = data_bytes
            .as_deref()
            .or(dict_bytes.as_deref())
            .ok_or_else(|| MltError::DecodeError("Missing data stream for strings".into()))?;
        decode_plain_strings(lengths, data)
    } else {
        Err(MltError::DecodeError(
            "Missing required string streams".into(),
        ))
    }
}
fn raw_bytes(s: Stream<'_>) -> Vec<u8> {
    match s.data {
        StreamData::Raw(d) => d.data.to_vec(),
        StreamData::VarInt(d) => d.data.to_vec(),
    }
}

/// Split `data` into string slices using `lengths` as byte lengths for each entry.
fn split_strings_by_lengths<'a>(lengths: &[u32], data: &'a [u8]) -> Result<Vec<&'a str>, MltError> {
    let mut strings = Vec::with_capacity(lengths.len());
    let mut offset = 0;
    for &len in lengths {
        let len = len as usize;
        let Some(v) = data.get(offset..offset + len) else {
            return Err(MltError::BufferUnderflow {
                needed: len,
                remaining: data.len().saturating_sub(offset),
            });
        };
        strings.push(str::from_utf8(v)?);
        offset += len;
    }
    Ok(strings)
}

fn decode_plain_strings(lengths: &[u32], data: &[u8]) -> Result<Vec<String>, MltError> {
    split_strings_by_lengths(lengths, data).map(|s| s.into_iter().map(str::to_string).collect())
}

fn decode_dict_strings(
    dict_lengths: &[u32],
    dict_data: &[u8],
    offsets: &[u32],
) -> Result<Vec<String>, MltError> {
    let dict = split_strings_by_lengths(dict_lengths, dict_data)?;
    Ok(offsets
        .iter()
        .map(|&idx| dict[idx as usize].to_string())
        .collect())
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
