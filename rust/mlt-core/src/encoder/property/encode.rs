use super::model::{PresenceKind, StagedProperty};
use super::strings::write_str_col;
use crate::MltError::NotImplemented;
use crate::MltResult;
use crate::decoder::{ColumnType, DictionaryType, StreamType};
use crate::encoder::model::ColumnKind;
use crate::encoder::property::shared_dict::write_shared_dict;
use crate::encoder::stream::{
    write_i32_stream, write_i64_stream, write_u32_stream, write_u64_stream,
};
use crate::encoder::{EncodedStream, Encoder, StagedScalar, StagedStrings};
use crate::utils::BinarySerializer as _;

/// Encode all property columns and write them to `enc`.
pub fn write_properties(props: &[StagedProperty], enc: &mut Encoder) -> MltResult<()> {
    for prop in props {
        write_prop(prop, enc)?;
    }
    Ok(())
}

/// Encode a single property column, dispatching on variant.
///
/// Returns `false` when the column is omitted (empty or all-null).
fn write_prop(prop: &StagedProperty, enc: &mut Encoder) -> MltResult<bool> {
    use ColumnType as CT;
    use StagedProperty as D;

    let has_presence = if matches!(prop, StagedProperty::SharedDict(_)) {
        false
    } else {
        match prop.presence() {
            PresenceKind::Empty => return Ok(false),
            PresenceKind::Mixed => true,
            PresenceKind::AllNull => {
                if enc.override_presence(ColumnKind::Property, prop.name(), None) {
                    true
                } else {
                    return Ok(false);
                }
            }
            PresenceKind::AllPresent => {
                enc.override_presence(ColumnKind::Property, prop.name(), None)
            }
        }
    };

    let presence = if has_presence {
        Some(EncodedStream::encode_presence(&prop.as_presence_stream()?)?)
    } else {
        None
    };
    let presence = presence.as_ref();

    match prop {
        D::Bool(v) => {
            CT::write_one_of(presence.is_some(), CT::OptBool, CT::Bool, &mut enc.meta)?;
            enc.meta.write_string(&v.name)?;
            enc.write_optional_stream(presence)?;
            enc.write_boolean_stream(&EncodedStream::encode_bools(&unapply_presence(&v.values))?)?;
        }
        D::F32(v) => {
            CT::write_one_of(presence.is_some(), CT::OptF32, CT::F32, &mut enc.meta)?;
            enc.meta.write_string(&v.name)?;
            enc.write_optional_stream(presence)?;
            enc.write_stream(&EncodedStream::encode_f32(&unapply_presence(&v.values))?)?;
        }
        D::F64(v) => {
            CT::write_one_of(presence.is_some(), CT::OptF64, CT::F64, &mut enc.meta)?;
            enc.meta.write_string(&v.name)?;
            enc.write_optional_stream(presence)?;
            enc.write_stream(&EncodedStream::encode_f64(&unapply_presence(&v.values))?)?;
        }
        D::I8(v) => {
            CT::write_one_of(presence.is_some(), CT::OptI8, CT::I8, &mut enc.meta)?;
            enc.meta.write_string(&v.name)?;
            enc.write_optional_stream(presence)?;
            let non_null: Vec<i8> = unapply_presence(&v.values);
            let widened: Vec<i32> = non_null.iter().map(|&v1| i32::from(v1)).collect();
            let typ = StreamType::Data(DictionaryType::None);
            write_i32_stream(&widened, typ, ColumnKind::Property, &v.name, "", enc)?;
        }
        D::U8(v) => {
            CT::write_one_of(presence.is_some(), CT::OptU8, CT::U8, &mut enc.meta)?;
            enc.meta.write_string(&v.name)?;
            enc.write_optional_stream(presence)?;
            let non_null: Vec<u8> = unapply_presence(&v.values);
            let widened: Vec<u32> = non_null.iter().map(|&v1| u32::from(v1)).collect();
            let typ = StreamType::Data(DictionaryType::None);
            write_u32_stream(&widened, typ, ColumnKind::Property, &v.name, "", enc)?;
        }
        D::I32(v) => {
            CT::write_one_of(presence.is_some(), CT::OptI32, CT::I32, &mut enc.meta)?;
            enc.meta.write_string(&v.name)?;
            enc.write_optional_stream(presence)?;
            let non_null: Vec<i32> = unapply_presence(&v.values);
            let typ = StreamType::Data(DictionaryType::None);
            write_i32_stream(&non_null, typ, ColumnKind::Property, &v.name, "", enc)?;
        }
        D::U32(v) => {
            CT::write_one_of(presence.is_some(), CT::OptU32, CT::U32, &mut enc.meta)?;
            enc.meta.write_string(&v.name)?;
            enc.write_optional_stream(presence)?;
            let non_null: Vec<u32> = unapply_presence(&v.values);
            let typ = StreamType::Data(DictionaryType::None);
            write_u32_stream(&non_null, typ, ColumnKind::Property, &v.name, "", enc)?;
        }
        D::I64(v) => {
            CT::write_one_of(presence.is_some(), CT::OptI64, CT::I64, &mut enc.meta)?;
            enc.meta.write_string(&v.name)?;
            enc.write_optional_stream(presence)?;
            let non_null: Vec<i64> = unapply_presence(&v.values);
            let typ = StreamType::Data(DictionaryType::None);
            write_i64_stream(&non_null, typ, ColumnKind::Property, &v.name, "", enc)?;
        }
        D::U64(v) => {
            CT::write_one_of(presence.is_some(), CT::OptU64, CT::U64, &mut enc.meta)?;
            enc.meta.write_string(&v.name)?;
            enc.write_optional_stream(presence)?;
            let non_null: Vec<u64> = unapply_presence(&v.values);
            let typ = StreamType::Data(DictionaryType::None);
            write_u64_stream(&non_null, typ, ColumnKind::Property, &v.name, "", enc)?;
        }
        D::Str(v) => {
            CT::write_one_of(presence.is_some(), CT::OptStr, CT::Str, &mut enc.meta)?;
            enc.meta.write_string(&v.name)?;
            write_str_col(v, presence, enc)?;
        }
        D::SharedDict(v) => {
            // return right away - skip adding column count
            return write_shared_dict(v, enc);
        }
    }
    enc.increment_column_count();
    Ok(true)
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
