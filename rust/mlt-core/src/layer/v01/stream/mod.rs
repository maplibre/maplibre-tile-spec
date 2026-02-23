mod decode;
mod logical;
mod physical;

use std::fmt::Debug;
use std::io::Write;
use std::{fmt, io};

use borrowme::borrowme;
use integer_encoding::VarIntWriter as _;
use num_enum::TryFromPrimitive;

use crate::analyse::{Analyze, StatType};
use crate::utils::{
    BinarySerializer as _, all, decode_byte_rle, decode_bytes_to_bools, decode_bytes_to_u32s,
    decode_bytes_to_u64s, encode_bools_to_bytes, encode_byte_rle, parse_u8, parse_varint,
    parse_varint_vec, take,
};
use crate::v01::stream::decode::decode_fastpfor_composite;
pub use crate::v01::stream::logical::{
    LogicalCodec, LogicalData, LogicalEncoding, LogicalTechnique, LogicalValue,
};
pub use crate::v01::stream::physical::{PhysicalCodec, PhysicalEncoding, PhysicalStreamType};
use crate::{MltError, MltRefResult};

#[derive(Debug, Eq, PartialEq, Clone, Copy)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub struct Encoding {
    pub logical: LogicalEncoding,
    pub physical: PhysicalEncoding,
}

impl Encoding {
    #[must_use]
    pub const fn new(logical: LogicalEncoding, physical: PhysicalEncoding) -> Self {
        Self { logical, physical }
    }
}

/// Representation of an encoded stream
#[borrowme]
#[derive(Debug, PartialEq)]
pub struct Stream<'a> {
    pub meta: StreamMeta,
    pub data: StreamData<'a>,
}

impl Analyze for Stream<'_> {
    fn for_each_stream(&self, cb: &mut dyn FnMut(&Stream<'_>)) {
        cb(self);
    }
}

impl OwnedStream {
    /// Creates an empty stream
    #[must_use]
    pub fn empty_without_codec() -> Self {
        Self {
            meta: StreamMeta {
                physical_type: PhysicalStreamType::Data(DictionaryType::None),
                num_values: 0,
                logical_codec: LogicalCodec::None,
                physical_codec: PhysicalCodec::None,
            },
            data: OwnedStreamData::Encoded(OwnedEncodedData { data: Vec::new() }),
        }
    }

    /// Creates a plain stream with values encoded literally
    #[must_use]
    fn new_plain(data: Vec<u8>, num_values: u32) -> OwnedStream {
        let meta = StreamMeta {
            physical_type: PhysicalStreamType::Data(DictionaryType::None),
            num_values,
            logical_codec: LogicalCodec::None,
            physical_codec: PhysicalCodec::None,
        };
        let data = OwnedStreamData::Encoded(OwnedEncodedData { data });
        Self { meta, data }
    }

    /// Encode a boolean stream: byte-RLE <- packed bitmap <- `Vec<bool>`
    pub fn encode_bools(values: &[bool]) -> Result<Self, MltError> {
        let num_values = u32::try_from(values.len())?;
        let bytes = encode_bools_to_bytes(values);
        let data = encode_byte_rle(&bytes);
        // byte RLE is how bits are always encoded, not rle -> plain
        Ok(Self::new_plain(data, num_values))
    }

    /// Encodes `f32`s into a stream
    pub fn encode_f32(values: &[f32]) -> Result<Self, MltError> {
        let num_values = u32::try_from(values.len())?;
        let data = values
            .iter()
            .flat_map(|f| f.to_le_bytes())
            .collect::<Vec<u8>>();

        Ok(Self::new_plain(data, num_values))
    }

