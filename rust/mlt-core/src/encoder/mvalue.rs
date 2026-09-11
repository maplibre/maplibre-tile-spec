//! Columnar staging form of a layer's vertex-scoped columns.

use crate::encoder::StagedStrings;
use crate::tile::PropKind;

/// An m-value column prepared for encoding: one value per vertex of every
/// feature whose presence bit is set.
///
/// Its values are flat across features, the way the wire holds them, so nothing
/// here remembers where one feature's run ends - the geometry says that.
#[derive(Debug, Clone, PartialEq)]
pub struct StagedMValue {
    pub(crate) name: String,
    /// One bit per feature, or [`None`] when every feature carries values.
    pub(crate) presence: Option<Vec<bool>>,
    pub(crate) values: StagedMValues,
}

/// The values of a staged m-value column, of whichever type the column holds.
#[derive(Debug, Clone, PartialEq)]
pub enum StagedMValues {
    Bool(Vec<bool>),
    I8(Vec<i8>),
    U8(Vec<u8>),
    I32(Vec<i32>),
    U32(Vec<u32>),
    I64(Vec<i64>),
    U64(Vec<u64>),
    F32(Vec<f32>),
    F64(Vec<f64>),
    Str(Vec<String>),
}

impl StagedMValues {
    /// How many values the column holds, over every feature that has any.
    #[must_use]
    pub fn count(&self) -> usize {
        match self {
            Self::Bool(v) => v.len(),
            Self::I8(v) => v.len(),
            Self::U8(v) => v.len(),
            Self::I32(v) => v.len(),
            Self::U32(v) => v.len(),
            Self::I64(v) => v.len(),
            Self::U64(v) => v.len(),
            Self::F32(v) => v.len(),
            Self::F64(v) => v.len(),
            Self::Str(v) => v.len(),
        }
    }

    /// An empty column of this kind, to append a feature's values to.
    #[must_use]
    pub(crate) fn empty(kind: PropKind) -> Self {
        match kind {
            PropKind::Bool => Self::Bool(Vec::new()),
            PropKind::I8 => Self::I8(Vec::new()),
            PropKind::U8 => Self::U8(Vec::new()),
            PropKind::I32 => Self::I32(Vec::new()),
            PropKind::U32 => Self::U32(Vec::new()),
            PropKind::I64 => Self::I64(Vec::new()),
            PropKind::U64 => Self::U64(Vec::new()),
            PropKind::F32 => Self::F32(Vec::new()),
            PropKind::F64 => Self::F64(Vec::new()),
            PropKind::Str => Self::Str(Vec::new()),
        }
    }
}

impl StagedMValue {
    /// A column of `values`, carried by every feature whose bit in `presence` is
    /// set, or by all of them when there is no mask.
    #[must_use]
    pub fn new(
        name: impl Into<String>,
        presence: Option<Vec<bool>>,
        values: StagedMValues,
    ) -> Self {
        Self {
            name: name.into(),
            presence,
            values,
        }
    }

    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[must_use]
    pub fn values(&self) -> &StagedMValues {
        &self.values
    }

    /// How many features this column's presence mask covers, or [`None`] when it has no mask.
    #[must_use]
    pub(crate) fn feature_count(&self) -> Option<usize> {
        self.presence.as_ref().map(Vec::len)
    }

    /// The string column this one's values make up, which the string writers take.
    pub(crate) fn strings(&self, values: &[String]) -> StagedStrings {
        StagedStrings::from_strings(self.name.clone(), values)
    }
}
