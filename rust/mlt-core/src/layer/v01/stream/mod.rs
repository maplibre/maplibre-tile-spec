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
    decode_bytes_to_u64s, decode_fastpfor_composite, encode_bools_to_bytes, encode_byte_rle,
    parse_u8, parse_varint, parse_varint_vec, take,
};
pub use crate::v01::stream::logical::{
    LogicalData, LogicalEncoder, LogicalEncoding, LogicalTechnique, LogicalValue,
};
pub use crate::v01::stream::physical::{PhysicalEncoder, PhysicalEncoding, StreamType};
use crate::{MltError, MltRefResult};

#[derive(Debug, Eq, PartialEq, Clone, Copy)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub struct Encoder {
    pub logical: LogicalEncoder,
    pub physical: PhysicalEncoder,
}

impl Encoder {
    #[must_use]
    pub const fn new(logical: LogicalEncoder, physical: PhysicalEncoder) -> Self {
        Self { logical, physical }
    }

    #[must_use]
    pub fn plain() -> Encoder {
        Encoder::new(LogicalEncoder::None, PhysicalEncoder::None)
    }
    #[must_use]
    pub fn varint() -> Encoder {
        Encoder::new(LogicalEncoder::None, PhysicalEncoder::VarInt)
    }
    #[must_use]
    pub fn rle_varint() -> Encoder {
        Encoder::new(LogicalEncoder::Rle, PhysicalEncoder::VarInt)
    }
    #[must_use]
    pub fn fastpfor() -> Encoder {
        Encoder::new(LogicalEncoder::None, PhysicalEncoder::FastPFOR)
    }
    #[must_use]
    pub fn rle_fastpfor() -> Encoder {
        Encoder::new(LogicalEncoder::Rle, PhysicalEncoder::FastPFOR)
    }
}

