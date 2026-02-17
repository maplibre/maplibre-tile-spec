use std::fmt::{self, Debug};

use borrowme::borrowme;

use crate::MltError;
use crate::analyse::{Analyze, StatType};
use crate::decodable::{FromRaw, impl_decodable};
use crate::utils::f32_to_json;
use crate::v01::{DictionaryType, LengthType, OffsetType, PhysicalStreamType, Stream, StreamData};

/// Property representation, either raw or decoded
#[borrowme]
#[derive(Debug, PartialEq)]
pub enum Property<'a> {
    Raw(RawProperty<'a>),
    Decoded(DecodedProperty),
}

impl Analyze for Property<'_> {
    fn decoded(&self, stat: StatType) -> usize {
        match self {
            Self::Raw(d) => d.decoded(stat),
            Self::Decoded(d) => d.decoded(stat),
        }
    }

    fn for_each_stream(&self, cb: &mut dyn FnMut(&Stream<'_>)) {
        match self {
            Self::Raw(d) => d.for_each_stream(cb),
            Self::Decoded(d) => d.for_each_stream(cb),
        }
    }
}

/// Unparsed property data as read directly from the tile
#[borrowme]
#[derive(Debug, PartialEq)]
pub struct RawProperty<'a> {
    name: &'a str,
    optional: Option<Stream<'a>>,
    value: RawPropValue<'a>,
}

impl Analyze for RawProperty<'_> {
    fn for_each_stream(&self, cb: &mut dyn FnMut(&Stream<'_>)) {
        self.optional.for_each_stream(cb);
        self.value.for_each_stream(cb);
    }
}

