use crate::utils::{f32_to_json, f64_to_json};
use crate::v01::{ParsedProperty, PropertyKind};

impl ParsedProperty<'_> {
    #[must_use]
    pub fn name(&self) -> &str {
        match self {
            Self::Bool(v) => v.name.as_ref(),
            Self::I8(v) => v.name.as_ref(),
            Self::U8(v) => v.name.as_ref(),
            Self::I32(v) => v.name.as_ref(),
            Self::U32(v) => v.name.as_ref(),
            Self::I64(v) => v.name.as_ref(),
            Self::U64(v) => v.name.as_ref(),
            Self::F32(v) => v.name.as_ref(),
            Self::F64(v) => v.name.as_ref(),
            Self::Str(v) => v.name.as_ref(),
            Self::SharedDict(shared_dict) => shared_dict.prefix.as_ref(),
        }
    }

    #[must_use]
    pub fn kind(&self) -> PropertyKind {
        match self {
            Self::Bool(_) => PropertyKind::Bool,
            Self::I8(_)
            | Self::U8(_)
            | Self::I32(_)
            | Self::U32(_)
            | Self::I64(_)
            | Self::U64(_) => PropertyKind::Integer,
            Self::F32(_) | Self::F64(_) => PropertyKind::Float,
            Self::Str(..) => PropertyKind::String,
            Self::SharedDict(..) => PropertyKind::SharedDict,
        }
    }

    /// Convert the value at index `i` to a [`serde_json::Value`]
    #[must_use]
    pub fn to_geojson(&self, i: usize) -> Option<serde_json::Value> {
        use serde_json::Value;

        match self {
            Self::Bool(v) => v.values[i].map(Value::Bool),
            Self::I8(v) => v.values[i].map(Value::from),
            Self::U8(v) => v.values[i].map(Value::from),
            Self::I32(v) => v.values[i].map(Value::from),
            Self::U32(v) => v.values[i].map(Value::from),
            Self::I64(v) => v.values[i].map(Value::from),
            Self::U64(v) => v.values[i].map(Value::from),
            Self::F32(v) => v.values[i].map(f32_to_json),
            Self::F64(v) => v.values[i].map(f64_to_json),
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