/// Representation of an encoded stream
#[borrowme]
#[derive(Debug, PartialEq, Clone)]
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
    pub fn empty_without_encoding() -> Self {
        Self {
            meta: StreamMeta::new(
                StreamType::Data(DictionaryType::None),
                LogicalEncoding::None,
                PhysicalEncoding::None,
                0,
            ),
            data: OwnedStreamData::Encoded(OwnedEncodedData { data: Vec::new() }),
        }
    }

    /// Creates a plain stream with values encoded literally
    #[must_use]
    fn new_plain(data: Vec<u8>, num_values: u32) -> OwnedStream {
        let meta = StreamMeta::new(
            StreamType::Data(DictionaryType::None),
            LogicalEncoding::None,
            PhysicalEncoding::None,
            num_values,
        );
        let data = OwnedStreamData::Encoded(OwnedEncodedData { data });
        Self { meta, data }
    }

    /// Encode a boolean stream: byte-RLE <- packed bitmap <- `Vec<bool>`
    /// Boolean streams always use byte-RLE encoding with `LogicalEncoding::Rle` metadata.
    /// The `RleMeta` values are computed by readers from the stream itself.
    pub fn encode_bools(values: &[bool]) -> Result<Self, MltError> {
        let num_values = u32::try_from(values.len())?;
        let bytes = encode_bools_to_bytes(values);
        let data = encode_byte_rle(&bytes);
        // Boolean streams use byte-RLE encoding with RLE metadata
        let runs = num_values.div_ceil(8);
        let num_rle_values = u32::try_from(data.len())?;
        let meta = StreamMeta::new(
            StreamType::Data(DictionaryType::None),
            LogicalEncoding::Rle(RleMeta {
                runs,
                num_rle_values,
            }),
            PhysicalEncoding::None,
            num_values,
        );
        Ok(Self {
            meta,
            data: OwnedStreamData::Encoded(OwnedEncodedData { data }),
        })
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

    pub fn encode_i8s(values: &[i8], encoding: Encoder) -> Result<Self, MltError> {
        let as_i32: Vec<i32> = values.iter().map(|&v| i32::from(v)).collect();
        let (physical_u32s, logical_encoding) = encoding.logical.encode_i32s(&as_i32)?;
        let num_values = u32::try_from(physical_u32s.len())?;
        let (data, physical_encoding) = encoding.physical.encode_u32s(physical_u32s)?;
        Ok(Self {
            meta: StreamMeta::new(
                StreamType::Data(DictionaryType::None),
                logical_encoding,
                physical_encoding,
                num_values,
            ),
            data,
        })
    }
    pub fn encode_u8s(values: &[u8], encoding: Encoder) -> Result<Self, MltError> {
        let as_u32: Vec<u32> = values.iter().map(|&v| u32::from(v)).collect();
        let (physical_u32s, logical_encoding) = encoding.logical.encode_u32s(&as_u32)?;
        let num_values = u32::try_from(physical_u32s.len())?;
        let (data, physical_encoding) = encoding.physical.encode_u32s(physical_u32s)?;
        Ok(Self {
            meta: StreamMeta::new(
                StreamType::Data(DictionaryType::None),
                logical_encoding,
                physical_encoding,
                num_values,
            ),
            data,
        })
    }
    pub fn encode_i32s(values: &[i32], encoding: Encoder) -> Result<Self, MltError> {
        let (physical_u32s, logical_encoding) = encoding.logical.encode_i32s(values)?;
        let num_values = u32::try_from(physical_u32s.len())?;
        let (data, physical_encoding) = encoding.physical.encode_u32s(physical_u32s)?;
        Ok(Self {
            meta: StreamMeta::new(
                StreamType::Data(DictionaryType::None),
                logical_encoding,
                physical_encoding,
                num_values,
            ),
            data,
        })
    }
    pub fn encode_u32s(values: &[u32], encoding: Encoder) -> Result<Self, MltError> {
        Self::encode_u32s_of_type(values, encoding, StreamType::Data(DictionaryType::None))
    }
    pub fn encode_u32s_of_type(
        values: &[u32],
        encoding: Encoder,
        stream_type: StreamType,
    ) -> Result<Self, MltError> {
        let (physical_u32s, logical_encoding) = encoding.logical.encode_u32s(values)?;
        let num_values = u32::try_from(physical_u32s.len())?;
        let (data, physical_encoding) = encoding.physical.encode_u32s(physical_u32s)?;
        Ok(Self {
            meta: StreamMeta::new(stream_type, logical_encoding, physical_encoding, num_values),
            data,
        })
    }

    pub fn encode_i64s(values: &[i64], encoding: Encoder) -> Result<Self, MltError> {
        let (physical_u64s, logical_encoding) = encoding.logical.encode_i64s(values)?;
        let num_values = u32::try_from(physical_u64s.len())?;
        let (data, physical_encoding) = encoding.physical.encode_u64s(physical_u64s)?;
        Ok(Self {
            meta: StreamMeta::new(
                StreamType::Data(DictionaryType::None),
                logical_encoding,
                physical_encoding,
                num_values,
            ),
            data,
        })
    }
    pub fn encode_u64s(values: &[u64], encoding: Encoder) -> Result<Self, MltError> {
        let (physical_u64s, logical_encoding) = encoding.logical.encode_u64s(values)?;
        let num_values = u32::try_from(physical_u64s.len())?;
        let (data, physical_encoding) = encoding.physical.encode_u64s(physical_u64s)?;
        Ok(Self {
            meta: StreamMeta::new(
                StreamType::Data(DictionaryType::None),
                logical_encoding,
                physical_encoding,
                num_values,
            ),
            data,
        })
    }

    /// Encode a sequence of strings into a length stream and a data stream.
    pub fn encode_strings(values: &[String], encoding: Encoder) -> Result<Vec<Self>, MltError> {
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
            encoding,
            StreamType::Length(LengthType::VarBinary),
        )?;

        let data_stream = Self::new_plain(data, u32::try_from(values.len())?);

        Ok(vec![length_stream, data_stream])
    }

    /// Encode a sequence of strings using FSST compression.
    ///
    /// Produces 4 streams:
    /// 1. Symbol lengths stream (Length, `LengthType::Symbol`)
    /// 2. Symbol table data stream (Data, `DictionaryType::Fsst`)
    /// 3. Value lengths stream (Length, `LengthType::Dictionary`)
    /// 4. Compressed corpus stream (Data, `DictionaryType::Single`)
    ///
    /// Note: The FSST algorithm implementation may differ from Java's, so the
    /// compressed output may not be byte-for-byte identical. Both implementations
    /// are semantically compatible and can decode each other's output.
    pub fn encode_strings_fsst(
        values: &[String],
        encoding: Encoder,
    ) -> Result<Vec<Self>, MltError> {
        use fsst::Compressor;

        // Build byte slices for training
        let byte_slices: Vec<&[u8]> = values.iter().map(String::as_bytes).collect();

        // Train FSST compressor on the corpus
        let compressor = Compressor::train(&byte_slices);

        // Get symbol table info
        let symbols = compressor.symbol_table();
        let symbol_lengths_u8 = compressor.symbol_lengths();

        // Build concatenated symbol bytes (only the actual bytes for each symbol)
        let mut symbol_bytes = Vec::new();
        for sym in symbols {
            let bytes = sym.to_u64().to_le_bytes();
            let len = sym.len();
            symbol_bytes.extend_from_slice(&bytes[..len]);
        }

        // Convert symbol lengths to u32 for encoding
        let symbol_lengths: Vec<u32> = symbol_lengths_u8
            .iter()
            .take(symbols.len())
            .map(|&l| u32::from(l))
            .collect();

        // Compress all strings and concatenate into a single corpus
        let mut compressed = Vec::new();
        for s in values {
            let comp = compressor.compress(s.as_bytes());
            compressed.extend(comp);
        }

        // Get original string lengths (UTF-8 byte lengths)
        let value_lengths: Vec<u32> = values
            .iter()
            .map(|s| u32::try_from(s.len()))
            .collect::<Result<Vec<_>, _>>()?;

        // Stream 1: Symbol lengths
        let symbol_length_stream = Self::encode_u32s_of_type(
            &symbol_lengths,
            encoding,
            StreamType::Length(LengthType::Symbol),
        )?;

        // Stream 2: Symbol table data
        let symbol_table_stream = Self {
            meta: StreamMeta::new(
                StreamType::Data(DictionaryType::Fsst),
                LogicalEncoding::None,
                PhysicalEncoding::None,
                u32::try_from(symbol_lengths.len())?,
            ),
            data: OwnedStreamData::Encoded(OwnedEncodedData { data: symbol_bytes }),
        };

        // Stream 3: Value lengths (original UTF-8 byte lengths)
        let value_length_stream = Self::encode_u32s_of_type(
            &value_lengths,
            encoding,
            StreamType::Length(LengthType::Dictionary),
        )?;

        // Stream 4: Compressed corpus
        let compressed_stream = Self {
            meta: StreamMeta::new(
                StreamType::Data(DictionaryType::Single),
                LogicalEncoding::None,
                PhysicalEncoding::None,
                u32::try_from(values.len())?,
            ),
            data: OwnedStreamData::Encoded(OwnedEncodedData { data: compressed }),
        };

        Ok(vec![
            symbol_length_stream,
            symbol_table_stream,
            value_length_stream,
            compressed_stream,
        ])
    }
}
/// Metadata about an encoded stream
#[derive(Clone, Copy, PartialEq)]
pub struct StreamMeta {
    pub stream_type: StreamType,
    pub logical_encoding: LogicalEncoding,
    pub physical_encoding: PhysicalEncoding,
    pub num_values: u32,
}
impl StreamMeta {
    #[must_use]
    pub fn new(
        stream_type: StreamType,
        logical_encoding: LogicalEncoding,
        physical_encoding: PhysicalEncoding,
        num_values: u32,
    ) -> Self {
        Self {
            stream_type,
            logical_encoding,
            physical_encoding,
            num_values,
        }
    }