    pub fn encode_i8s(values: &[i8], encoding: Encoding) -> Result<Self, MltError> {
        let as_i32: Vec<i32> = values.iter().map(|&v| i32::from(v)).collect();
        let (physical_u32s, logical_codec) = encoding.logical.encode_i32s(&as_i32)?;
        let num_values = u32::try_from(physical_u32s.len())?;
        let (data, physical_codec) = encoding.physical.encode_u32s(physical_u32s);
        Ok(Self {
            meta: StreamMeta {
                physical_type: PhysicalStreamType::Data(DictionaryType::None),
                num_values,
                logical_codec,
                physical_codec,
            },
            data,
        })
    }
    pub fn encode_u8s(values: &[u8], encoding: Encoding) -> Result<Self, MltError> {
        let as_u32: Vec<u32> = values.iter().map(|&v| u32::from(v)).collect();
        let (physical_u32s, logical_codec) = encoding.logical.encode_u32s(&as_u32)?;
        let num_values = u32::try_from(physical_u32s.len())?;
        let (data, physical_codec) = encoding.physical.encode_u32s(physical_u32s);
        Ok(Self {
            meta: StreamMeta {
                physical_type: PhysicalStreamType::Data(DictionaryType::None),
                num_values,
                logical_codec,
                physical_codec,
            },
            data,
        })
    }
    pub fn encode_i32s(values: &[i32], encoding: Encoding) -> Result<Self, MltError> {
        let (physical_u32s, logical_codec) = encoding.logical.encode_i32s(values)?;
        let num_values = u32::try_from(physical_u32s.len())?;
        let (data, physical_codec) = encoding.physical.encode_u32s(physical_u32s);
        Ok(Self {
            meta: StreamMeta {
                physical_type: PhysicalStreamType::Data(DictionaryType::None),
                num_values,
                logical_codec,
                physical_codec,
            },
            data,
        })
    }
    pub fn encode_u32s(values: &[u32], encoding: Encoding) -> Result<Self, MltError> {
        Self::encode_u32s_of_type(
            values,
            encoding,
            PhysicalStreamType::Data(DictionaryType::None),
        )
    }
    pub fn encode_u32s_of_type(
        values: &[u32],
        encoding: Encoding,
        physical_type: PhysicalStreamType,
    ) -> Result<Self, MltError> {
        let Encoding { logical, physical } = encoding;
        Self::encode_u32s_of_type(
            values,
            logical,
            physical,
            PhysicalStreamType::Data(DictionaryType::None),
        )
    }
    pub fn encode_u32s_of_type(
        values: &[u32],
        logical: LogicalEncoding,
        physical: PhysicalEncoding,
        physical_type: PhysicalStreamType,
    ) -> Result<Self, MltError> {
        let (physical_u32s, logical_codec) = logical.encode_u32s(values)?;
        let num_values = u32::try_from(physical_u32s.len())?;
        let (data, physical_codec) = physical.encode_u32s(physical_u32s);
        Ok(Self {
            meta: StreamMeta {
                physical_type,
                num_values,
                logical_codec,
                physical_codec,
            },
            data,
        })
    }

    pub fn encode_i64s(values: &[i64], encoding: Encoding) -> Result<Self, MltError> {
        let (physical_u64s, logical_codec) = encoding.logical.encode_i64s(values)?;
        let num_values = u32::try_from(physical_u64s.len())?;
        let (data, physical_codec) = encoding.physical.encode_u64s(physical_u64s);
        Ok(Self {
            meta: StreamMeta {
                physical_type: PhysicalStreamType::Data(DictionaryType::None),
                num_values,
                logical_codec,
                physical_codec,
            },
            data,
        })
    }
    pub fn encode_u64s(values: &[u64], encoding: Encoding) -> Result<Self, MltError> {
        let (physical_u64s, logical_codec) = encoding.logical.encode_u64s(values)?;
        let num_values = u32::try_from(physical_u64s.len())?;
        let (data, physical_codec) = encoding.physical.encode_u64s(physical_u64s);
        Ok(Self {
            meta: StreamMeta {
                physical_type: PhysicalStreamType::Data(DictionaryType::None),
                num_values,
                logical_codec,
                physical_codec,
            },
            data,
        })
    }

    /// Encode a sequence of strings into a length stream and a data stream.
    pub fn encode_strings(
        values: &[String],
        logical: LogicalEncoding,
        physical: PhysicalEncoding,
    ) -> Result<Vec<Self>, MltError> {
        let lengths: Vec<u32> = values
            .iter()
            .map(|s| u32::try_from(s.len()))
            .collect::<Result<Vec<_>, _>>()?;
        let data: Vec<u8> = values
            .iter()
            .flat_map(|s| s.as_bytes().iter().copied())
            .collect();

        let length_stream = Self::encode_u32s_of_type(
            &lengths,
            logical,
            physical,
            PhysicalStreamType::Length(LengthType::VarBinary),
        )?;

        let data_stream = Self::new_plain(data, u32::try_from(values.len())?);

        Ok(vec![length_stream, data_stream])
    }
}
/// Metadata about an encoded stream
#[derive(Clone, Copy, PartialEq)]
pub struct StreamMeta {
    pub physical_type: PhysicalStreamType,
    pub num_values: u32,
    pub logical_codec: LogicalCodec,
    pub physical_codec: PhysicalCodec,
}
impl StreamMeta {
    /// Parse stream from the input
    ///
    /// If `is_bool` is true, compute RLE parameters for boolean streams
    /// automatically instead of reading them from the input.
    ///
    /// Returns the stream metadata and the size of the stream in bytes
    fn parse(input: &[u8], is_bool: bool) -> MltRefResult<'_, (Self, u32)> {
        use crate::v01::LogicalTechnique as LT;

        let (input, physical_type) = PhysicalStreamType::parse(input)?;
        let (input, val) = parse_u8(input)?;
        let logical1 = LT::parse(val >> 5)?;
        let logical2 = LT::parse((val >> 2) & 0x7)?;
        let physical_codec = PhysicalCodec::parse(val & 0x3)?;

        let (input, num_values) = parse_varint::<u32>(input)?;
        let (input, byte_length) = parse_varint::<u32>(input)?;

