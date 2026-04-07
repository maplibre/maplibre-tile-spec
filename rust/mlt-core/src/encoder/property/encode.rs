use super::model::{PresenceKind, StagedOptScalar, StagedProperty};
use super::strings::write_str_col;
use crate::MltResult;
use crate::decoder::{ColumnType, DictionaryType, StreamType};
use crate::encoder::model::StreamCtx;
use crate::encoder::property::shared_dict::write_shared_dict;
use crate::encoder::stream::{
    write_i32_stream, write_i64_stream, write_u32_stream, write_u64_stream,
};
use crate::encoder::{EncodedStream, Encoder, StagedScalar, StagedStrings};
use crate::utils::BinarySerializer as _;

/// Encode all property columns and write them to `enc`.
#[hotpath::measure]
pub fn write_properties(props: &[StagedProperty], enc: &mut Encoder) -> MltResult<()> {
    for prop in props {
        write_prop(prop, enc)?;
    }
    Ok(())
}

/// Encode a single property column, dispatching on variant.
///
/// Returns `false` when the column is omitted (empty or all-null).
#[hotpath::measure]
fn write_prop(prop: &StagedProperty, enc: &mut Encoder) -> MltResult<bool> {
    use ColumnType as CT;
    use StagedProperty as D;

    match prop {
        D::Bool(v) if !v.values.is_empty() => {
            CT::Bool.write_to(&mut enc.meta)?;
            enc.meta.write_string(&v.name)?;
            enc.write_boolean_stream(&EncodedStream::encode_bools(&v.values)?)?;
        }
        D::OptBool(v) => {
            begin_opt_col(CT::OptBool, &v.name, &v.presence, enc)?;
            enc.write_boolean_stream(&EncodedStream::encode_bools(&v.values)?)?;
        }
        D::F32(v) if !v.values.is_empty() => {
            CT::F32.write_to(&mut enc.meta)?;
            enc.meta.write_string(&v.name)?;
            enc.write_stream(&EncodedStream::encode_f32(&v.values)?)?;
        }
        D::OptF32(v) => {
            begin_opt_col(CT::OptF32, &v.name, &v.presence, enc)?;
            enc.write_stream(&EncodedStream::encode_f32(&v.values)?)?;
        }
        D::F64(v) if !v.values.is_empty() => {
            CT::F64.write_to(&mut enc.meta)?;
            enc.meta.write_string(&v.name)?;
            enc.write_stream(&EncodedStream::encode_f64(&v.values)?)?;
        }
        D::OptF64(v) => {
            begin_opt_col(CT::OptF64, &v.name, &v.presence, enc)?;
            enc.write_stream(&EncodedStream::encode_f64(&v.values)?)?;
        }
        D::I8(v) if !v.values.is_empty() => {
            CT::I8.write_to(&mut enc.meta)?;
            enc.meta.write_string(&v.name)?;
            let widened: Vec<i32> = v.values.iter().map(|&x| i32::from(x)).collect();
            let ctx = StreamCtx::prop(StreamType::Data(DictionaryType::None), &v.name);
            write_i32_stream(&widened, &ctx, enc)?;
        }
        D::OptI8(v) => {
            begin_opt_col(CT::OptI8, &v.name, &v.presence, enc)?;
            let widened: Vec<i32> = v.values.iter().map(|&x| i32::from(x)).collect();
            let ctx = StreamCtx::prop(StreamType::Data(DictionaryType::None), &v.name);
            write_i32_stream(&widened, &ctx, enc)?;
        }
        D::U8(v) if !v.values.is_empty() => {
            CT::U8.write_to(&mut enc.meta)?;
            enc.meta.write_string(&v.name)?;
            let widened: Vec<u32> = v.values.iter().map(|&x| u32::from(x)).collect();
            let ctx = StreamCtx::prop(StreamType::Data(DictionaryType::None), &v.name);
            write_u32_stream(&widened, &ctx, enc)?;
        }
        D::OptU8(v) => {
            begin_opt_col(CT::OptU8, &v.name, &v.presence, enc)?;
            let widened: Vec<u32> = v.values.iter().map(|&x| u32::from(x)).collect();
            let ctx = StreamCtx::prop(StreamType::Data(DictionaryType::None), &v.name);
            write_u32_stream(&widened, &ctx, enc)?;
        }
        D::I32(v) if !v.values.is_empty() => {
            CT::I32.write_to(&mut enc.meta)?;
            enc.meta.write_string(&v.name)?;
            let ctx = StreamCtx::prop(StreamType::Data(DictionaryType::None), &v.name);
            write_i32_stream(&v.values, &ctx, enc)?;
        }
        D::OptI32(v) => {
            begin_opt_col(CT::OptI32, &v.name, &v.presence, enc)?;
            let ctx = StreamCtx::prop(StreamType::Data(DictionaryType::None), &v.name);
            write_i32_stream(&v.values, &ctx, enc)?;
        }
        D::U32(v) if !v.values.is_empty() => {
            CT::U32.write_to(&mut enc.meta)?;
            enc.meta.write_string(&v.name)?;
            let ctx = StreamCtx::prop(StreamType::Data(DictionaryType::None), &v.name);
            write_u32_stream(&v.values, &ctx, enc)?;
        }
        D::OptU32(v) => {
            begin_opt_col(CT::OptU32, &v.name, &v.presence, enc)?;
            let ctx = StreamCtx::prop(StreamType::Data(DictionaryType::None), &v.name);
            write_u32_stream(&v.values, &ctx, enc)?;
        }
        D::I64(v) if !v.values.is_empty() => {
            CT::I64.write_to(&mut enc.meta)?;
            enc.meta.write_string(&v.name)?;
            let ctx = StreamCtx::prop(StreamType::Data(DictionaryType::None), &v.name);
            write_i64_stream(&v.values, &ctx, enc)?;
        }
        D::OptI64(v) => {
            begin_opt_col(CT::OptI64, &v.name, &v.presence, enc)?;
            let ctx = StreamCtx::prop(StreamType::Data(DictionaryType::None), &v.name);
            write_i64_stream(&v.values, &ctx, enc)?;
        }
        D::U64(v) if !v.values.is_empty() => {
            CT::U64.write_to(&mut enc.meta)?;
            enc.meta.write_string(&v.name)?;
            let ctx = StreamCtx::prop(StreamType::Data(DictionaryType::None), &v.name);
            write_u64_stream(&v.values, &ctx, enc)?;
        }
        D::OptU64(v) => {
            begin_opt_col(CT::OptU64, &v.name, &v.presence, enc)?;
            let ctx = StreamCtx::prop(StreamType::Data(DictionaryType::None), &v.name);
            write_u64_stream(&v.values, &ctx, enc)?;
        }
        D::Str(v) => {
            if !write_str_prop(v, enc)? {
                return Ok(false);
            }
        }
        D::SharedDict(v) => return write_shared_dict(v, enc),
        _ => return Ok(false), // non-opt scalar with empty values
    }
    enc.increment_column_count();
    Ok(true)
}