    /// Parse stream from the input
    ///
    /// If `is_bool` is true, compute RLE parameters for boolean streams
    /// automatically instead of reading them from the input.
    ///
    /// Returns the stream metadata and the size of the stream in bytes
    fn parse(input: &[u8], is_bool: bool) -> MltRefResult<'_, (Self, u32)> {
        use crate::v01::LogicalTechnique as LT;

        let (input, stream_type) = StreamType::parse(input)?;
        let (input, val) = parse_u8(input)?;
        let logical1 = LT::parse(val >> 5)?;
        let logical2 = LT::parse((val >> 2) & 0x7)?;
        let physical_encoding = PhysicalEncoding::parse(val & 0x3)?;

        let (input, num_values) = parse_varint::<u32>(input)?;
        let (input, byte_length) = parse_varint::<u32>(input)?;

        let mut input = input;
        let logical_encoding = match (logical1, logical2) {
            (LT::None, LT::None) => LogicalEncoding::None,
            (LT::Delta, LT::None) => LogicalEncoding::Delta,
            (LT::ComponentwiseDelta, LT::None) => LogicalEncoding::ComponentwiseDelta,
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
                    LogicalEncoding::Rle(rle)
                } else {
                    LogicalEncoding::DeltaRle(rle)
                }
            }
            (LT::Morton, LT::None) => {
                let num_bits;
                let coordinate_shift;
                (input, num_bits) = parse_varint::<u32>(input)?;
                (input, coordinate_shift) = parse_varint::<u32>(input)?;
                LogicalEncoding::Morton(MortonMeta {
                    num_bits,
                    coordinate_shift,
                })
            }
            (LT::PseudoDecimal, LT::None) => LogicalEncoding::PseudoDecimal,
            _ => Err(MltError::InvalidLogicalEncodings(logical1, logical2))?,
        };

        let meta = StreamMeta::new(stream_type, logical_encoding, physical_encoding, num_values);
        Ok((input, (meta, byte_length)))
    }

    pub fn write_to<W: Write>(
        &self,
        writer: &mut W,
        is_bool: bool,
        byte_length: u32,
    ) -> io::Result<()> {
        use crate::v01::LogicalTechnique as LT;
        writer.write_u8(self.stream_type.as_u8())?;
        let logical_enc_u8: u8 = match self.logical_encoding {
            LogicalEncoding::None => (LT::None as u8) << 5,
            LogicalEncoding::Delta => (LT::Delta as u8) << 5,
            LogicalEncoding::DeltaRle(_) => ((LT::Delta as u8) << 5) | ((LT::Rle as u8) << 2),
            LogicalEncoding::ComponentwiseDelta => (LT::ComponentwiseDelta as u8) << 5,
            LogicalEncoding::Rle(_) => (LT::Rle as u8) << 5,
            LogicalEncoding::Morton(_) => (LT::Morton as u8) << 5,
            LogicalEncoding::PseudoDecimal => (LT::PseudoDecimal as u8) << 5,
        };
        let physical_enc_u8: u8 = match self.physical_encoding {
            PhysicalEncoding::None => 0x0,
            PhysicalEncoding::FastPFOR => 0x1,
            PhysicalEncoding::VarInt => 0x2,
            PhysicalEncoding::Alp => 0x3,
        };
        writer.write_u8(logical_enc_u8 | physical_enc_u8)?;
        writer.write_varint(self.num_values)?;
        writer.write_varint(byte_length)?;

        // some encoding have settings inside them
        match self.logical_encoding {
            LogicalEncoding::DeltaRle(r) | LogicalEncoding::Rle(r) => {
                if !is_bool {
                    writer.write_varint(r.runs)?;
                    writer.write_varint(r.num_rle_values)?;
                }
            }
            LogicalEncoding::Morton(m) => {
                writer.write_varint(m.num_bits)?;
                writer.write_varint(m.coordinate_shift)?;
            }
            LogicalEncoding::None
            | LogicalEncoding::Delta
            | LogicalEncoding::ComponentwiseDelta
            | LogicalEncoding::PseudoDecimal => {}
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
            stream_type,
            logical_encoding,
            physical_encoding,
            num_values,
        } = self;
        f.debug_struct("StreamMeta")
            .field("stream_type", &format_args!("{stream_type:?}"))
            .field("logical_encoding", &format_args!("{logical_encoding:?}"))
            .field("physical_encoding", &format_args!("{physical_encoding:?}"))
            .field("num_values", &format_args!("{num_values:?}"))
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
        #[derive(Debug, PartialEq, Clone)]
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
            #[derive(PartialEq, Clone)]
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
        use PhysicalEncoding as PD;
        let (input, (meta, byte_length)) = StreamMeta::parse(input, is_bool)?;

        let (input, data) = take(input, usize::try_from(byte_length)?)?;

        let stream_data = match meta.physical_encoding {
            PD::None | PD::FastPFOR => EncodedData::new(data),
            PD::VarInt => DataVarInt::new(data),
            PD::Alp => return Err(MltError::UnsupportedPhysicalEncoding("ALP")),
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
        let value = match self.meta.physical_encoding {
            PhysicalEncoding::VarInt => match self.data {
                StreamData::VarInt(data) => all(parse_varint_vec::<u32, u32>(
                    data.data,
                    self.meta.num_values,
                )?),
                StreamData::Encoded(_) => {
                    return Err(MltError::StreamDataMismatch("VarInt", "Encoded"));
                }
            },
            PhysicalEncoding::None => match self.data {
                StreamData::Encoded(data) => {
                    all(decode_bytes_to_u32s(data.data, self.meta.num_values)?)
                }
                StreamData::VarInt(_) => {
                    return Err(MltError::StreamDataMismatch("Encoded", "VarInt"));
                }
            },
            PhysicalEncoding::FastPFOR => match self.data {
                StreamData::Encoded(data) => Ok(decode_fastpfor_composite(
                    data.data,
                    self.meta.num_values as usize,
                )?),
                StreamData::VarInt(_) => {
                    return Err(MltError::StreamDataMismatch("Encoded", "VarInt"));
                }
            },
            PhysicalEncoding::Alp => return Err(MltError::UnsupportedPhysicalEncoding("ALP")),
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
        let value = match self.meta.physical_encoding {
            PhysicalEncoding::VarInt => match self.data {
                StreamData::VarInt(data) => all(parse_varint_vec::<u64, u64>(
                    data.data,
                    self.meta.num_values,
                )?),
                StreamData::Encoded(_) => {
                    return Err(MltError::StreamDataMismatch("VarInt", "Encoded"));
                }
            },
            PhysicalEncoding::None => match self.data {
                StreamData::Encoded(data) => {
                    all(decode_bytes_to_u64s(data.data, self.meta.num_values)?)
                }
                StreamData::VarInt(_) => {
                    return Err(MltError::StreamDataMismatch("Encoded", "VarInt"));
                }
            },
            PhysicalEncoding::FastPFOR => {
                return Err(MltError::UnsupportedPhysicalEncoding(
                    "FastPFOR decoding u64",
                ));
            }
            PhysicalEncoding::Alp => return Err(MltError::UnsupportedPhysicalEncoding("ALP")),
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
    //     match self.stream_type {
    //         StreamType::Present => {
    //             todo!()
    //         }
    //         StreamType::Data(_v) => parse_varint_vec::<u32, u32>(&[], self.num_values),
    //         StreamType::Offset(_v) => {
    //             todo!()
    //         }
    //         StreamType::Length(_v) => {
    //             todo!()
    //         }
    //     }
    // }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;
    use rstest::rstest;

    use super::*;
    use crate::v01::property::decode::decode_string_streams;

    /// Strategy for `PhysicalEncoder` that excludes `FastPFOR` to support 64bit ints
    fn physical_no_fastpfor() -> impl Strategy<Value = PhysicalEncoder> {
        any::<PhysicalEncoder>().prop_filter("not fastpfor", |v| *v != PhysicalEncoder::FastPFOR)
    }

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
                meta: StreamMeta::new(
                    StreamType::Data(DictionaryType::None),
                    LogicalEncoding::None,
                    PhysicalEncoding::VarInt,
                    4,
                ),
                data: &[0x04, 0x03, 0x02, 0x01],
                expected_u32_logical_value: Some(LogicalValue::new(
                    StreamMeta::new(
                        StreamType::Data(DictionaryType::None),
                        LogicalEncoding::None,
                        PhysicalEncoding::VarInt,
                        4,
                    ),
                    LogicalData::VecU32(vec![4, 3, 2, 1]),
                )),
                expected_u64_logical_value: None,
            },
            // Basic Encoded test case
            StreamTestCase {
                name: "simple_raw_bytes_to_u32",
                meta: StreamMeta::new(
                    StreamType::Data(DictionaryType::None),
                    LogicalEncoding::None,
                    PhysicalEncoding::None,
                    1,
                ),
                data: &[0x04, 0x03, 0x02, 0x01],
                expected_u32_logical_value: Some(LogicalValue::new(
                    StreamMeta::new(
                        StreamType::Data(DictionaryType::None),
                        LogicalEncoding::None,
                        PhysicalEncoding::None,
                        1,
                    ),
                    LogicalData::VecU32(vec![0x0102_0304]),
                )),
                expected_u64_logical_value: None,
            },
        ]
    }

    fn create_stream_from_test_case(test_case: &StreamTestCase) -> Stream<'_> {
        let data = match test_case.meta.physical_encoding {
            PhysicalEncoding::VarInt => DataVarInt::new(test_case.data),
            PhysicalEncoding::None => EncodedData::new(test_case.data),
            _ => panic!(
                "Unsupported physical encoding in test: {:?}",
                test_case.meta.physical_encoding
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

    fn make_logical_val(logical_encoding: LogicalEncoding, input_data: Vec<u32>) -> LogicalValue {
        let meta = StreamMeta::new(
            StreamType::Data(DictionaryType::None),
            logical_encoding,
            PhysicalEncoding::VarInt,
            u32::try_from(input_data.len()).expect("input_data length fits in u32"),
        );
        let data = LogicalData::VecU32(input_data);
        LogicalValue::new(meta, data)
    }

    #[rstest]
    // ZigZag pairs: [(0,0),(2,4),(2,4)] -> [(0,0),(1,2),(1,2)]
    // Delta: [(0,0),(1,2),(1,2)] -> [(0,0),(1,2),(2,4)]
    #[case::componentwise_delta(LogicalEncoding::ComponentwiseDelta, vec![0, 0, 2, 4, 2, 4], vec![0, 0, 1, 2, 2, 4])]
    // ZigZag: [0,1,2,1,2] -> [0,-1,1,-1,1]
    // Delta: [0,-1,1,-1,1] -> [0,-1,0,-1,0]
    #[case::delta(LogicalEncoding::Delta, vec![0, 1, 2, 1, 2], vec![0, -1, 0, -1, 0])]
    // RLE: [3,2] [0,2] -> [0,0,0,2,2]
    // ZigZag: [0,0,0,2,2] -> [0,0,0,1,1]
    // Delta: [0,0,0,1,1] -> [0,0,0,1,2]
    #[case::delta_rle(LogicalEncoding::DeltaRle(RleMeta { runs: 2, num_rle_values: 5 }), vec![3, 2, 0, 2], vec![0, 0, 0, 1, 2])]
    #[case::delta(LogicalEncoding::Delta, vec![], vec![])]
    fn test_decode_i32(
        #[case] logical_encoding: LogicalEncoding,
        #[case] input_data: Vec<u32>,
        #[case] expected: Vec<i32>,
    ) {
        let result = make_logical_val(logical_encoding, input_data).decode_i32();
        assert!(result.is_ok(), "should decode successfully");
        assert_eq!(result.unwrap(), expected, "should match expected output");
    }

    #[rstest]
    #[case::empty(LogicalEncoding::None, vec![], vec![])]
    #[case::new_encoded(LogicalEncoding::None, vec![10, 20, 30, 40], vec![10, 20, 30, 40])]
    #[case::rle(LogicalEncoding::Rle(RleMeta { runs: 3, num_rle_values: 6 }), vec![3, 2, 1, 10, 20, 30], vec![10, 10, 10, 20, 20, 30])]
    // ZigZag: [0,2,2,2,2] -> [0,1,1,1,1]
    // Delta: [0,1,1,1,1] -> [0,1,2,3,4]
    #[case::delta(LogicalEncoding::Delta, vec![0, 2, 2, 2, 2], vec![0, 1, 2, 3, 4])]
    fn test_decode_u32(
        #[case] logical_encoding: LogicalEncoding,
        #[case] input_data: Vec<u32>,
        #[case] expected: Vec<u32>,
    ) {
        let result = make_logical_val(logical_encoding, input_data).decode_u32();
        assert!(result.is_ok(), "should decode successfully");
        assert_eq!(result.unwrap(), expected, "should match expected output");
    }

    #[rstest]
    #[case::basic(vec![1, 2, 3, 4, 5, 100, 1000])]
    #[case::large(vec![1_000_000; 256])]
    #[case::edge_values(vec![0, 1, 2, 4, 8, 16, 1024, 65535, 1_000_000_000, u32::MAX])]
    #[case::empty(vec![])]
    fn test_fastpfor_roundtrip(#[case] values: Vec<u32>) {
        use crate::utils::BinarySerializer as _;
        let encoder = Encoder::new(LogicalEncoder::None, PhysicalEncoder::FastPFOR);
        let owned_stream = OwnedStream::encode_u32s(&values, encoder).unwrap();

        let mut buffer = Vec::new();
        buffer.write_stream(&owned_stream).unwrap();

        let (remaining, parsed_stream) = Stream::parse(&buffer).unwrap();
        assert!(remaining.is_empty());

        let decoded_values = parsed_stream
            .decode_bits_u32()
            .unwrap()
            .decode_u32()
            .unwrap();

        assert_eq!(decoded_values, values);
    }

    /// Test roundtrip: write -> parse -> equality for stream serialization
    #[rstest]
    #[case::new_encoded(StreamType::Data(DictionaryType::None), 2, LogicalEncoding::None, PhysicalEncoding::None, vec![0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08], false)]
    #[case::new_encoded(StreamType::Data(DictionaryType::None), 2, LogicalEncoding::ComponentwiseDelta, PhysicalEncoding::None, vec![0x00, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00], false)]
    #[case::new_encoded(StreamType::Offset(OffsetType::Vertex), 3, LogicalEncoding::None, PhysicalEncoding::None, vec![0x00, 0x00, 0x00, 0x00, 0x04, 0x00, 0x00, 0x00, 0x08, 0x00, 0x00, 0x00], false)]
    #[case::varint(StreamType::Data(DictionaryType::None), 4, LogicalEncoding::None, PhysicalEncoding::VarInt, vec![0x0A, 0x14, 0x1E, 0x28], false)]
    #[case::varint(StreamType::Data(DictionaryType::None), 5, LogicalEncoding::Delta, PhysicalEncoding::VarInt, vec![0x00, 0x02, 0x02, 0x02, 0x02], false)]
    #[case::varint(StreamType::Data(DictionaryType::None), 3, LogicalEncoding::PseudoDecimal, PhysicalEncoding::VarInt, vec![0x01, 0x02, 0x03], false)]
    #[case::varint(StreamType::Length(LengthType::VarBinary), 3, LogicalEncoding::Delta, PhysicalEncoding::VarInt, vec![0x00, 0x02, 0x02], false)]
    #[case::rle(StreamType::Data(DictionaryType::None), 6, LogicalEncoding::Rle(RleMeta { runs: 3, num_rle_values: 3 }), PhysicalEncoding::VarInt, vec![0x03, 0x02, 0x01, 0x0A, 0x14, 0x1E], false)]
    #[case::rle(StreamType::Data(DictionaryType::None), 5, LogicalEncoding::DeltaRle(RleMeta { runs: 2, num_rle_values: 5 }), PhysicalEncoding::VarInt, vec![0x03, 0x02, 0x00, 0x02], false)]
    #[case::morton(StreamType::Data(DictionaryType::Morton), 4, LogicalEncoding::Morton(MortonMeta { num_bits: 32, coordinate_shift: 0 }), PhysicalEncoding::VarInt, vec![0x01, 0x02, 0x03, 0x04], false)]
    #[case::boolean(StreamType::Present, 16, LogicalEncoding::Rle(RleMeta { runs: 2, num_rle_values: 2 }), PhysicalEncoding::VarInt, vec![0xFF, 0x00], true)]
    fn test_stream_roundtrip(
        #[case] stream_type: StreamType,
        #[case] num_values: u32,
        #[case] logical_encoding: LogicalEncoding,
        #[case] physical_encoding: PhysicalEncoding,
        #[case] data_bytes: Vec<u8>,
        #[case] is_bool: bool,
    ) {
        use crate::utils::BinarySerializer as _;

        let stream_data = match physical_encoding {
            PhysicalEncoding::None | PhysicalEncoding::FastPFOR => {
                OwnedStreamData::Encoded(OwnedEncodedData { data: data_bytes })
            }
            PhysicalEncoding::VarInt => {
                OwnedStreamData::VarInt(OwnedDataVarInt { data: data_bytes })
            }
            PhysicalEncoding::Alp => panic!("ALP not supported"),
        };
        let stream = OwnedStream {
            meta: StreamMeta::new(stream_type, logical_encoding, physical_encoding, num_values),
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

    fn encoding_no_fastpfor() -> impl Strategy<Value = Encoder> {
        any::<Encoder>().prop_filter("not fastpfor", |v| v.physical != PhysicalEncoder::FastPFOR)
    }

    proptest! {
        #[test]
        fn test_u32_roundtrip(
            values in prop::collection::vec(any::<u32>(), 0..100),
            encoding in any::<Encoder>()
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
            encoding in any::<Encoder>(),
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
            encoding in encoding_no_fastpfor()
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
            encoding in encoding_no_fastpfor()
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
            encoding in any::<Encoder>(),
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
            encoding in any::<Encoder>()
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

        #[test]
        fn test_string_roundtrip(
            values in prop::collection::vec(any::<String>(), 0..100),
            encoding in any::<Encoder>(),
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
