use crate::v01::ParsedProperty;

impl ParsedProperty<'_> {
    #[must_use]
    pub fn name(&self) -> &str {
        match self {
            Self::Bool(v) => v.name,
            Self::I8(v) => v.name,
            Self::U8(v) => v.name,
            Self::I32(v) => v.name,
            Self::U32(v) => v.name,
            Self::I64(v) => v.name,
            Self::U64(v) => v.name,
            Self::F32(v) => v.name,
            Self::F64(v) => v.name,
            Self::Str(v) => v.name,
            Self::SharedDict(shared_dict) => shared_dict.prefix,
        }
    }
}
