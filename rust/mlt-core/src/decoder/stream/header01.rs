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
//! [`StreamMeta`] itself is a format-independent in-memory descriptor (see `model.rs`), so other layer
//! formats can parse into / write from the same types with their own header codec.

use std::io;

use integer_encoding::VarIntWriter as _;
use num_enum::TryFromPrimitive;
use usize_cast::IntoUsize as _;

use crate::MltError::ParsingStreamType;
#[cfg(feature = "unstable-v2")]
use crate::MltError::UnsupportedLogicalEncoding;
use crate::codecs::varint::parse_varint;
use crate::decoder::{
    DictionaryType, IntEncoding, LengthType, LogicalCombination, LogicalEncoding, LogicalTechnique,
    Morton, OffsetType, PhysicalEncoding, RawStream, RleMeta, StreamMeta, StreamType,
};
use crate::errors::{AsMltError as _, fail_if_invalid_stream_size};
use crate::utils::{BinarySerializer as _, parse_u8, take};
use crate::{MltError, MltRefResult, MltResult, Parser};

/// Mask of the `stream_type` byte holding the [`CategoryField`].
const CATEGORY_MASK: u8 = 0b1111_0000;

/// Mask of the `stream_type` byte holding the subtype.
const SUBTYPE_MASK: u8 = 0b0000_1111;

/// Mask of the encoding byte holding the primary [`LogicalTechnique`].
const LOGICAL1_MASK: u8 = 0b1110_0000;

/// Mask of the encoding byte holding the secondary [`LogicalTechnique`].
const LOGICAL2_MASK: u8 = 0b0001_1100;

/// Distance between the primary logical field and the secondary one.
const LOGICAL2_SHIFT: u32 = 3;

/// Mask of the encoding byte holding both logical fields, i.e. a [`LogicalCombination`].
const LOGICAL_MASK: u8 = LOGICAL1_MASK | LOGICAL2_MASK;

/// Mask of the encoding byte holding the [`PhysicalEncoding`].
const PHYSICAL_MASK: u8 = 0b0000_0011;

/// Category field (bits 7-4 of the `stream_type` byte), already shifted into place.
#[derive(Debug, Clone, Copy, PartialEq, TryFromPrimitive)]
#[repr(u8)]
enum CategoryField {
    Present = 0b0000_0000,
    Data = 0b0001_0000,
    Offset = 0b0010_0000,
    Length = 0b0011_0000,
}

/// Read both logical fields of the encoding byte as the combination they spell out.
///
/// Combinations that are not legal on the wire are reported as [`MltError::InvalidLogicalEncodings`]
/// naming both fields, so decompose the byte again on that path.
fn parse_logical(encoding_byte: u8) -> MltResult<LogicalCombination> {
    LogicalCombination::try_from(encoding_byte & LOGICAL_MASK).or_else(|_| {
        let primary = LogicalTechnique::parse(encoding_byte & LOGICAL1_MASK)?;
        let secondary = LogicalTechnique::parse((encoding_byte & LOGICAL2_MASK) << LOGICAL2_SHIFT)?;
        Err(MltError::InvalidLogicalEncodings(primary, secondary))
    })
}

/// Assemble the encoding byte from its logical and physical fields.
fn encoding_byte(logical: LogicalCombination, physical: PhysicalEncoding) -> u8 {
    logical as u8 | physical as u8
}

/// Parse the v1 `stream_type` byte: category in the high nibble, subtype in the low nibble.
fn stream_type_from_byte(value: u8) -> Option<StreamType> {
    let category = CategoryField::try_from(value & CATEGORY_MASK).ok()?;
    let subtype = value & SUBTYPE_MASK;
    // when fuzzing, we cannot have ignored bits, to preserve roundtrip-ability
    #[cfg(fuzzing)]
    if category == CategoryField::Present && subtype != 0 {
        return None;
    }
    Some(match category {
        CategoryField::Present => StreamType::Present,
        CategoryField::Data => StreamType::Data(DictionaryType::try_from(subtype).ok()?),
        CategoryField::Offset => StreamType::Offset(OffsetType::try_from(subtype).ok()?),
        CategoryField::Length => StreamType::Length(LengthType::try_from(subtype).ok()?),
    })
}

