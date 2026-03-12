use crate::utils::{f32_to_json, f64_to_json};
use crate::v01::{DecodedProperty, EncodedProperty, PropertyKind};

impl<'a> EncodedProperty<'a> {
    #[allow(clippy::match_same_arms)]
    pub(super) fn name(&self) -> &'a str {
        match self {
            Self::Bool(name, _)
            | Self::I8(name, _)
            | Self::U8(name, _)
            | Self::I32(name, _)
            | Self::U32(name, _)
            | Self::I64(name, _)
            | Self::U64(name, _)
            | Self::F32(name, _)
            | Self::F64(name, _) => name.0,
            Self::BoolOpt(name, _, _)
            | Self::I8Opt(name, _, _)
            | Self::U8Opt(name, _, _)
            | Self::I32Opt(name, _, _)
            | Self::U32Opt(name, _, _)
            | Self::I64Opt(name, _, _)
            | Self::U64Opt(name, _, _)
            | Self::F32Opt(name, _, _)
            | Self::F64Opt(name, _, _)
            | Self::Str(name, _, _)
            | Self::SharedDict(name, _, _) => name.0,
        }
    }
}

impl DecodedProperty<'_> {
    #[must_use]
    pub fn name(&self) -> &str {
        match self {
            Self::Bool(v) => v.name.as_ref(),
            Self::BoolOpt(v) => v.name.as_ref(),
            Self::I8(v) => v.name.as_ref(),
            Self::I8Opt(v) => v.name.as_ref(),
            Self::U8(v) => v.name.as_ref(),
            Self::U8Opt(v) => v.name.as_ref(),
            Self::I32(v) => v.name.as_ref(),
            Self::I32Opt(v) => v.name.as_ref(),
            Self::U32(v) => v.name.as_ref(),
            Self::U32Opt(v) => v.name.as_ref(),
            Self::I64(v) => v.name.as_ref(),
            Self::I64Opt(v) => v.name.as_ref(),
            Self::U64(v) => v.name.as_ref(),
            Self::U64Opt(v) => v.name.as_ref(),
            Self::F32(v) => v.name.as_ref(),
            Self::F32Opt(v) => v.name.as_ref(),
            Self::F64(v) => v.name.as_ref(),
            Self::F64Opt(v) => v.name.as_ref(),
            Self::Str(v) => v.name.as_ref(),
            Self::SharedDict(shared_dict) => shared_dict.prefix.as_ref(),
        }
    }

    #[must_use]
    pub fn kind(&self) -> PropertyKind {
        match self {
            Self::Bool(_) | Self::BoolOpt(_) => PropertyKind::Bool,
            Self::I8(_)
            | Self::I8Opt(_)
            | Self::U8(_)
            | Self::U8Opt(_)
            | Self::I32(_)
            | Self::I32Opt(_)
            | Self::U32(_)
            | Self::U32Opt(_)
            | Self::I64(_)
            | Self::I64Opt(_)
            | Self::U64(_)
            | Self::U64Opt(_) => PropertyKind::Integer,
            Self::F32(_) | Self::F32Opt(_) | Self::F64(_) | Self::F64Opt(_) => PropertyKind::Float,
            Self::Str(..) => PropertyKind::String,
            Self::SharedDict(..) => PropertyKind::SharedDict,
        }
    }

    /// Convert the value at index `i` to a [`serde_json::Value`]
    #[must_use]
    pub fn to_geojson(&self, i: usize) -> Option<serde_json::Value> {
        use serde_json::Value;

        match self {
            Self::Bool(v) => Some(Value::Bool(v.values[i])),
            Self::BoolOpt(v) => v.values[i].map(Value::Bool),
            Self::I8(v) => Some(Value::from(v.values[i])),
            Self::I8Opt(v) => v.values[i].map(Value::from),
            Self::U8(v) => Some(Value::from(v.values[i])),
            Self::U8Opt(v) => v.values[i].map(Value::from),
            Self::I32(v) => Some(Value::from(v.values[i])),
            Self::I32Opt(v) => v.values[i].map(Value::from),
            Self::U32(v) => Some(Value::from(v.values[i])),
            Self::U32Opt(v) => v.values[i].map(Value::from),
            Self::I64(v) => Some(Value::from(v.values[i])),
            Self::I64Opt(v) => v.values[i].map(Value::from),
            Self::U64(v) => Some(Value::from(v.values[i])),
            Self::U64Opt(v) => v.values[i].map(Value::from),
            Self::F32(v) => Some(f32_to_json(v.values[i])),
            Self::F32Opt(v) => v.values[i].map(f32_to_json),
            Self::F64(v) => Some(f64_to_json(v.values[i])),
            Self::F64Opt(v) => v.values[i].map(f64_to_json),
            Self::Str(v) => v
                .get(u32::try_from(i).ok()?)
                .map(|s| Value::String(s.to_string())),
            Self::SharedDict(shared_dict) => {
                let mut obj = serde_json::Map::new();
                for item in &shared_dict.items {
                    if let Some(s) = item.get(shared_dict, i) {
                        obj.insert(item.suffix.to_string(), Value::String(s.to_string()));
                    }
                }
                if obj.is_empty() {
                    None
                } else {
                    Some(Value::Object(obj))
                }
            }
        }
    }
}
