//! Stream-header wire codec for tag `0x02` (v2) layers.
//!
//! A v2 stream header is a single encoding byte followed by optional varints:
//!
//! ```text
//! [u8 encoding_byte]
//!      bit  7:   has_explicit_count (1 = a count varint follows)
//!      bits 6-4: logical, numbered densely within the stream's family
//!      bits 3-2: physical, interpreted per logical encoding
//!      bits 1-0: logical metadata extension, interpreted per logical encoding or reserved (must be 0)
//! [varint num_values]   only when has_explicit_count = 1; otherwise the count
//!                       comes from context (feature_count, or the presence
//!                       popcount for optional column data)
//! [varint byte_length]  present unless the physical field says none follows
//! ```
//!
//! What the fields mean is per [`Family`], which is fixed by context read before the encoding byte.
//! Each family numbers the encodings it has densely from `0b000`, in the canonical order [`Logical`] fixes.
//!
//! Compared to v1 ([`super::header01`]), the `stream_type` byte is gone (the
//! role is implied by stream position), `num_values` is omitted when derivable
//! from context, and RLE streams carry no `runs` / `num_rle_values` varints:
//! the data is interleaved `(run, value)` pairs and the expanded count comes
//! from the count context.
//!
//! Not yet implemented (rejected with [`MltError::NotImplemented`]):
//! `None-noLen` (requires element-width context), `FastPFor128`, `Morton`, and
//! RLE over a bool or float column.

use std::io;

use integer_encoding::VarIntWriter as _;
use num_enum::TryFromPrimitive;

use crate::codecs::varint::parse_varint;
use crate::decoder::{
    DataType02, DictionaryType, IntEncoding, LengthType, LogicalEncoding, PhysicalEncoding,
    RawStream, RleMeta, StreamMeta, StreamType,
};
use crate::utils::{BinarySerializer as _, parse_u8, take};
use crate::{MltError, MltRefResult, MltResult, Parser};

/// Bit 7 of the encoding byte: an explicit count varint follows.
pub(crate) const HAS_EXPLICIT_COUNT: u8 = 0b1000_0000;

/// Mask of the encoding byte holding the logical field.
pub(crate) const LOGICAL_MASK: u8 = 0b0111_0000;

/// Bit position of the logical field's low bit.
const LOGICAL_SHIFT: u32 = 4;

/// Mask of the encoding byte holding the physical field.
pub(crate) const PHYSICAL_MASK: u8 = 0b0000_1100;

/// Bit position of the physical field's low bit.
const PHYSICAL_SHIFT: u32 = 2;

/// Mask of the encoding byte holding the per-encoding extension field.
pub(crate) const EXTENSION_MASK: u8 = 0b0000_0011;

/// Every logical encoding a v2 encoding byte can name, in the canonical order families number from.
/// New encodings append to it, so no family's existing codes move.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Logical {
    None,
    Delta,
    CwDelta,
    Rle,
    DeltaRle,
    Morton,
}

/// Which logical encodings a stream's context admits, and how each reads the rest of the encoding byte.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum Family {
    #[default]
    Int,
    Bool,
    Float,
    Vertex,
}

impl Family {
    /// The encodings this family has, in [`Logical`]'s canonical order, indexed by their wire code.
    #[expect(
        clippy::match_same_arms,
        reason = "the families are separate tables that happen to agree today; \
                  float gains Alp and Dict, bool does not"
    )]
    const fn members(self) -> &'static [Logical] {
        use Logical as L;
        match self {
            Self::Int => &[L::None, L::Delta, L::Rle, L::DeltaRle],
            Self::Bool => &[L::None, L::Rle],
            Self::Float => &[L::None, L::Rle],
            Self::Vertex => &[L::None, L::Delta, L::CwDelta, L::Morton],
        }
    }

    /// The encoding this family numbers at `code`.
    fn logical(self, code: u8) -> Option<Logical> {
        self.members().get(usize::from(code)).copied()
    }

    /// The code this family numbers `logical` at, or [`None`] if it has no such member.
    fn code(self, logical: Logical) -> Option<u8> {
        let index = self.members().iter().position(|&m| m == logical)?;
        u8::try_from(index).ok()
    }
}