        let mut input = input;
        let logical_codec = match (logical1, logical2) {
            (LT::None, LT::None) => LogicalCodec::None,
            (LT::Delta, LT::None) => LogicalCodec::Delta,
            (LT::ComponentwiseDelta, LT::None) => LogicalCodec::ComponentwiseDelta,
            (LT::Delta, LT::Rle) | (LT::Rle, LT::None) => {
                let runs;
                let num_rle_values;
                if is_bool {
                    runs = num_values.div_ceil(8);
                    num_rle_values = byte_length;
                } else {
                    (input, runs) = parse_varint::<u32>(input)?;
                    (input, num_rle_values) = parse_varint::<u32>(input)?;
                }
                let rle = RleMeta {
                    runs,
                    num_rle_values,
                };
                if logical1 == LT::Rle {
                    LogicalCodec::Rle(rle)
                } else {
                    LogicalCodec::DeltaRle(rle)
                }
            }
            (LT::Morton, LT::None) => {
                let num_bits;
                let coordinate_shift;
                (input, num_bits) = parse_varint::<u32>(input)?;
                (input, coordinate_shift) = parse_varint::<u32>(input)?;
                LogicalCodec::Morton(MortonMeta {
                    num_bits,
                    coordinate_shift,
                })
            }
            (LT::PseudoDecimal, LT::None) => LogicalCodec::PseudoDecimal,
            _ => Err(MltError::InvalidLogicalEncodings(logical1, logical2))?,
        };

        let meta = StreamMeta {
            physical_type,
            num_values,
            logical_codec,
            physical_codec,
        };
        Ok((input, (meta, byte_length)))
    }

    pub fn write_to<W: Write>(
        &self,
        writer: &mut W,
        is_bool: bool,
        byte_length: u32,
    ) -> io::Result<()> {
        use crate::v01::LogicalTechnique as LT;
        writer.write_u8(self.physical_type.as_u8())?;
        let logical_codec_u8: u8 = match self.logical_codec {
            LogicalCodec::None => (LT::None as u8) << 5,
            LogicalCodec::Delta => (LT::Delta as u8) << 5,
            LogicalCodec::DeltaRle(_) => ((LT::Delta as u8) << 5) | ((LT::Rle as u8) << 2),
            LogicalCodec::ComponentwiseDelta => (LT::ComponentwiseDelta as u8) << 5,
            LogicalCodec::Rle(_) => (LT::Rle as u8) << 5,
            LogicalCodec::Morton(_) => (LT::Morton as u8) << 5,
            LogicalCodec::PseudoDecimal => (LT::PseudoDecimal as u8) << 5,
        };
        let physical_codec_u8: u8 = match self.physical_codec {
            PhysicalCodec::None => 0x0,
            PhysicalCodec::FastPFOR => 0x1,
            PhysicalCodec::VarInt => 0x2,
            PhysicalCodec::Alp => 0x3,
        };
        writer.write_u8(logical_codec_u8 | physical_codec_u8)?;
        writer.write_varint(self.num_values)?;
        writer.write_varint(byte_length)?;

        // some codecs have settings inside them
        match self.logical_codec {
            LogicalCodec::DeltaRle(r) | LogicalCodec::Rle(r) => {
                if !is_bool {
                    writer.write_varint(r.runs)?;
                    writer.write_varint(r.num_rle_values)?;
                }
            }
            LogicalCodec::Morton(m) => {
                writer.write_varint(m.num_bits)?;
                writer.write_varint(m.coordinate_shift)?;
            }
            LogicalCodec::None
            | LogicalCodec::Delta
            | LogicalCodec::ComponentwiseDelta
            | LogicalCodec::PseudoDecimal => {}
        }
        Ok(())
    }
}

impl Analyze for StreamMeta {
    fn collect_statistic(&self, stat: StatType) -> usize {
        if stat == StatType::DecodedMetaSize {
            size_of::<Self>()
        } else {
            0
        }
    }
}

impl Debug for StreamMeta {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // ensure we process all fields, and format them without the alt field
        let Self {
            physical_type,
            num_values,
            logical_codec,
            physical_codec,
        } = self;
        f.debug_struct("StreamMeta")
            .field("physical_type", &format_args!("{physical_type:?}"))
            .field("num_values", &format_args!("{num_values:?}"))
            .field("logical_codec", &format_args!("{logical_codec:?}"))
            .field("physical_codec", &format_args!("{physical_codec:?}"))
            .finish()
    }
}

/// Dictionary type used for a column, as stored in the tile
#[borrowme]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, TryFromPrimitive)]
#[repr(u8)]
pub enum DictionaryType {
    None = 0,
    Single = 1,
    Shared = 2,
    Vertex = 3,
    Morton = 4,
    Fsst = 5,
}

/// Offset type used for a column, as stored in the tile
#[borrowme]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, TryFromPrimitive)]
#[repr(u8)]
pub enum OffsetType {
    Vertex = 0,
    Index = 1,
    String = 2,
    Key = 3,
}

/// Length type used for a column, as stored in the tile
#[borrowme]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, TryFromPrimitive)]
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

/// Metadata for RLE decoding
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RleMeta {
    pub runs: u32,
    pub num_rle_values: u32,
}

/// Metadata for Morton decoding
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MortonMeta {
    pub num_bits: u32,
    pub coordinate_shift: u32,
}

