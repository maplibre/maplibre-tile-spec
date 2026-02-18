mod decode;
mod logical;
mod physical;

use std::fmt;
use std::fmt::Debug;

use borrowme::borrowme;
use num_enum::TryFromPrimitive;

use crate::analyse::{Analyze, StatType};
use crate::utils::{all, take};
use crate::v01::stream::decode::decode_fastpfor_composite;
pub use crate::v01::stream::logical::{
    LogicalData, LogicalDecoder, LogicalTechnique, LogicalValue,
};
pub use crate::v01::stream::physical::{PhysicalDecoder, PhysicalStreamType};
use crate::{MltError, MltRefResult, utils};

/// Representation of a raw stream
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

/// Metadata about a raw stream
#[derive(Clone, Copy, PartialEq)]
pub struct StreamMeta {
    pub physical_type: PhysicalStreamType,
    pub num_values: u32,
    pub logical_decoder: LogicalDecoder,
    pub physical_decoder: PhysicalDecoder,
}

impl Analyze for StreamMeta {
    fn collect_statistic(&self, stat: StatType) -> usize {
        if stat == StatType::MetadataOverheadBytes {
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
            logical_decoder,
            physical_decoder,
        } = self;
        f.debug_struct("StreamMeta")
            .field("physical_type", &format_args!("{physical_type:?}"))
            .field("num_values", &format_args!("{num_values:?}"))
            .field("logical_decoder", &format_args!("{logical_decoder:?}"))
            .field("physical_decoder", &format_args!("{physical_decoder:?}"))
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
    Raw: DataRaw / OwnedDataRaw,
];

impl<'a> Stream<'a> {
    #[must_use]
    pub fn new(meta: StreamMeta, data: StreamData<'a>) -> Self {
        Self { meta, data }
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

    pub fn parse(input: &'a [u8]) -> MltRefResult<'a, Self> {
        Self::parse_internal(input, false)
    }

    /// Parse stream from the input
    /// If `is_bool` is true, compute RLE parameters for boolean streams
    /// automatically instead of reading them from the input.
    fn parse_internal(input: &'a [u8], is_bool: bool) -> MltRefResult<'a, Self> {
        use crate::v01::{LogicalTechnique as LT, PhysicalDecoder as PT};

        let (input, val) = utils::parse_u8(input)?;
        let physical_type = PhysicalStreamType::parse(val)?;

        let (input, val) = utils::parse_u8(input)?;
        let logical1 = LT::parse(val >> 5)?;
        let logical2 = LT::parse((val >> 2) & 0x7)?;
        let physical = PT::parse(val & 0x3)?;

        let (input, num_values) = utils::parse_varint::<u32>(input)?;
        let (input, byte_length) = utils::parse_varint::<u32>(input)?;

        let mut input = input;
        let val1;
        let val2;
        let logical_decoder = match (logical1, logical2) {
            (LT::None, LT::None) => LogicalDecoder::None,
            (LT::Delta, LT::None) => LogicalDecoder::Delta,
            (LT::ComponentwiseDelta, LT::None) => LogicalDecoder::ComponentwiseDelta,
            (LT::Delta, LT::Rle) | (LT::Rle, LT::None) => {
                if is_bool {
                    val1 = num_values.div_ceil(8);
                    val2 = byte_length;
                } else {
                    (input, val1) = utils::parse_varint::<u32>(input)?;
                    (input, val2) = utils::parse_varint::<u32>(input)?;
                }
                let rle = RleMeta {
                    runs: val1,
                    num_rle_values: val2,
                };
                if logical1 == LT::Rle {
                    LogicalDecoder::Rle(rle)
                } else {
                    LogicalDecoder::DeltaRle(rle)
                }
            }
            (LT::Morton, LT::None) => {
                (input, val1) = utils::parse_varint::<u32>(input)?;
                (input, val2) = utils::parse_varint::<u32>(input)?;
                LogicalDecoder::Morton(MortonMeta {
                    num_bits: val1,
                    coordinate_shift: val2,
                })
            }
            (LT::PseudoDecimal, LT::None) => LogicalDecoder::PseudoDecimal,
            _ => Err(MltError::UnsupportedLogicalTechnique(logical1, logical2))?,
        };

        let (input, data) = take(input, usize::try_from(byte_length)?)?;

        let meta = StreamMeta {
            physical_type,
            logical_decoder,
            physical_decoder: physical,
            num_values,
        };

        let stream_data = match physical {
            PT::None | PT::FastPFOR => DataRaw::new(data),
            PT::VarInt => DataVarInt::new(data),
            PT::Alp => {
                return Err(MltError::DecodeError(format!(
                    "Unsupported logical/physical technique combination: {physical:?}"
                )));
            }
        };

        Ok((input, Stream::new(meta, stream_data)))
    }

    pub fn decode_signed_int_stream<T>(self) -> Result<Vec<T>, MltError>
    where
        T: TryFrom<i32>,
        MltError: From<<T as TryFrom<i32>>::Error>,
    {
        self.decode_bits_u32()?
            .decode_i32()?
            .into_iter()
            .map(T::try_from)
            .collect::<Result<Vec<T>, _>>()
            .map_err(Into::into)
    }

    pub fn decode_unsigned_int_stream<T>(self) -> Result<Vec<T>, MltError>
    where
        T: TryFrom<u32>,
        MltError: From<<T as TryFrom<u32>>::Error>,
    {
        self.decode_bits_u32()?
            .decode_u32()?
            .into_iter()
            .map(T::try_from)
            .collect::<Result<Vec<T>, _>>()
            .map_err(Into::into)
    }

    pub fn decode_bits_u64(self) -> Result<LogicalValue, MltError> {
        let value = match self.meta.physical_decoder {
            PhysicalDecoder::VarInt => match self.data {
                StreamData::VarInt(data) => all(utils::parse_varint_vec::<u64, u64>(
                    data.data,
                    self.meta.num_values,
                )?),
                StreamData::Raw(_) => {
                    return Err(MltError::InvalidStreamData {
                        expected: "VarInt",
                        got: "Raw".to_string(),
                    });
                }
            },
            PhysicalDecoder::None => {
                // For raw data, we'd need to read 8 bytes per value
                // But typically 64-bit IDs use VarInt encoding
                return Err(MltError::DecodeError(
                    "Raw physical decoder not supported for u64".to_string(),
                ));
            }
            PhysicalDecoder::FastPFOR => {
                return Err(MltError::UnsupportedPhysicalDecoder("FastPFOR"));
            }
            PhysicalDecoder::Alp => return Err(MltError::UnsupportedPhysicalDecoder("ALP")),
        }?;

        Ok(LogicalValue::new(self.meta, LogicalData::VecU64(value)))
    }

    pub fn decode_u64(self) -> Result<Vec<u64>, MltError> {
        self.decode_bits_u64()?.decode_u64()
    }

    /// Decode a boolean stream: byte-RLE → packed bitmap → `Vec<bool>`
    #[must_use]
    pub fn decode_bools(self) -> Vec<bool> {
        let num_values = self.meta.num_values as usize;
        let num_bytes = num_values.div_ceil(8);
        let raw = match &self.data {
            StreamData::Raw(d) => d.data,
            StreamData::VarInt(d) => d.data,
        };
        let decoded = utils::decode_byte_rle(raw, num_bytes);
        (0..num_values)
            .map(|i| (decoded[i / 8] >> (i % 8)) & 1 == 1)
            .collect()
    }

    /// Decode a stream of f32 values from raw little-endian bytes
    #[must_use]
    pub fn decode_f32s(self) -> Vec<f32> {
        let raw = match &self.data {
            StreamData::Raw(d) => d.data,
            StreamData::VarInt(d) => d.data,
        };
        let num = self.meta.num_values as usize;
        (0..num)
            .map(|i| {
                let o = i * 4;
                f32::from_le_bytes([raw[o], raw[o + 1], raw[o + 2], raw[o + 3]])
            })
            .collect()
    }

    /// Decode a signed i64 stream
    pub fn decode_i64(self) -> Result<Vec<i64>, MltError> {
        self.decode_bits_u64()?.decode_i64()
    }

    pub fn decode_bits_u32(self) -> Result<LogicalValue, MltError> {
        let value = match self.meta.physical_decoder {
            PhysicalDecoder::VarInt => match self.data {
                StreamData::VarInt(data) => all(utils::parse_varint_vec::<u32, u32>(
                    data.data,
                    self.meta.num_values,
                )?),
                StreamData::Raw(_) => {
                    return Err(MltError::InvalidStreamData {
                        expected: "VarInt",
                        got: format!("{:?}", self.data),
                    });
                }
            },
            PhysicalDecoder::None => match self.data {
                StreamData::Raw(data) => all(utils::decode_bytes_to_u32s(
                    data.data,
                    self.meta.num_values,
                )?),
                StreamData::VarInt(_) => {
                    return Err(MltError::InvalidStreamData {
                        expected: "Raw",
                        got: format!("{:?}", self.data),
                    });
                }
            },
            PhysicalDecoder::FastPFOR => match self.data {
                StreamData::Raw(data) => Ok(decode_fastpfor_composite(
                    data.data,
                    self.meta.num_values as usize,
                )?),
                StreamData::VarInt(_) => {
                    return Err(MltError::InvalidStreamData {
                        expected: "Raw",
                        got: format!("{:?}", self.data),
                    });
                }
            },
            PhysicalDecoder::Alp => return Err(MltError::UnsupportedPhysicalDecoder("ALP")),
        }?;

        Ok(LogicalValue::new(self.meta, LogicalData::VecU32(value)))
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
    use super::*;

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
                    logical_decoder: LogicalDecoder::None,
                    physical_decoder: PhysicalDecoder::VarInt,
                },
                data: &[0x04, 0x03, 0x02, 0x01],
                expected_u32_logical_value: Some(LogicalValue::new(
                    StreamMeta {
                        physical_type: PhysicalStreamType::Data(DictionaryType::None),
                        num_values: 4,
                        logical_decoder: LogicalDecoder::None,
                        physical_decoder: PhysicalDecoder::VarInt,
                    },
                    LogicalData::VecU32(vec![4, 3, 2, 1]),
                )),
                expected_u64_logical_value: None,
            },
            // Basic Raw test case
            StreamTestCase {
                name: "simple_raw_bytes_to_u32",
                meta: StreamMeta {
                    physical_type: PhysicalStreamType::Data(DictionaryType::None),
                    num_values: 1,
                    logical_decoder: LogicalDecoder::None,
                    physical_decoder: PhysicalDecoder::None,
                },
                data: &[0x04, 0x03, 0x02, 0x01],
                expected_u32_logical_value: Some(LogicalValue::new(
                    StreamMeta {
                        physical_type: PhysicalStreamType::Data(DictionaryType::None),
                        num_values: 1,
                        logical_decoder: LogicalDecoder::None,
                        physical_decoder: PhysicalDecoder::None,
                    },
                    LogicalData::VecU32(vec![0x0102_0304]),
                )),
                expected_u64_logical_value: None,
            },
        ]
    }

