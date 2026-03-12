use borrowme::borrowme;
use num_enum::TryFromPrimitive;

use crate::v01::{Geometry, Id, Property};

/// Column definition
#[borrowme]
#[derive(Debug, PartialEq)]
pub struct Column<'a> {
    pub typ: ColumnType,
    pub name: Option<&'a str>,
    pub children: Vec<Column<'a>>,
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
#[cfg(not(fuzzing))]
#[borrowme]
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(
    all(not(test), not(fuzzing), feature = "arbitrary"),
    owned_attr(derive(arbitrary::Arbitrary))
)]
pub struct Layer01<'a> {
    pub name: &'a str,
    pub extent: u32,
    pub id: Id<'a>,
    pub geometry: Geometry<'a>,
    pub properties: Vec<Property<'a>>,
}

/// FIXME: fuzzing is only adding layer_order but this borrowme does not codegen correctly in this case
#[cfg(fuzzing)]
#[borrowme]
#[derive(Debug, Clone, PartialEq)]
pub struct Layer01<'a> {
    pub name: &'a str,
    pub extent: u32,
    pub id: Id<'a>,
    pub geometry: Geometry<'a>,
    pub properties: Vec<Property<'a>>,
    pub layer_order: Vec<crate::frames::v01::root::LayerOrdering>,
}