/// Serialize to the v1 `stream_type` byte.
fn stream_type_to_byte(stream_type: StreamType) -> u8 {
    let (category, subtype) = match stream_type {
        StreamType::Present => (CategoryField::Present, 0),
        StreamType::Data(i) => (CategoryField::Data, i as u8),
        StreamType::Offset(i) => (CategoryField::Offset, i as u8),
        StreamType::Length(i) => (CategoryField::Length, i as u8),
    };
    debug_assert!(
        subtype <= SUBTYPE_MASK,
        "secondary types should not exceed 4 bit"
    );
    category as u8 | subtype
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
    use LogicalCombination as LC;

    let (input, st_byte) = parse_u8(input)?;
    let stream_type = stream_type_from_byte(st_byte).ok_or(ParsingStreamType(st_byte))?;

    let (input, val) = parse_u8(input)?;
    let logical = parse_logical(val)?;
    let physical_encoding = PhysicalEncoding::parse(val & PHYSICAL_MASK)?;

    let (input, num_values) = parse_varint::<u32>(input)?;
    let (input, byte_length) = parse_varint::<u32>(input)?;

    let mut input = input;
    let logical_encoding = match logical {
        LC::None | LC::Delta | LC::ComponentwiseDelta => {
            // Reserve decoded memory upper bound: worst case u64 = 8 bytes per value
            let decoded_bytes = num_values.saturating_mul(8);
            parser.reserve(decoded_bytes)?;
            if logical == LC::None {
                LogicalEncoding::None
            } else if logical == LC::Delta {
                LogicalEncoding::Delta
            } else {
                LogicalEncoding::ComponentwiseDelta
            }
        }
        LC::Rle | LC::DeltaRle => {
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
            if logical == LC::Rle {
                LogicalEncoding::Rle(rle)
            } else {
                LogicalEncoding::DeltaRle(rle)
            }
        }
        LC::Morton | LC::MortonRle | LC::MortonDelta => {
            // Reserve decoded memory upper bound: worst case u64 = 8 bytes per value
            let decoded_bytes = num_values.saturating_mul(8);
            parser.reserve(decoded_bytes)?;
            let bits;
            let shift;
            (input, bits) = parse_varint::<u32>(input)?;
            (input, shift) = parse_varint::<u32>(input)?;
            let morton = Morton::new(bits, shift)?;
            if logical == LC::MortonRle {
                LogicalEncoding::MortonRle(morton)
            } else if logical == LC::MortonDelta {
                LogicalEncoding::MortonDelta(morton)
            } else {
                LogicalEncoding::Morton(morton)
            }
        }
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
    use LogicalCombination as LC;
    use LogicalEncoding as LE;

    writer.write_u8(stream_type_to_byte(meta.stream_type))?;
    let logical = match meta.encoding.logical {
        LE::None => LC::None,
        LE::Delta => LC::Delta,
        LE::DeltaRle(_) => LC::DeltaRle,
        LE::ComponentwiseDelta => LC::ComponentwiseDelta,
        LE::Rle(_) => LC::Rle,
        LE::Morton(_) => LC::Morton,
        LE::MortonRle(_) => LC::MortonRle,
        LE::MortonDelta(_) => LC::MortonDelta,
    };
    writer.write_u8(encoding_byte(logical, meta.encoding.physical))?;
    writer.write_varint(meta.num_values)?;
    writer.write_varint(byte_length)?;

    // some encoding have settings inside them
    match meta.encoding.logical {
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
        #[cfg(feature = "unstable-v2")]
        LE::DeltaRle(RleMeta::Interleaved { .. }) | LE::Rle(RleMeta::Interleaved { .. }) => {
            return Err(UnsupportedLogicalEncoding(
                meta.encoding.logical,
                "v1 stream header codec requires the Split RLE layout",
            ));
        }
        LE::Morton(m) | LE::MortonDelta(m) | LE::MortonRle(m) => {
            writer.write_varint(m.bits)?;
            writer.write_varint(m.shift)?;
        }
        LE::None | LE::Delta | LE::ComponentwiseDelta => {}
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

    /// Place two techniques into the logical fields of the encoding byte.
    fn logical_bits(primary: LogicalTechnique, secondary: LogicalTechnique) -> u8 {
        primary as u8 | ((secondary as u8) >> LOGICAL2_SHIFT)
    }

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
        let rle = RleMeta::Split {
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

    #[rstest]
    #[case::none(
        LogicalCombination::None,
        LogicalTechnique::None,
        LogicalTechnique::None
    )]
    #[case::delta(
        LogicalCombination::Delta,
        LogicalTechnique::Delta,
        LogicalTechnique::None
    )]
    #[case::delta_rle(
        LogicalCombination::DeltaRle,
        LogicalTechnique::Delta,
        LogicalTechnique::Rle
    )]
    #[case::cw_delta(
        LogicalCombination::ComponentwiseDelta,
        LogicalTechnique::ComponentwiseDelta,
        LogicalTechnique::None
    )]
    #[case::rle(LogicalCombination::Rle, LogicalTechnique::Rle, LogicalTechnique::None)]
    #[case::morton(
        LogicalCombination::Morton,
        LogicalTechnique::Morton,
        LogicalTechnique::None
    )]
    #[case::morton_delta(
        LogicalCombination::MortonDelta,
        LogicalTechnique::Morton,
        LogicalTechnique::Delta
    )]
    #[case::morton_rle(
        LogicalCombination::MortonRle,
        LogicalTechnique::Morton,
        LogicalTechnique::Rle
    )]
    fn combination_holds_both_field_bits(
        #[case] combination: LogicalCombination,
        #[case] primary: LogicalTechnique,
        #[case] secondary: LogicalTechnique,
    ) {
        assert_eq!(combination as u8, logical_bits(primary, secondary));
    }

    #[cfg(feature = "unstable-v2")]
    #[rstest]
    #[case::rle(true)]
    #[case::delta_rle(false)]
    fn write_rejects_interleaved_rle(#[case] plain_rle: bool) {
        let rle = RleMeta::Interleaved { num_rle_values: 5 };
        let logical = if plain_rle {
            LogicalEncoding::Rle(rle)
        } else {
            LogicalEncoding::DeltaRle(rle)
        };
        let meta = meta(logical, PhysicalEncoding::VarInt, 4);
        let mut buf = Vec::new();
        let err = write_stream_meta(&meta, &mut buf, false, 4).unwrap_err();
        assert!(matches!(err, UnsupportedLogicalEncoding(_, _)));
    }

    #[test]
    fn rejects_invalid_logical_combination() {
        let enc_byte = logical_bits(LogicalTechnique::ComponentwiseDelta, LogicalTechnique::Rle);
        let buf = [0u8, enc_byte, 0, 0];
        let err = parse_stream(&buf, &mut parser()).unwrap_err();
        assert!(matches!(err, MltError::InvalidLogicalEncodings(_, _)));
    }
}