/// What a v2 stream holds.
/// The context that fixes both its [`Family`] and the [`StreamType`] its position implies.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum StreamCtx02 {
    /// A counted column's data stream, typed by the column's data type.
    Property(DataType02),
    GeomTypes,
    GeomVertices,
    GeomOffsets(LengthType),
}

impl StreamCtx02 {
    /// The family this stream's logical field is numbered in.
    pub(crate) fn family(self) -> Family {
        match self {
            Self::Property(DataType02::Bool) => Family::Bool,
            Self::Property(DataType02::F32 | DataType02::F64) => Family::Float,
            Self::Property(_) | Self::GeomTypes | Self::GeomOffsets(_) => Family::Int,
            Self::GeomVertices => Family::Vertex,
        }
    }

    /// The stream role this position implies, which v2 does not store on the wire.
    fn stream_type(self) -> StreamType {
        match self {
            Self::Property(_) => StreamType::Data(DictionaryType::None),
            Self::GeomTypes => StreamType::Length(LengthType::VarBinary),
            Self::GeomVertices => StreamType::Data(DictionaryType::Vertex),
            Self::GeomOffsets(length_type) => StreamType::Length(length_type),
        }
    }
}

/// Physical field of a stream of integer words.
#[derive(Debug, Clone, Copy, PartialEq, Eq, TryFromPrimitive)]
#[repr(u8)]
pub(crate) enum PhysicalInt {
    NoneNoLen = 0b0000_0000,
    NoneWithLen = 0b0000_0100,
    VarInt = 0b0000_1000,
    FastPFor128 = 0b0000_1100,
}

/// Physical field of a stream of opaque fixed-width elements, i.e. a float column's values or a bool column's bitmap.
/// Only whether a byte length follows is open, so the field's two high patterns are unassigned.
#[derive(Debug, Clone, Copy, PartialEq, Eq, TryFromPrimitive)]
#[repr(u8)]
pub(crate) enum PhysicalBits {
    NoLen = 0b0000_0000,
    WithLen = 0b0000_0100,
}

/// Logical encoding of an integer-valued stream, carrying the physical field each of its members admits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LogicalInt {
    None(PhysicalInt),
    Delta(PhysicalInt),
    /// Varint-coded `(run, value)` pairs, so the physical field is reserved.
    Rle,
    /// As [`Self::Rle`], over zigzag deltas.
    DeltaRle,
}

/// Logical encoding of a bool column's data stream.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LogicalBool {
    /// A raw packed bitmap, one bit per value.
    None(PhysicalBits),
    Rle,
}

/// Logical encoding of a float column's data stream.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LogicalFloat {
    /// Fixed-width little-endian values, one per element.
    None(PhysicalBits),
    Rle,
}

/// Logical encoding of a geometry vertex stream.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LogicalVertex {
    None(PhysicalInt),
    Delta(PhysicalInt),
    CwDelta(PhysicalInt),
    Morton,
}

/// One encoding byte's logical and physical fields, read in its family's terms.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Encoding02 {
    Int(LogicalInt),
    Bool(LogicalBool),
    Float(LogicalFloat),
    Vertex(LogicalVertex),
}

/// Read the physical field as an integer codec.
fn physical_int(enc_byte: u8) -> MltResult<PhysicalInt> {
    PhysicalInt::try_from(enc_byte & PHYSICAL_MASK)
        .map_err(|_| MltError::ParsingEncodingByte(enc_byte))
}

/// Read the physical field of a fixed-width element stream.
fn physical_bits(enc_byte: u8) -> MltResult<PhysicalBits> {
    PhysicalBits::try_from(enc_byte & PHYSICAL_MASK)
        .map_err(|_| MltError::ParsingEncodingByte(enc_byte))
}

/// Require the physical field to be zero, for an encoding that implies it.
fn no_physical(enc_byte: u8) -> MltResult<()> {
    if enc_byte & PHYSICAL_MASK == 0 {
        Ok(())
    } else {
        Err(MltError::ParsingEncodingByte(enc_byte))
    }
}

/// Require the extension field to be zero, for an encoding that defines nothing in it.
fn no_extension(enc_byte: u8) -> MltResult<()> {
    if enc_byte & EXTENSION_MASK == 0 {
        Ok(())
    } else {
        Err(MltError::ParsingEncodingByte(enc_byte))
    }
}

