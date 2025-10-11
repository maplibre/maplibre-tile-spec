// use borrowme::borrowme;

use integer_encoding::VarInt;

use crate::structures::enums::{DictionaryType, LengthType, LogicalTechnique, OffsetType, PhysicalTechnique};
use crate::structures::v1::{Geometry};
use crate::{MltError, MltResult};
use crate::utils::parse_varint_vec;

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

//#[borrowme]
// #[derive(Debug, PartialEq)]
// pub enum StreamType<'a> {
// Alp,
// CompDelta2,
// DeltaCompDeltaAlp,
// DeltaFastPFOR,
// DeltaMorton,
// DeltaNoneVarInt,
// DeltaPseudoDecimal,
// DeltaPseudoDecimalAlp,
// DeltaPseudoDecimalVarInt,
// DeltaVarInt,
// DetaMortonVarInt,
// Morton,
// MortonRleFastPFOR,
// None,
// NoneCompDeltaAlp,
// NoneCompDeltaNone,
// NoneDelta,
// NoneDeltaAlp,
// NoneDeltaFastPFOR,
// NoneDeltaVarInt,
// NoneFastPFOR,
// NoneMorton,
// NoneMortonAlp,
// NoneMortonFastPFOR,
// NonePseudoDecimal,
// NonePseudoDecimalAlp,
// NoneRle,
// NoneRleFastPFOR,
// NoneRleVar,
// NoneRleVarInt,
// PseudoDecimal,
// RleVarInt,
// DeltaRle,
// Rle,
// ComponentwiseDeltaVarInt(DataComponentwiseDeltaVarInt<'a>),
// Raw(DataRaw<'a>),
// VarInt(DataVarInt<'a>),
// }

/// MVT-compatible feature table data
//#[borrowme]
#[derive(Debug, PartialEq)]
pub struct StreamMeta {
    pub physical_type: PhysicalStreamType,
    pub num_values: usize,
    pub logical_technique1: LogicalTechnique,
    pub logical_technique2: LogicalTechnique,
    pub physical_technique: PhysicalTechnique,
}


macro_rules! data {
    ($($enm:ident : $ty:ident),+ $(,)?) => {
        #[derive(Debug, PartialEq)]
        pub enum Stream<'a> {
            $($enm($ty<'a>),)+
        }

        $(
            #[derive(Debug, PartialEq)]
            pub struct $ty<'a> {
                pub meta: StreamMeta,
                pub data: &'a [u8],
            }
            impl<'a> $ty<'a> {
                pub fn new(meta: StreamMeta, data: &'a [u8]) -> Stream<'a> {
                    Stream::$enm(Self { meta, data } )
                }
            }
        )+
    };
}

data![
    ComponentwiseDeltaVarInt: DataComponentwiseDeltaVarInt,
    VarInt: DataVarInt,
    Raw: DataRaw,
];

impl<'a, T, U> TryFrom<DataVarInt<'a>> for Vec<U>
where
    T: VarInt,
    U: TryFrom<T>,
    MltError: From<<U as TryFrom<T>>::Error>,
{
    type Error = MltError;
    fn try_from(value: DataVarInt) -> MltResult<Self> {
        parse_varint_vec(value.data, value.meta.num_values)
    }
}

/// Column type enumeration
//#[borrowme]
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
