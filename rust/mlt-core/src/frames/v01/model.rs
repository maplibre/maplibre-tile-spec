use num_enum::TryFromPrimitive;

use crate::v01::{
    EncodedGeometry, EncodedProperty, Geometry, Id, ParsedGeometry, ParsedId, Property,
    StagedProperty,
};

/// Column definition
#[derive(Debug, PartialEq)]
pub struct Column<'a> {
    pub typ: ColumnType,
    pub name: Option<&'a str>,
    pub children: Vec<Column<'a>>,
}

/// Owned variant of [`Column`].
#[derive(Debug, PartialEq, Clone)]
pub struct OwnedColumn {
    pub typ: ColumnType,
    pub name: Option<String>,
    pub children: Vec<OwnedColumn>,
}

/// Column data type, as stored in the tile
#[derive(Debug, Clone, Copy, PartialEq, TryFromPrimitive)]
#[repr(u8)]
pub enum ColumnType {
    Id = 0,
    OptId = 1,
    LongId = 2,
    OptLongId = 3,
    Geometry = 4,
    Bool = 10,
    OptBool = 11,
    I8 = 12,
    OptI8 = 13,
    U8 = 14,
    OptU8 = 15,
    I32 = 16,
    OptI32 = 17,
    U32 = 18,
    OptU32 = 19,
    I64 = 20,
    OptI64 = 21,
    U64 = 22,
    OptU64 = 23,
    F32 = 24,
    OptF32 = 25,
    F64 = 26,
    OptF64 = 27,
    Str = 28,
    OptStr = 29,
    SharedDict = 30,
}

/// Representation of a feature table layer encoded as MLT tag `0x01`
#[derive(Debug, PartialEq)]
pub struct Layer01<'a> {
    pub name: &'a str,
    pub extent: u32,
    pub id: Option<Id<'a>>,
    pub geometry: Geometry<'a>,
    pub properties: Vec<Property<'a>>,
    #[cfg(fuzzing)]
    pub layer_order: Vec<crate::frames::v01::root::LayerOrdering>,
}

/// Representation of a feature table layer in the staging pipeline.
/// Holds parsed (decoded) data ready for encoding.
#[derive(Debug, PartialEq, Clone)]
#[cfg_attr(
    all(not(test), not(fuzzing), feature = "arbitrary"),
    derive(arbitrary::Arbitrary)
)]
pub struct StagedLayer01 {
    pub name: String,
    pub extent: u32,
    pub id: Option<ParsedId>,
    pub geometry: ParsedGeometry,
    pub properties: Vec<StagedProperty>,
    #[cfg(fuzzing)]
    pub layer_order: Vec<crate::frames::v01::root::LayerOrdering>,
}

/// Wire-ready encoded representation of a feature table layer (MLT tag `0x01`).
/// Holds all data in encoded form, ready for serialization to bytes.
#[derive(Debug, PartialEq, Clone)]
pub struct EncodedLayer01 {
    pub name: String,
    pub extent: u32,
    pub id: Option<crate::v01::EncodedId>,
    pub geometry: EncodedGeometry,
    pub properties: Vec<EncodedProperty>,
    #[cfg(fuzzing)]
    pub layer_order: Vec<crate::frames::v01::root::LayerOrdering>,
}
