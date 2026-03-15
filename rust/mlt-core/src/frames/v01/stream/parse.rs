use std::fmt::{self, Debug};
use std::io::{self, Write};

use integer_encoding::VarIntWriter as _;

use crate::analyse::{Analyze, StatType};
use crate::utils::{BinarySerializer as _, parse_u8, parse_varint, take};
use crate::v01::{
    IntEncoding, LogicalEncoding, LogicalTechnique, MortonMeta, PhysicalEncoding, RawStream,
    RawStreamData, RleMeta, StreamMeta, StreamType,
};
use crate::{MltError, MltRefResult};

impl IntEncoding {
    #[must_use]
    pub const fn new(logical: LogicalEncoding, physical: PhysicalEncoding) -> Self {
        Self { logical, physical }
    }

    #[must_use]
    pub const fn none() -> Self {
        Self::new(LogicalEncoding::None, PhysicalEncoding::None)
    }
}

impl StreamMeta {
    #[must_use]
    pub fn new(stream_type: StreamType, encoding: IntEncoding, num_values: u32) -> Self {
        Self {
            stream_type,
            encoding,
            num_values,
        }
    }

    /// Parse stream from the input
    ///
    /// If `is_bool` is true, compute RLE parameters for boolean streams
    /// automatically instead of reading them from the input.
    ///
    /// Returns the stream metadata and the size of the stream in bytes
    pub(super) fn from_bytes(input: &[u8], is_bool: bool) -> MltRefResult<'_, (Self, u32)> {
        use crate::v01::LogicalTechnique as LT;

        let (input, stream_type) = StreamType::from_bytes(input)?;
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
            (LT::Morton, LT::None | LT::Rle | LT::Delta) => {
                let num_bits;
                let coordinate_shift;
                (input, num_bits) = parse_varint::<u32>(input)?;
                (input, coordinate_shift) = parse_varint::<u32>(input)?;
                let meta = MortonMeta {
                    num_bits,
                    coordinate_shift,
                };
                match logical2 {
                    LT::Rle => LogicalEncoding::MortonRle(meta),
                    LT::Delta => LogicalEncoding::MortonDelta(meta),
                    _ => LogicalEncoding::Morton(meta),
                }
            }
            (LT::PseudoDecimal, LT::None) => LogicalEncoding::PseudoDecimal,
            _ => Err(MltError::InvalidLogicalEncodings(logical1, logical2))?,
        };

        let meta = StreamMeta::new(
            stream_type,
            IntEncoding::new(logical_encoding, physical_encoding),
            num_values,
        );
        Ok((input, (meta, byte_length)))
    }

    pub fn write_to<W: Write>(
        &self,
        writer: &mut W,
        is_bool: bool,
        byte_length: u32,
    ) -> io::Result<()> {
        use LogicalEncoding as LE;
        use LogicalTechnique as LT;

        writer.write_u8(self.stream_type.as_u8())?;
        let logical_enc_u8: u8 = match self.encoding.logical {
            LE::None => (LT::None as u8) << 5,
            LE::Delta => (LT::Delta as u8) << 5,
            LE::DeltaRle(_) => ((LT::Delta as u8) << 5) | ((LT::Rle as u8) << 2),
            LE::ComponentwiseDelta => (LT::ComponentwiseDelta as u8) << 5,
            LE::Rle(_) => (LT::Rle as u8) << 5,
            LE::Morton(_) => (LT::Morton as u8) << 5,
            LE::MortonRle(_) => (LT::Morton as u8) << 5 | ((LT::Rle as u8) << 2),
            LE::MortonDelta(_) => (LT::Morton as u8) << 5 | ((LT::Delta as u8) << 2),
            LE::PseudoDecimal => (LT::PseudoDecimal as u8) << 5,
        };
        let physical_enc_u8: u8 = match self.encoding.physical {
            PhysicalEncoding::None => 0x0,
            PhysicalEncoding::FastPFOR => 0x1,
            PhysicalEncoding::VarInt => 0x2,
            PhysicalEncoding::Alp => 0x3,
        };
        writer.write_u8(logical_enc_u8 | physical_enc_u8)?;
        writer.write_varint(self.num_values)?;
        writer.write_varint(byte_length)?;

        // some encoding have settings inside them
        match self.encoding.logical {
            LE::DeltaRle(r) | LE::Rle(r) => {
                if !is_bool {
                    writer.write_varint(r.runs)?;
                    writer.write_varint(r.num_rle_values)?;
                }
            }
            LE::Morton(m) | LE::MortonDelta(m) | LE::MortonRle(m) => {
                writer.write_varint(m.num_bits)?;
                writer.write_varint(m.coordinate_shift)?;
            }
            LE::None | LE::Delta | LE::ComponentwiseDelta | LE::PseudoDecimal => {}
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
            encoding,
            num_values,
        } = self;
        f.debug_struct("StreamMeta")
            .field("stream_type", &format_args!("{stream_type:?}"))
            .field("logical_encoding", &format_args!("{:?}", encoding.logical))
            .field(
                "physical_encoding",
                &format_args!("{:?}", encoding.physical),
            )
            .field("num_values", &format_args!("{num_values:?}"))
            .finish()
    }
}

impl<'a> RawStream<'a> {
    #[must_use]
    pub fn new(meta: StreamMeta, data: RawStreamData<'a>) -> Self {
        Self { meta, data }
    }

    #[must_use]
    pub fn as_bytes(&self) -> &'a [u8] {
        match &self.data {
            RawStreamData::Encoded(v) | RawStreamData::VarInt(v) => v,
        }
    }

    pub fn from_bytes(input: &'a [u8]) -> MltRefResult<'a, Self> {
        Self::from_bytes_internal(input, false)
    }

    pub fn parse_multiple(mut input: &'a [u8], count: usize) -> MltRefResult<'a, Vec<Self>> {
        let mut result = Vec::with_capacity(count);
        for _ in 0..count {
            let stream;
            (input, stream) = RawStream::from_bytes_internal(input, false)?;
            result.push(stream);
        }
        Ok((input, result))
    }

    pub fn parse_bool(input: &'a [u8]) -> MltRefResult<'a, Self> {
        Self::from_bytes_internal(input, true)
    }

    /// Parse stream from the input
    /// If `is_bool` is true, compute RLE parameters for boolean streams
    /// automatically instead of reading them from the input.
    fn from_bytes_internal(input: &'a [u8], is_bool: bool) -> MltRefResult<'a, Self> {
        use PhysicalEncoding as PD;
        let (input, (meta, byte_length)) = StreamMeta::from_bytes(input, is_bool)?;

        let (input, data) = take(input, byte_length)?;

        let stream_data = match meta.encoding.physical {
            PD::None | PD::FastPFOR => RawStreamData::Encoded(data),
            PD::VarInt => RawStreamData::VarInt(data),
            PD::Alp => return Err(MltError::UnsupportedPhysicalEncoding("ALP")),
        };

        Ok((input, RawStream::new(meta, stream_data)))
    }
}
