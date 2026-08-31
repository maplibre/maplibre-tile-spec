//! Stream-header wire codec for tag `0x02` (v2) layers.
//!
//! A v2 stream header is a single encoding byte followed by optional varints:
//!
//! ```text
//! [u8 encoding_byte]
//!      bit  7:   has_explicit_count (1 = a count varint follows)
//!      bits 6-4: logical  (0=None, 1=Delta, 2=CwDelta, 3=Rle, 4=DeltaRle,
//!                          5=Morton, 6=PseudoDecimal, 7=reserved)
//!      bits 3-2: physical (0=None-noLen, 1=None-withLen, 2=VarInt, 3=FastPFor128);
//!                reserved (must be 0) for Rle/DeltaRle, whose physical
//!                encoding is implied to be VarInt
//!      bits 1-0: reserved (must be 0)
//! [varint num_values]   only when has_explicit_count = 1; otherwise the count
//!                       comes from context (feature_count, or the presence
//!                       popcount for optional column data)
//! [varint byte_length]  present unless physical = None-noLen
//! ```
//!
//! Compared to v1 ([`super::header01`]), the `stream_type` byte is gone (the
//! role is implied by stream position), `num_values` is omitted when derivable
//! from context, and RLE streams carry no `runs` / `num_rle_values` varints:
//! the data is interleaved `(run, value)` pairs and the expanded count comes
//! from the count context.
//!
//! Not yet implemented (rejected with [`MltError::NotImplemented`]):
//! `None-noLen` (requires element-width context), `FastPFor128`, `Morton`,
//! and `PseudoDecimal`.

use std::io;

use integer_encoding::VarIntWriter as _;
use num_enum::TryFromPrimitive;

use crate::codecs::varint::parse_varint;
use crate::decoder::{
    IntEncoding, LogicalEncoding, PhysicalEncoding, RawStream, RleMeta, StreamMeta, StreamType,
};
use crate::utils::{BinarySerializer as _, parse_u8, take};
use crate::{MltError, MltRefResult, MltResult, Parser};

/// Bit 7 of the encoding byte: an explicit count varint follows.
pub(crate) const HAS_EXPLICIT_COUNT: u8 = 0b1000_0000;

/// Mask of the encoding byte holding the [`LogicalField`].
pub(crate) const LOGICAL_MASK: u8 = 0b0111_0000;

/// Mask of the encoding byte holding the [`PhysicalField`].
pub(crate) const PHYSICAL_MASK: u8 = 0b0000_1100;

/// Mask of the encoding byte bits held in reserve, which must be zero.
pub(crate) const ENCODING_RESERVED_MASK: u8 = 0b0000_0011;

/// Logical field (bits 6-4 of the encoding byte). Bit pattern 7 is reserved.
///
/// Variants are already shifted into place, so the byte is matched with
/// `byte & LOGICAL_MASK` rather than shifted down first.
#[derive(Debug, Clone, Copy, PartialEq, TryFromPrimitive)]
#[repr(u8)]
pub(crate) enum LogicalField {
    None = 0b0000_0000,
    Delta = 0b0001_0000,
    CwDelta = 0b0010_0000,
    Rle = 0b0011_0000,
    DeltaRle = 0b0100_0000,
    Morton = 0b0101_0000,
    PseudoDecimal = 0b0110_0000,
}

/// Physical field (bits 3-2 of the encoding byte), also shifted into place.
#[derive(Debug, Clone, Copy, PartialEq, TryFromPrimitive)]
#[repr(u8)]
pub(crate) enum PhysicalField {
    NoneNoLen = 0b0000_0000,
    NoneWithLen = 0b0000_0100,
    VarInt = 0b0000_1000,
    FastPFor128 = 0b0000_1100,
}