/// A sequence of encoded (raw) property values of various types
#[borrowme]
#[derive(Debug, PartialEq)]
pub enum RawPropValue<'a> {
    Bool(Stream<'a>),
    I8(Stream<'a>),
    U8(Stream<'a>),
    I32(Stream<'a>),
    U32(Stream<'a>),
    I64(Stream<'a>),
    U64(Stream<'a>),
    F32(Stream<'a>),
    F64(Stream<'a>),
    Str(Vec<Stream<'a>>),
    Struct(Stream<'a>),
}

impl Analyze for RawPropValue<'_> {
    fn for_each_stream(&self, cb: &mut dyn FnMut(&Stream<'_>)) {
        match self {
            Self::Bool(s)
            | Self::I8(s)
            | Self::U8(s)
            | Self::I32(s)
            | Self::U32(s)
            | Self::I64(s)
            | Self::U64(s)
            | Self::F32(s)
            | Self::F64(s)
            | Self::Struct(s) => s.for_each_stream(cb),
            Self::Str(streams) => streams.for_each_stream(cb),
        }
    }
}

/// Decoded property values as a name and a vector of optional typed values
#[derive(Debug, Clone, Default, PartialEq)]
pub struct DecodedProperty {
    pub name: String,
    pub values: PropValue,
}

impl Analyze for DecodedProperty {
    fn decoded(&self, stat: StatType) -> usize {
        let meta = if stat == StatType::MetadataOverheadBytes {
            self.name.len()
        } else {
            0
        };
        meta + self.values.decoded(stat)
    }
}

/// Decoded property value types
#[derive(Clone, Default, PartialEq)]
pub enum PropValue {
    Bool(Vec<Option<bool>>),
    I8(Vec<Option<i8>>),
    U8(Vec<Option<u8>>),
    I32(Vec<Option<i32>>),
    U32(Vec<Option<u32>>),
    I64(Vec<Option<i64>>),
    U64(Vec<Option<u64>>),
    F32(Vec<Option<f32>>),
    F64(Vec<Option<f64>>),
    Str(Vec<Option<String>>),
    #[default]
    Struct,
}

impl Analyze for PropValue {
    fn decoded(&self, stat: StatType) -> usize {
        match self {
            Self::Bool(v) => v.decoded(stat),
            Self::I8(v) => v.decoded(stat),
            Self::U8(v) => v.decoded(stat),
            Self::I32(v) => v.decoded(stat),
            Self::U32(v) => v.decoded(stat),
            Self::I64(v) => v.decoded(stat),
            Self::U64(v) => v.decoded(stat),
            Self::F32(v) => v.decoded(stat),
            Self::F64(v) => v.decoded(stat),
            Self::Str(v) => v.decoded(stat),
            Self::Struct => 0,
        }
    }
}

/// Format `Option` values on a single line each, even in alternate/pretty mode.
struct FmtOptVec<'a, T>(&'a [Option<T>]);

impl<T: Debug> Debug for FmtOptVec<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut list = f.debug_list();
        for item in self.0 {
            // Always format each element in compact (non-alternate) mode
            list.entry(&format_args!("{item:?}"));
        }
        list.finish()
    }
}

impl Debug for PropValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Bool(v) => f.debug_tuple("Bool").field(&FmtOptVec(v)).finish(),
            Self::I8(v) => f.debug_tuple("I8").field(&FmtOptVec(v)).finish(),
            Self::U8(v) => f.debug_tuple("U8").field(&FmtOptVec(v)).finish(),
            Self::I32(v) => f.debug_tuple("I32").field(&FmtOptVec(v)).finish(),
            Self::U32(v) => f.debug_tuple("U32").field(&FmtOptVec(v)).finish(),
            Self::I64(v) => f.debug_tuple("I64").field(&FmtOptVec(v)).finish(),
            Self::U64(v) => f.debug_tuple("U64").field(&FmtOptVec(v)).finish(),
            Self::F32(v) => f.debug_tuple("F32").field(&FmtOptVec(v)).finish(),
            Self::F64(v) => f.debug_tuple("F64").field(&FmtOptVec(v)).finish(),
            Self::Str(v) => f.debug_tuple("Str").field(&FmtOptVec(v)).finish(),
            Self::Struct => write!(f, "Struct"),
        }
    }
}

impl PropValue {
    /// Convert the value at index `i` to a [`serde_json::Value`]
    #[must_use]
    #[expect(clippy::cast_possible_truncation)] // f64 stored as f32 in wire format
    pub fn to_geojson(&self, i: usize) -> Option<serde_json::Value> {
        use serde_json::Value;

        match self {
            Self::Bool(v) => v[i].map(Value::Bool),
            Self::I8(v) => v[i].map(Value::from),
            Self::U8(v) => v[i].map(Value::from),
            Self::I32(v) => v[i].map(Value::from),
            Self::U32(v) => v[i].map(Value::from),
            Self::I64(v) => v[i].map(Value::from),
            Self::U64(v) => v[i].map(Value::from),
            Self::F32(v) => v[i].map(f32_to_json),
            Self::F64(v) => v[i].map(|f| f32_to_json(f as f32)),
            Self::Str(v) => v[i].as_ref().map(|s| Value::String(s.clone())),
            Self::Struct => None,
        }
    }
}

impl_decodable!(Property<'a>, RawProperty<'a>, DecodedProperty);

impl<'a> From<RawProperty<'a>> for Property<'a> {
    fn from(value: RawProperty<'a>) -> Self {
        Self::Raw(value)
    }
}

impl<'a> Property<'a> {
    #[must_use]
    pub fn raw(name: &'a str, optional: Option<Stream<'a>>, value: RawPropValue<'a>) -> Self {
        Self::Raw(RawProperty {
            name,
            optional,
            value,
        })
    }

    #[inline]
    pub fn decode(self) -> Result<DecodedProperty, MltError> {
        Ok(match self {
            Self::Raw(v) => DecodedProperty::from_raw(v)?,
            Self::Decoded(v) => v,
        })
    }
}

impl<'a> FromRaw<'a> for DecodedProperty {
    type Input = RawProperty<'a>;

    fn from_raw(v: RawProperty<'_>) -> Result<Self, MltError> {
        let present = v.optional.map(Stream::decode_bools);
        let values = match v.value {
            RawPropValue::Bool(s) => {
                PropValue::Bool(apply_present(present.as_ref(), s.decode_bools()))
            }
            RawPropValue::I8(s) => PropValue::I8(apply_present(
                present.as_ref(),
                s.decode_signed_int_stream()?,
            )),
            RawPropValue::U8(s) => PropValue::U8(apply_present(
                present.as_ref(),
                s.decode_unsigned_int_stream()?,
            )),
            RawPropValue::I32(s) => PropValue::I32(apply_present(
                present.as_ref(),
                s.decode_signed_int_stream()?,
            )),
            RawPropValue::U32(s) => PropValue::U32(apply_present(
                present.as_ref(),
                s.decode_unsigned_int_stream()?,
            )),
            RawPropValue::I64(s) => {
                PropValue::I64(apply_present(present.as_ref(), s.decode_i64()?))
            }
            RawPropValue::U64(s) => {
                PropValue::U64(apply_present(present.as_ref(), s.decode_u64()?))
            }
            RawPropValue::F32(s) => {
                PropValue::F32(apply_present(present.as_ref(), s.decode_f32s()))
            }
            RawPropValue::F64(s) => PropValue::F64(apply_present(
                present.as_ref(),
                s.decode_f32s().into_iter().map(f64::from).collect(),
            )),
            RawPropValue::Str(streams) => PropValue::Str(apply_present(
                present.as_ref(),
                decode_string_streams(streams)?,
            )),
            RawPropValue::Struct(_) => PropValue::Struct,
        };
        Ok(DecodedProperty {
            name: v.name.to_string(),
            values,
        })
    }
}

/// Apply an optional present bitmap to a vector of values.
/// If present is None (non-optional column), all values are wrapped in Some.
/// If present is Some, values are interleaved with None according to the bitmap.
fn apply_present<T>(present: Option<&Vec<bool>>, values: Vec<T>) -> Vec<Option<T>> {
    let Some(present) = present else {
        return values.into_iter().map(Some).collect();
    };
    let mut result = Vec::with_capacity(present.len());
    let mut val_iter = values.into_iter();
    for &p in present {
        result.push(if p { val_iter.next() } else { None });
    }
    result
}

/// Decode string property from its sub-streams
fn decode_string_streams(streams: Vec<Stream<'_>>) -> Result<Vec<String>, MltError> {
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
        let decompressed = decode_fsst(sym_data, sym_lens, dd);
        Ok(decode_dict_strings(dl, &decompressed, offs))
    } else if let (Some(dl), Some(dd), Some(offs)) = (&dict_lengths, &dict_bytes, &offsets) {
        // Dictionary
        Ok(decode_dict_strings(dl, dd, offs))
    } else if let Some(lengths) = &var_binary_lengths {
        // Plain (VarBinary lengths + raw data)
        let data = data_bytes
            .as_deref()
            .or(dict_bytes.as_deref())
            .ok_or_else(|| MltError::DecodeError("Missing data stream for strings".into()))?;
        Ok(decode_plain_strings(lengths, data))
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

fn decode_plain_strings(lengths: &[u32], data: &[u8]) -> Vec<String> {
    let mut strings = Vec::with_capacity(lengths.len());
    let mut offset = 0;
    for &len in lengths {
        let len = len as usize;
        strings.push(String::from_utf8_lossy(&data[offset..offset + len]).into_owned());
        offset += len;
    }
    strings
}

fn decode_dict_strings(dict_lengths: &[u32], dict_data: &[u8], offsets: &[u32]) -> Vec<String> {
    // Build dictionary entries
    let mut dict = Vec::with_capacity(dict_lengths.len());
    let mut offset = 0;
    for &len in dict_lengths {
        let len = len as usize;
        dict.push(String::from_utf8_lossy(&dict_data[offset..offset + len]).into_owned());
        offset += len;
    }
    // Look up values
    offsets
        .iter()
        .map(|&idx| dict[idx as usize].clone())
        .collect()
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