/// Representation of the raw stream data, in various physical formats
macro_rules! stream_data {
    ($($enm:ident : $ty:ident / $owned:ident),+ $(,)?) => {
        #[borrowme]
        #[derive(Debug, PartialEq)]
        pub enum StreamData<'a> {
            $($enm($ty<'a>),)+
        }

    impl crate::Analyze for StreamData<'_> {
        fn collect_statistic(&self, stat: crate::StatType) -> usize {
            match &self {
                $(StreamData::$enm(d) => d.data.collect_statistic(stat),)+
            }
        }
    }

        $(
            #[borrowme]
            #[derive(PartialEq)]
            pub struct $ty<'a> {
                #[borrowme(borrow_with = Vec::as_slice)]
                pub data: &'a [u8],
            }
            impl<'a> $ty<'a> {
                #[expect(clippy::new_ret_no_self)]
                pub fn new(data: &'a [u8]) -> StreamData<'a> {
                    StreamData::$enm(Self { data } )
                }
            }
            impl<'a> Debug for $ty<'a> {
                fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                    $crate::utils::fmt_byte_array(self.data, f)
                }
            }
            impl<'a> Debug for $owned {
                fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                    $crate::utils::fmt_byte_array(&self.data, f)
                }
            }
        )+
    };
}

stream_data![
    VarInt: DataVarInt / OwnedDataVarInt,
    Encoded: EncodedData / OwnedEncodedData,
];

impl OwnedStreamData {
    pub fn write_to<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        match self {
            OwnedStreamData::VarInt(d) => writer.write_all(&d.data),
            OwnedStreamData::Encoded(d) => writer.write_all(&d.data),
        }
    }
}

impl<'a> Stream<'a> {
    #[must_use]
    pub fn new(meta: StreamMeta, data: StreamData<'a>) -> Self {
        Self { meta, data }
    }

