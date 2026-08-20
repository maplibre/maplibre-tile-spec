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
//! Only the wire (de)serialization lives in this module, as a set of free
//! functions mirroring [`super::header02`]'s shape. [`StreamMeta`] itself is a
//! format-independent in-memory descriptor (see `model.rs`), so other layer
//! formats can parse into / write from the same types with their own header
//! codec.

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

/// Parse the v1 `stream_type` byte: category in the high nibble, subtype in the low nibble.
fn stream_type_from_byte(value: u8) -> Option<StreamType> {
    let high4 = value >> 4;
    let low4 = value & 0x0F;
    Some(match high4 {
        #[cfg(fuzzing)]
        // when fuzzing, we cannot have ignored bits, to preserve roundtrip-ability
        0 if low4 == 0 => StreamType::Present,
        #[cfg(not(fuzzing))]
        0 => StreamType::Present,
        1 => StreamType::Data(DictionaryType::try_from(low4).ok()?),
        2 => StreamType::Offset(OffsetType::try_from(low4).ok()?),
        3 => StreamType::Length(LengthType::try_from(low4).ok()?),
        _ => return None,
    })
}

/// Serialize to the v1 `stream_type` byte.
fn stream_type_to_byte(stream_type: StreamType) -> u8 {
    let proto_high4 = match stream_type {
        StreamType::Present => 0,
        StreamType::Data(_) => 1,
        StreamType::Offset(_) => 2,
        StreamType::Length(_) => 3,
    };
    let high4 = proto_high4 << 4;
    let low4 = match stream_type {
        StreamType::Present => 0,
        StreamType::Data(i) => i as u8,
        StreamType::Offset(i) => i as u8,
        StreamType::Length(i) => i as u8,
    };
    debug_assert!(low4 <= 0x0F, "secondary types should not exceed 4 bit");
    high4 | low4
}

/// Parse the metadata portion of a v1 stream header (everything before the payload).
///
/// If `is_bool` is true, compute RLE parameters for boolean streams automatically
/// instead of reading them from the input.
///
/// Returns the stream metadata and the size of the payload in bytes. Reserves an
/// upper-bound estimate of decoded bytes (`num_values * 8`) on the parser for all
/// stream types. RLE uses `num_rle_values * 8` since that is the actual expanded count.
pub(crate) fn parse_stream_meta<'a>(
    input: &'a [u8],
    is_bool: bool,
    parser: &mut Parser,
) -> MltRefResult<'a, (StreamMeta, u32)> {
    use LogicalTechnique as LT;

    let (input, st_byte) = parse_u8(input)?;
    let stream_type = stream_type_from_byte(st_byte).ok_or(ParsingStreamType(st_byte))?;

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
            let morton = Morton::new(bits, shift)?;
            match logical2 {
                LT::Rle => LogicalEncoding::MortonRle(morton),
                LT::Delta => LogicalEncoding::MortonDelta(morton),
                _ => LogicalEncoding::Morton(morton),
            }
        }
        _ => Err(MltError::InvalidLogicalEncodings(logical1, logical2))?,
    };

    let meta = StreamMeta::new(
        stream_type,
        IntEncoding::new(logical_encoding, physical_encoding),
        num_values,
    );
    Ok((input, (meta, byte_length)))
}

