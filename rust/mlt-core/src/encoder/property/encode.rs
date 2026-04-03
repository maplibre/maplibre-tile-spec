use integer_encoding::VarIntWriter as _;

use super::model::{
    PresenceKind, PropertyEncoder, ScalarEncoder, ScalarValueEncoder, StagedProperty, StrEncoder,
};
use super::strings::write_shared_dict_prop_to;
use crate::MltError::{
    EncodingInstructionCountMismatch, NotImplemented, UnsupportedPropertyEncoderCombination,
};
use crate::MltResult;
use crate::decoder::{ColumnType, DictionaryType, LengthType};
use crate::encoder::{EncodedStream, Encoder, StagedScalar, StagedStrings};
use crate::utils::BinarySerializer as _;

/// Encode and write a set of properties using explicit per-column encoders.
///
/// Returns the count of columns actually written (all-null/empty columns are skipped).
pub fn write_properties_to(
    props: &[StagedProperty],
    encoders: Vec<PropertyEncoder>,
    enc: &mut Encoder,
) -> MltResult<u32> {
    if props.len() != encoders.len() {
        return Err(EncodingInstructionCountMismatch {
            input_len: props.len(),
            config_len: encoders.len(),
        });
    }

    let mut count = 0u32;
    for (prop, encoder) in props.iter().zip(encoders) {
        let written = match encoder {
            PropertyEncoder::Scalar(enc_cfg) => write_scalar_prop_to(prop, enc_cfg, enc)?,
            PropertyEncoder::SharedDict(enc_cfg) => {
                let StagedProperty::SharedDict(shared_dict) = prop else {
                    return Err(UnsupportedPropertyEncoderCombination(
                        prop.into(),
                        "shared_dict",
                    ));
                };
                write_shared_dict_prop_to(shared_dict, &enc_cfg, enc)?
            }
        };
        if written {
            count += 1;
        }
    }
    Ok(count)
}