/// Parse one v2 stream (header + data), synthesizing [`StreamMeta`] from the
/// wire header plus positional context.
///
/// - `stream_type` is the role implied by the stream's position; it is not
///   stored on the v2 wire.
/// - `implicit_count` is the count implied by context: `feature_count`, or the
///   presence popcount for an optional column's data stream.
///
/// Reserves an upper-bound estimate of decoded bytes (`num_values * 8`) on the
/// parser, mirroring the v1 codec.
pub(crate) fn parse_stream<'a>(
    input: &'a [u8],
    stream_type: StreamType,
    implicit_count: u32,
    parser: &mut Parser,
) -> MltRefResult<'a, RawStream<'a>> {
    let (input, enc_byte) = parse_u8(input)?;
    if enc_byte & ENCODING_RESERVED_MASK != 0 {
        return Err(MltError::ParsingEncodingByte(enc_byte));
    }
    let logical_field = LogicalField::try_from(enc_byte & LOGICAL_MASK)
        .map_err(|_| MltError::ParsingEncodingByte(enc_byte))?;
    let physical_field = PhysicalField::try_from(enc_byte & PHYSICAL_MASK)
        .map_err(|_| MltError::ParsingEncodingByte(enc_byte))?;

    let (input, num_values) = if enc_byte & HAS_EXPLICIT_COUNT == 0 {
        (input, implicit_count)
    } else {
        parse_varint::<u32>(input)?
    };
    // Reserve decoded memory upper bound: worst case u64 = 8 bytes per value.
    parser.reserve(num_values.saturating_mul(8))?;

    let encoding = match logical_field {
        LogicalField::Rle | LogicalField::DeltaRle => {
            if physical_field != PhysicalField::NoneNoLen {
                return Err(MltError::ParsingEncodingByte(enc_byte));
            }
            let rle = RleMeta::Interleaved {
                num_rle_values: num_values,
            };
            let logical = if logical_field == LogicalField::Rle {
                LogicalEncoding::Rle(rle)
            } else {
                LogicalEncoding::DeltaRle(rle)
            };
            IntEncoding::new(logical, PhysicalEncoding::VarInt)
        }
        LogicalField::Morton => return Err(MltError::NotImplemented("v2 Morton streams")),
        LogicalField::PseudoDecimal => {
            return Err(MltError::NotImplemented("v2 PseudoDecimal streams"));
        }
        LogicalField::None => IntEncoding::new(
            LogicalEncoding::None,
            physical_encoding_for(physical_field)?,
        ),
        LogicalField::Delta => IntEncoding::new(
            LogicalEncoding::Delta,
            physical_encoding_for(physical_field)?,
        ),
        LogicalField::CwDelta => IntEncoding::new(
            LogicalEncoding::ComponentwiseDelta,
            physical_encoding_for(physical_field)?,
        ),
    };

    let (input, byte_length) = parse_varint::<u32>(input)?;
    let (input, data) = take(input, byte_length)?;
    let meta = StreamMeta::new(stream_type, encoding, num_values);
    Ok((input, RawStream::new(meta, data)))
}

/// Serialize a v2 stream header for `meta`.
///
/// `implicit_count` is the count the decoder will infer from context; an
/// explicit count varint is emitted only when `meta.num_values` differs.
///
/// The physical field is emitted as `None-withLen` for raw streams - the
/// `None-noLen` optimization (deriving byte length from an element width)
/// is not implemented yet.
// TODO(v2): emit None-noLen when the element width is unambiguous, saving the
//           byte_length varint on raw fixed-width streams.
pub(crate) fn write_stream_meta<W: io::Write>(
    meta: &StreamMeta,
    writer: &mut W,
    byte_length: u32,
    implicit_count: u32,
) -> MltResult<()> {
    use LogicalEncoding as LE;

    let (logical_field, physical_field) = match meta.encoding.logical {
        LE::None => (
            LogicalField::None,
            physical_field_for(meta.encoding.physical)?,
        ),
        LE::Delta => (
            LogicalField::Delta,
            physical_field_for(meta.encoding.physical)?,
        ),
        LE::ComponentwiseDelta => (
            LogicalField::CwDelta,
            physical_field_for(meta.encoding.physical)?,
        ),
        LE::Rle(rle) | LE::DeltaRle(rle) => {
            if !matches!(rle, RleMeta::Interleaved { .. }) {
                return Err(MltError::UnsupportedLogicalEncoding(
                    meta.encoding.logical,
                    "v2 stream header codec requires the Interleaved RLE layout",
                ));
            }
            if meta.encoding.physical != PhysicalEncoding::VarInt {
                return Err(MltError::UnsupportedPhysicalEncoding(
                    "v2 RLE requires VarInt",
                ));
            }
            let logical = if matches!(meta.encoding.logical, LE::Rle(_)) {
                LogicalField::Rle
            } else {
                LogicalField::DeltaRle
            };
            (logical, PhysicalField::NoneNoLen)
        }
        LE::Morton(_) | LE::MortonDelta(_) | LE::MortonRle(_) => {
            return Err(MltError::NotImplemented("v2 Morton streams"));
        }
        LE::PseudoDecimal => return Err(MltError::NotImplemented("v2 PseudoDecimal streams")),
    };

    // For RLE streams the wire count is the *decoded* count (the encoder's
    // in-memory `num_values` holds the encoded word count, which a v2 decoder
    // derives by scanning the pairs to `byte_length`).
    let num_values = match meta.encoding.logical {
        LE::Rle(rle) | LE::DeltaRle(rle) => rle.num_rle_values(),
        _ => meta.num_values,
    };
    let explicit = num_values != implicit_count;
    let enc_byte =
        if explicit { HAS_EXPLICIT_COUNT } else { 0 } | logical_field as u8 | physical_field as u8;
    writer.write_u8(enc_byte)?;
    if explicit {
        writer.write_varint(num_values)?;
    }
    writer.write_varint(byte_length)?;
    Ok(())
}