    pub fn parse(input: &'a [u8]) -> MltRefResult<'a, Self> {
        Self::parse_internal(input, false)
    }

    pub fn parse_multiple(mut input: &'a [u8], count: usize) -> MltRefResult<'a, Vec<Self>> {
        let mut result = Vec::with_capacity(count);
        for _ in 0..count {
            let stream;
            (input, stream) = Stream::parse_internal(input, false)?;
            result.push(stream);
        }
        Ok((input, result))
    }

    pub fn parse_bool(input: &'a [u8]) -> MltRefResult<'a, Self> {
        Self::parse_internal(input, true)
    }

    /// Parse stream from the input
    /// If `is_bool` is true, compute RLE parameters for boolean streams
    /// automatically instead of reading them from the input.
    fn parse_internal(input: &'a [u8], is_bool: bool) -> MltRefResult<'a, Self> {
        use PhysicalCodec as PD;
        let (input, (meta, byte_length)) = StreamMeta::parse(input, is_bool)?;

        let (input, data) = take(input, usize::try_from(byte_length)?)?;

        let stream_data = match meta.physical_codec {
            PD::None | PD::FastPFOR => EncodedData::new(data),
            PD::VarInt => DataVarInt::new(data),
            PD::Alp => return Err(MltError::UnsupportedPhysicalCodec("ALP")),
        };

        Ok((input, Stream::new(meta, stream_data)))
    }

    /// Decode a boolean stream: byte-RLE → packed bitmap → `Vec<bool>`
    pub fn decode_bools(self) -> Result<Vec<bool>, MltError> {
        let num_values = self.meta.num_values as usize;
        let num_bytes = num_values.div_ceil(8);
        let raw = match &self.data {
            StreamData::Encoded(d) => d.data,
            StreamData::VarInt(_) => {
                return Err(MltError::NotImplemented("varint bool decoding"));
            }
        };
        let decoded = decode_byte_rle(raw, num_bytes);
        Ok(decode_bytes_to_bools(&decoded, num_values))
    }

    pub fn decode_i8s(self) -> Result<Vec<i8>, MltError> {
        self.decode_bits_u32()?
            .decode_i32()?
            .into_iter()
            .map(i8::try_from)
            .collect::<Result<Vec<_>, _>>()
            .map_err(Into::into)
    }

    pub fn decode_u8s(self) -> Result<Vec<u8>, MltError> {
        let decoded = self
            .decode_bits_u32()?
            .decode_u32()?
            .into_iter()
            .map(u8::try_from)
            .collect::<Result<Vec<u8>, _>>()?;
        Ok(decoded)
    }

    pub fn decode_i32s(self) -> Result<Vec<i32>, MltError> {
        self.decode_bits_u32()?.decode_i32()
    }

    pub fn decode_u32s(self) -> Result<Vec<u32>, MltError> {
        self.decode_bits_u32()?.decode_u32()
    }

    pub fn decode_bits_u32(self) -> Result<LogicalValue, MltError> {
        let value = match self.meta.physical_codec {
            PhysicalCodec::VarInt => match self.data {
                StreamData::VarInt(data) => all(parse_varint_vec::<u32, u32>(
                    data.data,
                    self.meta.num_values,
                )?),
                StreamData::Encoded(_) => {
                    return Err(MltError::StreamDataMismatch("VarInt", "Encoded"));
                }
            },
            PhysicalCodec::None => match self.data {
                StreamData::Encoded(data) => {
                    all(decode_bytes_to_u32s(data.data, self.meta.num_values)?)
                }
                StreamData::VarInt(_) => {
                    return Err(MltError::StreamDataMismatch("Encoded", "VarInt"));
                }
            },
            PhysicalCodec::FastPFOR => match self.data {
                StreamData::Encoded(data) => Ok(decode_fastpfor_composite(
                    data.data,
                    self.meta.num_values as usize,
                )?),
                StreamData::VarInt(_) => {
                    return Err(MltError::StreamDataMismatch("Encoded", "VarInt"));
                }
            },
            PhysicalCodec::Alp => return Err(MltError::UnsupportedPhysicalCodec("ALP")),
        }?;

        Ok(LogicalValue::new(self.meta, LogicalData::VecU32(value)))
    }

    pub fn decode_u64(self) -> Result<Vec<u64>, MltError> {
        self.decode_bits_u64()?.decode_u64()
    }
    /// Decode a signed i64 stream
    pub fn decode_i64(self) -> Result<Vec<i64>, MltError> {
        self.decode_bits_u64()?.decode_i64()
    }

    pub fn decode_bits_u64(self) -> Result<LogicalValue, MltError> {
        let value = match self.meta.physical_codec {
            PhysicalCodec::VarInt => match self.data {
                StreamData::VarInt(data) => all(parse_varint_vec::<u64, u64>(
                    data.data,
                    self.meta.num_values,
                )?),
                StreamData::Encoded(_) => {
                    return Err(MltError::StreamDataMismatch("VarInt", "Encoded"));
                }
            },
            PhysicalCodec::None => match self.data {
                StreamData::Encoded(data) => {
                    all(decode_bytes_to_u64s(data.data, self.meta.num_values)?)
                }
                StreamData::VarInt(_) => {
                    return Err(MltError::StreamDataMismatch("Encoded", "VarInt"));
                }
            },
            PhysicalCodec::FastPFOR => {
                return Err(MltError::UnsupportedPhysicalCodec("FastPFOR"));
            }
            PhysicalCodec::Alp => return Err(MltError::UnsupportedPhysicalCodec("ALP")),
        }?;

        Ok(LogicalValue::new(self.meta, LogicalData::VecU64(value)))
    }

    /// Decode a stream of f32 values from raw little-endian bytes
    pub fn decode_f32(self) -> Result<Vec<f32>, MltError> {
        let raw = match &self.data {
            StreamData::Encoded(d) => d.data,
            StreamData::VarInt(_) => {
                return Err(MltError::NotImplemented("varint f32 decoding"));
            }
        };
        let num = self.meta.num_values as usize;
        Ok(raw
            .chunks_exact(4)
            .map(|chunk| {
                // `chunks_exact(4)` guarantees `chunk` has length 4, so this is infallible.
                let bytes = [chunk[0], chunk[1], chunk[2], chunk[3]];
                f32::from_le_bytes(bytes)
            })
            .take(num)
            .collect())
    }

    // pub fn decode<'a, T, U>(&'_ self) -> Result<Vec<U>, MltError>
    // where
    //     T: VarInt,
    //     U: TryFrom<T>, // + ZigZag,
    //     MltError: From<<U as TryFrom<T>>::Error>,
    // {
    //     match &self.stream {
    //         StreamType::VarInt(data) => all(parse_varint_vec::<T, U>(data, self.num_values)?),
    //         StreamType::ComponentwiseDeltaVarInt(data) => {
    //             // let physical_decode = all(parse_varint_vec::<T, U>(self.data, self.num_values)?)?;
    //             todo!();
    //             // decode_componentwise_delta_vec2s(physical_decode.as_slice())
    //         }
    //         _ => panic!("Unsupported physical type: {:?}", self.stream),
    //     }
    // }

    // pub fn decode2<'a>(&'_ self) -> MltResult<'_, Vec<u32>> {
    //     match self.physical_type {
    //         PhysicalStreamType::Present => {
    //             todo!()
    //         }
    //         PhysicalStreamType::Data(_v) => parse_varint_vec::<u32, u32>(&[], self.num_values),
    //         PhysicalStreamType::Offset(_v) => {
    //             todo!()
    //         }
    //         PhysicalStreamType::Length(_v) => {
    //             todo!()
    //         }
    //     }
    // }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::v01::property::decode::decode_string_streams;

    /// Test case for stream decoding tests
    #[derive(Debug)]
    struct StreamTestCase {
        name: &'static str,
        meta: StreamMeta,
        data: &'static [u8],
        expected_u32_logical_value: Option<LogicalValue>,
        expected_u64_logical_value: Option<LogicalValue>,
    }

    /// Generator function that creates a set of test cases for stream decoding
    fn generate_stream_test_cases() -> Vec<StreamTestCase> {
        vec![
            // Basic VarInt test case
            StreamTestCase {
                name: "simple_varint_u32",
                meta: StreamMeta {
                    physical_type: PhysicalStreamType::Data(DictionaryType::None),
                    num_values: 4,
                    logical_codec: LogicalCodec::None,
                    physical_codec: PhysicalCodec::VarInt,
                },
                data: &[0x04, 0x03, 0x02, 0x01],
                expected_u32_logical_value: Some(LogicalValue::new(
                    StreamMeta {
                        physical_type: PhysicalStreamType::Data(DictionaryType::None),
                        num_values: 4,
                        logical_codec: LogicalCodec::None,
                        physical_codec: PhysicalCodec::VarInt,
                    },
                    LogicalData::VecU32(vec![4, 3, 2, 1]),
                )),
                expected_u64_logical_value: None,
            },
            // Basic Encoded test case
            StreamTestCase {
                name: "simple_raw_bytes_to_u32",
                meta: StreamMeta {
                    physical_type: PhysicalStreamType::Data(DictionaryType::None),
                    num_values: 1,
                    logical_codec: LogicalCodec::None,
                    physical_codec: PhysicalCodec::None,
                },
                data: &[0x04, 0x03, 0x02, 0x01],
                expected_u32_logical_value: Some(LogicalValue::new(
                    StreamMeta {
                        physical_type: PhysicalStreamType::Data(DictionaryType::None),
                        num_values: 1,
                        logical_codec: LogicalCodec::None,
                        physical_codec: PhysicalCodec::None,
                    },
                    LogicalData::VecU32(vec![0x0102_0304]),
                )),
                expected_u64_logical_value: None,
            },
        ]
    }

    fn create_stream_from_test_case(test_case: &StreamTestCase) -> Stream<'_> {
        let data = match test_case.meta.physical_codec {
            PhysicalCodec::VarInt => DataVarInt::new(test_case.data),
            PhysicalCodec::None => EncodedData::new(test_case.data),
            _ => panic!(
                "Unsupported physical codec in test: {:?}",
                test_case.meta.physical_codec
            ),
        };
        Stream::new(test_case.meta, data)
    }

    #[test]
    fn test_decode_bits_u32() {
        let test_cases = generate_stream_test_cases();

        for test_case in test_cases {
            if let Some(expected_u32_logical_value) = &test_case.expected_u32_logical_value {
                let stream = create_stream_from_test_case(&test_case);
                let result = stream.decode_bits_u32();
                assert!(result.is_ok(), "Should successfully decode LogicalValue");
                let logical_value = result.unwrap();
                assert_eq!(
                    logical_value, *expected_u32_logical_value,
                    "Should produce LogicalValue correctly"
                );
            }
        }
    }

    fn make_logical_val(logical_codec: LogicalCodec, input_data: Vec<u32>) -> LogicalValue {
        let meta = StreamMeta {
            physical_type: PhysicalStreamType::Data(DictionaryType::None),
            num_values: u32::try_from(input_data.len()).expect("input_data length fits in u32"),
            logical_codec,
            physical_codec: PhysicalCodec::VarInt,
        };
        let data = LogicalData::VecU32(input_data);
        LogicalValue::new(meta, data)
    }

    #[rstest]
    // ZigZag pairs: [(0,0),(2,4),(2,4)] -> [(0,0),(1,2),(1,2)]
    // Delta: [(0,0),(1,2),(1,2)] -> [(0,0),(1,2),(2,4)]
    #[case::componentwise_delta(LogicalCodec::ComponentwiseDelta, vec![0, 0, 2, 4, 2, 4], vec![0, 0, 1, 2, 2, 4])]
    // ZigZag: [0,1,2,1,2] -> [0,-1,1,-1,1]
    // Delta: [0,-1,1,-1,1] -> [0,-1,0,-1,0]
    #[case::delta(LogicalCodec::Delta, vec![0, 1, 2, 1, 2], vec![0, -1, 0, -1, 0])]
    // RLE: [3,2] [0,2] -> [0,0,0,2,2]
    // ZigZag: [0,0,0,2,2] -> [0,0,0,1,1]
    // Delta: [0,0,0,1,1] -> [0,0,0,1,2]
    #[case::delta_rle(LogicalCodec::DeltaRle(RleMeta { runs: 2, num_rle_values: 5 }), vec![3, 2, 0, 2], vec![0, 0, 0, 1, 2])]
    #[case::delta(LogicalCodec::Delta, vec![], vec![])]
    fn test_decode_i32(
        #[case] logical_codec: LogicalCodec,
        #[case] input_data: Vec<u32>,
        #[case] expected: Vec<i32>,
    ) {
        let result = make_logical_val(logical_codec, input_data).decode_i32();
        assert!(result.is_ok(), "should decode successfully");
        assert_eq!(result.unwrap(), expected, "should match expected output");
    }

    #[rstest]
    #[case::empty(LogicalCodec::None, vec![], vec![])]
    #[case::new_encoded(LogicalCodec::None, vec![10, 20, 30, 40], vec![10, 20, 30, 40])]
    #[case::rle(LogicalCodec::Rle(RleMeta { runs: 3, num_rle_values: 6 }), vec![3, 2, 1, 10, 20, 30], vec![10, 10, 10, 20, 20, 30])]
    // ZigZag: [0,2,2,2,2] -> [0,1,1,1,1]
    // Delta: [0,1,1,1,1] -> [0,1,2,3,4]
    #[case::delta(LogicalCodec::Delta, vec![0, 2, 2, 2, 2], vec![0, 1, 2, 3, 4])]
    fn test_decode_u32(
        #[case] logical_codec: LogicalCodec,
        #[case] input_data: Vec<u32>,
        #[case] expected: Vec<u32>,
    ) {
        let result = make_logical_val(logical_codec, input_data).decode_u32();
        assert!(result.is_ok(), "should decode successfully");
        assert_eq!(result.unwrap(), expected, "should match expected output");
    }

    /// Test roundtrip: write -> parse -> equality for stream serialization
    #[rstest]
    #[case::new_encoded(PhysicalStreamType::Data(DictionaryType::None), 2, LogicalCodec::None, PhysicalCodec::None, vec![0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08], false)]
    #[case::new_encoded(PhysicalStreamType::Data(DictionaryType::None), 2, LogicalCodec::ComponentwiseDelta, PhysicalCodec::None, vec![0x00, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00], false)]
    #[case::new_encoded(PhysicalStreamType::Offset(OffsetType::Vertex), 3, LogicalCodec::None, PhysicalCodec::None, vec![0x00, 0x00, 0x00, 0x00, 0x04, 0x00, 0x00, 0x00, 0x08, 0x00, 0x00, 0x00], false)]
    #[case::varint(PhysicalStreamType::Data(DictionaryType::None), 4, LogicalCodec::None, PhysicalCodec::VarInt, vec![0x0A, 0x14, 0x1E, 0x28], false)]
    #[case::varint(PhysicalStreamType::Data(DictionaryType::None), 5, LogicalCodec::Delta, PhysicalCodec::VarInt, vec![0x00, 0x02, 0x02, 0x02, 0x02], false)]
    #[case::varint(PhysicalStreamType::Data(DictionaryType::None), 3, LogicalCodec::PseudoDecimal, PhysicalCodec::VarInt, vec![0x01, 0x02, 0x03], false)]
    #[case::varint(PhysicalStreamType::Length(LengthType::VarBinary), 3, LogicalCodec::Delta, PhysicalCodec::VarInt, vec![0x00, 0x02, 0x02], false)]
    #[case::rle(PhysicalStreamType::Data(DictionaryType::None), 6, LogicalCodec::Rle(RleMeta { runs: 3, num_rle_values: 3 }), PhysicalCodec::VarInt, vec![0x03, 0x02, 0x01, 0x0A, 0x14, 0x1E], false)]
    #[case::rle(PhysicalStreamType::Data(DictionaryType::None), 5, LogicalCodec::DeltaRle(RleMeta { runs: 2, num_rle_values: 5 }), PhysicalCodec::VarInt, vec![0x03, 0x02, 0x00, 0x02], false)]
    #[case::morton(PhysicalStreamType::Data(DictionaryType::Morton), 4, LogicalCodec::Morton(MortonMeta { num_bits: 32, coordinate_shift: 0 }), PhysicalCodec::VarInt, vec![0x01, 0x02, 0x03, 0x04], false)]
    #[case::boolean(PhysicalStreamType::Present, 16, LogicalCodec::Rle(RleMeta { runs: 2, num_rle_values: 2 }), PhysicalCodec::VarInt, vec![0xFF, 0x00], true)]
    fn test_stream_roundtrip(
        #[case] physical_type: PhysicalStreamType,
        #[case] num_values: u32,
        #[case] logical_codec: LogicalCodec,
        #[case] physical_codec: PhysicalCodec,
        #[case] data_bytes: Vec<u8>,
        #[case] is_bool: bool,
    ) {
        use crate::utils::BinarySerializer as _;

        let stream_data = match physical_codec {
            PhysicalCodec::None | PhysicalCodec::FastPFOR => {
                OwnedStreamData::Encoded(OwnedEncodedData { data: data_bytes })
            }
            PhysicalCodec::VarInt => OwnedStreamData::VarInt(OwnedDataVarInt { data: data_bytes }),
            PhysicalCodec::Alp => panic!("ALP not supported"),
        };
        let stream = OwnedStream {
            meta: StreamMeta {
                physical_type,
                num_values,
                logical_codec,
                physical_codec,
            },
            data: stream_data,
        };

        // Write to buffer
        let mut buffer = Vec::new();
        if is_bool {
            buffer.write_boolean_stream(&stream).unwrap();
        } else {
            buffer.write_stream(&stream).unwrap();
        }

        // Parse back
        let (remaining, parsed) = if is_bool {
            Stream::parse_bool(&buffer).unwrap()
        } else {
            Stream::parse(&buffer).unwrap()
        };

        assert!(remaining.is_empty(), "{} bytes remain", remaining.len());
        assert_eq!(parsed.meta, stream.meta, "metadata mismatch");

        match (&stream.data, &parsed.data) {
            (OwnedStreamData::Encoded(exp), StreamData::Encoded(act)) => {
                assert_eq!(exp.data.as_slice(), act.data, "raw data mismatch");
            }
            (OwnedStreamData::VarInt(exp), StreamData::VarInt(act)) => {
                assert_eq!(exp.data.as_slice(), act.data, "varint data mismatch");
            }
            _ => panic!("data type mismatch"),
        }
    }
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn test_u32_roundtrip(
            values in prop::collection::vec(any::<u32>(), 0..100),
            encoding in any::<Encoding>()
        ) {
            let owned_stream = OwnedStream::encode_u32s(&values, encoding).unwrap();

                let mut buffer = Vec::new();
                buffer.write_stream(&owned_stream).unwrap();

                let (remaining, parsed_stream) = Stream::parse(&buffer).unwrap();
                assert!(remaining.is_empty());

                let decoded_values = parsed_stream.decode_bits_u32().unwrap().decode_u32().unwrap();

                assert_eq!(decoded_values, values);
            }

        #[test]
        fn test_i32_roundtrip(
            values in prop::collection::vec(any::<i32>(), 0..100),
            encoding in any::<Encoding>(),
        ) {
            let owned_stream = OwnedStream::encode_i32s(&values, encoding).unwrap();

                let mut buffer = Vec::new();
                buffer.write_stream(&owned_stream).unwrap();

                let (remaining, parsed_stream) = Stream::parse(&buffer).unwrap();
                assert!(remaining.is_empty());

                let decoded_values = parsed_stream.decode_bits_u32().unwrap().decode_i32().unwrap();

                assert_eq!(decoded_values, values);
            }

        #[test]
        fn test_u64_roundtrip(
            values in prop::collection::vec(any::<u64>(), 0..100),
            encoding in any::<Encoding>(),
        ) {
            let owned_stream = OwnedStream::encode_u64s(&values, encoding).unwrap();

                let mut buffer = Vec::new();
                buffer.write_stream(&owned_stream).unwrap();

                let (remaining, parsed_stream) = Stream::parse(&buffer).unwrap();
                assert!(remaining.is_empty());

                let decoded_values = parsed_stream.decode_bits_u64().unwrap().decode_u64().unwrap();

                assert_eq!(decoded_values, values);
            }

        #[test]
        fn test_i64_roundtrip(
            values in prop::collection::vec(any::<i64>(), 0..100),
            encoding in any::<Encoding>()
        ) {
            let owned_stream = OwnedStream::encode_i64s(&values, encoding).unwrap();

                let mut buffer = Vec::new();
                buffer.write_stream(&owned_stream).unwrap();

                let (remaining, parsed_stream) = Stream::parse(&buffer).unwrap();
                assert!(remaining.is_empty());

                let decoded_values = parsed_stream.decode_bits_u64().unwrap().decode_i64().unwrap();

                assert_eq!(decoded_values, values);
            }

        #[test]
        fn test_i8_roundtrip(
            values in prop::collection::vec(any::<i8>(), 0..100),
            encoding in any::<Encoding>(),
        ) {
            let owned_stream = OwnedStream::encode_i8s(&values, encoding).unwrap();

                let mut buffer = Vec::new();
                buffer.write_stream(&owned_stream).unwrap();

                let (remaining, parsed_stream) = Stream::parse(&buffer).unwrap();
                assert!(remaining.is_empty());

                let decoded_values = parsed_stream.decode_i8s().unwrap();
                assert_eq!(decoded_values, values);
            }

        #[test]
        fn test_u8_roundtrip(
            values in prop::collection::vec(any::<u8>(), 0..100),
            encoding in any::<Encoding>()
        ) {
            let owned_stream = OwnedStream::encode_u8s(&values, encoding).unwrap();

                let mut buffer = Vec::new();
                buffer.write_stream(&owned_stream).unwrap();

                let (remaining, parsed_stream) = Stream::parse(&buffer).unwrap();
                assert!(remaining.is_empty());

                let decoded_values = parsed_stream.decode_u8s().unwrap();
                assert_eq!(decoded_values, values);
            }

            #[test]
            fn test_f32_roundtrip(values in prop::collection::vec(any::<f32>(), 0..100)) {
                let owned_stream = OwnedStream::encode_f32(&values).unwrap();

                let mut buffer = Vec::new();
                buffer.write_stream(&owned_stream).unwrap();

                let (remaining, parsed_stream) = Stream::parse(&buffer).unwrap();
                assert!(remaining.is_empty());

                let decoded_values = parsed_stream.decode_f32().unwrap();
                assert_eq!(decoded_values, values);
            }
        }

        #[test]
        fn test_string_roundtrip(
            values in prop::collection::vec(any::<String>(), 0..100),
            encoding in any::<Encoding>(),
        ) {
            let owned_streams = OwnedStream::encode_strings(&values, encoding).unwrap();

            let mut buffers = Vec::new();
            for owned_stream in &owned_streams {
                let mut buffer = Vec::new();
                buffer.write_stream(owned_stream).unwrap();
                buffers.push(buffer);
            }

            let mut parsed_streams = Vec::new();
            for buffer in &buffers {
                let (remaining, parsed_stream) = Stream::parse(buffer).unwrap();
                assert!(remaining.is_empty());
                parsed_streams.push(parsed_stream);
            }

            let decoded_values = decode_string_streams(parsed_streams).unwrap();
            assert_eq!(decoded_values, values);
        }
    }
}