/// Encode a single scalar property and write it directly to `enc`.
///
/// Returns `false` when the column is omitted (empty or all-null).
pub(crate) fn write_scalar_prop_to(
    value: &StagedProperty,
    encoder: ScalarEncoder,
    enc: &mut Encoder,
) -> MltResult<bool> {
    use PresenceKind as Kind;
    use StagedProperty as D;

    let kind = value.presence();

    #[cfg(feature = "__private")]
    let kind = if encoder.forced_presence {
        Kind::Mixed
    } else {
        kind
    };

    match kind {
        Kind::Empty | Kind::AllNull => return Ok(false),
        Kind::Mixed | Kind::AllPresent => {}
    }

    let has_presence = matches!(kind, Kind::Mixed);
    let presence_stream = if has_presence {
        Some(EncodedStream::encode_presence(
            &value.as_presence_stream()?,
        )?)
    } else {
        None
    };

    // Write column type byte + name to enc.meta, then data streams to enc.data.
    match (value, encoder.value) {
        (D::Bool(v), ScalarValueEncoder::Bool) => {
            let col_type = if has_presence {
                ColumnType::OptBool
            } else {
                ColumnType::Bool
            };
            col_type.write_to(&mut enc.meta)?;
            enc.meta.write_string(&v.name)?;
            enc.write_optional_stream(presence_stream.as_ref())?;
            let data = unapply_presence(&v.values);
            enc.write_boolean_stream(&EncodedStream::encode_bools(&data)?)?;
        }
        (D::I8(v), ScalarValueEncoder::Int(ie)) => {
            let col_type = if has_presence {
                ColumnType::OptI8
            } else {
                ColumnType::I8
            };
            col_type.write_to(&mut enc.meta)?;
            enc.meta.write_string(&v.name)?;
            enc.write_optional_stream(presence_stream.as_ref())?;
            enc.write_stream(&EncodedStream::encode_i8s(
                &unapply_presence(&v.values),
                ie,
            )?)?;
        }
        (D::U8(v), ScalarValueEncoder::Int(ie)) => {
            let col_type = if has_presence {
                ColumnType::OptU8
            } else {
                ColumnType::U8
            };
            col_type.write_to(&mut enc.meta)?;
            enc.meta.write_string(&v.name)?;
            enc.write_optional_stream(presence_stream.as_ref())?;
            enc.write_stream(&EncodedStream::encode_u8s(
                &unapply_presence(&v.values),
                ie,
            )?)?;
        }
        (D::I32(v), ScalarValueEncoder::Int(ie)) => {
            let col_type = if has_presence {
                ColumnType::OptI32
            } else {
                ColumnType::I32
            };
            col_type.write_to(&mut enc.meta)?;
            enc.meta.write_string(&v.name)?;
            enc.write_optional_stream(presence_stream.as_ref())?;
            enc.write_stream(&EncodedStream::encode_i32s(
                &unapply_presence(&v.values),
                ie,
            )?)?;
        }
        (D::U32(v), ScalarValueEncoder::Int(ie)) => {
            let col_type = if has_presence {
                ColumnType::OptU32
            } else {
                ColumnType::U32
            };
            col_type.write_to(&mut enc.meta)?;
            enc.meta.write_string(&v.name)?;
            enc.write_optional_stream(presence_stream.as_ref())?;
            enc.write_stream(&EncodedStream::encode_u32s(
                &unapply_presence(&v.values),
                ie,
            )?)?;
        }
        (D::I64(v), ScalarValueEncoder::Int(ie)) => {
            let col_type = if has_presence {
                ColumnType::OptI64
            } else {
                ColumnType::I64
            };
            col_type.write_to(&mut enc.meta)?;
            enc.meta.write_string(&v.name)?;
            enc.write_optional_stream(presence_stream.as_ref())?;
            enc.write_stream(&EncodedStream::encode_i64s(
                &unapply_presence(&v.values),
                ie,
            )?)?;
        }
        (D::U64(v), ScalarValueEncoder::Int(ie)) => {
            let col_type = if has_presence {
                ColumnType::OptU64
            } else {
                ColumnType::U64
            };
            col_type.write_to(&mut enc.meta)?;
            enc.meta.write_string(&v.name)?;
            enc.write_optional_stream(presence_stream.as_ref())?;
            enc.write_stream(&EncodedStream::encode_u64s(
                &unapply_presence(&v.values),
                ie,
            )?)?;
        }
        (D::F32(v), ScalarValueEncoder::Float) => {
            let col_type = if has_presence {
                ColumnType::OptF32
            } else {
                ColumnType::F32
            };
            col_type.write_to(&mut enc.meta)?;
            enc.meta.write_string(&v.name)?;
            enc.write_optional_stream(presence_stream.as_ref())?;
            enc.write_stream(&EncodedStream::encode_f32(&unapply_presence(&v.values))?)?;
        }
        (D::F64(v), ScalarValueEncoder::Float) => {
            let col_type = if has_presence {
                ColumnType::OptF64
            } else {
                ColumnType::F64
            };
            col_type.write_to(&mut enc.meta)?;
            enc.meta.write_string(&v.name)?;
            enc.write_optional_stream(presence_stream.as_ref())?;
            enc.write_stream(&EncodedStream::encode_f64(&unapply_presence(&v.values))?)?;
        }
        (D::Str(v), ScalarValueEncoder::String(str_enc)) => {
            write_str_col_to(v, str_enc, has_presence, presence_stream.as_ref(), enc)?;
        }
        (D::SharedDict(..), _) => {
            return Err(NotImplemented(
                "SharedDict cannot be encoded via ScalarEncoder",
            ));
        }
        (v, e) => return Err(UnsupportedPropertyEncoderCombination(v.into(), e.into())),
    }

    Ok(true)
}

/// Write a string-typed column (meta + data) to enc.
fn write_str_col_to(
    v: &StagedStrings,
    str_enc: StrEncoder,
    has_presence: bool,
    presence_stream: Option<&EncodedStream>,
    enc: &mut Encoder,
) -> MltResult<()> {
    let col_type = if has_presence {
        ColumnType::OptStr
    } else {
        ColumnType::Str
    };
    col_type.write_to(&mut enc.meta)?;
    enc.meta.write_string(&v.name)?;

    let dense_values = v.dense_values();
    let encoding = match str_enc {
        StrEncoder::Plain { string_lengths } => EncodedStream::encode_strings_with_type(
            &dense_values,
            string_lengths,
            LengthType::VarBinary,
            DictionaryType::None,
        )?,
        StrEncoder::Dict {
            string_lengths,
            offsets,
        } => EncodedStream::encode_strings_dict(&dense_values, string_lengths, offsets)?,
        StrEncoder::Fsst(fsst_enc) => EncodedStream::encode_strings_fsst_with_type(
            &dense_values,
            fsst_enc,
            DictionaryType::Single,
        )?,
        StrEncoder::FsstDict { fsst, offsets } => {
            EncodedStream::encode_strings_fsst_dict(&dense_values, fsst, offsets)?
        }
    };

    let content_streams = encoding.streams();
    let stream_count =
        u32::try_from(content_streams.len() + usize::from(presence_stream.is_some()))?;
    enc.write_varint(stream_count)?;
    enc.write_optional_stream(presence_stream)?;
    for stream in content_streams {
        enc.write_stream(stream)?;
    }
    Ok(())
}

