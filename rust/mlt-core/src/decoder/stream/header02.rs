//! Stream-header wire codec for tag `0x02` (v2) layers.
//!
//! A v2 stream header is a single encoding byte followed by optional varints:
//!
//! ```text
//! [u8 encoding_byte]
//!      bit  7:   has_explicit_count (1 = a count varint follows)
//!      bits 6-4: logical, numbered densely within the stream's family
//!      bits 3-2: physical, interpreted per logical encoding
//!      bits 1-0: logical metadata extension, holding a string column's StrLayout,
//!                otherwise interpreted per logical encoding or reserved (must be 0)
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
//! `None-noLen` (requires element-width context), `FastPFor128`, `Morton`,
//! and RLE over a bool or float column.

use std::io;

use integer_encoding::VarIntWriter as _;
use num_enum::TryFromPrimitive;

use crate::codecs::varint::parse_varint;
use crate::decoder::{
    Alp, BoolLogical, DataType02, DictionaryType, FloatLogical, IntEncoding, IntLogical,
    LengthType, LogicalEncoding, OffsetType, PhysicalEncoding, RawStream, RleMeta, StreamMeta,
    StreamType, VertexLogical,
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, strum::Display, strum::EnumCount, strum::EnumIter)]
pub(crate) enum Logical {
    None,
    Delta,
    CwDelta,
    Rle,
    DeltaRle,
    Morton,
    Alp,
    Dict,
}

/// How a string column lays its streams out, named by the extension bits of its leading stream.
/// The two bits have exactly four patterns, so every one of them is a layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub(crate) enum StrLayout {
    /// `lengths` then the values' bytes.
    #[default]
    Plain = 0b00,
    /// `codes` then the distinct values' lengths and bytes.
    Dict = 0b01,
    /// `lengths` then the FSST symbol table and the compressed corpus.
    Fsst = 0b10,
    /// `codes` then the distinct values' lengths, the FSST symbol table and the compressed corpus.
    FsstDict = 0b11,
}

impl StrLayout {
    /// The layout an encoding byte's extension field names.
    pub(crate) fn from_bits(enc_byte: u8) -> Self {
        match enc_byte & EXTENSION_MASK {
            0b00 => Self::Plain,
            0b01 => Self::Dict,
            0b10 => Self::Fsst,
            _ => Self::FsstDict,
        }
    }
}

/// Which logical encodings a stream's context admits, and how each reads the rest of the encoding byte.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, strum::EnumIter, strum::IntoStaticStr)]
pub(crate) enum Family {
    #[default]
    #[strum(serialize = "a v2 integer stream")]
    Int,
    #[strum(serialize = "a v2 bool column")]
    Bool,
    #[strum(serialize = "a v2 float column")]
    Float,
    #[strum(serialize = "a v2 vertex stream")]
    Vertex,
    /// A string column's leading stream: an integer stream whose extension bits carry [`StrLayout`].
    #[strum(serialize = "a v2 string column")]
    Str(StrLayout),
    /// An opaque byte blob, whose value count is its byte length.
    #[strum(serialize = "a v2 byte blob")]
    Bytes,
}

impl Family {
    /// The encodings this family has, in [`Logical`]'s canonical order, indexed by their wire code.
    const fn members(self) -> &'static [Logical] {
        use Logical as L;
        match self {
            Self::Int | Self::Str(_) => &[L::None, L::Delta, L::Rle, L::DeltaRle],
            Self::Bool => &[L::None, L::Rle],
            Self::Float => &[L::None, L::Rle, L::Alp, L::Dict],
            Self::Vertex => &[L::None, L::Delta, L::CwDelta, L::Morton],
            Self::Bytes => &[L::None],
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
    /// A scalar column's data stream, typed by the column's data type.
    /// A string column's streams have their own contexts, since their roles follow its [`StrLayout`].
    Property(DataType02),
    /// The dictionary a column's codes index into, following its codes stream, of the column's own type.
    PropertyDictionary(DataType02),
    /// A string column's leading stream, of one entry per present value.
    StrData(StrLayout),
    /// The lengths of the distinct values a string column stores once.
    StrDictLengths,
    /// The lengths of a string column's FSST symbols.
    StrSymbolLengths,
    /// One of a string column's byte blobs, named by the dictionary role it fills.
    StrBlob(DictionaryType),
    GeomTypes,
    GeomVertices,
    GeomOffsets(LengthType),
}

impl StreamCtx02 {
    /// The family this stream's logical field is numbered in.
    pub(crate) fn family(self) -> Family {
        match self {
            Self::Property(DataType02::Bool) | Self::PropertyDictionary(DataType02::Bool) => {
                Family::Bool
            }
            Self::Property(DataType02::F32 | DataType02::F64)
            | Self::PropertyDictionary(DataType02::F32 | DataType02::F64) => Family::Float,
            Self::StrData(layout) => Family::Str(layout),
            Self::StrBlob(_) => Family::Bytes,
            Self::Property(_)
            | Self::PropertyDictionary(_)
            | Self::StrDictLengths
            | Self::StrSymbolLengths
            | Self::GeomTypes
            | Self::GeomOffsets(_) => Family::Int,
            Self::GeomVertices => Family::Vertex,
        }
    }

