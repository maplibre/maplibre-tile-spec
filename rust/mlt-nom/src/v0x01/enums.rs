use borrowme::borrowme;
// use borrowme::borrowme;
use num_enum::TryFromPrimitive;

use crate::MltError::ParsingColumnType;
use crate::{MltError, MltRefResult, utils};

#[borrowme]
#[derive(Debug, PartialEq, Clone, Copy, TryFromPrimitive)]
#[repr(u8)]
pub enum LogicalTechnique {
    None = 0,
    Delta = 1,
    ComponentwiseDelta = 2,
    Rle = 3,
    Morton = 4,
    PseudoDecimal = 5,
}

impl LogicalTechnique {
    pub fn parse(value: u8) -> Result<Self, MltError> {
        Self::try_from(value).or(Err(MltError::ParsingLogicalTechnique(value)))
    }
}

#[borrowme]
#[derive(Debug, PartialEq, Clone, Copy, TryFromPrimitive)]
#[repr(u8)]
pub enum PhysicalDecoder {
    None = 0,
    /// Preferred, tends to produce the best compression ratio and decoding performance.
    /// But currently limited to 32-bit integer.
    FastPFOR = 1,
    /// Can produce better results in combination with a heavyweight compression scheme like `Gzip`.
    /// Simple compression scheme where the decoder are easier to implement compared to `FastPfor`.
    VarInt = 2,
    /// Adaptive Lossless floating-Point Compression
    Alp = 3,
}

impl PhysicalDecoder {
    pub fn parse(value: u8) -> Result<Self, MltError> {
        Self::try_from(value).or(Err(MltError::ParsingPhysicalDecoder(value)))
    }
}

#[borrowme]
#[derive(Debug, PartialEq, Clone, Copy, TryFromPrimitive)]
#[repr(u8)]
pub enum DictionaryType {
    None = 0,
    Single = 1,
    Shared = 2,
    Vertex = 3,
    Morton = 4,
    Fsst = 5,
}

#[borrowme]
#[derive(Debug, PartialEq, Clone, Copy, TryFromPrimitive)]
#[repr(u8)]
pub enum LengthType {
    VarBinary = 0,
    Geometries = 1,
    Parts = 2,
    Rings = 3,
    Triangles = 4,
    Symbol = 5,
    Dictionary = 6,
}

#[borrowme]
#[derive(Debug, PartialEq, Clone, Copy, TryFromPrimitive)]
#[repr(u8)]
pub enum OffsetType {
    Vertex = 0,
    Index = 1,
    String = 2,
    Key = 3,
}

/// Column type enumeration
#[derive(Debug, PartialEq, Clone, Copy, TryFromPrimitive)]
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
    Struct = 30,
}

impl ColumnType {
    /// Parse a column type from u8
    pub fn parse(input: &[u8]) -> MltRefResult<'_, Self> {
        let (input, value) = utils::parse_u8(input)?;
        let value = Self::try_from(value).or(Err(ParsingColumnType(value)))?;
        Ok((input, value))
    }

    #[must_use]
    pub fn has_name(self) -> bool {
        #[allow(clippy::enum_glob_use)]
        use ColumnType::*;
        !matches!(self, Id | OptId | LongId | OptLongId | Geometry)
    }

    #[must_use]
    pub fn is_optional(self) -> bool {
        (self as u8) & 1 != 0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, TryFromPrimitive)]
#[repr(u8)]
pub enum GeometryType {
    Point,
    LineString,
    Polygon,
    MultiPoint,
    MultiLineString,
    MultiPolygon,
}

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub enum VertexBufferType {
    Morton,
    Vec2,
    Vec3,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum VectorType {
    Flat,
    Const,
    Sequence,
    // Dictionary,
    // FsstDictionary,
}

#[derive(Debug, Clone, PartialEq)]
pub enum RawDecoded<'a, T: ToOwned, U> {
    Raw(&'a T),
    Decoded(U),
}
