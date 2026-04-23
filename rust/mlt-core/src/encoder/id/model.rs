/// How wide are the IDs
#[derive(Debug, Clone, Copy, PartialEq, strum::EnumIter)]
#[cfg_attr(all(not(test), feature = "arbitrary"), derive(arbitrary::Arbitrary))]
pub enum IdWidth {
    /// 32-bit encoding
    Id32,
    /// 32-bit encoding with nulls
    OptId32,
    /// 64-bit encoding (delta + zigzag + varint)
    Id64,
    /// 64-bit encoding with nulls
    OptId64,
}

/// Staged ID column (encode-side, fully owned).
///
/// Mirrors the `StagedProperty` enum pattern but without a column name (IDs are nameless)
/// and without a compile-time 32 / 64-bit distinction (the encoder picks the width from
/// the actual values).
///
/// - [`StagedId::Id`] — every feature has an ID; no nulls.
/// - [`StagedId::OptId`] — some features may be null.  `presence[i]` is `true` when
///   feature `i` has a value; `values` holds only the non-null entries in dense order,
///   using the same `Vec<bool>` + dense-values layout as `StagedOptScalar`.
#[derive(Debug, Clone, PartialEq)]
pub enum StagedId {
    /// Non-optional: every feature has an ID.
    Id(Vec<u64>),
    /// Optional: some features may be absent.
    OptId {
        /// One `bool` per feature; `true` means the feature has a value.
        presence: Vec<bool>,
        /// Dense values: only entries where the corresponding presence flag is `true`.
        values: Vec<u64>,
    },
}

impl StagedId {
    /// Construct from a sparse `Vec<Option<u64>>`.
    ///
    /// All-present input produces [`StagedId::Id`]; any `None` produces
    /// [`StagedId::OptId`] with a dense values vector.
    #[must_use]
    pub fn from_optional(ids: Vec<Option<u64>>) -> Self {
        if !ids.iter().any(Option::is_none) {
            return Self::Id(ids.into_iter().map(Option::unwrap).collect());
        }
        let mut presence = Vec::with_capacity(ids.len());
        let mut values = Vec::new();
        for id in ids {
            presence.push(id.is_some());
            if let Some(v) = id {
                values.push(v);
            }
        }
        Self::OptId { presence, values }
    }

    /// Total number of features (present and absent).
    #[inline]
    #[must_use]
    pub fn feature_count(&self) -> usize {
        match self {
            Self::Id(values) => values.len(),
            Self::OptId { presence, .. } => presence.len(),
        }
    }

    /// Return the dense values slice (present entries only).
    #[inline]
    #[must_use]
    pub fn dense_values(&self) -> &[u64] {
        match self {
            Self::Id(values) | Self::OptId { values, .. } => values,
        }
    }

    /// Expand into a `Vec<Option<u64>>` with one entry per feature.
    ///
    /// Allocates; used for cross-type equality comparisons and tests.
    #[must_use]
    pub fn materialize(&self) -> Vec<Option<u64>> {
        match self {
            Self::Id(values) => values.iter().copied().map(Some).collect(),
            Self::OptId { presence, values } => {
                let mut dense = values.iter().copied();
                presence
                    .iter()
                    .map(|&p| if p { dense.next() } else { None })
                    .collect()
            }
        }
    }
}
