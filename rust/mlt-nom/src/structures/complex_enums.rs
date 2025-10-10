use borrowme::borrowme;

use crate::structures::enums::{DictionaryType, LengthType, OffsetType};
use crate::structures::v1::{Geometry, Stream};

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum PhysicalStreamType {
    Present,
    Data(DictionaryType),
    Offset(OffsetType),
    Length(LengthType),
}

impl PhysicalStreamType {
    pub fn from_u8(value: u8) -> Option<Self> {
        let high4 = value >> 4;
        let low4 = value & 0x0F;
        Some(match high4 {
            0 => PhysicalStreamType::Present,
            1 => PhysicalStreamType::Data(DictionaryType::try_from(low4).ok()?),
            2 => PhysicalStreamType::Offset(OffsetType::try_from(low4).ok()?),
            3 => PhysicalStreamType::Length(LengthType::try_from(low4).ok()?),
            _ => return None,
        })
    }
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum StreamType {
    Alp,
    CompDelta2,
    ComponentwiseDeltaVarInt,
    DeltaCompDeltaAlp,
    DeltaFastPFOR,
    DeltaVarInt,
    Morton,
    None,
    NoneCompDeltaAlp,
    NoneDelta,
    NoneDeltaAlp,
    NoneDeltaFastPFOR,
    NoneMortonAlp,
    NoneRle,
    PseudoDecimal,
    Rle,
    RleVarInt,
    VarInt,
    DeltaNoneVarInt,
    NoneMorton,
    NoneFastPFOR,
    NoneRleFastPFOR,
    DeltaPseudoDecimalVarInt,
    NoneMortonFastPFOR,
    DeltaPseudoDecimalAlp,
    NonePseudoDecimalAlp,
    NonePseudoDecimal,
    NoneDeltaVarInt,
    DeltaPseudoDecimal,
    DetaMortonVarInt,
    NoneRleVar,
    NoneRleVarInt,
    DeltaMorton,
    MortonRleFastPFOR,
    NoneCompDeltaNone,
    DeltaRle,
}

/// Column type enumeration
#[borrowme]
#[derive(Debug, PartialEq)]
pub enum ColumnStreams<'a> {
    Id(Stream<'a>),
    OptId(Stream<'a>, Stream<'a>),
    LongId(Stream<'a>),
    OptLongId(Stream<'a>, Stream<'a>),
    Geometry(Geometry),
    Bool(&'a str, Stream<'a>),
    OptBool(&'a str, Stream<'a>, Stream<'a>),
    I8(&'a str, Stream<'a>),
    OptI8(&'a str, Stream<'a>, Stream<'a>),
    U8(&'a str, Stream<'a>),
    OptU8(&'a str, Stream<'a>, Stream<'a>),
    I32(&'a str, Stream<'a>),
    OptI32(&'a str, Stream<'a>, Stream<'a>),
    U32(&'a str, Stream<'a>),
    OptU32(&'a str, Stream<'a>, Stream<'a>),
    I64(&'a str, Stream<'a>),
    OptI64(&'a str, Stream<'a>, Stream<'a>),
    U64(&'a str, Stream<'a>),
    OptU64(&'a str, Stream<'a>, Stream<'a>),
    F32(&'a str, Stream<'a>),
    OptF32(&'a str, Stream<'a>, Stream<'a>),
    F64(&'a str, Stream<'a>),
    OptF64(&'a str, Stream<'a>, Stream<'a>),
    Str(&'a str, Stream<'a>),
    OptStr(&'a str, Stream<'a>, Stream<'a>),
    Struct(&'a str, Stream<'a>),
}

pub enum _Decoder {
    None,
    Delta,
    DeltaRle {
        runs: u32,
        num_rle_values: u32,
    },
    ComponentwiseDelta,
    Rle {
        runs: u32,
        num_rle_values: u32,
    },
    Morton {
        num_bits: u32,
        coordinate_shift: u32,
    },
    PseudoDecimal,
}