impl Encoding02 {
    /// Read the logical, physical and extension fields in `family`'s terms.
    fn parse(family: Family, enc_byte: u8) -> MltResult<Self> {
        let code = (enc_byte & LOGICAL_MASK) >> LOGICAL_SHIFT;
        let logical = family
            .logical(code)
            .ok_or(MltError::ParsingEncodingByte(enc_byte))?;
        no_extension(enc_byte)?;
        Ok(match family {
            Family::Int => Self::Int(match logical {
                Logical::None => LogicalInt::None(physical_int(enc_byte)?),
                Logical::Delta => LogicalInt::Delta(physical_int(enc_byte)?),
                Logical::Rle => {
                    no_physical(enc_byte)?;
                    LogicalInt::Rle
                }
                Logical::DeltaRle => {
                    no_physical(enc_byte)?;
                    LogicalInt::DeltaRle
                }
                Logical::CwDelta | Logical::Morton => unreachable_member(family, logical)?,
            }),
            Family::Bool => Self::Bool(match logical {
                Logical::None => LogicalBool::None(physical_bits(enc_byte)?),
                Logical::Rle => {
                    no_physical(enc_byte)?;
                    LogicalBool::Rle
                }
                _ => unreachable_member(family, logical)?,
            }),
            Family::Float => Self::Float(match logical {
                Logical::None => LogicalFloat::None(physical_bits(enc_byte)?),
                Logical::Rle => {
                    no_physical(enc_byte)?;
                    LogicalFloat::Rle
                }
                _ => unreachable_member(family, logical)?,
            }),
            Family::Vertex => Self::Vertex(match logical {
                Logical::None => LogicalVertex::None(physical_int(enc_byte)?),
                Logical::Delta => LogicalVertex::Delta(physical_int(enc_byte)?),
                Logical::CwDelta => LogicalVertex::CwDelta(physical_int(enc_byte)?),
                Logical::Morton => {
                    no_physical(enc_byte)?;
                    LogicalVertex::Morton
                }
                _ => unreachable_member(family, logical)?,
            }),
        })
    }

    /// The canonical encoding this byte named.
    fn logical(self) -> Logical {
        match self {
            Self::Int(LogicalInt::None(_))
            | Self::Bool(LogicalBool::None(_))
            | Self::Float(LogicalFloat::None(_))
            | Self::Vertex(LogicalVertex::None(_)) => Logical::None,
            Self::Int(LogicalInt::Delta(_)) | Self::Vertex(LogicalVertex::Delta(_)) => {
                Logical::Delta
            }
            Self::Vertex(LogicalVertex::CwDelta(_)) => Logical::CwDelta,
            Self::Int(LogicalInt::Rle)
            | Self::Bool(LogicalBool::Rle)
            | Self::Float(LogicalFloat::Rle) => Logical::Rle,
            Self::Int(LogicalInt::DeltaRle) => Logical::DeltaRle,
            Self::Vertex(LogicalVertex::Morton) => Logical::Morton,
        }
    }

    /// What the physical field said, named for tooling.
    fn physical_label(self) -> String {
        match self {
            Self::Int(LogicalInt::None(p) | LogicalInt::Delta(p))
            | Self::Vertex(
                LogicalVertex::None(p) | LogicalVertex::Delta(p) | LogicalVertex::CwDelta(p),
            ) => format!("{p:?}"),
            Self::Bool(LogicalBool::None(p)) | Self::Float(LogicalFloat::None(p)) => {
                format!("{p:?}")
            }
            Self::Int(LogicalInt::Rle | LogicalInt::DeltaRle)
            | Self::Bool(LogicalBool::Rle)
            | Self::Float(LogicalFloat::Rle)
            | Self::Vertex(LogicalVertex::Morton) => "implied".to_string(),
        }
    }

