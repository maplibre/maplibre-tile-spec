use super::model::{StagedOptScalar, StagedProperty};
use crate::MltResult;
use crate::decoder::{ColumnType, DictionaryType, StreamType};
use crate::encoder::model::StreamCtx;
use crate::encoder::{
    Codecs, Encoder, LogicalCodecs, LogicalIntCodec, LogicalIntStreamKind, StagedScalar,
    StagedStrings,
};

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
#[hotpath::measure]
fn write_prop(prop: &StagedProperty, enc: &mut Encoder, codecs: &mut Codecs) -> MltResult<()> {
    use ColumnType as CT;
    use StagedProperty as D;

    match prop {
        D::Bool(v) => {
            enc.write_column_header(CT::Bool, &v.name)?;
            let values = v.values.iter().copied();
            codecs.write_bool_stream(values, StreamType::Data(DictionaryType::None), enc)
        }
        D::OptBool(v) => {
            codecs.begin_opt_col(CT::OptBool, &v.name, &v.presence, enc)?;
            let values = v.values.iter().copied();
            codecs.write_bool_stream(values, StreamType::Data(DictionaryType::None), enc)
        }
        D::F32(v) => {
            enc.write_column_header(CT::F32, &v.name)?;
            codecs.write_float_stream(&v.values, StreamType::Data(DictionaryType::None), enc)
        }
        D::OptF32(v) => {
            codecs.begin_opt_col(CT::OptF32, &v.name, &v.presence, enc)?;
            codecs.write_float_stream(&v.values, StreamType::Data(DictionaryType::None), enc)
        }
        D::F64(v) => {
            enc.write_column_header(CT::F64, &v.name)?;
            codecs.write_float_stream(&v.values, StreamType::Data(DictionaryType::None), enc)
        }
        D::OptF64(v) => {
            codecs.begin_opt_col(CT::OptF64, &v.name, &v.presence, enc)?;
            codecs.write_float_stream(&v.values, StreamType::Data(DictionaryType::None), enc)
        }
        D::I8(v) => codecs.write_scalar_col(CT::I8, Some(&v.name), v, enc),
        D::OptI8(v) => codecs.write_opt_scalar_col(CT::OptI8, Some(&v.name), v, enc),
        D::U8(v) => codecs.write_scalar_col(CT::U8, Some(&v.name), v, enc),
        D::OptU8(v) => codecs.write_opt_scalar_col(CT::OptU8, Some(&v.name), v, enc),
        D::I32(v) => codecs.write_scalar_col(CT::I32, Some(&v.name), v, enc),
        D::OptI32(v) => codecs.write_opt_scalar_col(CT::OptI32, Some(&v.name), v, enc),
        D::U32(v) => codecs.write_scalar_col(CT::U32, Some(&v.name), v, enc),
        D::OptU32(v) => codecs.write_opt_scalar_col(CT::OptU32, Some(&v.name), v, enc),
        D::I64(v) => codecs.write_scalar_col(CT::I64, Some(&v.name), v, enc),
        D::OptI64(v) => codecs.write_opt_scalar_col(CT::OptI64, Some(&v.name), v, enc),
        D::U64(v) => codecs.write_scalar_col(CT::U64, Some(&v.name), v, enc),
        D::OptU64(v) => codecs.write_opt_scalar_col(CT::OptU64, Some(&v.name), v, enc),
        D::Str(v) => {
            enc.write_column_header(ColumnType::Str, &v.name)?;
            codecs.write_str_col(v, None, enc)
        }
        D::OptStr(v) => {
            enc.write_column_header(ColumnType::OptStr, &v.name)?;
            codecs.write_str_col(v, Some(v), enc)
        }
        D::SharedDict(v) => codecs.write_shared_dict(v, enc),
    }
}

impl Codecs {
    /// Writes the column-type byte, name, and presence stream for an optional column.
    ///
    /// Presence is always written: the invariant is that an optional-variant column always
    /// has a presence stream. Ensuring the column is non-empty and not all-null is the
    /// caller's responsibility.
    fn begin_opt_col(
        &mut self,
        ct: ColumnType,
        name: &str,
        presence: &[bool],
        enc: &mut Encoder,
    ) -> MltResult<()> {
        enc.write_column_header(ct, name)?;
        self.write_presence_stream(presence.iter().copied(), enc)
    }

    pub(crate) fn write_scalar_col<T>(
        &mut self,
        ct: ColumnType,
        name: Option<&str>,
        v: &StagedScalar<T>,
        enc: &mut Encoder,
    ) -> MltResult<()>
    where
        T: Copy + PartialEq,
        [T]: LogicalIntStreamKind<Input = T>,
        LogicalCodecs: LogicalIntCodec<[T]>,
    {
        begin_scalar_col(ct, name, enc)?;
        self.write_int_stream(&v.values, &scalar_ctx(name), enc)
    }

    pub(crate) fn write_opt_scalar_col<T>(
        &mut self,
        ct: ColumnType,
        name: Option<&str>,
        v: &StagedOptScalar<T>,
        enc: &mut Encoder,
    ) -> MltResult<()>
    where
        T: Copy + PartialEq,
        [T]: LogicalIntStreamKind<Input = T>,
        LogicalCodecs: LogicalIntCodec<[T]>,
    {
        begin_scalar_col(ct, name, enc)?;
        self.write_presence_stream(v.presence.iter().copied(), enc)?;
        self.write_int_stream(&v.values, &scalar_ctx(name), enc)
    }
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
        Some(name) => StreamCtx::prop_data(name),
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