/// Writes the column-type byte, name, and presence stream for an optional column.
///
/// Presence is always written: the invariant is that an optional-variant column always
/// has a presence stream. Ensuring the column is non-empty and not all-null is the
/// caller's responsibility.
fn begin_opt_col(
    ct: ColumnType,
    name: &str,
    presence_bools: &[bool],
    enc: &mut Encoder,
) -> MltResult<()> {
    ct.write_to(&mut enc.meta)?;
    enc.meta.write_string(name)?;
    enc.write_boolean_stream(&EncodedStream::encode_presence(presence_bools)?)?;
    Ok(())
}

/// Encodes a string column, including its column-type byte, name, presence stream, and data.
///
/// The column type is [`ColumnType::OptStr`] when a presence stream is written,
/// [`ColumnType::Str`] otherwise — strings encode nulls as negative lengths, so
/// the column type must be derived at runtime.
///
/// Returns `Ok(false)` when the column should be skipped (empty or all-null).
fn write_str_prop(v: &StagedStrings, enc: &mut Encoder) -> MltResult<bool> {
    let ctx = StreamCtx::prop(StreamType::Present, &v.name);
    let has_presence = match v.presence() {
        PresenceKind::Empty => return Ok(false),
        PresenceKind::AllNull => {
            if enc.override_presence(&ctx) {
                true
            } else {
                return Ok(false);
            }
        }
        PresenceKind::Mixed => true,
        PresenceKind::AllPresent => enc.override_presence(&ctx),
    };
    let presence = if has_presence {
        Some(EncodedStream::encode_presence(&v.presence_bools())?)
    } else {
        None
    };
    ColumnType::write_one_of(has_presence, ColumnType::OptStr, ColumnType::Str, &mut enc.meta)?;
    enc.meta.write_string(&v.name)?;
    write_str_col(v, presence.as_ref(), enc)?;
    Ok(true)
}


impl StagedProperty {
    // ── Non-optional constructors (Vec<T>) ────────────────────────────────────

