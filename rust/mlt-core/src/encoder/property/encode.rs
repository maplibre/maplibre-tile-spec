use super::model::{StagedOptScalar, StagedProperty};
use super::strings::write_str_col;
use crate::MltResult;
use crate::decoder::{ColumnType, DictionaryType, StreamType};
use crate::encoder::model::StreamCtx;
use crate::encoder::property::shared_dict::write_shared_dict;
use crate::encoder::stream::write::{write_alternatives, write_i64_stream_as};
use crate::encoder::stream::{
    write_bool_stream, write_i32_stream, write_u32_stream, write_u64_stream,
};
use crate::encoder::{Codecs, DataProfile, EncodedStream, Encoder, StagedScalar, StagedStrings};
use crate::utils::BinarySerializer as _;

/// Encode all property columns and write them to `enc`.
#[hotpath::measure]
pub fn write_properties(
    props: &[StagedProperty],
    enc: &mut Encoder,
    codecs: &mut Codecs,
) -> MltResult<()> {
    for prop in props {
        write_prop(prop, enc, codecs)?;
    }
    Ok(())
}

/// Encode a single property column, dispatching on variant.
///
/// Returns `false` when the column is omitted (empty or all-null).
#[hotpath::measure]
fn write_prop(prop: &StagedProperty, enc: &mut Encoder, codecs: &mut Codecs) -> MltResult<bool> {
    use ColumnType as CT;
    use StagedProperty as D;

    match prop {
        D::Bool(v) => {
            enc.write_column_header(CT::Bool, &v.name)?;
            let values = v.values.iter().copied();
            write_bool_stream(values, StreamType::Data(DictionaryType::None), enc, codecs)?;
        }
        D::OptBool(v) => {
            begin_opt_col(
                CT::OptBool,
                &v.name,
                v.presence.iter().copied(),
                enc,
                codecs,
            )?;
            let values = v.values.iter().copied();
            write_bool_stream(values, StreamType::Data(DictionaryType::None), enc, codecs)?;
        }
        D::F32(v) => {
            enc.write_column_header(CT::F32, &v.name)?;
            enc.write_stream(&EncodedStream::encode_f32(&v.values)?)?;
        }
        D::OptF32(v) => {
            begin_opt_col(CT::OptF32, &v.name, v.presence.iter().copied(), enc, codecs)?;
            enc.write_stream(&EncodedStream::encode_f32(&v.values)?)?;
        }
        D::F64(v) => {
            enc.write_column_header(CT::F64, &v.name)?;
            enc.write_stream(&EncodedStream::encode_f64(&v.values)?)?;
        }
        D::OptF64(v) => {
            begin_opt_col(CT::OptF64, &v.name, v.presence.iter().copied(), enc, codecs)?;
            enc.write_stream(&EncodedStream::encode_f64(&v.values)?)?;
        }
        D::I8(v) => {
            enc.write_column_header(CT::I8, &v.name)?;
            let widened: Vec<i32> = v.values.iter().map(|&x| i32::from(x)).collect();
            let ctx = StreamCtx::prop(StreamType::Data(DictionaryType::None), &v.name);
            write_i32_stream(&widened, &ctx, enc, codecs)?;
        }
        D::OptI8(v) => {
            begin_opt_col(CT::OptI8, &v.name, v.presence.iter().copied(), enc, codecs)?;
            let widened: Vec<i32> = v.values.iter().map(|&x| i32::from(x)).collect();
            let ctx = StreamCtx::prop(StreamType::Data(DictionaryType::None), &v.name);
            write_i32_stream(&widened, &ctx, enc, codecs)?;
        }
        D::U8(v) => {
            enc.write_column_header(CT::U8, &v.name)?;
            let widened: Vec<u32> = v.values.iter().map(|&x| u32::from(x)).collect();
            let ctx = StreamCtx::prop(StreamType::Data(DictionaryType::None), &v.name);
            write_u32_stream(&widened, &ctx, enc, codecs)?;
        }
        D::OptU8(v) => {
            begin_opt_col(CT::OptU8, &v.name, v.presence.iter().copied(), enc, codecs)?;
            let widened: Vec<u32> = v.values.iter().map(|&x| u32::from(x)).collect();
            let ctx = StreamCtx::prop(StreamType::Data(DictionaryType::None), &v.name);
            write_u32_stream(&widened, &ctx, enc, codecs)?;
        }
        D::I32(v) => {
            enc.write_column_header(CT::I32, &v.name)?;
            let ctx = StreamCtx::prop(StreamType::Data(DictionaryType::None), &v.name);
            write_i32_stream(&v.values, &ctx, enc, codecs)?;
        }
        D::OptI32(v) => {
            begin_opt_col(CT::OptI32, &v.name, v.presence.iter().copied(), enc, codecs)?;
            let ctx = StreamCtx::prop(StreamType::Data(DictionaryType::None), &v.name);
            write_i32_stream(&v.values, &ctx, enc, codecs)?;
        }
        D::U32(v) => write_u32_scalar_col(CT::U32, Some(&v.name), v, enc, codecs)?,
        D::OptU32(v) => write_opt_u32_scalar_col(CT::OptU32, Some(&v.name), v, enc, codecs)?,
        D::I64(v) => {
            enc.write_column_header(CT::I64, &v.name)?;
            let ctx = StreamCtx::prop(StreamType::Data(DictionaryType::None), &v.name);
            write_i64_stream(&v.values, &ctx, enc, codecs)?;
        }
        D::OptI64(v) => {
            begin_opt_col(CT::OptI64, &v.name, v.presence.iter().copied(), enc, codecs)?;
            let ctx = StreamCtx::prop(StreamType::Data(DictionaryType::None), &v.name);
            write_i64_stream(&v.values, &ctx, enc, codecs)?;
        }
        D::U64(v) => write_u64_scalar_col(CT::U64, Some(&v.name), v, enc, codecs)?,
        D::OptU64(v) => write_opt_u64_scalar_col(CT::OptU64, Some(&v.name), v, enc, codecs)?,
        D::Str(v) => {
            enc.write_column_header(ColumnType::Str, &v.name)?;
            write_str_col(v, None, enc, codecs)?;
        }
        D::OptStr(v) => {
            enc.write_column_header(ColumnType::OptStr, &v.name)?;
            write_str_col(v, Some(v), enc, codecs)?;
        }
        D::SharedDict(v) => return write_shared_dict(v, enc, codecs),
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
    presence_bools: impl ExactSizeIterator<Item = bool>,
    enc: &mut Encoder,
    codecs: &mut Codecs,
) -> MltResult<()> {
    enc.write_column_header(ct, name)?;
    write_bool_stream(presence_bools, StreamType::Present, enc, codecs)
}

pub(crate) fn write_u32_scalar_col(
    ct: ColumnType,
    name: Option<&str>,
    v: &StagedScalar<u32>,
    enc: &mut Encoder,
    codecs: &mut Codecs,
) -> MltResult<()> {
    begin_scalar_col(ct, name, enc)?;
    write_u32_stream(&v.values, &scalar_ctx(name), enc, codecs)
}

pub(crate) fn write_opt_u32_scalar_col(
    ct: ColumnType,
    name: Option<&str>,
    v: &StagedOptScalar<u32>,
    enc: &mut Encoder,
    codecs: &mut Codecs,
) -> MltResult<()> {
    begin_scalar_col(ct, name, enc)?;
    write_bool_stream(v.presence.iter().copied(), StreamType::Present, enc, codecs)?;
    write_u32_stream(&v.values, &scalar_ctx(name), enc, codecs)
}

/// Write an `i64` integer stream.
///
/// Zigzag-encodes the values for candidate pruning but encodes the original
/// signed values via the logical encoder's `encode_i64s`.
pub(crate) fn write_i64_stream(
    values: &[i64],
    ctx: &StreamCtx<'_>,
    enc: &mut Encoder,
    codecs: &mut Codecs,
) -> MltResult<()> {
    let stream_type = ctx.stream_type;
    if let Some(int_enc) = enc.override_int_enc(ctx) {
        write_i64_stream_as(values, stream_type, int_enc, enc, codecs)?;
    } else {
        let profiled = codecs.logical.encode_zigzag_i64(values);
        let candidates = DataProfile::prune_candidates::<i64>(profiled);
        write_alternatives(enc, codecs, candidates, |enc, codecs, cand| {
            write_i64_stream_as(values, stream_type, cand, enc, codecs)
        })?;
    }
    Ok(())
}

pub(crate) fn write_u64_scalar_col(
    ct: ColumnType,
    name: Option<&str>,
    v: &StagedScalar<u64>,
    enc: &mut Encoder,
    codecs: &mut Codecs,
) -> MltResult<()> {
    begin_scalar_col(ct, name, enc)?;
    write_u64_stream(&v.values, &scalar_ctx(name), enc, codecs)
}

pub(crate) fn write_opt_u64_scalar_col(
    ct: ColumnType,
    name: Option<&str>,
    v: &StagedOptScalar<u64>,
    enc: &mut Encoder,
    codecs: &mut Codecs,
) -> MltResult<()> {
    let presence_bools = v.presence.iter().copied();
    begin_scalar_col(ct, name, enc)?;
    write_bool_stream(presence_bools, StreamType::Present, enc, codecs)?;
    write_u64_stream(&v.values, &scalar_ctx(name), enc, codecs)
}

fn begin_scalar_col(ct: ColumnType, name: Option<&str>, enc: &mut Encoder) -> MltResult<()> {
    if let Some(name) = name {
        enc.write_column_header(ct, name)
    } else {
        enc.write_column_type(ct)
    }
}

fn scalar_ctx(name: Option<&str>) -> StreamCtx<'_> {
    match name {
        Some(name) => StreamCtx::prop(StreamType::Data(DictionaryType::None), name),
        None => StreamCtx::id(StreamType::Data(DictionaryType::None)),
    }
}

