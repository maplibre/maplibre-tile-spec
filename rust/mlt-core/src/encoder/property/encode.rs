use super::model::{PresenceKind, StagedProperty};
use super::strings::{write_shared_dict, write_str_col};
use crate::MltError::NotImplemented;
use crate::MltResult;
use crate::codecs::zigzag::encode_zigzag;
use crate::decoder::ColumnType;
use crate::encoder::stream::DataProfile;
use crate::encoder::{EncodedStream, Encoder, StagedScalar, StagedStrings};
use crate::utils::BinarySerializer as _;

// ── Unified encoding path ────────────────────────────────────────────────────
//
// [`Encoder::get_int_encoder`] / [`Encoder::get_str_encoding`] return [`None`] → auto path:
// tries multiple IntEncoder candidates via `start_alternative`/`finish_alternatives`.
// [`Some`] → explicit path: single deterministic encoding; all-present presence is
// [`Encoder::explicit_override_presence`].

/// Encode all property columns and write them to `enc`.
///
/// Uses [`Encoder::explicit`] to choose between automatic and callback-driven encoding.
///
/// Each written column calls [`Encoder::push_layer_column`] at the end of its encode path;
/// skipped columns (all-null / empty) do not.
pub(crate) fn write_properties(props: &[StagedProperty], enc: &mut Encoder) -> MltResult<()> {
    for prop in props {
        write_prop(prop, enc)?;
    }
    Ok(())
}

/// Encode a single property column, dispatching on variant.
///
/// Returns `false` when the column is omitted (empty or all-null).
fn write_prop(prop: &StagedProperty, enc: &mut Encoder) -> MltResult<bool> {
    // SharedDict manages its own item-level presence check.
    if let StagedProperty::SharedDict(sd) = prop {
        return write_shared_dict(sd, enc);
    }

    let has_presence = match prop.presence() {
        PresenceKind::Empty | PresenceKind::AllNull => return Ok(false),
        PresenceKind::Mixed => true,
        // Explicit encoder may override presence for all-present columns.
        PresenceKind::AllPresent => enc.override_presence("prop", prop.name(), None),
    };
    let presence_stream = if has_presence {
        Some(EncodedStream::encode_presence(&prop.as_presence_stream()?)?)
    } else {
        None
    };

    match prop {
        StagedProperty::Str(v) => {
            write_str_col(v, has_presence, presence_stream.as_ref(), enc)?;
        }
        StagedProperty::SharedDict(_) => unreachable!("handled above"),
        _ => {
            write_scalar_prop(prop, has_presence, presence_stream.as_ref(), enc)?;
        }
    }
    Ok(true)
}

fn write_int_prop_header<T: Copy>(
    name: &str,
    non_null: &[T],
    col_type: ColumnType,
    has_presence: bool,
    presence_stream: Option<&EncodedStream>,
    enc: &mut Encoder,
) -> MltResult<()> {
    let _ = (non_null, has_presence); // used by callers for col_type selection only
    col_type.write_to(&mut enc.meta)?;
    enc.meta.write_string(name)?;
    enc.write_optional_stream(presence_stream)?;
    Ok(())
}

fn write_scalar_prop(
    prop: &StagedProperty,
    has_presence: bool,
    presence_stream: Option<&EncodedStream>,
    enc: &mut Encoder,
) -> MltResult<()> {
    use StagedProperty as D;
    match prop {
        D::Bool(v) => {
            let col_type = if has_presence {
                ColumnType::OptBool
            } else {
                ColumnType::Bool
            };
            col_type.write_to(&mut enc.meta)?;
            enc.meta.write_string(&v.name)?;
            enc.write_optional_stream(presence_stream)?;
            enc.write_boolean_stream(&EncodedStream::encode_bools(&unapply_presence(&v.values))?)?;
        }
        D::F32(v) => {
            let col_type = if has_presence {
                ColumnType::OptF32
            } else {
                ColumnType::F32
            };
            col_type.write_to(&mut enc.meta)?;
            enc.meta.write_string(&v.name)?;
            enc.write_optional_stream(presence_stream)?;
            enc.write_stream(&EncodedStream::encode_f32(&unapply_presence(&v.values))?)?;
        }
        D::F64(v) => {
            let col_type = if has_presence {
                ColumnType::OptF64
            } else {
                ColumnType::F64
            };
            col_type.write_to(&mut enc.meta)?;
            enc.meta.write_string(&v.name)?;
            enc.write_optional_stream(presence_stream)?;
            enc.write_stream(&EncodedStream::encode_f64(&unapply_presence(&v.values))?)?;
        }
        D::I8(v) => write_int_prop_i8(v, has_presence, presence_stream, enc)?,
        D::U8(v) => write_int_prop_u8(v, has_presence, presence_stream, enc)?,
        D::I32(v) => write_int_prop_i32(v, has_presence, presence_stream, enc)?,
        D::U32(v) => write_int_prop_u32(v, has_presence, presence_stream, enc)?,
        D::I64(v) => write_int_prop_i64(v, has_presence, presence_stream, enc)?,
        D::U64(v) => write_int_prop_u64(v, has_presence, presence_stream, enc)?,
        D::Str(..) | D::SharedDict(..) => {
            return Err(NotImplemented("use write_str_col / write_shared_dict"));
        }
    }
    enc.push_layer_column();
    Ok(())
}

