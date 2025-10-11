use crate::{MltError, };
use crate::structures::enums::{
    DictionaryType, LengthType, LogicalTechnique, OffsetType, PhysicalTechnique,
};
use crate::structures::v1::Geometry;
use crate::utils::{decode_componentwise_delta_vec2s, parse_varint_vec};

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

macro_rules! stream_data {
    ($($enm:ident : $ty:ident),+ $(,)?) => {
        #[derive(Debug, PartialEq)]
        pub enum StreamData<'a> {
            $($enm($ty<'a>),)+
        }

        $(
            #[derive(Debug, PartialEq)]
            pub struct $ty<'a> {
                pub data: &'a [u8],
            }
            impl<'a> $ty<'a> {
                pub fn new(data: &'a [u8]) -> StreamData<'a> {
                    StreamData::$enm(Self { data } )
                }
            }
        )+
    };
}

stream_data![
    VarInt: DataVarInt,
    Raw: DataRaw,
];

macro_rules! stream_logical_data {
    ($($enm:ident : $ty:ident($ty2:ty)),+ $(,)?) => {
        #[derive(Debug, PartialEq)]
        pub enum LogicalStreamData {
            $($enm($ty),)+
        }

        $(
            #[derive(Debug, PartialEq)]
            pub struct $ty {
                pub data: $ty2,
            }
            impl $ty {
                pub fn new(data: $ty2) -> LogicalStreamData {
                    LogicalStreamData::$enm(Self { data } )
                }
            }
        )+
    };
}

stream_logical_data![
    None: LogDataNone(Vec<u32>),
    ComponentwiseDelta: LogDataComponentwiseDelta(Vec<u32>),
];

#[derive(Debug, PartialEq)]
pub struct Stream<'a> {
    pub meta: StreamMeta,
    pub data: StreamData<'a>,
}

impl<'a> Stream<'a> {
    pub fn new(meta: StreamMeta, data: StreamData<'a>) -> Self {
        Self { meta, data }
    }
}

#[derive(Debug, PartialEq)]
pub struct LogicalStream2<T> {
    pub meta: StreamMeta,
    pub data: Vec<T>,
}

impl<T> LogicalStream2<T> {
    pub fn new(meta: StreamMeta, data: Vec<T>) -> Self {
        Self { meta, data }
    }
}
impl LogicalStream2<u32> {
    pub fn u32(self) -> Result<LogicalStream, MltError> {
        Ok(match self.meta.logical_technique1 {
            LogicalTechnique::None => LogicalStream {
                meta: self.meta,
                data: LogDataNone::new(self.data),
            },
            LogicalTechnique::ComponentwiseDelta => LogicalStream {
                meta: self.meta,
                data: LogDataComponentwiseDelta::new(self.data),
            },
            _ => panic!("Unsupported logical technique for i32"),
        })
    }
}

#[derive(Debug, PartialEq)]
pub struct LogicalStream {
    pub meta: StreamMeta,
    pub data: LogicalStreamData,
}

impl LogicalStream {
    pub fn decode_i32(self) -> Result<Vec<i32>, MltError> {
        match self.data {
            LogicalStreamData::ComponentwiseDelta(value) => {
                decode_componentwise_delta_vec2s(&value.data)
            }
            _ => panic!("Unsupported logical technique for i32"),
        }
    }

    pub fn decode_u32(self) -> Result<Vec<u32>, MltError> {
        match self.data {
            LogicalStreamData::None(value) => Ok(value.data),
            _ => panic!("Unsupported logical technique for u32"),
        }
    }
}

impl<'a> LogicalStream {
    pub fn new(meta: StreamMeta, data: LogicalStreamData) -> Self {
        Self { meta, data }
    }
}

pub trait TryFromStream<T>: Sized {
    fn try_from2(value: T, meta: &StreamMeta) -> Result<Self, MltError>;
}

impl<'a, T> TryFromStream<DataVarInt<'a>> for Vec<T>
where
    T: TryFrom<u64>,
    MltError: From<<T as TryFrom<u64>>::Error>,
{
    fn try_from2(value: DataVarInt, meta: &StreamMeta) -> Result<Vec<T>, MltError> {
        let (_, result) = parse_varint_vec(value.data, meta.num_values)?;
        Ok(result)
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

#[derive(Debug, PartialEq)]
pub enum Decoder {
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
