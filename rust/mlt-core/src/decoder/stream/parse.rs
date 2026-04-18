use std::{fmt, io};

use integer_encoding::VarIntWriter as _;

use crate::codecs::varint::parse_varint;
use crate::decoder::{
    GridParams, IntEncoding, LogicalEncoding, LogicalTechnique, PhysicalEncoding, RawStream,
    RleMeta, StreamMeta, StreamType,
};
use crate::errors::{AsMltError as _, fail_if_invalid_stream_size};
use crate::utils::{AsUsize as _, BinarySerializer as _, parse_u8, take};
use crate::{MltError, MltRefResult, MltResult, Parser};

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
    /// Returns the stream metadata and the size of the stream in bytes.
    /// Reserves an upper-bound estimate of decoded bytes (`num_values * 8`) on the parser
    /// for all stream types. RLE uses `num_rle_values * 8` since that is the actual expanded count.
    pub(crate) fn from_bytes<'a>(
        input: &'a [u8],
        is_bool: bool,
        parser: &mut Parser,
    ) -> MltRefResult<'a, (Self, u32)> {
        use crate::decoder::LogicalTechnique as LT;

        let (input, stream_type) = StreamType::from_bytes(input)?;
        let (input, val) = parse_u8(input)?;
        let logical1 = LT::parse(val >> 5)?;
        let logical2 = LT::parse((val >> 2) & 0x7)?;
        let physical_encoding = PhysicalEncoding::parse(val & 0x3)?;

        let (input, num_values) = parse_varint::<u32>(input)?;
        let (input, byte_length) = parse_varint::<u32>(input)?;

        let mut input = input;
        let logical_encoding = match (logical1, logical2) {
            (LT::None | LT::Delta | LT::ComponentwiseDelta | LT::PseudoDecimal, LT::None) => {
                // Reserve decoded memory upper bound: worst case u64 = 8 bytes per value
                let decoded_bytes = num_values.saturating_mul(8);
                parser.reserve(decoded_bytes)?;
                match logical1 {
                    LT::None => LogicalEncoding::None,
                    LT::Delta => LogicalEncoding::Delta,
                    LT::ComponentwiseDelta => LogicalEncoding::ComponentwiseDelta,
                    _ => LogicalEncoding::PseudoDecimal,
                }
            }
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
                // Reserve decoded memory (worst case: u64 = 8 bytes per value)
                let decoded_bytes = num_rle_values.saturating_mul(8);
                parser.reserve(decoded_bytes)?;
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
                // Reserve decoded memory upper bound: worst case u64 = 8 bytes per value
                let decoded_bytes = num_values.saturating_mul(8);
                parser.reserve(decoded_bytes)?;
                let bits;
                let shift;
                (input, bits) = parse_varint::<u32>(input)?;
                (input, shift) = parse_varint::<u32>(input)?;
                let meta = GridParams { bits, shift };
                match logical2 {
                    LT::Rle => LogicalEncoding::MortonRle(meta),
                    LT::Delta => LogicalEncoding::MortonDelta(meta),
                    _ => LogicalEncoding::Morton(meta),
                }
            }
            _ => Err(MltError::InvalidLogicalEncodings(logical1, logical2))?,
        };

        let meta = Self::new(
            stream_type,
            IntEncoding::new(logical_encoding, physical_encoding),
            num_values,
        );
        Ok((input, (meta, byte_length)))
    }

    pub fn write_to<W: io::Write>(
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
            PhysicalEncoding::FastPFor256 => 0x1,
            PhysicalEncoding::VarInt => 0x2,
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
                writer.write_varint(m.bits)?;
                writer.write_varint(m.shift)?;
            }
            LE::None | LE::Delta | LE::ComponentwiseDelta | LE::PseudoDecimal => {}
        }
        Ok(())
    }
}

impl fmt::Debug for StreamMeta {
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
    pub fn new(meta: StreamMeta, data: &'a [u8]) -> Self {
        Self { meta, data }
    }

    pub fn from_bytes(input: &'a [u8], parser: &mut Parser) -> MltRefResult<'a, Self> {
        Self::from_bytes_internal(input, false, parser)
    }

    pub fn parse_multiple(
        mut input: &'a [u8],
        count: usize,
        parser: &mut Parser,
    ) -> MltRefResult<'a, Vec<Self>> {
        let mut result = Vec::with_capacity(count);
        for _ in 0..count {
            let stream;
            (input, stream) = RawStream::from_bytes_internal(input, false, parser)?;
            result.push(stream);
        }
        Ok((input, result))
    }

    pub fn parse_bool(input: &'a [u8], parser: &mut Parser) -> MltRefResult<'a, Self> {
        Self::from_bytes_internal(input, true, parser)
    }

    /// Parse stream from the input
    /// If `is_bool` is true, compute RLE parameters for boolean streams
    /// automatically instead of reading them from the input.
    /// For RLE streams with `VarInt` data, validates that run lengths sum to `num_rle_values`.
    fn from_bytes_internal(
        input: &'a [u8],
        is_bool: bool,
        parser: &mut Parser,
    ) -> MltRefResult<'a, Self> {
        use LogicalEncoding as LE;
        use PhysicalEncoding as PD;

        let (input, (meta, byte_length)) = StreamMeta::from_bytes(input, is_bool, parser)?;
        let (input, data) = take(input, byte_length)?;

        // For RLE with VarInt physical encoding, validate stream: run lengths must sum to num_rle_values
        if let LE::Rle(r) | LE::DeltaRle(r) = meta.encoding.logical
            && matches!(meta.encoding.physical, PD::VarInt)
            && !is_bool
        {
            validate_rle_varint_stream(data, r.runs, r.num_rle_values)?;
        }

        Ok((input, RawStream::new(meta, data)))
    }
}

/// Validate RLE stream data: first `runs` varints must sum to `num_rle_values`.
fn validate_rle_varint_stream(data: &[u8], runs: u32, num_rle_values: u32) -> MltResult<()> {
    let mut rest = data;
    let mut sum: u64 = 0;
    for _ in 0..runs {
        let (next, len) = parse_varint::<u32>(rest)?;
        rest = next;
        sum = sum.checked_add(len.into()).or_overflow()?;
    }
    if sum != u64::from(num_rle_values) {
        let sum_usize = usize::try_from(sum).map_err(|_| MltError::IntegerOverflow)?;
        fail_if_invalid_stream_size(sum_usize, num_rle_values.as_usize())?;
    }
    Ok(())
}
