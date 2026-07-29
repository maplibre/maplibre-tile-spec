//! Stream-header wire codec for tag `0x01` (v1) layers.
//!
//! A v1 stream header is laid out as:
//!
//! ```text
//! [u8     stream_type]      category / subtype nibbles
//! [u8     encoding]         logical1 (bits 7-5), logical2 (bits 4-2), physical (bits 1-0)
//! [varint num_values]
//! [varint byte_length]
//! [varint runs]             RLE streams only (non-bool)
//! [varint num_rle_values]   RLE streams only (non-bool)
//! [varint bits]             Morton streams only
//! [varint shift]            Morton streams only
//! ```
//!
//! Only the wire (de)serialization lives in this module. [`StreamMeta`] itself
//! is a format-independent in-memory descriptor (see `model.rs`), so other
//! layer formats can parse into / write from the same types with their own
//! header codec.

use std::io;

use integer_encoding::VarIntWriter as _;
use usize_cast::IntoUsize as _;

use crate::MltError::ParsingStreamType;
use crate::codecs::varint::parse_varint;
use crate::decoder::{
    DictionaryType, IntEncoding, LengthType, LogicalEncoding, LogicalTechnique, Morton, OffsetType,
    PhysicalEncoding, RawStream, RleMeta, StreamMeta, StreamType,
};
use crate::errors::{AsMltError as _, fail_if_invalid_stream_size};
use crate::utils::{BinarySerializer as _, parse_u8, take};
use crate::{MltError, MltRefResult, MltResult, Parser};

impl StreamType {
    /// Parse the v1 `stream_type` byte: category in the high nibble, subtype in the low nibble.
    pub fn from_bytes(input: &'_ [u8]) -> MltRefResult<'_, Self> {
        let (input, value) = parse_u8(input)?;
        let pt = Self::from_u8(value).ok_or(ParsingStreamType(value))?;
        Ok((input, pt))
    }

    fn from_u8(value: u8) -> Option<Self> {
        let high4 = value >> 4;
        let low4 = value & 0x0F;
        Some(match high4 {
            #[cfg(fuzzing)]
            // when fuzzing, we cannot have ignored bits, to preserve roundtrip-ability
            0 if low4 == 0 => StreamType::Present,
            #[cfg(not(fuzzing))]
            0 => Self::Present,
            1 => Self::Data(DictionaryType::try_from(low4).ok()?),
            2 => Self::Offset(OffsetType::try_from(low4).ok()?),
            3 => Self::Length(LengthType::try_from(low4).ok()?),
            _ => return None,
        })
    }

    /// Serialize to the v1 `stream_type` byte.
    #[must_use]
    pub fn as_u8(self) -> u8 {
        let proto_high4 = match self {
            Self::Present => 0,
            Self::Data(_) => 1,
            Self::Offset(_) => 2,
            Self::Length(_) => 3,
        };
        let high4 = proto_high4 << 4;
        let low4 = match self {
            Self::Present => 0,
            Self::Data(i) => i as u8,
            Self::Offset(i) => i as u8,
            Self::Length(i) => i as u8,
        };
        debug_assert!(low4 <= 0x0F, "secondary types should not exceed 4 bit");
        high4 | low4
    }
}

impl StreamMeta {
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
                let rle = RleMeta::Split {
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
                let morton = Morton::new(bits, shift)?;
                match logical2 {
                    LT::Rle => LogicalEncoding::MortonRle(morton),
                    LT::Delta => LogicalEncoding::MortonDelta(morton),
                    _ => LogicalEncoding::Morton(morton),
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

    pub(crate) fn write_to<W: io::Write>(
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
            // v1 always uses the Split layout; interleaved is a v2-only concern.
            LE::DeltaRle(RleMeta::Split {
                runs,
                num_rle_values,
            })
            | LE::Rle(RleMeta::Split {
                runs,
                num_rle_values,
            }) => {
                if !is_bool {
                    writer.write_varint(runs)?;
                    writer.write_varint(num_rle_values)?;
                }
            }
            LE::DeltaRle(RleMeta::Interleaved { .. }) | LE::Rle(RleMeta::Interleaved { .. }) => {
                debug_assert!(false, "v1 stream header codec cannot emit interleaved RLE");
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

impl<'a> RawStream<'a> {
    pub(crate) fn from_bytes(input: &'a [u8], parser: &mut Parser) -> MltRefResult<'a, Self> {
        Self::from_bytes_internal(input, false, parser)
    }

    pub(crate) fn parse_multiple(
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

    pub(crate) fn parse_bool(input: &'a [u8], parser: &mut Parser) -> MltRefResult<'a, Self> {
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

        // For RLE with VarInt physical encoding, validate stream: run lengths must sum to num_rle_values.
        // v1 parsing only ever produces the Split layout.
        if let LE::Rle(RleMeta::Split {
            runs,
            num_rle_values,
        })
        | LE::DeltaRle(RleMeta::Split {
            runs,
            num_rle_values,
        }) = meta.encoding.logical
            && matches!(meta.encoding.physical, PD::VarInt)
            && !is_bool
        {
            validate_rle_varint_stream(data, runs, num_rle_values)?;
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
        fail_if_invalid_stream_size(sum_usize, num_rle_values.into_usize())?;
    }
    Ok(())
}