    #[must_use]
    pub fn bool(name: impl Into<String>, values: Vec<bool>) -> Self {
        Self::Bool(StagedScalar {
            name: name.into(),
            values,
        })
    }
    #[must_use]
    pub fn i8(name: impl Into<String>, values: Vec<i8>) -> Self {
        Self::I8(StagedScalar {
            name: name.into(),
            values,
        })
    }
    #[must_use]
    pub fn u8(name: impl Into<String>, values: Vec<u8>) -> Self {
        Self::U8(StagedScalar {
            name: name.into(),
            values,
        })
    }
    #[must_use]
    pub fn i32(name: impl Into<String>, values: Vec<i32>) -> Self {
        Self::I32(StagedScalar {
            name: name.into(),
            values,
        })
    }
    #[must_use]
    pub fn u32(name: impl Into<String>, values: Vec<u32>) -> Self {
        Self::U32(StagedScalar {
            name: name.into(),
            values,
        })
    }
    #[must_use]
    pub fn i64(name: impl Into<String>, values: Vec<i64>) -> Self {
        Self::I64(StagedScalar {
            name: name.into(),
            values,
        })
    }
    #[must_use]
    pub fn u64(name: impl Into<String>, values: Vec<u64>) -> Self {
        Self::U64(StagedScalar {
            name: name.into(),
            values,
        })
    }
    #[must_use]
    pub fn f32(name: impl Into<String>, values: Vec<f32>) -> Self {
        Self::F32(StagedScalar {
            name: name.into(),
            values,
        })
    }
    #[must_use]
    pub fn f64(name: impl Into<String>, values: Vec<f64>) -> Self {
        Self::F64(StagedScalar {
            name: name.into(),
            values,
        })
    }
    #[must_use]
    pub fn str(
        name: impl Into<String>,
        values: impl IntoIterator<Item = Option<impl AsRef<str>>>,
    ) -> Self {
        Self::Str(StagedStrings::from_optional(name, values))
    }

    // ── Optional constructors (Vec<Option<T>>) ────────────────────────────────

    #[must_use]
    pub fn opt_bool(name: impl Into<String>, values: Vec<Option<bool>>) -> Self {
        Self::OptBool(StagedOptScalar::from_optional(name, values))
    }
    #[must_use]
    pub fn opt_i8(name: impl Into<String>, values: Vec<Option<i8>>) -> Self {
        Self::OptI8(StagedOptScalar::from_optional(name, values))
    }
    #[must_use]
    pub fn opt_u8(name: impl Into<String>, values: Vec<Option<u8>>) -> Self {
        Self::OptU8(StagedOptScalar::from_optional(name, values))
    }
    #[must_use]
    pub fn opt_i32(name: impl Into<String>, values: Vec<Option<i32>>) -> Self {
        Self::OptI32(StagedOptScalar::from_optional(name, values))
    }
    #[must_use]
    pub fn opt_u32(name: impl Into<String>, values: Vec<Option<u32>>) -> Self {
        Self::OptU32(StagedOptScalar::from_optional(name, values))
    }
    #[must_use]
    pub fn opt_i64(name: impl Into<String>, values: Vec<Option<i64>>) -> Self {
        Self::OptI64(StagedOptScalar::from_optional(name, values))
    }
    #[must_use]
    pub fn opt_u64(name: impl Into<String>, values: Vec<Option<u64>>) -> Self {
        Self::OptU64(StagedOptScalar::from_optional(name, values))
    }
    #[must_use]
    pub fn opt_f32(name: impl Into<String>, values: Vec<Option<f32>>) -> Self {
        Self::OptF32(StagedOptScalar::from_optional(name, values))
    }
    #[must_use]
    pub fn opt_f64(name: impl Into<String>, values: Vec<Option<f64>>) -> Self {
        Self::OptF64(StagedOptScalar::from_optional(name, values))
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
            Self::OptBool(v) => &v.name,
            Self::OptI8(v) => &v.name,
            Self::OptU8(v) => &v.name,
            Self::OptI32(v) => &v.name,
            Self::OptU32(v) => &v.name,
            Self::OptI64(v) => &v.name,
            Self::OptU64(v) => &v.name,
            Self::OptF32(v) => &v.name,
            Self::OptF64(v) => &v.name,
            Self::Str(v) => &v.name,
            Self::SharedDict(v) => &v.prefix,
        }
    }
}

impl<T: Copy + PartialEq> StagedOptScalar<T> {
    /// Build from optional values: dense non-null entries in `values`,
    /// per-feature flags in `presence`.
    #[must_use]
    pub fn from_optional(name: impl Into<String>, values: Vec<Option<T>>) -> Self {
        let presence: Vec<bool> = values.iter().map(Option::is_some).collect();
        let values: Vec<T> = values.into_iter().flatten().collect();
        Self {
            name: name.into(),
            values,
            presence,
        }
    }
}

