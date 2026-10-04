//! Column kind inference shared by the importers of dynamically typed
//! properties (MVT, `GeoJSON`), which need one fixed [`PropKind`] per column.

use std::collections::HashMap;

use crate::tile::PropKind;

/// Column kind inferred so far. `Unknown` is a column seen only as null, which
/// resolves to [`PropKind::Str`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum InferredKind {
    Unknown,
    Bool,
    I64,
    U64,
    F32,
    F64,
    Str,
}

impl InferredKind {
    /// Merge with another kind: `I64`+`U64` widen to `I64`, floats widen to
    /// `F64`, integers mixed with floats widen to `F64`, and any other conflict
    /// falls back to `Str`.
    pub fn merge(self, other: Self) -> Self {
        if self == Self::Unknown {
            return other;
        }
        if other == Self::Unknown || self == other {
            return self;
        }
        #[expect(clippy::match_same_arms)]
        match (self, other) {
            // Signed and unsigned integers share a signed column.
            (Self::I64 | Self::U64, Self::I64 | Self::U64) => Self::I64,
            // Every f32 is exactly representable as f64.
            (Self::F32 | Self::F64, Self::F32 | Self::F64) => Self::F64,
            // Integers mixed with floats are most likely whole-number values of a float column
            // that the encoder wrote as integers, so the column stays a float.
            (Self::I64 | Self::U64, Self::F32 | Self::F64)
            | (Self::F32 | Self::F64, Self::I64 | Self::U64) => Self::F64,
            _ => Self::Str,
        }
    }

    fn prop_kind(self) -> PropKind {
        match self {
            Self::Unknown | Self::Str => PropKind::Str,
            Self::Bool => PropKind::Bool,
            Self::I64 => PropKind::I64,
            Self::U64 => PropKind::U64,
            Self::F32 => PropKind::F32,
            Self::F64 => PropKind::F64,
        }
    }
}

/// Column names in first-seen order, each with the kind merged from every value seen.
#[derive(Default)]
pub(crate) struct ColumnInference<'a> {
    index: HashMap<&'a str, usize>,
    names: Vec<String>,
    kinds: Vec<InferredKind>,
}

impl<'a> ColumnInference<'a> {
    /// Record a value of `kind` in column `name`, returning the column's index.
    pub fn observe(&mut self, name: &'a str, kind: InferredKind) -> usize {
        let idx = self.column(name);
        self.merge(idx, kind);
        idx
    }

    /// The index of column `name`, adding it on first sight.
    pub fn column(&mut self, name: &'a str) -> usize {
        *self.index.entry(name).or_insert_with(|| {
            self.names.push(name.to_string());
            self.kinds.push(InferredKind::Unknown);
            self.names.len() - 1
        })
    }

    /// Record a value of `kind` in the column at `idx`.
    pub fn merge(&mut self, idx: usize, kind: InferredKind) {
        // One bounds check rather than one per index expression.
        let slot = &mut self.kinds[idx];
        *slot = slot.merge(kind);
    }

    /// The column names and their resolved kinds, in first-seen order.
    pub fn finish(self) -> (Vec<String>, Vec<PropKind>) {
        let kinds = self
            .kinds
            .into_iter()
            .map(InferredKind::prop_kind)
            .collect();
        (self.names, kinds)
    }
}