    /// The stream role this position implies, which v2 does not store on the wire.
    pub(crate) fn stream_type(self) -> StreamType {
        match self {
            Self::Property(_) => StreamType::Data(DictionaryType::None),
            Self::PropertyDictionary(_) => StreamType::Data(DictionaryType::Single),
            Self::StrData(StrLayout::Plain) | Self::GeomTypes => {
                StreamType::Length(LengthType::VarBinary)
            }
            Self::StrData(StrLayout::Fsst) | Self::StrDictLengths => {
                StreamType::Length(LengthType::Dictionary)
            }
            Self::StrData(StrLayout::Dict | StrLayout::FsstDict) => {
                StreamType::Offset(OffsetType::String)
            }
            Self::StrSymbolLengths => StreamType::Length(LengthType::Symbol),
            Self::StrBlob(dictionary) => StreamType::Data(dictionary),
            Self::GeomVertices => StreamType::Data(DictionaryType::Vertex),
            Self::GeomOffsets(length_type) => StreamType::Length(length_type),
        }
    }
}

/// Physical field of a stream of integer words.
#[derive(Debug, Clone, Copy, PartialEq, Eq, TryFromPrimitive, strum::IntoStaticStr)]
#[repr(u8)]
pub(crate) enum PhysicalInt {
    NoneNoLen = 0b0000_0000,
    NoneWithLen = 0b0000_0100,
    VarInt = 0b0000_1000,
    FastPFor128 = 0b0000_1100,
}

/// Physical field of a stream of opaque fixed-width elements, i.e. a float column's values or a bool column's bitmap.
/// Only whether a byte length follows is open, so the field's two high patterns are unassigned.
#[derive(Debug, Clone, Copy, PartialEq, Eq, TryFromPrimitive, strum::IntoStaticStr)]
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
    /// Numbered so the family matches the format, but not yet readable.
    // TODO(v2): decide what RLE over a bool column means - v1's is a byte-RLE compressed
    //           bitmap, while every other v2 RLE is varint `(run, value)` pairs.
    Rle,
}

/// Logical encoding of a float column's data stream.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LogicalFloat {
    /// Fixed-width little-endian values, one per element.
    None(PhysicalBits),
    Rle,
    /// Adaptive lossless floating-point compression.
    /// The physical field codes the scaled integers, not the column's element layout.
    // TODO(v2): extension bit 1 = an exception count varint and exception stream follow.
    Alp(PhysicalInt),
    /// One code per element, then the dictionary of distinct values as a second stream.
    /// The physical field codes the codes, not the values.
    Dict(PhysicalInt),
}

/// Logical encoding of an opaque byte blob.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LogicalBytes {
    /// The bytes as they are, their count being the stream's byte length.
    None(PhysicalBits),
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
    /// A string column's leading stream, coded as an integer one, plus the layout its extension bits name.
    Str(LogicalInt, StrLayout),
    Bytes(LogicalBytes),
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

/// Read an integer stream's logical field, in the terms the integer and string families share.
fn logical_int(family: Family, enc_byte: u8, logical: Logical) -> MltResult<LogicalInt> {
    Ok(match logical {
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
        Logical::CwDelta | Logical::Morton | Logical::Alp | Logical::Dict => {
            unreachable_member(family, logical)?
        }
    })
}

