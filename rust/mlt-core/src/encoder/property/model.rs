/// Staged property column (encode-side, fully owned).
///
/// Unlike `ParsedProperty` (decode-side, potentially borrowed), all string names
/// and corpus data are owned strings.  No lifetime parameter needed.
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
    OptBool(StagedOptScalar<bool>),
    OptI8(StagedOptScalar<i8>),
    OptU8(StagedOptScalar<u8>),
    OptI32(StagedOptScalar<i32>),
    OptU32(StagedOptScalar<u32>),
    OptI64(StagedOptScalar<i64>),
    OptU64(StagedOptScalar<u64>),
    OptF32(StagedOptScalar<f32>),
    OptF64(StagedOptScalar<f64>),
    OptStr(StagedStrings),
    SharedDict(StagedSharedDict),
}

/// Owned non-optional scalar column prepared for encoding (bool, integer, or float).
///
/// Every feature in this column has a value; there are no nulls.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub struct StagedScalar<T: Copy + PartialEq> {
    pub(crate) name: String,
    pub values: Vec<T>,
}

/// Owned optional scalar column prepared for encoding (bool, integer, or float).
///
/// `values` contains only the non-null (present) values in dense order.
/// `presence[i]` is `true` when feature `i` has a value; the corresponding dense
/// entry is `values[k]` where `k` is the count of `true` entries before index `i`.
#[derive(Debug, Clone, PartialEq)]
pub struct StagedOptScalar<T: Copy + PartialEq> {
    pub(crate) name: String,
    pub presence: Vec<bool>,
    pub values: Vec<T>,
}

/// Owned non-optional string column prepared for encoding.
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
    /// It's OK to write unneeded one, but can't be false with nulls.
    pub(crate) has_presence: bool,
}