    /// Map down to the flat encoding the rest of the decoder shares with v1.
    /// `num_values` is the expanded element count, which the RLE members need.
    fn to_flat(self, num_values: u32) -> MltResult<IntEncoding> {
        let rle = || RleMeta::Interleaved {
            num_rle_values: num_values,
        };
        Ok(match self {
            Self::Int(LogicalInt::None(p)) | Self::Vertex(LogicalVertex::None(p)) => {
                IntEncoding::new(LogicalEncoding::None, flat_int(p)?)
            }
            Self::Int(LogicalInt::Delta(p)) | Self::Vertex(LogicalVertex::Delta(p)) => {
                IntEncoding::new(LogicalEncoding::Delta, flat_int(p)?)
            }
            Self::Vertex(LogicalVertex::CwDelta(p)) => {
                IntEncoding::new(LogicalEncoding::ComponentwiseDelta, flat_int(p)?)
            }
            Self::Int(LogicalInt::Rle) => {
                IntEncoding::new(LogicalEncoding::Rle(rle()), PhysicalEncoding::VarInt)
            }
            Self::Int(LogicalInt::DeltaRle) => {
                IntEncoding::new(LogicalEncoding::DeltaRle(rle()), PhysicalEncoding::VarInt)
            }
            Self::Bool(LogicalBool::None(p)) | Self::Float(LogicalFloat::None(p)) => {
                IntEncoding::new(LogicalEncoding::None, flat_bits(p)?)
            }
            Self::Bool(LogicalBool::Rle) => {
                return Err(MltError::NotImplemented("v2 RLE over a bool column"));
            }
            Self::Float(LogicalFloat::Rle) => {
                return Err(MltError::NotImplemented("v2 RLE over a float column"));
            }
            Self::Vertex(LogicalVertex::Morton) => {
                return Err(MltError::NotImplemented("v2 Morton streams"));
            }
        })
    }
}

/// A family only resolves a code to one of its own members, so any other pairing is a bug in [`Family::members`].
fn unreachable_member<T>(family: Family, logical: Logical) -> MltResult<T> {
    unreachable!("{family:?} does not list {logical:?}")
}

/// Map an integer stream's physical field to the flat shared encoding.
fn flat_int(physical: PhysicalInt) -> MltResult<PhysicalEncoding> {
    match physical {
        PhysicalInt::NoneWithLen => Ok(PhysicalEncoding::None),
        PhysicalInt::VarInt => Ok(PhysicalEncoding::VarInt),
        PhysicalInt::NoneNoLen => Err(MltError::NotImplemented("v2 None-noLen physical encoding")),
        PhysicalInt::FastPFor128 => {
            Err(MltError::NotImplemented("v2 FastPFor128 physical encoding"))
        }
    }
}

/// Map a fixed-width element stream's physical field to the flat shared encoding.
fn flat_bits(physical: PhysicalBits) -> MltResult<PhysicalEncoding> {
    match physical {
        PhysicalBits::WithLen => Ok(PhysicalEncoding::None),
        PhysicalBits::NoLen => Err(MltError::NotImplemented("v2 None-noLen physical encoding")),
    }
}

/// The logical encoding and physical field bits `encoding` is written as, the reverse of [`Encoding02::to_flat`].
/// The caller checks the result against the stream's family, which is where an illegal pairing is caught.
fn wire_fields(encoding: IntEncoding, family: Family) -> MltResult<(Logical, u8)> {
    use LogicalEncoding as LE;

    let physical = |encoding: IntEncoding| -> MltResult<u8> {
        Ok(match (family, encoding.physical) {
            (Family::Bool | Family::Float, PhysicalEncoding::None) => PhysicalBits::WithLen as u8,
            (Family::Bool | Family::Float, _) => {
                return Err(MltError::UnsupportedPhysicalEncoding(
                    "v2 bool and float streams store fixed-width elements",
                ));
            }
            (_, PhysicalEncoding::None) => PhysicalInt::NoneWithLen as u8,
            (_, PhysicalEncoding::VarInt) => PhysicalInt::VarInt as u8,
            (_, PhysicalEncoding::FastPFor256) => {
                return Err(MltError::NotImplemented(
                    "v2 FastPFor: requires the FastPFor128-LE codec",
                ));
            }
        })
    };

    Ok(match encoding.logical {
        LE::None => (Logical::None, physical(encoding)?),
        LE::Delta => (Logical::Delta, physical(encoding)?),
        LE::ComponentwiseDelta => (Logical::CwDelta, physical(encoding)?),
        LE::Rle(rle) | LE::DeltaRle(rle) => {
            if !matches!(rle, RleMeta::Interleaved { .. }) {
                return Err(MltError::UnsupportedLogicalEncoding(
                    encoding.logical,
                    "v2 stream header codec requires the Interleaved RLE layout",
                ));
            }
            if encoding.physical != PhysicalEncoding::VarInt {
                return Err(MltError::UnsupportedPhysicalEncoding(
                    "v2 RLE requires VarInt",
                ));
            }
            let logical = if matches!(encoding.logical, LE::Rle(_)) {
                Logical::Rle
            } else {
                Logical::DeltaRle
            };
            // The physical encoding is implied, so the field stays zero.
            (logical, 0)
        }
        LE::Morton(_) | LE::MortonDelta(_) | LE::MortonRle(_) => {
            return Err(MltError::NotImplemented("v2 Morton streams"));
        }
    })
}