impl Encoding02 {
    /// Read the logical, physical and extension fields in `family`'s terms.
    fn parse(family: Family, enc_byte: u8) -> MltResult<Self> {
        let code = (enc_byte & LOGICAL_MASK) >> LOGICAL_SHIFT;
        let logical = family
            .logical(code)
            .ok_or(MltError::ParsingEncodingByte(enc_byte))?;
        // Only a string column's leading stream assigns anything to the extension field.
        if !matches!(family, Family::Str(_)) {
            no_extension(enc_byte)?;
        }
        Ok(match family {
            Family::Int => Self::Int(logical_int(family, enc_byte, logical)?),
            Family::Str(layout) => {
                // The caller resolves the layout from this very byte, so a mismatch means it read another.
                if StrLayout::from_bits(enc_byte) != layout {
                    return Err(MltError::ParsingEncodingByte(enc_byte));
                }
                Self::Str(logical_int(family, enc_byte, logical)?, layout)
            }
            Family::Bytes => Self::Bytes(match logical {
                Logical::None => LogicalBytes::None(physical_bits(enc_byte)?),
                Logical::Delta
                | Logical::CwDelta
                | Logical::Rle
                | Logical::DeltaRle
                | Logical::Morton
                | Logical::Alp
                | Logical::Dict => unreachable_member(family, logical)?,
            }),
            Family::Bool => Self::Bool(match logical {
                Logical::None => LogicalBool::None(physical_bits(enc_byte)?),
                Logical::Rle => {
                    no_physical(enc_byte)?;
                    LogicalBool::Rle
                }
                Logical::Delta
                | Logical::CwDelta
                | Logical::DeltaRle
                | Logical::Morton
                | Logical::Alp
                | Logical::Dict => unreachable_member(family, logical)?,
            }),
            Family::Float => Self::Float(match logical {
                Logical::None => LogicalFloat::None(physical_bits(enc_byte)?),
                Logical::Rle => {
                    no_physical(enc_byte)?;
                    LogicalFloat::Rle
                }
                Logical::Alp => LogicalFloat::Alp(physical_int(enc_byte)?),
                Logical::Dict => LogicalFloat::Dict(physical_int(enc_byte)?),
                Logical::Delta | Logical::CwDelta | Logical::DeltaRle | Logical::Morton => {
                    unreachable_member(family, logical)?
                }
            }),
            Family::Vertex => Self::Vertex(match logical {
                Logical::None => LogicalVertex::None(physical_int(enc_byte)?),
                Logical::Delta => LogicalVertex::Delta(physical_int(enc_byte)?),
                Logical::CwDelta => LogicalVertex::CwDelta(physical_int(enc_byte)?),
                Logical::Morton => {
                    no_physical(enc_byte)?;
                    LogicalVertex::Morton
                }
                Logical::Rle | Logical::DeltaRle | Logical::Alp | Logical::Dict => {
                    unreachable_member(family, logical)?
                }
            }),
        })
    }

    /// The canonical encoding this byte named.
    fn logical(self) -> Logical {
        match self {
            Self::Str(logical, _) => Self::Int(logical).logical(),
            Self::Bytes(LogicalBytes::None(_))
            | Self::Int(LogicalInt::None(_))
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
            Self::Float(LogicalFloat::Alp(_)) => Logical::Alp,
            Self::Float(LogicalFloat::Dict(_)) => Logical::Dict,
        }
    }

    /// What the physical field said, named for tooling.
    fn physical_label(self) -> &'static str {
        match self {
            Self::Str(logical, _) => Self::Int(logical).physical_label(),
            Self::Int(LogicalInt::None(p) | LogicalInt::Delta(p))
            | Self::Float(LogicalFloat::Dict(p) | LogicalFloat::Alp(p))
            | Self::Vertex(
                LogicalVertex::None(p) | LogicalVertex::Delta(p) | LogicalVertex::CwDelta(p),
            ) => p.into(),
            Self::Bool(LogicalBool::None(p))
            | Self::Float(LogicalFloat::None(p))
            | Self::Bytes(LogicalBytes::None(p)) => p.into(),
            Self::Int(LogicalInt::Rle | LogicalInt::DeltaRle)
            | Self::Bool(LogicalBool::Rle)
            | Self::Float(LogicalFloat::Rle)
            | Self::Vertex(LogicalVertex::Morton) => "implied",
        }
    }

    /// Map to the shared model, reading any parameter varints the encoding carries.
    ///
    /// `num_values` is the expanded element count, which the RLE members need.
    /// Parameters follow the byte length, as v1 writes Morton's and RLE's.
    fn to_model(self, input: &[u8], num_values: u32) -> MltRefResult<'_, IntEncoding> {
        let rle = || RleMeta::Interleaved {
            num_rle_values: num_values,
        };
        let mut rest = input;
        let encoding = match self {
            // A string column's leading stream is an integer one; only its extension bits differ.
            Self::Str(logical, _) => return Self::Int(logical).to_model(input, num_values),
            Self::Bytes(LogicalBytes::None(p)) => {
                IntEncoding::new(LogicalEncoding::Int(IntLogical::None), flat_bits(p)?)
            }
            Self::Int(LogicalInt::None(p)) => {
                IntEncoding::new(LogicalEncoding::Int(IntLogical::None), flat_int(p)?)
            }
            Self::Vertex(LogicalVertex::None(p)) => {
                IntEncoding::new(LogicalEncoding::Vertex(VertexLogical::None), flat_int(p)?)
            }
            Self::Int(LogicalInt::Delta(p)) => {
                IntEncoding::new(LogicalEncoding::Int(IntLogical::Delta), flat_int(p)?)
            }
            Self::Vertex(LogicalVertex::Delta(p)) => {
                IntEncoding::new(LogicalEncoding::Vertex(VertexLogical::Delta), flat_int(p)?)
            }
            Self::Vertex(LogicalVertex::CwDelta(p)) => IntEncoding::new(
                LogicalEncoding::Vertex(VertexLogical::ComponentwiseDelta),
                flat_int(p)?,
            ),
            Self::Int(LogicalInt::Rle) => IntEncoding::new(
                LogicalEncoding::Int(IntLogical::Rle(rle())),
                PhysicalEncoding::VarInt,
            ),
            Self::Int(LogicalInt::DeltaRle) => IntEncoding::new(
                LogicalEncoding::Int(IntLogical::DeltaRle(rle())),
                PhysicalEncoding::VarInt,
            ),
            Self::Bool(LogicalBool::None(p)) => {
                IntEncoding::new(LogicalEncoding::Bool(BoolLogical::None), flat_bits(p)?)
            }
            Self::Float(LogicalFloat::None(p)) => {
                IntEncoding::new(LogicalEncoding::Float(FloatLogical::None), flat_bits(p)?)
            }
            Self::Bool(LogicalBool::Rle) => {
                return Err(MltError::NotImplemented("v2 RLE over a bool column"));
            }
            Self::Float(LogicalFloat::Rle) => {
                return Err(MltError::NotImplemented("v2 RLE over a float column"));
            }
            Self::Float(LogicalFloat::Alp(p)) => {
                let (after, e) = parse_varint::<u8>(input)?;
                let (after, f) = parse_varint::<u8>(after)?;
                rest = after;
                IntEncoding::new(
                    LogicalEncoding::Float(FloatLogical::Alp(Alp::new(e, f)?)),
                    flat_int(p)?,
                )
            }
            Self::Float(LogicalFloat::Dict(p)) => {
                IntEncoding::new(LogicalEncoding::Float(FloatLogical::Dict), flat_int(p)?)
            }
            Self::Vertex(LogicalVertex::Morton) => {
                return Err(MltError::NotImplemented("v2 Morton streams"));
            }
        };
        Ok((rest, encoding))
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