impl StagedProperty {
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
    pub fn str(name: impl Into<String>, values: impl IntoIterator<Item = impl AsRef<str>>) -> Self {
        Self::Str(StagedStrings::from_strings(name, values))
    }

    #[must_use]
    pub fn opt_str(
        name: impl Into<String>,
        values: impl IntoIterator<Item = Option<impl AsRef<str>>>,
    ) -> Self {
        Self::OptStr(StagedStrings::from_optional(name, values))
    }

    // ── Optional constructors ─────────────────────────────────────────────────

    #[must_use]
    pub fn opt_bool(
        name: impl Into<String>,
        values: impl IntoIterator<Item = Option<bool>>,
    ) -> Self {
        Self::OptBool(StagedOptScalar::from_optional(name, values))
    }
    #[must_use]
    pub fn opt_i8(name: impl Into<String>, values: impl IntoIterator<Item = Option<i8>>) -> Self {
        Self::OptI8(StagedOptScalar::from_optional(name, values))
    }
    #[must_use]
    pub fn opt_u8(name: impl Into<String>, values: impl IntoIterator<Item = Option<u8>>) -> Self {
        Self::OptU8(StagedOptScalar::from_optional(name, values))
    }
    #[must_use]
    pub fn opt_i32(name: impl Into<String>, values: impl IntoIterator<Item = Option<i32>>) -> Self {
        Self::OptI32(StagedOptScalar::from_optional(name, values))
    }
    #[must_use]
    pub fn opt_u32(name: impl Into<String>, values: impl IntoIterator<Item = Option<u32>>) -> Self {
        Self::OptU32(StagedOptScalar::from_optional(name, values))
    }
    #[must_use]
    pub fn opt_i64(name: impl Into<String>, values: impl IntoIterator<Item = Option<i64>>) -> Self {
        Self::OptI64(StagedOptScalar::from_optional(name, values))
    }
    #[must_use]
    pub fn opt_u64(name: impl Into<String>, values: impl IntoIterator<Item = Option<u64>>) -> Self {
        Self::OptU64(StagedOptScalar::from_optional(name, values))
    }
    #[must_use]
    pub fn opt_f32(name: impl Into<String>, values: impl IntoIterator<Item = Option<f32>>) -> Self {
        Self::OptF32(StagedOptScalar::from_optional(name, values))
    }
    #[must_use]
    pub fn opt_f64(name: impl Into<String>, values: impl IntoIterator<Item = Option<f64>>) -> Self {
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
            Self::Str(v) | Self::OptStr(v) => &v.name,
            Self::SharedDict(v) => &v.prefix,
        }
    }
}

impl<T: Copy + PartialEq> StagedOptScalar<T> {
    /// Build from optional values: dense non-null entries in `values`,
    /// per-feature flags in `presence`.
    #[must_use]
    pub fn from_optional(
        name: impl Into<String>,
        values: impl IntoIterator<Item = Option<T>>,
    ) -> Self {
        let values = values.into_iter();
        let (lower, upper) = values.size_hint();
        let capacity = upper.unwrap_or(lower);
        let mut presence = Vec::with_capacity(capacity);
        let mut dense = Vec::with_capacity(capacity);
        for value in values {
            presence.push(value.is_some());
            if let Some(value) = value {
                dense.push(value);
            }
        }
        Self::from_parts(name, presence, dense)
    }

    #[must_use]
    pub(crate) fn from_parts(name: impl Into<String>, presence: Vec<bool>, values: Vec<T>) -> Self {
        Self {
            name: name.into(),
            presence,
            values,
        }
    }
}