/// Map an in-memory physical encoding to the v2 physical field.
fn physical_field_for(physical: PhysicalEncoding) -> MltResult<PhysicalField> {
    match physical {
        PhysicalEncoding::None => Ok(PhysicalField::NoneWithLen),
        PhysicalEncoding::VarInt => Ok(PhysicalField::VarInt),
        PhysicalEncoding::FastPFor256 => Err(MltError::NotImplemented(
            "v2 FastPFor: requires the FastPFor128-LE codec",
        )),
    }
}

/// Map the v2 physical field to an in-memory physical encoding, for the
/// None/Delta/ComponentwiseDelta logical arms.
fn physical_encoding_for(field: PhysicalField) -> MltResult<PhysicalEncoding> {
    match field {
        PhysicalField::NoneWithLen => Ok(PhysicalEncoding::None),
        PhysicalField::VarInt => Ok(PhysicalEncoding::VarInt),
        PhysicalField::NoneNoLen => {
            Err(MltError::NotImplemented("v2 None-noLen physical encoding"))
        }
        PhysicalField::FastPFor128 => {
            Err(MltError::NotImplemented("v2 FastPFor128 physical encoding"))
        }
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::decoder::{DictionaryType, StreamType};
    use crate::test_helpers::parser;

    const DATA: StreamType = StreamType::Data(DictionaryType::None);

    fn meta(logical: LogicalEncoding, physical: PhysicalEncoding, num: u32) -> StreamMeta {
        StreamMeta::new(DATA, IntEncoding::new(logical, physical), num)
    }

    fn rle(num: u32) -> RleMeta {
        RleMeta::Interleaved {
            num_rle_values: num,
        }
    }

    /// Wire values of the encoding byte for the combinations PR2 emits.
    #[rstest]
    #[case::varint_implicit(
        meta(LogicalEncoding::None, PhysicalEncoding::VarInt, 5),
        5,
        0b0000_1000
    )]
    #[case::varint_explicit(
        meta(LogicalEncoding::None, PhysicalEncoding::VarInt, 5),
        9,
        0b1000_1000
    )]
    #[case::raw_implicit(meta(LogicalEncoding::None, PhysicalEncoding::None, 5), 5, 0b0000_0100)]
    #[case::delta_varint(
        meta(LogicalEncoding::Delta, PhysicalEncoding::VarInt, 5),
        5,
        0b0001_1000
    )]
    #[case::cw_delta_explicit(
        meta(LogicalEncoding::ComponentwiseDelta, PhysicalEncoding::VarInt, 8),
        5,
        0b1010_1000
    )]
    #[case::rle_implicit(
        meta(LogicalEncoding::Rle(rle(5)), PhysicalEncoding::VarInt, 5),
        5,
        0b0011_0000
    )]
    #[case::delta_rle(
        meta(LogicalEncoding::DeltaRle(rle(5)), PhysicalEncoding::VarInt, 5),
        5,
        0b0100_0000
    )]
    fn encoding_byte_values(
        #[case] meta: StreamMeta,
        #[case] implicit_count: u32,
        #[case] expected: u8,
    ) {
        let mut buf = Vec::new();
        write_stream_meta(&meta, &mut buf, 0, implicit_count).unwrap();
        assert_eq!(buf[0], expected);
    }

    /// Round-trip write -> parse for every PR2 combination.
    #[rstest]
    #[case::varint(meta(LogicalEncoding::None, PhysicalEncoding::VarInt, 5), 5)]
    #[case::varint_explicit(meta(LogicalEncoding::None, PhysicalEncoding::VarInt, 7), 5)]
    #[case::raw(meta(LogicalEncoding::None, PhysicalEncoding::None, 5), 5)]
    #[case::delta(meta(LogicalEncoding::Delta, PhysicalEncoding::VarInt, 5), 5)]
    #[case::cw_delta(
        meta(LogicalEncoding::ComponentwiseDelta, PhysicalEncoding::VarInt, 10),
        5
    )]
    #[case::rle(meta(LogicalEncoding::Rle(rle(5)), PhysicalEncoding::VarInt, 5), 5)]
    #[case::delta_rle(
        meta(LogicalEncoding::DeltaRle(rle(9)), PhysicalEncoding::VarInt, 9),
        5
    )]
    fn header_roundtrip(#[case] meta: StreamMeta, #[case] implicit_count: u32) {
        let payload = [1_u8, 2, 3];
        let mut buf = Vec::new();
        let byte_length = u32::try_from(payload.len()).unwrap();
        write_stream_meta(&meta, &mut buf, byte_length, implicit_count).unwrap();
        buf.extend_from_slice(&payload);

        let (rest, stream) = parse_stream(&buf, DATA, implicit_count, &mut parser()).unwrap();
        assert!(rest.is_empty());
        assert_eq!(stream.meta, meta);
        assert_eq!(stream.data, payload);
    }

    /// Bits 1-0 and the RLE physical bits are reserved and must be rejected.
    #[rstest]
    #[case::reserved_low_bits(0b0000_1001)]
    #[case::reserved_low_bits2(0b0000_1010)]
    #[case::rle_with_physical(0b0011_0100)]
    #[case::delta_rle_with_physical(0b0100_1000)]
    #[case::logical_reserved(0b0111_1000)]
    fn rejects_reserved_bits(#[case] enc_byte: u8) {
        let buf = [enc_byte, 0];
        let err = parse_stream(&buf, DATA, 0, &mut parser()).unwrap_err();
        assert!(matches!(err, MltError::ParsingEncodingByte(b) if b == enc_byte));
    }

    /// Spec'd but not-yet-implemented encodings fail cleanly.
    #[rstest]
    #[case::none_no_len(0b0000_0000)]
    #[case::fastpfor128(0b0000_1100)]
    #[case::morton(0b0101_0000)]
    #[case::pseudo_decimal(0b0110_0000)]
    fn rejects_unimplemented(#[case] enc_byte: u8) {
        let buf = [enc_byte, 0];
        let err = parse_stream(&buf, DATA, 0, &mut parser()).unwrap_err();
        assert!(matches!(err, MltError::NotImplemented(_)));
    }

    #[rstest]
    #[case::rle(true)]
    #[case::delta_rle(false)]
    fn write_rejects_split_rle(#[case] plain_rle: bool) {
        let rle = RleMeta::Split {
            runs: 2,
            num_rle_values: 5,
        };
        let logical = if plain_rle {
            LogicalEncoding::Rle(rle)
        } else {
            LogicalEncoding::DeltaRle(rle)
        };
        let meta = meta(logical, PhysicalEncoding::VarInt, 5);
        let mut buf = Vec::new();
        let err = write_stream_meta(&meta, &mut buf, 0, 5).unwrap_err();
        assert!(matches!(err, MltError::UnsupportedLogicalEncoding(_, _)));
    }
}