    fn create_stream_from_test_case(test_case: &StreamTestCase) -> Stream<'_> {
        let data = match test_case.meta.physical_decoder {
            PhysicalDecoder::VarInt => DataVarInt::new(test_case.data),
            PhysicalDecoder::None => DataRaw::new(test_case.data),
            _ => panic!(
                "Unsupported physical decoder in test: {:?}",
                test_case.meta.physical_decoder
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

    /// Test case for logical decoding tests
    #[derive(Debug)]
    struct LogicalDecodeTestCase {
        name: &'static str,
        logical_decoder: LogicalDecoder,
        input_data: Vec<u32>,
        expected_u32: Option<Vec<u32>>,
        expected_i32: Option<Vec<i32>>,
    }

    impl LogicalDecodeTestCase {
        fn to_logical_val(&self) -> LogicalValue {
            let meta = StreamMeta {
                physical_type: PhysicalStreamType::Data(DictionaryType::None),
                num_values: u32::try_from(self.input_data.len())
                    .expect("input_data length fits in u32"),
                logical_decoder: self.logical_decoder,
                physical_decoder: PhysicalDecoder::VarInt,
            };
            let data = LogicalData::VecU32(self.input_data.clone());
            LogicalValue::new(meta, data)
        }
    }

    fn generate_logical_decode_test_cases() -> Vec<LogicalDecodeTestCase> {
        vec![
            // decode_i32 tests
            LogicalDecodeTestCase {
                name: "i32_componentwise_delta",
                logical_decoder: LogicalDecoder::ComponentwiseDelta,
                // ZigZag pairs: [(0,0),(2,4),(2,4)] -> [(0,0),(1,2),(1,2)]
                // Delta: [(0,0),(1,2),(1,2)] -> [(0,0),(1,2),(2,4)]
                input_data: vec![0, 0, 2, 4, 2, 4],
                expected_u32: None,
                expected_i32: Some(Vec::<i32>::from([0, 0, 1, 2, 2, 4])),
            },
            LogicalDecodeTestCase {
                name: "i32_delta",
                logical_decoder: LogicalDecoder::Delta,
                // ZigZag: [0,1,2,1,2] -> [0,-1,1,-1,1]
                // Delta: [0,-1,1,-1,1] -> [0,-1,0,-1,0]
                input_data: vec![0, 1, 2, 1, 2],
                expected_u32: None,
                expected_i32: Some(Vec::<i32>::from([0, -1, 0, -1, 0])),
            },
            LogicalDecodeTestCase {
                name: "i32_delta_rle",
                logical_decoder: LogicalDecoder::DeltaRle(RleMeta {
                    runs: 2,
                    num_rle_values: 5,
                }),
                // RLE: [3,2] [0,2] -> [0,0,0,2,2]
                // ZigZag: [0,0,0,2,2] -> [0,0,0,1,1]
                // Delta: [0,0,0,1,1] -> [0,0,0,1,2]
                input_data: vec![3, 2, 0, 2],
                expected_u32: None,
                expected_i32: Some(Vec::<i32>::from([0, 0, 0, 1, 2])),
            },
            LogicalDecodeTestCase {
                name: "i32_empty",
                logical_decoder: LogicalDecoder::Delta,
                input_data: vec![],
                expected_u32: None,
                expected_i32: Some(Vec::<i32>::new()),
            },
            // decode_u32 tests
            LogicalDecodeTestCase {
                name: "u32_none",
                logical_decoder: LogicalDecoder::None,
                input_data: vec![10, 20, 30, 40],
                expected_u32: Some(Vec::<u32>::from([10, 20, 30, 40])),
                expected_i32: None,
            },
            LogicalDecodeTestCase {
                name: "u32_rle",
                logical_decoder: LogicalDecoder::Rle(RleMeta {
                    runs: 3,
                    num_rle_values: 6,
                }),
                input_data: vec![3, 2, 1, 10, 20, 30],
                expected_u32: Some(Vec::<u32>::from([10, 10, 10, 20, 20, 30])),
                expected_i32: None,
            },
            LogicalDecodeTestCase {
                name: "u32_delta",
                logical_decoder: LogicalDecoder::Delta,
                // ZigZag: [0,2,2,2,2] -> [0,1,1,1,1]
                // Delta: [0,1,1,1,1] -> [0,1,2,3,4]
                input_data: vec![0, 2, 2, 2, 2],
                expected_u32: Some(Vec::<u32>::from([0, 1, 2, 3, 4])),
                expected_i32: None,
            },
            LogicalDecodeTestCase {
                name: "u32_empty",
                logical_decoder: LogicalDecoder::None,
                input_data: vec![],
                expected_u32: Some(Vec::<u32>::new()),
                expected_i32: None,
            },
        ]
    }

    #[test]
    fn test_decode_u32() {
        let test_cases = generate_logical_decode_test_cases();

        for test_case in test_cases {
            if let Some(expected) = &test_case.expected_u32 {
                let result = test_case.to_logical_val().decode_u32();
                assert!(
                    result.is_ok(),
                    "Case '{}' should decode successfully",
                    test_case.name
                );
                assert_eq!(
                    &result.unwrap(),
                    expected,
                    "Case '{}' should match expected output",
                    test_case.name
                );
            }
        }
    }

    #[test]
    fn test_decode_i32() {
        let test_cases = generate_logical_decode_test_cases();

        for test_case in test_cases {
            if let Some(expected) = &test_case.expected_i32 {
                let result = test_case.to_logical_val().decode_i32();
                assert!(
                    result.is_ok(),
                    "Case '{}' should decode successfully",
                    test_case.name
                );
                assert_eq!(
                    &result.unwrap(),
                    expected,
                    "Case '{}' should match expected output",
                    test_case.name
                );
            }
        }
    }
}