/// Parse one v2 stream (header + data), synthesizing [`StreamMeta`] from the
/// wire header plus the positional context in `ctx`.
///
/// - `ctx` fixes both the stream's role, which v2 does not store, and the
///   [`Family`] its logical field is numbered in.
/// - `implicit_count` is the count implied by context: `feature_count`, or the
///   presence popcount for an optional column's data stream.
///
/// Reserves an upper-bound estimate of decoded bytes (`num_values * 8`) on the
/// parser, mirroring the v1 codec.
pub(crate) fn parse_stream<'a>(
    input: &'a [u8],
    ctx: StreamCtx02,
    implicit_count: u32,
    parser: &mut Parser,
) -> MltRefResult<'a, RawStream<'a>> {
    let (input, enc_byte) = parse_u8(input)?;
    let encoding = Encoding02::parse(ctx.family(), enc_byte)?;

    let (input, num_values) = if enc_byte & HAS_EXPLICIT_COUNT == 0 {
        (input, implicit_count)
    } else {
        parse_varint::<u32>(input)?
    };
    // Reserve decoded memory upper bound: worst case u64 = 8 bytes per value.
    parser.reserve(num_values.saturating_mul(8))?;

    let encoding = encoding.to_flat(num_values)?;
    let (input, byte_length) = parse_varint::<u32>(input)?;
    let (input, data) = take(input, byte_length)?;
    let meta = StreamMeta::new(ctx.stream_type(), encoding, num_values);
    Ok((input, RawStream::new(meta, data)))
}

/// Serialize a v2 stream header for `meta`, numbering its logical field in `family`.
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
    family: Family,
) -> MltResult<()> {
    use LogicalEncoding as LE;

    let (logical, physical_bits) = wire_fields(meta.encoding, family)?;
    let code = family.code(logical).ok_or_else(|| {
        MltError::UnsupportedLogicalEncoding(meta.encoding.logical, family_name(family))
    })?;

    // For RLE streams the wire count is the *decoded* count (the encoder's
    // in-memory `num_values` holds the encoded word count, which a v2 decoder
    // derives by scanning the pairs to `byte_length`).
    let num_values = match meta.encoding.logical {
        LE::Rle(rle) | LE::DeltaRle(rle) => rle.num_rle_values(),
        LE::None
        | LE::Delta
        | LE::ComponentwiseDelta
        | LE::Morton(_)
        | LE::MortonDelta(_)
        | LE::MortonRle(_) => meta.num_values,
    };
    let explicit = num_values != implicit_count;
    let enc_byte =
        if explicit { HAS_EXPLICIT_COUNT } else { 0 } | (code << LOGICAL_SHIFT) | physical_bits;
    writer.write_u8(enc_byte)?;
    if explicit {
        writer.write_varint(num_values)?;
    }
    writer.write_varint(byte_length)?;
    Ok(())
}

/// How a family names itself in an error about an encoding it does not list.
fn family_name(family: Family) -> &'static str {
    match family {
        Family::Int => "a v2 integer stream",
        Family::Bool => "a v2 bool column",
        Family::Float => "a v2 float column",
        Family::Vertex => "a v2 vertex stream",
    }
}