fn unapply_presence<T: Clone>(v: &[Option<T>]) -> Vec<T> {
    v.iter().filter_map(|x| x.as_ref()).cloned().collect()
}

impl StagedProperty {
    pub(crate) fn as_presence_stream(&self) -> MltResult<Vec<bool>> {
        Ok(match self {
            Self::Bool(v) => v.values.iter().map(Option::is_some).collect(),
            Self::I8(v) => v.values.iter().map(Option::is_some).collect(),
            Self::U8(v) => v.values.iter().map(Option::is_some).collect(),
            Self::I32(v) => v.values.iter().map(Option::is_some).collect(),
            Self::U32(v) => v.values.iter().map(Option::is_some).collect(),
            Self::I64(v) => v.values.iter().map(Option::is_some).collect(),
            Self::U64(v) => v.values.iter().map(Option::is_some).collect(),
            Self::F32(v) => v.values.iter().map(Option::is_some).collect(),
            Self::F64(v) => v.values.iter().map(Option::is_some).collect(),
            Self::Str(v) => v.presence_bools(),
            Self::SharedDict(..) => {
                return Err(NotImplemented("presence stream for shared dict"));
            }
        })
    }

    #[must_use]
    pub fn bool(name: impl Into<String>, values: Vec<Option<bool>>) -> Self {
        Self::Bool(StagedScalar {
            name: name.into(),
            values,
        })
    }
    #[must_use]
    pub fn i8(name: impl Into<String>, values: Vec<Option<i8>>) -> Self {
        Self::I8(StagedScalar {
            name: name.into(),
            values,
        })
    }
    #[must_use]
    pub fn u8(name: impl Into<String>, values: Vec<Option<u8>>) -> Self {
        Self::U8(StagedScalar {
            name: name.into(),
            values,
        })
    }
    #[must_use]
    pub fn i32(name: impl Into<String>, values: Vec<Option<i32>>) -> Self {
        Self::I32(StagedScalar {
            name: name.into(),
            values,
        })
    }
    #[must_use]
    pub fn u32(name: impl Into<String>, values: Vec<Option<u32>>) -> Self {
        Self::U32(StagedScalar {
            name: name.into(),
            values,
        })
    }
    #[must_use]
    pub fn i64(name: impl Into<String>, values: Vec<Option<i64>>) -> Self {
        Self::I64(StagedScalar {
            name: name.into(),
            values,
        })
    }
    #[must_use]
    pub fn u64(name: impl Into<String>, values: Vec<Option<u64>>) -> Self {
        Self::U64(StagedScalar {
            name: name.into(),
            values,
        })
    }
    #[must_use]
    pub fn f32(name: impl Into<String>, values: Vec<Option<f32>>) -> Self {
        Self::F32(StagedScalar {
            name: name.into(),
            values,
        })
    }
    #[must_use]
    pub fn f64(name: impl Into<String>, values: Vec<Option<f64>>) -> Self {
        Self::F64(StagedScalar {
            name: name.into(),
            values,
        })
    }
    #[must_use]
    pub fn str(name: impl Into<String>, values: Vec<Option<String>>) -> Self {
        Self::Str(StagedStrings::from_optional(name, values))
    }

    #[must_use]
    pub fn presence(&self) -> PresenceKind {
        match self {
            Self::Bool(v) => presence_of_options(&v.values),
            Self::I8(v) => presence_of_options(&v.values),
            Self::U8(v) => presence_of_options(&v.values),
            Self::I32(v) => presence_of_options(&v.values),
            Self::U32(v) => presence_of_options(&v.values),
            Self::I64(v) => presence_of_options(&v.values),
            Self::U64(v) => presence_of_options(&v.values),
            Self::F32(v) => presence_of_options(&v.values),
            Self::F64(v) => presence_of_options(&v.values),
            Self::Str(v) => v.presence(),
            Self::SharedDict(..) => PresenceKind::Empty,
        }
    }

    /// Returns the column name regardless of variant.
    #[must_use]
    pub fn name(&self) -> &str {
        match self {
            Self::Bool(v) => &v.name,
            Self::I8(v) => &v.name,
            Self::U8(v) => &v.name,
            Self::I32(v) => &v.name,
            Self::U32(v) => &v.name,
            Self::I64(v) => &v.name,
            Self::U64(v) => &v.name,
            Self::F32(v) => &v.name,
            Self::F64(v) => &v.name,
            Self::Str(v) => &v.name,
            Self::SharedDict(v) => &v.prefix,
        }
    }
}

fn presence_of_options<T>(values: &[Option<T>]) -> PresenceKind {
    let mut has_null = false;
    let mut has_present = false;
    for v in values {
        if v.is_none() {
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