/// Serialize a v1 stream header for `meta`.
///
/// If `is_bool` is true, the RLE `runs` / `num_rle_values` varints are omitted
/// (the reader derives them from context, as boolean streams always do).
pub(crate) fn write_stream_meta<W: io::Write>(
    meta: &StreamMeta,
    writer: &mut W,
    is_bool: bool,
    byte_length: u32,
) -> MltResult<()> {
    use LogicalEncoding as LE;
    use LogicalTechnique as LT;

    writer.write_u8(stream_type_to_byte(meta.stream_type))?;
    let logical_enc_u8: u8 = match meta.encoding.logical {
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
    let physical_enc_u8: u8 = match meta.encoding.physical {
        PhysicalEncoding::None => 0x0,
        PhysicalEncoding::FastPFor256 => 0x1,
        PhysicalEncoding::VarInt => 0x2,
    };
    writer.write_u8(logical_enc_u8 | physical_enc_u8)?;
    writer.write_varint(meta.num_values)?;
    writer.write_varint(byte_length)?;

    // some encoding have settings inside them
    match meta.encoding.logical {
        LE::DeltaRle(RleMeta {
            runs,
            num_rle_values,
        })
        | LE::Rle(RleMeta {
            runs,
            num_rle_values,
        }) => {
            if !is_bool {
                writer.write_varint(runs)?;
                writer.write_varint(num_rle_values)?;
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

/// Parse one v1 stream (header + data).
pub(crate) fn parse_stream<'a>(
    input: &'a [u8],
    parser: &mut Parser,
) -> MltRefResult<'a, RawStream<'a>> {
    parse_stream_internal(input, false, parser)
}

/// Parse `count` consecutive v1 streams.
pub(crate) fn parse_multiple_streams<'a>(
    mut input: &'a [u8],
    count: usize,
    parser: &mut Parser,
) -> MltRefResult<'a, Vec<RawStream<'a>>> {
    let mut result = Vec::with_capacity(count);
    for _ in 0..count {
        let stream;
        (input, stream) = parse_stream_internal(input, false, parser)?;
        result.push(stream);
    }
    Ok((input, result))
}

/// Parse one v1 boolean stream (header + data), deriving RLE parameters from context.
pub(crate) fn parse_bool_stream<'a>(
    input: &'a [u8],
    parser: &mut Parser,
) -> MltRefResult<'a, RawStream<'a>> {
    parse_stream_internal(input, true, parser)
}

/// Parse stream from the input.
/// If `is_bool` is true, compute RLE parameters for boolean streams
/// automatically instead of reading them from the input.
/// For RLE streams with `VarInt` data, validates that run lengths sum to `num_rle_values`.
fn parse_stream_internal<'a>(
    input: &'a [u8],
    is_bool: bool,
    parser: &mut Parser,
) -> MltRefResult<'a, RawStream<'a>> {
    use LogicalEncoding as LE;
    use PhysicalEncoding as PD;

    let (input, (meta, byte_length)) = parse_stream_meta(input, is_bool, parser)?;
    let (input, data) = take(input, byte_length)?;

    // For RLE with VarInt physical encoding, validate stream: run lengths must sum to num_rle_values.
    // v1 parsing only ever produces the Split layout.
    if let LE::Rle(RleMeta {
        runs,
        num_rle_values,
    })
    | LE::DeltaRle(RleMeta {
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

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::test_helpers::parser;

    const DATA: StreamType = StreamType::Data(DictionaryType::None);

    fn meta(logical: LogicalEncoding, physical: PhysicalEncoding, num: u32) -> StreamMeta {
        StreamMeta::new(DATA, IntEncoding::new(logical, physical), num)
    }

    #[rstest]
    #[case::none_varint(meta(LogicalEncoding::None, PhysicalEncoding::VarInt, 5), 0x02)]
    #[case::none_raw(meta(LogicalEncoding::None, PhysicalEncoding::None, 5), 0x00)]
    #[case::delta(meta(LogicalEncoding::Delta, PhysicalEncoding::VarInt, 5), 0x22)]
    #[case::cw_delta(
        meta(LogicalEncoding::ComponentwiseDelta, PhysicalEncoding::VarInt, 5),
        0x42
    )]
    fn encoding_byte_values(#[case] meta: StreamMeta, #[case] expected: u8) {
        let mut buf = Vec::new();
        write_stream_meta(&meta, &mut buf, false, 0).unwrap();
        assert_eq!(buf[1], expected);
    }

    #[rstest]
    #[case::none_varint(meta(LogicalEncoding::None, PhysicalEncoding::VarInt, 5))]
    #[case::none_raw(meta(LogicalEncoding::None, PhysicalEncoding::None, 5))]
    #[case::delta(meta(LogicalEncoding::Delta, PhysicalEncoding::VarInt, 5))]
    #[case::cw_delta(meta(LogicalEncoding::ComponentwiseDelta, PhysicalEncoding::VarInt, 5))]
    fn header_roundtrip(#[case] meta: StreamMeta) {
        let payload = [1_u8, 2, 3];
        let mut buf = Vec::new();
        let byte_length = u32::try_from(payload.len()).unwrap();
        write_stream_meta(&meta, &mut buf, false, byte_length).unwrap();
        buf.extend_from_slice(&payload);

        let (rest, stream) = parse_stream(&buf, &mut parser()).unwrap();
        assert!(rest.is_empty());
        assert_eq!(stream.meta, meta);
        assert_eq!(stream.data, payload);
    }

    #[rstest]
    #[case::rle(true)]
    #[case::delta_rle(false)]
    fn rle_header_roundtrip(#[case] plain_rle: bool) {
        let run_lengths = [2_u8, 3];
        let values = [10_u8, 20];
        let rle = RleMeta {
            runs: u32::try_from(run_lengths.len()).unwrap(),
            num_rle_values: u32::from(run_lengths.iter().sum::<u8>()),
        };
        let logical = if plain_rle {
            LogicalEncoding::Rle(rle)
        } else {
            LogicalEncoding::DeltaRle(rle)
        };
        let meta = meta(
            logical,
            PhysicalEncoding::VarInt,
            u32::try_from(run_lengths.len() + values.len()).unwrap(),
        );
        let payload = [run_lengths, values].concat();
        let mut buf = Vec::new();
        let byte_length = u32::try_from(payload.len()).unwrap();
        write_stream_meta(&meta, &mut buf, false, byte_length).unwrap();
        buf.extend_from_slice(&payload);

        let (rest, stream) = parse_stream(&buf, &mut parser()).unwrap();
        assert!(rest.is_empty());
        assert_eq!(stream.meta, meta);
        assert_eq!(stream.data, payload);
    }

    #[test]
    fn rejects_invalid_logical_combination() {
        let logical1 = LogicalTechnique::ComponentwiseDelta as u8;
        let logical2 = LogicalTechnique::Rle as u8;
        let enc_byte = (logical1 << 5) | (logical2 << 2);
        let buf = [0u8, enc_byte, 0, 0];
        let err = parse_stream(&buf, &mut parser()).unwrap_err();
        assert!(matches!(err, MltError::InvalidLogicalEncodings(_, _)));
    }
}