/// Name an encoding byte's logical and physical fields in `family`'s terms, for tooling that annotates a byte.
/// The extension field is masked off, since whether those bits are set says nothing about what the other two name.
pub(crate) fn describe_encoding(family: Family, byte: u8) -> (String, String) {
    let code = (byte & LOGICAL_MASK) >> LOGICAL_SHIFT;
    let physical = (byte & PHYSICAL_MASK) >> PHYSICAL_SHIFT;
    match Encoding02::parse(family, byte & !EXTENSION_MASK) {
        Ok(encoding) => (
            format!("{:?}", encoding.logical()),
            encoding.physical_label(),
        ),
        Err(_) => (
            family
                .logical(code)
                .map_or_else(|| format!("unassigned({code})"), |l| format!("{l:?}")),
            format!("invalid({physical})"),
        ),
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::test_helpers::parser;

    const INT: StreamCtx02 = StreamCtx02::Property(DataType02::U32);
    const BOOL: StreamCtx02 = StreamCtx02::Property(DataType02::Bool);
    const FLOAT: StreamCtx02 = StreamCtx02::Property(DataType02::F64);
    const VERTEX: StreamCtx02 = StreamCtx02::GeomVertices;

    /// Every logical encoding v2 can name, in the canonical order of record 0006.
    const CANONICAL: [Logical; 6] = [
        Logical::None,
        Logical::Delta,
        Logical::CwDelta,
        Logical::Rle,
        Logical::DeltaRle,
        Logical::Morton,
    ];

    const ALL_FAMILIES: [Family; 4] = [Family::Int, Family::Bool, Family::Float, Family::Vertex];

    fn meta(logical: LogicalEncoding, physical: PhysicalEncoding, num: u32) -> StreamMeta {
        let stream_type = StreamType::Data(DictionaryType::None);
        StreamMeta::new(stream_type, IntEncoding::new(logical, physical), num)
    }

    fn rle(num: u32) -> RleMeta {
        RleMeta::Interleaved {
            num_rle_values: num,
        }
    }

    #[rstest]
    #[case::int_none(Family::Int, Logical::None, 0b000)]
    #[case::int_delta(Family::Int, Logical::Delta, 0b001)]
    #[case::int_rle(Family::Int, Logical::Rle, 0b010)]
    #[case::int_delta_rle(Family::Int, Logical::DeltaRle, 0b011)]
    #[case::bool_none(Family::Bool, Logical::None, 0b000)]
    #[case::bool_rle(Family::Bool, Logical::Rle, 0b001)]
    #[case::float_none(Family::Float, Logical::None, 0b000)]
    #[case::float_rle(Family::Float, Logical::Rle, 0b001)]
    #[case::vertex_none(Family::Vertex, Logical::None, 0b000)]
    #[case::vertex_delta(Family::Vertex, Logical::Delta, 0b001)]
    #[case::vertex_cw_delta(Family::Vertex, Logical::CwDelta, 0b010)]
    #[case::vertex_morton(Family::Vertex, Logical::Morton, 0b011)]
    fn wire_code_of_each_family_member(
        #[case] family: Family,
        #[case] logical: Logical,
        #[case] code: u8,
    ) {
        assert_eq!(family.code(logical), Some(code));
        assert_eq!(family.logical(code), Some(logical));
    }

    #[test]
    fn every_family_lists_its_members_in_canonical_order() {
        for family in ALL_FAMILIES {
            let ranks: Vec<usize> = family
                .members()
                .iter()
                .map(|m| {
                    CANONICAL
                        .iter()
                        .position(|c| c == m)
                        .expect("member is in the canonical order")
                })
                .collect();
            assert!(
                ranks.windows(2).all(|w| w[0] < w[1]),
                "{family:?} lists {:?}",
                family.members()
            );
        }
    }

    #[rstest]
    #[case::id(DataType02::Id, Family::Int)]
    #[case::long_id(DataType02::LongId, Family::Int)]
    #[case::i8(DataType02::I8, Family::Int)]
    #[case::u64(DataType02::U64, Family::Int)]
    #[case::bool(DataType02::Bool, Family::Bool)]
    #[case::f32(DataType02::F32, Family::Float)]
    #[case::f64(DataType02::F64, Family::Float)]
    fn column_data_type_picks_its_family(#[case] data: DataType02, #[case] family: Family) {
        assert_eq!(StreamCtx02::Property(data).family(), family);
    }

    #[rstest]
    #[case::types(StreamCtx02::GeomTypes, Family::Int)]
    #[case::lengths(StreamCtx02::GeomOffsets(LengthType::Parts), Family::Int)]
    #[case::vertices(StreamCtx02::GeomVertices, Family::Vertex)]
    fn geometry_role_picks_its_family(#[case] ctx: StreamCtx02, #[case] family: Family) {
        assert_eq!(ctx.family(), family);
    }

    #[rstest]
    #[case::varint_implicit(
        meta(LogicalEncoding::None, PhysicalEncoding::VarInt, 5),
        5,
        Family::Int,
        0b0000_1000
    )]
    #[case::varint_explicit(
        meta(LogicalEncoding::None, PhysicalEncoding::VarInt, 5),
        9,
        Family::Int,
        0b1000_1000
    )]
    #[case::raw_implicit(
        meta(LogicalEncoding::None, PhysicalEncoding::None, 5),
        5,
        Family::Int,
        0b0000_0100
    )]
    #[case::delta_varint(
        meta(LogicalEncoding::Delta, PhysicalEncoding::VarInt, 5),
        5,
        Family::Int,
        0b0001_1000
    )]
    #[case::rle_implicit(
        meta(LogicalEncoding::Rle(rle(5)), PhysicalEncoding::VarInt, 5),
        5,
        Family::Int,
        0b0010_0000
    )]
    #[case::delta_rle(
        meta(LogicalEncoding::DeltaRle(rle(5)), PhysicalEncoding::VarInt, 5),
        5,
        Family::Int,
        0b0011_0000
    )]
    #[case::raw_float(
        meta(LogicalEncoding::None, PhysicalEncoding::None, 5),
        5,
        Family::Float,
        0b0000_0100
    )]
    #[case::raw_bool(
        meta(LogicalEncoding::None, PhysicalEncoding::None, 5),
        5,
        Family::Bool,
        0b0000_0100
    )]
    #[case::cw_delta_vertices_explicit(
        meta(LogicalEncoding::ComponentwiseDelta, PhysicalEncoding::VarInt, 8),
        5,
        Family::Vertex,
        0b1010_1000
    )]
    fn encoding_byte_value(
        #[case] meta: StreamMeta,
        #[case] implicit_count: u32,
        #[case] family: Family,
        #[case] expected: u8,
    ) {
        let mut buf = Vec::new();
        write_stream_meta(&meta, &mut buf, 0, implicit_count, family).unwrap();
        assert_eq!(buf[0], expected);
    }

    #[rstest]
    #[case::varint(meta(LogicalEncoding::None, PhysicalEncoding::VarInt, 5), 5, INT)]
    #[case::varint_explicit(meta(LogicalEncoding::None, PhysicalEncoding::VarInt, 7), 5, INT)]
    #[case::raw(meta(LogicalEncoding::None, PhysicalEncoding::None, 5), 5, INT)]
    #[case::delta(meta(LogicalEncoding::Delta, PhysicalEncoding::VarInt, 5), 5, INT)]
    #[case::rle(
        meta(LogicalEncoding::Rle(rle(5)), PhysicalEncoding::VarInt, 5),
        5,
        INT
    )]
    #[case::delta_rle(
        meta(LogicalEncoding::DeltaRle(rle(9)), PhysicalEncoding::VarInt, 9),
        5,
        INT
    )]
    #[case::raw_float(meta(LogicalEncoding::None, PhysicalEncoding::None, 5), 5, FLOAT)]
    #[case::raw_bool(meta(LogicalEncoding::None, PhysicalEncoding::None, 5), 5, BOOL)]
    #[case::cw_delta_vertices(
        meta(LogicalEncoding::ComponentwiseDelta, PhysicalEncoding::VarInt, 10),
        5,
        VERTEX
    )]
    fn header_roundtrip(
        #[case] meta: StreamMeta,
        #[case] implicit_count: u32,
        #[case] ctx: StreamCtx02,
    ) {
        let payload = [1_u8, 2, 3];
        let mut buf = Vec::new();
        let byte_length = u32::try_from(payload.len()).unwrap();
        write_stream_meta(&meta, &mut buf, byte_length, implicit_count, ctx.family()).unwrap();
        buf.extend_from_slice(&payload);

        let (rest, stream) = parse_stream(&buf, ctx, implicit_count, &mut parser()).unwrap();
        assert!(rest.is_empty());
        assert_eq!(stream.meta.encoding, meta.encoding);
        assert_eq!(stream.meta.num_values, meta.num_values);
        assert_eq!(stream.meta.stream_type, ctx.stream_type());
        assert_eq!(stream.data, payload);
    }

    #[rstest]
    #[case::extension_bit0(INT, 0b0000_1001)]
    #[case::extension_bit1(INT, 0b0000_1010)]
    #[case::extension_on_float(FLOAT, 0b0000_0101)]
    #[case::extension_on_rle(INT, 0b0010_0001)]
    #[case::rle_with_physical(INT, 0b0010_0100)]
    #[case::delta_rle_with_physical(INT, 0b0011_1000)]
    #[case::morton_with_physical(VERTEX, 0b0011_1000)]
    #[case::int_logical_past_table(INT, 0b0100_1000)]
    #[case::bool_logical_past_table(BOOL, 0b0010_0100)]
    #[case::float_logical_past_table(FLOAT, 0b0011_0100)]
    #[case::vertex_logical_past_table(VERTEX, 0b0100_1000)]
    #[case::float_physical_varint(FLOAT, 0b0000_1000)]
    #[case::float_physical_fastpfor(FLOAT, 0b0000_1100)]
    #[case::bool_physical_varint(BOOL, 0b0000_1000)]
    fn parse_rejects_malformed_encoding_byte(#[case] ctx: StreamCtx02, #[case] enc_byte: u8) {
        let buf = [enc_byte, 0];
        let err = parse_stream(&buf, ctx, 0, &mut parser()).unwrap_err();
        assert!(
            matches!(err, MltError::ParsingEncodingByte(b) if b == enc_byte),
            "{err:?}"
        );
    }

    #[rstest]
    #[case::none_no_len(INT, 0b0000_0000)]
    #[case::fastpfor128(INT, 0b0000_1100)]
    #[case::float_none_no_len(FLOAT, 0b0000_0000)]
    #[case::float_rle(FLOAT, 0b0001_0000)]
    #[case::bool_rle(BOOL, 0b0001_0000)]
    #[case::vertex_morton(VERTEX, 0b0011_0000)]
    fn parse_rejects_unimplemented_encoding(#[case] ctx: StreamCtx02, #[case] enc_byte: u8) {
        let buf = [enc_byte, 0];
        let err = parse_stream(&buf, ctx, 0, &mut parser()).unwrap_err();
        assert!(matches!(err, MltError::NotImplemented(_)), "{err:?}");
    }

    #[rstest]
    #[case::delta(0b0001_1000, LogicalEncoding::Delta, "encoding byte")]
    #[case::rle(0b0010_0000, LogicalEncoding::Rle(rle(1)), "encoding byte")]
    #[case::delta_rle(0b0011_0000, LogicalEncoding::DeltaRle(rle(1)), "encoding byte")]
    fn an_int_bit_pattern_never_means_the_same_on_a_float_column(
        #[case] enc_byte: u8,
        #[case] as_int: LogicalEncoding,
        #[case] float_error: &str,
    ) {
        let buf = [enc_byte, 0];
        let (_, stream) = parse_stream(&buf, INT, 1, &mut parser()).unwrap();
        assert_eq!(stream.meta.encoding.logical, as_int);

        let err = parse_stream(&buf, FLOAT, 1, &mut parser()).unwrap_err();
        assert!(err.to_string().contains(float_error), "{err}");
    }

    #[test]
    fn logical_code_one_is_delta_for_ints_and_rle_for_floats() {
        let buf = [0b0001_0000, 0];
        let as_int = parse_stream(&buf, INT, 1, &mut parser()).unwrap_err();
        let as_float = parse_stream(&buf, FLOAT, 1, &mut parser()).unwrap_err();
        assert!(as_int.to_string().contains("None-noLen"), "{as_int}");
        assert!(
            as_float.to_string().contains("RLE over a float"),
            "{as_float}"
        );
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
        let err = write_stream_meta(&meta, &mut buf, 0, 5, Family::Int).unwrap_err();
        assert!(matches!(err, MltError::UnsupportedLogicalEncoding(_, _)));
    }

    #[rstest]
    #[case::delta_on_a_float_column(LogicalEncoding::Delta, Family::Float)]
    #[case::delta_on_a_bool_column(LogicalEncoding::Delta, Family::Bool)]
    #[case::cw_delta_on_a_property_column(LogicalEncoding::ComponentwiseDelta, Family::Int)]
    fn write_rejects_a_logical_the_family_does_not_list(
        #[case] logical: LogicalEncoding,
        #[case] family: Family,
    ) {
        let meta = meta(logical, PhysicalEncoding::None, 5);
        let mut buf = Vec::new();
        let err = write_stream_meta(&meta, &mut buf, 0, 5, family).unwrap_err();
        assert!(
            matches!(err, MltError::UnsupportedLogicalEncoding(_, _)),
            "{err:?}"
        );
    }
}