/// The physical field bits for a stream of integers, whatever the column's own type is.
fn physical_int_field(physical: PhysicalEncoding) -> MltResult<u8> {
    Ok(match physical {
        PhysicalEncoding::None => PhysicalInt::NoneWithLen as u8,
        PhysicalEncoding::VarInt => PhysicalInt::VarInt as u8,
        PhysicalEncoding::FastPFor256 => {
            return Err(MltError::NotImplemented(
                "v2 FastPFor: requires the FastPFor128-LE codec",
            ));
        }
    })
}

/// The logical encoding and physical field bits `encoding` is written as, the reverse of [`Encoding02::to_model`].
/// The caller checks the result against the stream's family, which is where an illegal pairing is caught.
fn wire_fields(encoding: IntEncoding, family: Family) -> MltResult<(Logical, u8)> {
    use BoolLogical as BL;
    use FloatLogical as FL;
    use IntLogical as IL;
    use LogicalEncoding as LE;
    use VertexLogical as VL;

    let physical = |encoding: IntEncoding| -> MltResult<u8> {
        match (family, encoding.physical) {
            (Family::Bool | Family::Float | Family::Bytes, PhysicalEncoding::None) => {
                Ok(PhysicalBits::WithLen as u8)
            }
            (Family::Bool | Family::Float | Family::Bytes, _) => {
                Err(MltError::UnsupportedPhysicalEncoding(
                    "v2 bool, float and blob streams store their elements as they are",
                ))
            }
            _ => physical_int_field(encoding.physical),
        }
    };

    Ok(match encoding.logical {
        LE::Int(IL::None) | LE::Bool(BL::None) | LE::Float(FL::None) | LE::Vertex(VL::None) => {
            (Logical::None, physical(encoding)?)
        }
        LE::Int(IL::Delta) | LE::Vertex(VL::Delta) => (Logical::Delta, physical(encoding)?),
        LE::Vertex(VL::ComponentwiseDelta) => (Logical::CwDelta, physical(encoding)?),
        LE::Int(IL::Rle(rle) | IL::DeltaRle(rle)) => {
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
            let logical = if matches!(encoding.logical, LE::Int(IL::Rle(_))) {
                Logical::Rle
            } else {
                Logical::DeltaRle
            };
            // The physical encoding is implied, so the field stays zero.
            (logical, 0)
        }
        // Codes and scaled integers are integer streams, whatever the column's type is.
        LE::Float(FL::Dict) => (Logical::Dict, physical_int_field(encoding.physical)?),
        LE::Float(FL::Alp(_)) => (Logical::Alp, physical_int_field(encoding.physical)?),
        LE::Bool(BL::ByteRle(_)) => {
            return Err(MltError::UnsupportedLogicalEncoding(
                encoding.logical,
                "v2, whose bool columns have no byte-RLE",
            ));
        }
        LE::Vertex(VL::Morton(_) | VL::MortonDelta(_) | VL::MortonRle(_)) => {
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
    let family = ctx.family();
    let encoding = Encoding02::parse(family, enc_byte)?;

    let explicit = enc_byte & HAS_EXPLICIT_COUNT != 0;
    // A blob's count is its byte length, so it has nothing for a count varint to say.
    if explicit && family == Family::Bytes {
        return Err(MltError::ParsingEncodingByte(enc_byte));
    }
    let (input, num_values) = if explicit {
        parse_varint::<u32>(input)?
    } else {
        (input, implicit_count)
    };

    let (input, byte_length) = parse_varint::<u32>(input)?;
    let num_values = if family == Family::Bytes {
        byte_length
    } else {
        num_values
    };
    // Reserve decoded memory upper bound: a blob decodes to its own bytes, any other stream to a u64 per value.
    parser.reserve(if family == Family::Bytes {
        byte_length
    } else {
        num_values.saturating_mul(8)
    })?;
    let (input, encoding) = encoding.to_model(input, num_values)?;
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
        MltError::UnsupportedLogicalEncoding(meta.encoding.logical, family.into())
    })?;

    // For RLE streams the wire count is the *decoded* count (the encoder's
    // in-memory `num_values` holds the encoded word count, which a v2 decoder
    // derives by scanning the pairs to `byte_length`).
    let num_values = match meta.encoding.logical {
        LE::Int(IntLogical::Rle(rle) | IntLogical::DeltaRle(rle)) => rle.num_rle_values(),
        LE::Int(IntLogical::None | IntLogical::Delta)
        | LE::Bool(_)
        | LE::Float(_)
        | LE::Vertex(_) => meta.num_values,
    };
    // A blob's count is its byte length, which its length varint already carries.
    let explicit = family != Family::Bytes && num_values != implicit_count;
    let extension = match family {
        Family::Str(layout) => layout as u8,
        Family::Int | Family::Bool | Family::Float | Family::Vertex | Family::Bytes => 0,
    };
    let enc_byte = if explicit { HAS_EXPLICIT_COUNT } else { 0 }
        | (code << LOGICAL_SHIFT)
        | physical_bits
        | extension;
    writer.write_u8(enc_byte)?;
    if explicit {
        writer.write_varint(num_values)?;
    }
    writer.write_varint(byte_length)?;
    if let LE::Float(FloatLogical::Alp(alp)) = meta.encoding.logical {
        writer.write_varint(alp.e)?;
        writer.write_varint(alp.f)?;
    }
    Ok(())
}

/// Name an encoding byte's logical and physical fields in `family`'s terms, for tooling that annotates a byte.
/// The extension field is masked off, since whether those bits are set says nothing about what the other two name.
pub(crate) fn describe_encoding(family: Family, byte: u8) -> (String, String) {
    let code = (byte & LOGICAL_MASK) >> LOGICAL_SHIFT;
    let physical = (byte & PHYSICAL_MASK) >> PHYSICAL_SHIFT;
    // A string column's leading stream is the one family that reads those bits, so it keeps them.
    let masked = if matches!(family, Family::Str(_)) {
        byte
    } else {
        byte & !EXTENSION_MASK
    };
    match Encoding02::parse(family, masked) {
        Ok(encoding) => (
            encoding.logical().to_string(),
            encoding.physical_label().to_string(),
        ),
        Err(_) => (
            family
                .logical(code)
                .map_or_else(|| format!("unassigned({code})"), |l| l.to_string()),
            format!("invalid({physical})"),
        ),
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;
    use strum::{EnumCount as _, IntoEnumIterator as _};

    use super::*;
    use crate::decoder::Morton;
    use crate::test_helpers::parser;

    const INT: StreamCtx02 = StreamCtx02::Property(DataType02::U32);
    const BOOL: StreamCtx02 = StreamCtx02::Property(DataType02::Bool);
    const FLOAT: StreamCtx02 = StreamCtx02::Property(DataType02::F64);
    const VERTEX: StreamCtx02 = StreamCtx02::GeomVertices;
    const STR_PLAIN: StreamCtx02 = StreamCtx02::StrData(StrLayout::Plain);
    const STR_FSST_DICT: StreamCtx02 = StreamCtx02::StrData(StrLayout::FsstDict);
    const BLOB: StreamCtx02 = StreamCtx02::StrBlob(DictionaryType::None);

    use PhysicalEncoding as PE;

    fn meta(logical: LogicalEncoding, physical: PhysicalEncoding, num: u32) -> StreamMeta {
        let stream_type = StreamType::Data(DictionaryType::None);
        StreamMeta::new(stream_type, IntEncoding::new(logical, physical), num)
    }

    fn int(logical: IntLogical, physical: PhysicalEncoding, num: u32) -> StreamMeta {
        meta(LogicalEncoding::Int(logical), physical, num)
    }

    fn boolean(logical: BoolLogical, physical: PhysicalEncoding, num: u32) -> StreamMeta {
        meta(LogicalEncoding::Bool(logical), physical, num)
    }

    fn float(logical: FloatLogical, physical: PhysicalEncoding, num: u32) -> StreamMeta {
        meta(LogicalEncoding::Float(logical), physical, num)
    }

    fn vertex(logical: VertexLogical, physical: PhysicalEncoding, num: u32) -> StreamMeta {
        meta(LogicalEncoding::Vertex(logical), physical, num)
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
    #[case::float_alp(Family::Float, Logical::Alp, 0b010)]
    #[case::float_dict(Family::Float, Logical::Dict, 0b011)]
    #[case::vertex_none(Family::Vertex, Logical::None, 0b000)]
    #[case::vertex_delta(Family::Vertex, Logical::Delta, 0b001)]
    #[case::vertex_cw_delta(Family::Vertex, Logical::CwDelta, 0b010)]
    #[case::vertex_morton(Family::Vertex, Logical::Morton, 0b011)]
    #[case::str_none(Family::Str(StrLayout::Dict), Logical::None, 0b000)]
    #[case::str_delta_rle(Family::Str(StrLayout::Dict), Logical::DeltaRle, 0b011)]
    #[case::bytes_none(Family::Bytes, Logical::None, 0b000)]
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
        for family in Family::iter() {
            let mut canonical = Logical::iter();
            for &member in family.members() {
                assert!(
                    canonical.any(|c| c == member),
                    "{family:?} lists {:?}",
                    family.members()
                );
            }
        }
    }

    #[test]
    fn no_family_lists_an_encoding_twice() {
        for family in Family::iter() {
            for (index, &member) in family.members().iter().enumerate() {
                let code = u8::try_from(index).unwrap();
                assert_eq!(
                    family.code(member),
                    Some(code),
                    "{family:?} lists {member} twice"
                );
            }
        }
    }

    #[test]
    fn the_canonical_order_fits_the_logical_field() {
        let codes = usize::from(LOGICAL_MASK >> LOGICAL_SHIFT) + 1;
        assert!(Logical::COUNT <= codes, "{} encodings", Logical::COUNT);
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
    #[case::plain_lengths(
        STR_PLAIN,
        Family::Str(StrLayout::Plain),
        StreamType::Length(LengthType::VarBinary)
    )]
    #[case::dict_codes(
        StreamCtx02::StrData(StrLayout::Dict),
        Family::Str(StrLayout::Dict),
        StreamType::Offset(OffsetType::String)
    )]
    #[case::fsst_lengths(
        StreamCtx02::StrData(StrLayout::Fsst),
        Family::Str(StrLayout::Fsst),
        StreamType::Length(LengthType::Dictionary)
    )]
    #[case::fsst_dict_codes(
        STR_FSST_DICT,
        Family::Str(StrLayout::FsstDict),
        StreamType::Offset(OffsetType::String)
    )]
    #[case::dict_lengths(
        StreamCtx02::StrDictLengths,
        Family::Int,
        StreamType::Length(LengthType::Dictionary)
    )]
    #[case::symbol_lengths(
        StreamCtx02::StrSymbolLengths,
        Family::Int,
        StreamType::Length(LengthType::Symbol)
    )]
    #[case::plain_blob(BLOB, Family::Bytes, StreamType::Data(DictionaryType::None))]
    #[case::symbol_table(
        StreamCtx02::StrBlob(DictionaryType::Fsst),
        Family::Bytes,
        StreamType::Data(DictionaryType::Fsst)
    )]
    fn a_string_streams_position_picks_its_family_and_role(
        #[case] ctx: StreamCtx02,
        #[case] family: Family,
        #[case] stream_type: StreamType,
    ) {
        assert_eq!(ctx.family(), family);
        assert_eq!(ctx.stream_type(), stream_type);
    }

    #[rstest]
    #[case::varint_implicit(int(IntLogical::None, PE::VarInt, 5), 5, Family::Int, 0b0000_1000)]
    #[case::varint_explicit(int(IntLogical::None, PE::VarInt, 5), 9, Family::Int, 0b1000_1000)]
    #[case::raw_implicit(int(IntLogical::None, PE::None, 5), 5, Family::Int, 0b0000_0100)]
    #[case::delta_varint(int(IntLogical::Delta, PE::VarInt, 5), 5, Family::Int, 0b0001_1000)]
    #[case::rle_implicit(
        int(IntLogical::Rle(rle(5)), PE::VarInt, 5),
        5,
        Family::Int,
        0b0010_0000
    )]
    #[case::delta_rle(
        int(IntLogical::DeltaRle(rle(5)), PE::VarInt, 5),
        5,
        Family::Int,
        0b0011_0000
    )]
    #[case::raw_float(float(FloatLogical::None, PE::None, 5), 5, Family::Float, 0b0000_0100)]
    #[case::raw_bool(boolean(BoolLogical::None, PE::None, 5), 5, Family::Bool, 0b0000_0100)]
    #[case::float_dict_codes_varint(
        float(FloatLogical::Dict, PE::VarInt, 5),
        5,
        Family::Float,
        0b0011_1000
    )]
    #[case::float_alp_integers(
        float(FloatLogical::Alp(Alp::new(3, 1).unwrap()), PE::VarInt, 5),
        5,
        Family::Float,
        0b0010_1000
    )]
    #[case::cw_delta_vertices_explicit(
        vertex(VertexLogical::ComponentwiseDelta, PE::VarInt, 8),
        5,
        Family::Vertex,
        0b1010_1000
    )]
    #[case::str_plain_lengths(
        int(IntLogical::None, PE::VarInt, 5),
        5,
        Family::Str(StrLayout::Plain),
        0b0000_1000
    )]
    #[case::str_dict_codes_rle(
        int(IntLogical::Rle(rle(5)), PE::VarInt, 5),
        5,
        Family::Str(StrLayout::Dict),
        0b0010_0001
    )]
    #[case::str_fsst_lengths(
        int(IntLogical::Delta, PE::VarInt, 5),
        5,
        Family::Str(StrLayout::Fsst),
        0b0001_1010
    )]
    #[case::str_fsst_dict_codes(
        int(IntLogical::None, PE::VarInt, 5),
        5,
        Family::Str(StrLayout::FsstDict),
        0b0000_1011
    )]
    #[case::blob_never_carries_a_count(
        int(IntLogical::None, PE::None, 24),
        5,
        Family::Bytes,
        0b0000_0100
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
    #[case::varint(int(IntLogical::None, PE::VarInt, 5), 5, INT)]
    #[case::varint_explicit(int(IntLogical::None, PE::VarInt, 7), 5, INT)]
    #[case::raw(int(IntLogical::None, PE::None, 5), 5, INT)]
    #[case::delta(int(IntLogical::Delta, PE::VarInt, 5), 5, INT)]
    #[case::rle(int(IntLogical::Rle(rle(5)), PE::VarInt, 5), 5, INT)]
    #[case::delta_rle(int(IntLogical::DeltaRle(rle(9)), PE::VarInt, 9), 5, INT)]
    #[case::raw_float(float(FloatLogical::None, PE::None, 5), 5, FLOAT)]
    #[case::raw_bool(boolean(BoolLogical::None, PE::None, 5), 5, BOOL)]
    #[case::float_dict_codes_varint(float(FloatLogical::Dict, PE::VarInt, 5), 5, FLOAT)]
    #[case::float_dict_codes_raw(float(FloatLogical::Dict, PE::None, 5), 5, FLOAT)]
    #[case::float_alp(
        float(FloatLogical::Alp(Alp::new(6, 2).unwrap()), PE::VarInt, 5),
        5,
        FLOAT
    )]
    #[case::float_alp_explicit_count(
        float(FloatLogical::Alp(Alp::new(0, 0).unwrap()), PE::VarInt, 7),
        5,
        FLOAT
    )]
    #[case::cw_delta_vertices(vertex(VertexLogical::ComponentwiseDelta, PE::VarInt, 10), 5, VERTEX)]
    #[case::str_plain_lengths(int(IntLogical::None, PE::VarInt, 5), 5, STR_PLAIN)]
    #[case::str_fsst_dict_codes(int(IntLogical::DeltaRle(rle(5)), PE::VarInt, 5), 5, STR_FSST_DICT)]
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

        let (rest, parsed) = parse_stream(&buf, ctx, implicit_count, &mut parser()).unwrap();
        assert!(rest.is_empty());
        assert_eq!(parsed.meta.encoding, meta.encoding);
        assert_eq!(parsed.meta.num_values, meta.num_values);
        assert_eq!(parsed.meta.stream_type, ctx.stream_type());
        assert_eq!(parsed.data, payload);
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
    #[case::float_dict_with_extension(FLOAT, 0b0011_0101)]
    #[case::float_alp_with_extension(FLOAT, 0b0010_1001)]
    #[case::vertex_logical_past_table(VERTEX, 0b0100_1000)]
    #[case::float_physical_varint(FLOAT, 0b0000_1000)]
    #[case::float_physical_fastpfor(FLOAT, 0b0000_1100)]
    #[case::bool_physical_varint(BOOL, 0b0000_1000)]
    #[case::str_layout_disagrees_with_the_context(STR_PLAIN, 0b0000_1001)]
    #[case::str_logical_past_table(STR_PLAIN, 0b0100_1000)]
    #[case::blob_with_an_explicit_count(BLOB, 0b1000_0100)]
    #[case::blob_with_an_extension(BLOB, 0b0000_0101)]
    #[case::blob_logical_past_table(BLOB, 0b0001_0100)]
    #[case::blob_physical_varint(BLOB, 0b0000_1000)]
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
    #[case::float_alp(FLOAT, 0b0010_0000)]
    #[case::float_dict_no_len(FLOAT, 0b0011_0000)]
    #[case::bool_rle(BOOL, 0b0001_0000)]
    #[case::vertex_morton(VERTEX, 0b0011_0000)]
    fn parse_rejects_unimplemented_encoding(#[case] ctx: StreamCtx02, #[case] enc_byte: u8) {
        let buf = [enc_byte, 0, 0, 0];
        let err = parse_stream(&buf, ctx, 0, &mut parser()).unwrap_err();
        assert!(matches!(err, MltError::NotImplemented(_)), "{err:?}");
    }

    #[rstest]
    #[case::delta(0b0001_1000, LogicalEncoding::Int(IntLogical::Delta))]
    #[case::rle(0b0010_0000, LogicalEncoding::Int(IntLogical::Rle(rle(1))))]
    #[case::delta_rle(0b0011_0000, LogicalEncoding::Int(IntLogical::DeltaRle(rle(1))))]
    fn an_int_bit_pattern_never_means_the_same_on_a_float_column(
        #[case] enc_byte: u8,
        #[case] as_int: LogicalEncoding,
    ) {
        let buf = [enc_byte, 0, 0, 0];
        let (_, parsed) = parse_stream(&buf, INT, 1, &mut parser()).unwrap();
        assert_eq!(parsed.meta.encoding.logical, as_int);

        let as_float = parse_stream(&buf, FLOAT, 1, &mut parser())
            .ok()
            .map(|(_, p)| p.meta.encoding.logical);
        assert_ne!(as_float, Some(as_int));
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

    #[test]
    fn a_blob_reads_its_byte_length_as_its_value_count() {
        let payload = [1_u8, 2, 3];
        let meta = int(
            IntLogical::None,
            PE::None,
            u32::try_from(payload.len()).unwrap(),
        );
        let mut buf = Vec::new();
        write_stream_meta(&meta, &mut buf, 3, 99, Family::Bytes).unwrap();
        buf.extend_from_slice(&payload);
        assert_eq!(buf, [0b0000_0100, 3, 1, 2, 3]);

        let (rest, parsed) = parse_stream(&buf, BLOB, 99, &mut parser()).unwrap();
        assert!(rest.is_empty());
        assert_eq!(parsed.meta.num_values, 3);
        assert_eq!(parsed.data, payload);
    }

    #[rstest]
    #[case::rle(true)]
    #[case::delta_rle(false)]
    fn write_rejects_split_rle(#[case] plain_rle: bool) {
        let split = RleMeta::Split {
            runs: 2,
            num_rle_values: 5,
        };
        let logical = if plain_rle {
            IntLogical::Rle(split)
        } else {
            IntLogical::DeltaRle(split)
        };
        let meta = int(logical, PE::VarInt, 5);
        let mut buf = Vec::new();
        let err = write_stream_meta(&meta, &mut buf, 0, 5, Family::Int).unwrap_err();
        assert!(matches!(err, MltError::UnsupportedLogicalEncoding(_, _)));
    }

    #[rstest]
    #[case::cw_delta_on_a_property_column(
        LogicalEncoding::Vertex(VertexLogical::ComponentwiseDelta),
        Family::Int
    )]
    #[case::morton_on_a_property_column(
        LogicalEncoding::Vertex(VertexLogical::Morton(Morton::new(4, 0).unwrap())),
        Family::Int
    )]
    #[case::byte_rle_on_a_bool_column(
        LogicalEncoding::Bool(BoolLogical::ByteRle(RleMeta::Split {
            runs: 1,
            num_rle_values: 1
        })),
        Family::Bool
    )]
    #[case::dict_on_an_int_column(LogicalEncoding::Float(FloatLogical::Dict), Family::Int)]
    #[case::dict_on_a_vertex_stream(LogicalEncoding::Float(FloatLogical::Dict), Family::Vertex)]
    #[case::alp_on_an_int_column(
        LogicalEncoding::Float(FloatLogical::Alp(Alp::new(1, 0).unwrap())),
        Family::Int
    )]
    #[case::alp_on_a_bool_column(
        LogicalEncoding::Float(FloatLogical::Alp(Alp::new(1, 0).unwrap())),
        Family::Bool
    )]
    fn write_rejects_an_encoding_the_family_does_not_list(
        #[case] logical: LogicalEncoding,
        #[case] family: Family,
    ) {
        let meta = meta(logical, PE::None, 5);
        let mut buf = Vec::new();
        let err = write_stream_meta(&meta, &mut buf, 0, 5, family).unwrap_err();
        assert!(
            matches!(
                err,
                MltError::UnsupportedLogicalEncoding(_, _) | MltError::NotImplemented(_)
            ),
            "{err:?}"
        );
    }
}
