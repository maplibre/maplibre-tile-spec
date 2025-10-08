use borrowme::borrowme;
use num_enum::{TryFromPrimitive};

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum PhysicalStreamType {
    Present,
    Data(DictionaryType),
    Offset(OffsetType),
    Length(LengthType),
}

impl PhysicalStreamType {
    pub fn from_u8(value: u8) -> Option<Self> {
        let prefix = value >> 4;
        let suffix = value & 0x0F;
        Some(match prefix {
            0 => PhysicalStreamType::Present,
            1 => PhysicalStreamType::Data(DictionaryType::try_from(suffix).ok()?),
            2 => PhysicalStreamType::Offset(OffsetType::try_from(suffix).ok()?),
            3 => PhysicalStreamType::Length(LengthType::try_from(suffix).ok()?),
            _ => return None,
        })
    }
}

#[borrowme]
#[derive(Debug, PartialEq, Clone, Copy, TryFromPrimitive)]
#[repr(u8)]
pub enum LogicalLevelTechnique {
    None = 0,
    Delta = 1,
    ComponentwiseDelta = 2,
    Rle = 3,
    Morton = 4,
    PseudoDecimal = 5,
}

#[borrowme]
#[derive(Debug, PartialEq, Clone, Copy, TryFromPrimitive)]
#[repr(u8)]
pub enum PhysicalLevelTechnique {
    None = 0,
    /// Preferred, tends to produce the best compression ratio and decoding performance.
    /// But currently limited to 32-bit integer.
    FastPFOR = 1,
    /// Can produce better results in combination with a heavyweight compression scheme like Gzip.
    /// Simple compression scheme where the decoder are easier to implement compared to FastPfor.
    Varint = 2,
    /// Adaptive Lossless floating-Point Compression
    Alp = 3,
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
