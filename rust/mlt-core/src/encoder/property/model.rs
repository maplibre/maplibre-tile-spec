pub enum PropertyKind {
    Bool,
    Integer,
    Float,
    String,
    SharedDict,
}

/// Staged property column (encode-side, fully owned).
///
/// Unlike `ParsedProperty` (decode-side, potentially borrowed), all string names
/// and corpus data are owned `String`s.  No lifetime parameter needed.
#[derive(Debug, Clone, PartialEq, strum::IntoStaticStr)]
#[strum(serialize_all = "snake_case")]
pub enum StagedProperty {
    Bool(StagedScalar<bool>),
    I8(StagedScalar<i8>),
    U8(StagedScalar<u8>),
    I32(StagedScalar<i32>),
    U32(StagedScalar<u32>),
    I64(StagedScalar<i64>),
    U64(StagedScalar<u64>),
    F32(StagedScalar<f32>),
    F64(StagedScalar<f64>),
    Str(StagedStrings),
    SharedDict(StagedSharedDict),
}

/// Describes the null pattern of a single column.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PresenceKind {
    /// The column has no rows
    Empty,
    /// Every row has a value
    AllPresent,
    /// Every row is null
    AllNull,
    /// Some rows are null, some have values
    Mixed,
}

impl PresenceKind {
    /// Returns `true` when a presence/validity stream must be written to the
    /// wire format (i.e. the column has at least one `null` value).
    #[must_use]
    pub fn needs_presence_stream(self) -> bool {
        matches!(self, Self::AllNull | Self::Mixed)
    }
}

/// Owned scalar column prepared for encoding (bool, integer, or float).
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub struct StagedScalar<T: Copy + PartialEq> {
    pub(crate) name: String,
    pub values: Vec<Option<T>>,
}

/// Owned string column prepared for encoding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StagedStrings {
    pub(crate) name: String,
    /// Per-feature cumulative end offsets into `data` (same encoding as `ParsedStrings::lengths`).
    pub lengths: Vec<i32>,
    pub(crate) data: String,
}

/// Owned shared-dictionary column prepared for encoding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StagedSharedDict {
    pub(crate) prefix: String,
    pub(crate) data: String,
    pub items: Vec<StagedSharedDictItem>,
}

/// A single child within a staged shared-dictionary column.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StagedSharedDictItem {
    pub(crate) suffix: String,
    /// Per-feature `(start, end)` byte offsets into the shared corpus.
    pub ranges: Vec<(i32, i32)>,
}
