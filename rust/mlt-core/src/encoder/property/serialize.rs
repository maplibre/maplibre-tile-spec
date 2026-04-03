use crate::encoder::property::{PropertyKind, StagedProperty};

impl StagedProperty {
    #[must_use]
    pub fn kind(&self) -> PropertyKind {
        use PropertyKind as T;
        match self {
            Self::Bool(_) => T::Bool,
            Self::I8(_)
            | Self::I32(_)
            | Self::I64(_)
            | Self::U8(_)
            | Self::U32(_)
            | Self::U64(_) => T::Integer,
            Self::F32(_) | Self::F64(_) => T::Float,
            Self::Str(_) => T::String,
            Self::SharedDict(_) => T::SharedDict,
        }
    }
}