fn write_int_prop_i8(
    v: &StagedScalar<i8>,
    has_presence: bool,
    presence_stream: Option<&EncodedStream>,
    enc: &mut Encoder,
) -> MltResult<()> {
    let col_type = if has_presence {
        ColumnType::OptI8
    } else {
        ColumnType::I8
    };
    let non_null: Vec<i8> = unapply_presence(&v.values);
    let widened: Vec<i32> = non_null.iter().map(|&x| i32::from(x)).collect();
    let zigzagged = encode_zigzag(&widened);
    let candidates = match enc.get_int_encoder("prop", &v.name, None) {
        Some(e) => vec![e],
        None => DataProfile::prune_candidates::<i32>(&zigzagged),
    };
    write_int_prop_header(
        &v.name,
        &non_null,
        col_type,
        has_presence,
        presence_stream,
        enc,
    )?;
    for &cand in &candidates {
        enc.start_alternative();
        enc.write_stream(&EncodedStream::encode_i8s(&non_null, cand)?)?;
    }
    enc.finish_alternatives();
    Ok(())
}

fn write_int_prop_u8(
    v: &StagedScalar<u8>,
    has_presence: bool,
    presence_stream: Option<&EncodedStream>,
    enc: &mut Encoder,
) -> MltResult<()> {
    let col_type = if has_presence {
        ColumnType::OptU8
    } else {
        ColumnType::U8
    };
    let non_null: Vec<u8> = unapply_presence(&v.values);
    let as_u32: Vec<u32> = non_null.iter().map(|&x| u32::from(x)).collect();
    let candidates = match enc.get_int_encoder("prop", &v.name, None) {
        Some(e) => vec![e],
        None => DataProfile::prune_candidates::<i32>(&as_u32),
    };
    write_int_prop_header(
        &v.name,
        &non_null,
        col_type,
        has_presence,
        presence_stream,
        enc,
    )?;
    for &cand in &candidates {
        enc.start_alternative();
        enc.write_stream(&EncodedStream::encode_u8s(&non_null, cand)?)?;
    }
    enc.finish_alternatives();
    Ok(())
}

fn write_int_prop_i32(
    v: &StagedScalar<i32>,
    has_presence: bool,
    presence_stream: Option<&EncodedStream>,
    enc: &mut Encoder,
) -> MltResult<()> {
    let col_type = if has_presence {
        ColumnType::OptI32
    } else {
        ColumnType::I32
    };
    let non_null: Vec<i32> = unapply_presence(&v.values);
    let as_u32 = encode_zigzag(&non_null);
    let candidates = match enc.get_int_encoder("prop", &v.name, None) {
        Some(e) => vec![e],
        None => DataProfile::prune_candidates::<i32>(&as_u32),
    };
    write_int_prop_header(
        &v.name,
        &non_null,
        col_type,
        has_presence,
        presence_stream,
        enc,
    )?;
    for &cand in &candidates {
        enc.start_alternative();
        enc.write_stream(&EncodedStream::encode_i32s(&non_null, cand)?)?;
    }
    enc.finish_alternatives();
    Ok(())
}

fn write_int_prop_u32(
    v: &StagedScalar<u32>,
    has_presence: bool,
    presence_stream: Option<&EncodedStream>,
    enc: &mut Encoder,
) -> MltResult<()> {
    let col_type = if has_presence {
        ColumnType::OptU32
    } else {
        ColumnType::U32
    };
    let non_null: Vec<u32> = unapply_presence(&v.values);
    let candidates = match enc.get_int_encoder("prop", &v.name, None) {
        Some(e) => vec![e],
        None => DataProfile::prune_candidates::<i32>(&non_null),
    };
    write_int_prop_header(
        &v.name,
        &non_null,
        col_type,
        has_presence,
        presence_stream,
        enc,
    )?;
    for &cand in &candidates {
        enc.start_alternative();
        enc.write_stream(&EncodedStream::encode_u32s(&non_null, cand)?)?;
    }
    enc.finish_alternatives();
    Ok(())
}

fn write_int_prop_i64(
    v: &StagedScalar<i64>,
    has_presence: bool,
    presence_stream: Option<&EncodedStream>,
    enc: &mut Encoder,
) -> MltResult<()> {
    let col_type = if has_presence {
        ColumnType::OptI64
    } else {
        ColumnType::I64
    };
    let non_null: Vec<i64> = unapply_presence(&v.values);
    let as_u64: Vec<u64> = encode_zigzag(&non_null);
    let candidates = match enc.get_int_encoder("prop", &v.name, None) {
        Some(e) => vec![e],
        None => DataProfile::prune_candidates::<i64>(&as_u64),
    };
    write_int_prop_header(
        &v.name,
        &non_null,
        col_type,
        has_presence,
        presence_stream,
        enc,
    )?;
    for &cand in &candidates {
        enc.start_alternative();
        enc.write_stream(&EncodedStream::encode_i64s(&non_null, cand)?)?;
    }
    enc.finish_alternatives();
    Ok(())
}

fn write_int_prop_u64(
    v: &StagedScalar<u64>,
    has_presence: bool,
    presence_stream: Option<&EncodedStream>,
    enc: &mut Encoder,
) -> MltResult<()> {
    let col_type = if has_presence {
        ColumnType::OptU64
    } else {
        ColumnType::U64
    };
    let non_null: Vec<u64> = unapply_presence(&v.values);
    let candidates = match enc.get_int_encoder("prop", &v.name, None) {
        Some(e) => vec![e],
        None => DataProfile::prune_candidates::<i64>(&non_null),
    };
    write_int_prop_header(
        &v.name,
        &non_null,
        col_type,
        has_presence,
        presence_stream,
        enc,
    )?;
    for &cand in &candidates {
        enc.start_alternative();
        enc.write_stream(&EncodedStream::encode_u64s(&non_null, cand)?)?;
    }
    enc.finish_alternatives();
    Ok(())
}

pub(crate) fn unapply_presence<T: Clone>(v: &[Option<T>]) -> Vec<T> {
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
