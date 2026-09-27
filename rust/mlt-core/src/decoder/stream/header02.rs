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
//! [varint byte_length]  absent on a raw stream whose physical field is `00`,
//!                       whose length follows from its count and element width
//! [varint parameters]   what the logical encoding carries, if anything:
//!                       ALP's scale and frame of reference, or Morton's grid
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
//! Not yet implemented (rejected with [`MltError::NotImplemented`]): RLE over a bool or float column.

use std::io;

use integer_encoding::VarIntWriter as _;
use num_enum::TryFromPrimitive;
use strum::IntoEnumIterator as _;
use usize_cast::IntoUsize as _;

use crate::codecs::varint::parse_varint;
use crate::decoder::{
    Alp, AlpScale, BoolLogical, DataType02, DictLayout, DictionaryType, FastPForKind, FloatLogical,
    IntEncoding, IntLogical, LengthType, LogicalEncoding, Morton, OffsetType, PhysicalEncoding,
    RawStream, RleMeta, StreamMeta, StreamType, VertexLogical,
};
use crate::errors::{AsMltError as _, fail_if_invalid_stream_size};
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

/// Physical field of a raw stream that leaves its byte length unwritten, the same pattern in every family.
const NO_LEN: u8 = 0b0000_0000;

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
    FrontCoded,
    BitPacked,
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

/// The width of the words a raw stream stores, fixed by the type its context decodes into.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum WordWidth {
    /// Four bytes, for a 32-bit or narrower type.
    #[default]
    W32,
    /// Eight bytes, for a 64-bit type.
    W64,
}

impl WordWidth {
    /// How many bytes one word takes.
    const fn bytes(self) -> u32 {
        match self {
            Self::W32 => 4,
            Self::W64 => 8,
        }
    }
}

/// Which logical encodings a stream's context admits, and how each reads the rest of the encoding byte.
/// Carries what a raw stream's byte length follows from, so a header can leave it unwritten.
#[derive(Debug, Clone, Copy, PartialEq, Eq, strum::EnumIter, strum::IntoStaticStr)]
pub(crate) enum Family {
    #[strum(serialize = "integer stream")]
    Int(WordWidth),
    #[strum(serialize = "bool column")]
    Bool,
    #[strum(serialize = "float column")]
    Float(WordWidth),
    #[strum(serialize = "vertex stream")]
    Vertex,
    /// A string column's leading stream.
    /// Is an integer stream whose extension bits carry [`StrLayout`].
    #[strum(serialize = "string column")]
    Str(StrLayout),
    /// An opaque byte blob, whose value count is its byte length.
    #[strum(serialize = "byte blob")]
    Bytes,
}

impl Default for Family {
    fn default() -> Self {
        Self::Int(WordWidth::default())
    }
}

impl Family {
    /// The encodings this family has, in [`Logical`]'s canonical order, indexed by their wire code.
    const fn members(self) -> &'static [Logical] {
        use Logical as L;
        match self {
            Self::Int(_) | Self::Str(_) => &[L::None, L::Delta, L::Rle, L::DeltaRle, L::BitPacked],
            Self::Bool => &[L::None, L::Rle],
            Self::Float(_) => &[L::None, L::Rle, L::Alp, L::Dict],
            Self::Vertex => &[L::None, L::Delta, L::CwDelta, L::Morton],
            Self::Bytes => &[L::None, L::FrontCoded],
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

    /// The byte length of `count` untransformed elements, which every family but a blob's fixes.
    /// A blob's count is its byte length, so nothing else could give it.
    fn raw_byte_length(self, count: u32) -> MltResult<Option<u32>> {
        let words = |width: WordWidth| count.checked_mul(width.bytes()).or_overflow().map(Some);
        match self {
            Self::Int(width) | Self::Float(width) => words(width),
            Self::Str(_) | Self::Vertex => words(WordWidth::W32),
            Self::Bool => Ok(Some(count.div_ceil(8))),
            Self::Bytes => Ok(None),
        }
    }
}

/// What a v2 stream takes as its value count where its own header carries none.
///
/// Which of the two a stream was written against is what the encoding byte's
/// `has_explicit_count` bit records, so one choice drives both directions.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Count02 {
    /// The count context implies: the layer's `feature_count`, or the presence
    /// popcount of an optional column's data stream.
    Implied(u32),
    /// Context implies no count, as for an m-value column or a shared dictionary's
    /// corpus, so each of those streams carries its own.
    ///
    /// The default, since a stream that writes its own count is readable whatever
    /// context it is written in.
    #[default]
    Explicit,
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
    /// The vertices themselves, or the distinct ones a dictionary layout stores once.
    GeomVertices,
    /// One index per vertex into the dictionary the preceding stream holds.
    GeomVertexOffsets,
    /// The triangle indices of a tessellated layer, three per triangle.
    GeomIndices,
    GeomOffsets(LengthType),
    /// A nested list or map node's lengths, one per present value of the node.
    NestedLengths,
    /// A nested node's presence, one bit per value its parent hands it.
    NestedPresence,
    /// A shape-coded nested node's key-set table, `shape_count * key_count` bits.
    NestedShapeTable,
    /// One shape id per row a shape-coded nested node marks present.
    NestedShapeIds,
}

impl StreamCtx02 {
    /// The family this stream's logical field is numbered in.
    pub(crate) fn family(self) -> Family {
        use DataType02 as D;
        match self {
            Self::Property(typ) | Self::PropertyDictionary(typ) => match typ {
                D::Bool => Family::Bool,
                D::F32 => Family::Float(WordWidth::W32),
                D::F64 => Family::Float(WordWidth::W64),
                D::LongId | D::I64 | D::U64 => Family::Int(WordWidth::W64),
                // A string or nested column heads a stream set of its own, so its family is never read.
                D::Id | D::I8 | D::U8 | D::I32 | D::U32 | D::Str | D::Struct | D::List | D::Map => {
                    Family::Int(WordWidth::W32)
                }
            },
            Self::StrData(layout) => Family::Str(layout),
            Self::StrBlob(_) => Family::Bytes,
            Self::NestedPresence | Self::NestedShapeTable => Family::Bool,
            Self::StrDictLengths
            | Self::StrSymbolLengths
            | Self::GeomTypes
            | Self::GeomVertexOffsets
            | Self::GeomIndices
            | Self::GeomOffsets(_)
            | Self::NestedLengths
            | Self::NestedShapeIds => Family::Int(WordWidth::W32),
            Self::GeomVertices => Family::Vertex,
        }
    }

    /// The stream role this position implies, which v2 does not store on the wire.
    pub(crate) fn stream_type(self) -> StreamType {
        match self {
            Self::Property(_) | Self::NestedShapeIds => StreamType::Data(DictionaryType::None),
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
            Self::GeomVertexOffsets => StreamType::Offset(OffsetType::Vertex),
            Self::GeomIndices => StreamType::Offset(OffsetType::Index),
            Self::GeomOffsets(length_type) => StreamType::Length(length_type),
            Self::NestedLengths => StreamType::Length(LengthType::Nested),
            Self::NestedPresence | Self::NestedShapeTable => StreamType::Present,
        }
    }
}

impl DictLayout {
    /// The [`Family::Bytes`] encoding this layout is named by on the wire.
    const fn logical(self) -> Logical {
        match self {
            Self::Plain => Logical::None,
            Self::FrontCoded => Logical::FrontCoded,
        }
    }

    /// The layout a byte blob's encoding byte names, read before the stream itself is parsed.
    ///
    /// Returns [`None`] for a code this version does not have, which parsing the stream then reports.
    pub(crate) fn from_bits(enc_byte: u8) -> Option<Self> {
        let logical = Family::Bytes.logical((enc_byte & LOGICAL_MASK) >> LOGICAL_SHIFT)?;
        Self::iter().find(|layout| layout.logical() == logical)
    }
}

/// Write a byte blob's stream header, whose `layout` names how its bytes are arranged.
/// A blob's value count is its byte length, so no count varint is ever written.
pub(crate) fn write_blob_meta<W: io::Write>(
    writer: &mut W,
    layout: DictLayout,
    byte_length: u32,
) -> MltResult<()> {
    let logical = layout.logical();
    let code = Family::Bytes
        .code(logical)
        .unwrap_or_else(|| unreachable_member(Family::Bytes, logical));
    writer.write_u8((code << LOGICAL_SHIFT) | PhysicalBits::WithLen as u8)?;
    writer.write_varint(byte_length)?;
    Ok(())
}

/// Physical field of a stream of integer words that writes its byte length.
#[derive(Debug, Clone, Copy, PartialEq, Eq, TryFromPrimitive, strum::IntoStaticStr)]
#[repr(u8)]
pub(crate) enum PhysicalInt {
    NoneWithLen = 0b0000_0100,
    VarInt = 0b0000_1000,
    FastPFor128 = 0b0000_1100,
}

/// Physical field of a stream of untransformed integer words, the one integer stream that may leave its byte length unwritten.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RawInt {
    /// The words as they are, their byte length following from their count and width.
    NoneNoLen,
    WithLen(PhysicalInt),
}

impl RawInt {
    /// What the physical field said, named for tooling.
    fn label(self) -> &'static str {
        match self {
            Self::NoneNoLen => "NoneNoLen",
            Self::WithLen(physical) => physical.into(),
        }
    }
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
    None(RawInt),
    Delta(PhysicalInt),
    /// Varint-coded `(run, value)` pairs, so the physical field is reserved.
    Rle,
    /// As [`Self::Rle`], over zigzag deltas.
    DeltaRle,
    /// Every value in the same number of bits, so the physical field is reserved.
    /// The width leads the payload, since it is a property of the values rather than of the format.
    BitPacked,
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
/// A blob's count is its byte length, so every member writes one and none carries a physical field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LogicalBytes {
    /// The bytes as they are.
    None,
    /// A dictionary's entries with their shared prefixes removed, so the blob holds only suffixes.
    /// The preceding lengths stream holds the shared-prefix lengths, then the suffix lengths.
    FrontCoded,
}

/// Logical encoding of a geometry vertex stream.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LogicalVertex {
    None(RawInt),
    Delta(PhysicalInt),
    CwDelta(PhysicalInt),
    /// Deltas between the Morton codes of a sorted vertex dictionary.
    /// The grid the codes are laid on follows the byte length as two varints.
    Morton(PhysicalInt),
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

/// Read the physical field as an integer codec, for an encoding that always writes its byte length.
fn physical_int(enc_byte: u8) -> MltResult<PhysicalInt> {
    PhysicalInt::try_from(enc_byte & PHYSICAL_MASK)
        .map_err(|_| MltError::ParsingEncodingByte(enc_byte))
}

/// Read the physical field of untransformed integer words, which alone may leave the byte length unwritten.
fn raw_int(enc_byte: u8) -> MltResult<RawInt> {
    if enc_byte & PHYSICAL_MASK == NO_LEN {
        Ok(RawInt::NoneNoLen)
    } else {
        physical_int(enc_byte).map(RawInt::WithLen)
    }
}

/// Read the physical field of a fixed-width element stream.
fn physical_bits(enc_byte: u8) -> MltResult<PhysicalBits> {
    PhysicalBits::try_from(enc_byte & PHYSICAL_MASK)
        .map_err(|_| MltError::ParsingEncodingByte(enc_byte))
}

/// Require the physical field to be `bits`, for an encoding that admits one pattern.
fn require_physical(enc_byte: u8, bits: u8) -> MltResult<()> {
    if enc_byte & PHYSICAL_MASK == bits {
        Ok(())
    } else {
        Err(MltError::ParsingEncodingByte(enc_byte))
    }
}

/// Require the physical field to be zero, for an encoding that implies it.
fn no_physical(enc_byte: u8) -> MltResult<()> {
    require_physical(enc_byte, 0)
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
        Logical::None => LogicalInt::None(raw_int(enc_byte)?),
        Logical::Delta => LogicalInt::Delta(physical_int(enc_byte)?),
        Logical::Rle => {
            no_physical(enc_byte)?;
            LogicalInt::Rle
        }
        Logical::DeltaRle => {
            no_physical(enc_byte)?;
            LogicalInt::DeltaRle
        }
        Logical::BitPacked => {
            no_physical(enc_byte)?;
            LogicalInt::BitPacked
        }
        Logical::CwDelta | Logical::Morton | Logical::Alp | Logical::Dict | Logical::FrontCoded => {
            unreachable_member(family, logical)
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
            Family::Int(_) => Self::Int(logical_int(family, enc_byte, logical)?),
            Family::Str(layout) => {
                // The caller resolves the layout from this very byte, so a mismatch means it read another.
                if StrLayout::from_bits(enc_byte) != layout {
                    return Err(MltError::ParsingEncodingByte(enc_byte));
                }
                Self::Str(logical_int(family, enc_byte, logical)?, layout)
            }
            Family::Bytes => {
                // A blob's count is its byte length, so every blob writes one.
                require_physical(enc_byte, PhysicalBits::WithLen as u8)?;
                Self::Bytes(match logical {
                    Logical::None => LogicalBytes::None,
                    Logical::FrontCoded => LogicalBytes::FrontCoded,
                    Logical::Delta
                    | Logical::CwDelta
                    | Logical::Rle
                    | Logical::DeltaRle
                    | Logical::Morton
                    | Logical::Alp
                    | Logical::Dict
                    | Logical::BitPacked => unreachable_member(family, logical),
                })
            }
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
                | Logical::Dict
                | Logical::FrontCoded
                | Logical::BitPacked => unreachable_member(family, logical),
            }),
            Family::Float(_) => Self::Float(match logical {
                Logical::None => LogicalFloat::None(physical_bits(enc_byte)?),
                Logical::Rle => {
                    no_physical(enc_byte)?;
                    LogicalFloat::Rle
                }
                Logical::Alp => LogicalFloat::Alp(physical_int(enc_byte)?),
                Logical::Dict => LogicalFloat::Dict(physical_int(enc_byte)?),
                Logical::Delta
                | Logical::CwDelta
                | Logical::DeltaRle
                | Logical::Morton
                | Logical::FrontCoded
                | Logical::BitPacked => unreachable_member(family, logical),
            }),
            Family::Vertex => Self::Vertex(match logical {
                Logical::None => LogicalVertex::None(raw_int(enc_byte)?),
                Logical::Delta => LogicalVertex::Delta(physical_int(enc_byte)?),
                Logical::CwDelta => LogicalVertex::CwDelta(physical_int(enc_byte)?),
                Logical::Morton => LogicalVertex::Morton(physical_int(enc_byte)?),
                Logical::Rle
                | Logical::DeltaRle
                | Logical::Alp
                | Logical::Dict
                | Logical::FrontCoded
                | Logical::BitPacked => unreachable_member(family, logical),
            }),
        })
    }

    /// The canonical encoding this byte named.
    fn logical(self) -> Logical {
        match self {
            Self::Str(logical, _) => Self::Int(logical).logical(),
            Self::Bytes(LogicalBytes::None)
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
            Self::Int(LogicalInt::BitPacked) => Logical::BitPacked,
            Self::Vertex(LogicalVertex::Morton(_)) => Logical::Morton,
            Self::Float(LogicalFloat::Alp(_)) => Logical::Alp,
            Self::Float(LogicalFloat::Dict(_)) => Logical::Dict,
            Self::Bytes(LogicalBytes::FrontCoded) => Logical::FrontCoded,
        }
    }

    /// What the physical field said, named for tooling.
    fn physical_label(self) -> &'static str {
        match self {
            Self::Str(logical, _) => Self::Int(logical).physical_label(),
            Self::Int(LogicalInt::None(raw)) | Self::Vertex(LogicalVertex::None(raw)) => {
                raw.label()
            }
            Self::Int(LogicalInt::Delta(p))
            | Self::Float(LogicalFloat::Dict(p) | LogicalFloat::Alp(p))
            | Self::Vertex(
                LogicalVertex::Delta(p) | LogicalVertex::CwDelta(p) | LogicalVertex::Morton(p),
            ) => p.into(),
            Self::Bool(LogicalBool::None(p)) | Self::Float(LogicalFloat::None(p)) => p.into(),
            Self::Bytes(LogicalBytes::None | LogicalBytes::FrontCoded) => {
                PhysicalBits::WithLen.into()
            }
            Self::Int(LogicalInt::BitPacked) => "BitPacked",
            Self::Int(LogicalInt::Rle | LogicalInt::DeltaRle)
            | Self::Bool(LogicalBool::Rle)
            | Self::Float(LogicalFloat::Rle) => "implied",
        }
    }

    /// Whether the header leaves `byte_length` out, which only a raw stream's physical `00` says.
    /// The length then follows from the count and the element width the family fixes.
    fn omits_byte_length(self) -> bool {
        match self {
            Self::Str(logical, _) => Self::Int(logical).omits_byte_length(),
            Self::Int(LogicalInt::None(raw)) | Self::Vertex(LogicalVertex::None(raw)) => {
                raw == RawInt::NoneNoLen
            }
            Self::Bool(LogicalBool::None(p)) | Self::Float(LogicalFloat::None(p)) => {
                p == PhysicalBits::NoLen
            }
            Self::Int(_) | Self::Bool(_) | Self::Float(_) | Self::Vertex(_) | Self::Bytes(_) => {
                false
            }
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
            // Front coding restructures the dictionary, not the blob's words, so the
            // stream itself still reads as flat bytes and the layout is applied by the caller.
            Self::Bytes(LogicalBytes::None | LogicalBytes::FrontCoded) => IntEncoding::new(
                LogicalEncoding::Int(IntLogical::None),
                PhysicalEncoding::None,
            ),
            Self::Int(LogicalInt::None(raw)) => {
                IntEncoding::new(LogicalEncoding::Int(IntLogical::None), flat_raw_int(raw))
            }
            Self::Vertex(LogicalVertex::None(raw)) => IntEncoding::new(
                LogicalEncoding::Vertex(VertexLogical::None),
                flat_raw_int(raw),
            ),
            Self::Int(LogicalInt::Delta(p)) => {
                IntEncoding::new(LogicalEncoding::Int(IntLogical::Delta), flat_int(p))
            }
            Self::Vertex(LogicalVertex::Delta(p)) => {
                IntEncoding::new(LogicalEncoding::Vertex(VertexLogical::Delta), flat_int(p))
            }
            Self::Vertex(LogicalVertex::CwDelta(p)) => IntEncoding::new(
                LogicalEncoding::Vertex(VertexLogical::ComponentwiseDelta),
                flat_int(p),
            ),
            Self::Int(LogicalInt::Rle) => IntEncoding::new(
                LogicalEncoding::Int(IntLogical::Rle(rle())),
                PhysicalEncoding::VarInt,
            ),
            Self::Int(LogicalInt::DeltaRle) => IntEncoding::new(
                LogicalEncoding::Int(IntLogical::DeltaRle(rle())),
                PhysicalEncoding::VarInt,
            ),
            // Bit packing is a physical layout, so it reads back as one over untransformed values.
            Self::Int(LogicalInt::BitPacked) => IntEncoding::new(
                LogicalEncoding::Int(IntLogical::None),
                PhysicalEncoding::BitPacked,
            ),
            // Either physical pattern is the elements as they are, differing only in the header.
            Self::Bool(LogicalBool::None(_)) => IntEncoding::new(
                LogicalEncoding::Bool(BoolLogical::None),
                PhysicalEncoding::None,
            ),
            Self::Float(LogicalFloat::None(_)) => IntEncoding::new(
                LogicalEncoding::Float(FloatLogical::None),
                PhysicalEncoding::None,
            ),
            Self::Bool(LogicalBool::Rle) => {
                return Err(MltError::NotImplemented("v2 RLE over a bool column"));
            }
            Self::Float(LogicalFloat::Rle) => {
                return Err(MltError::NotImplemented("v2 RLE over a float column"));
            }
            Self::Float(LogicalFloat::Alp(p)) => {
                let (after, scale) = parse_u8(input)?;
                let (after, base) = parse_varint::<i64>(after)?;
                rest = after;
                let scale = AlpScale::from_byte(scale)?;
                IntEncoding::new(
                    LogicalEncoding::Float(FloatLogical::Alp(Alp { scale, base })),
                    flat_int(p),
                )
            }
            Self::Float(LogicalFloat::Dict(p)) => {
                IntEncoding::new(LogicalEncoding::Float(FloatLogical::Dict), flat_int(p))
            }
            Self::Vertex(LogicalVertex::Morton(p)) => {
                let (after, bits) = parse_varint::<u32>(input)?;
                let (after, shift) = parse_varint::<u32>(after)?;
                rest = after;
                IntEncoding::new(
                    LogicalEncoding::Vertex(VertexLogical::MortonDelta(Morton::new(bits, shift)?)),
                    flat_int(p),
                )
            }
        };
        Ok((rest, encoding))
    }
}

/// A family only resolves a code to one of its own members, so any other pairing is a bug in [`Family::members`].
fn unreachable_member(family: Family, logical: Logical) -> ! {
    unreachable!("{family:?} does not list {logical:?}")
}

/// Map an integer stream's physical field to the flat shared encoding.
fn flat_int(physical: PhysicalInt) -> PhysicalEncoding {
    match physical {
        PhysicalInt::NoneWithLen => PhysicalEncoding::None,
        PhysicalInt::VarInt => PhysicalEncoding::VarInt,
        PhysicalInt::FastPFor128 => PhysicalEncoding::FastPFor(FastPForKind::Block128Le),
    }
}

/// Map an untransformed integer stream's physical field to the flat shared encoding.
/// Whether the byte length was written is a property of the header, not of the words.
fn flat_raw_int(raw: RawInt) -> PhysicalEncoding {
    match raw {
        RawInt::NoneNoLen => PhysicalEncoding::None,
        RawInt::WithLen(physical) => flat_int(physical),
    }
}

/// The physical field bits for a stream of integers, whatever the column's own type is.
fn physical_int_field(physical: PhysicalEncoding) -> MltResult<u8> {
    Ok(match physical {
        PhysicalEncoding::None => PhysicalInt::NoneWithLen as u8,
        PhysicalEncoding::VarInt => PhysicalInt::VarInt as u8,
        PhysicalEncoding::FastPFor(FastPForKind::Block128Le) => PhysicalInt::FastPFor128 as u8,
        PhysicalEncoding::FastPFor(FastPForKind::Block256Be) => {
            return Err(MltError::UnsupportedPhysicalEncodingForType(
                physical,
                "v2, whose FastPFor streams are 128-value little-endian blocks",
            ));
        }
        // Bit packing is named by the logical field, which `wire_fields` takes before this.
        PhysicalEncoding::BitPacked => {
            return Err(MltError::UnsupportedPhysicalEncodingForType(
                physical,
                "a v2 physical field, which has no code for bit packing",
            ));
        }
    })
}

/// The header fields an encoding is written as, the reverse of [`Encoding02::to_model`].
struct WireFields {
    logical: Logical,
    physical_bits: u8,
    /// Whether `byte_length` follows, which only a raw stream of physical `00` leaves out.
    writes_length: bool,
}

/// The logical encoding and physical field bits `encoding` is written as, in `family`'s numbering.
/// The caller checks the logical against the family, which is where an illegal pairing is caught.
/// A raw stream of `num_values` elements whose width `family` fixes gets physical `00` and no `byte_length`.
fn wire_fields(
    encoding: IntEncoding,
    family: Family,
    num_values: u32,
    byte_length: u32,
) -> MltResult<WireFields> {
    use BoolLogical as BL;
    use FloatLogical as FL;
    use IntLogical as IL;
    use LogicalEncoding as LE;
    use VertexLogical as VL;

    let physical = |encoding: IntEncoding| -> MltResult<u8> {
        match (family, encoding.physical) {
            (Family::Bool | Family::Float(_) | Family::Bytes, PhysicalEncoding::None) => {
                Ok(PhysicalBits::WithLen as u8)
            }
            (Family::Bool | Family::Float(_) | Family::Bytes, _) => {
                Err(MltError::UnsupportedPhysicalEncoding(
                    "v2 bool, float and blob streams store their elements as they are",
                ))
            }
            _ => physical_int_field(encoding.physical),
        }
    };

    let with_length = |logical: Logical, physical_bits: u8| WireFields {
        logical,
        physical_bits,
        writes_length: true,
    };

    // Bit packing has a logical code of its own, since the physical field has no spare pattern.
    if encoding.physical == PhysicalEncoding::BitPacked {
        if encoding.logical != LE::Int(IL::None) {
            return Err(MltError::UnsupportedLogicalEncoding(
                encoding.logical,
                "v2 bit packing, which stores values as they are",
            ));
        }
        return Ok(with_length(Logical::BitPacked, 0));
    }

    Ok(match encoding.logical {
        LE::Int(IL::None) | LE::Bool(BL::None) | LE::Float(FL::None) | LE::Vertex(VL::None) => {
            match (encoding.physical, family.raw_byte_length(num_values)?) {
                (PhysicalEncoding::None, Some(expected)) => {
                    fail_if_invalid_stream_size(byte_length.into_usize(), expected.into_usize())?;
                    WireFields {
                        logical: Logical::None,
                        physical_bits: NO_LEN,
                        writes_length: false,
                    }
                }
                _ => with_length(Logical::None, physical(encoding)?),
            }
        }
        LE::Int(IL::Delta) | LE::Vertex(VL::Delta) => {
            with_length(Logical::Delta, physical(encoding)?)
        }
        LE::Vertex(VL::ComponentwiseDelta) => with_length(Logical::CwDelta, physical(encoding)?),
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
            with_length(logical, 0)
        }
        // Codes and scaled integers are integer streams, whatever the column's type is.
        LE::Float(FL::Dict) => with_length(Logical::Dict, physical_int_field(encoding.physical)?),
        LE::Float(FL::Alp(_)) => with_length(Logical::Alp, physical_int_field(encoding.physical)?),
        LE::Bool(BL::ByteRle(_)) => {
            return Err(MltError::UnsupportedLogicalEncoding(
                encoding.logical,
                "v2, whose bool columns have no byte-RLE",
            ));
        }
        // v2 stores Morton codes only as a sorted dictionary, whose deltas are always the shorter form.
        LE::Vertex(VL::MortonDelta(_)) => {
            with_length(Logical::Morton, physical_int_field(encoding.physical)?)
        }
        LE::Vertex(VL::Morton(_) | VL::MortonRle(_)) => {
            return Err(MltError::UnsupportedLogicalEncoding(
                encoding.logical,
                "v2, whose Morton streams are always delta-coded",
            ));
        }
    })
}

/// Parse one v2 stream (header + data), synthesizing [`StreamMeta`] from the
/// wire header plus the positional context in `ctx`.
///
/// - `ctx` fixes both the stream's role, which v2 does not store, and the
///   [`Family`] its logical field is numbered in.
/// - `count` is what the stream is read against where its header carries no count
///   of its own, which only a [`Count02::Implied`] one can supply.
///
/// Reserves an upper-bound estimate of decoded bytes (`num_values * 8`) on the
/// parser, mirroring the v1 codec.
pub(crate) fn parse_stream<'a>(
    input: &'a [u8],
    ctx: StreamCtx02,
    count: Count02,
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
    let (input, wire_count) = if explicit {
        let (input, wire_count) = parse_varint::<u32>(input)?;
        (input, Some(wire_count))
    } else {
        (input, None)
    };

    let (input, num_values, byte_length) = if family == Family::Bytes {
        // A blob's count is its byte length, which every blob writes.
        let (input, byte_length) = parse_varint::<u32>(input)?;
        (input, byte_length, byte_length)
    } else {
        let num_values = match wire_count {
            Some(wire_count) => wire_count,
            None => match count {
                Count02::Implied(count) => count,
                Count02::Explicit => {
                    return Err(MltError::StreamWithoutCount(ctx.stream_type(), enc_byte));
                }
            },
        };
        if encoding.omits_byte_length() {
            let byte_length = family
                .raw_byte_length(num_values)?
                .ok_or(MltError::ParsingEncodingByte(enc_byte))?;
            (input, num_values, byte_length)
        } else {
            let (input, byte_length) = parse_varint::<u32>(input)?;
            (input, num_values, byte_length)
        }
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
/// `count` is what the decoder will read this stream against, so a count varint is written only when the stream's own differs.
/// A raw stream whose element width `family` fixes gets physical `00` and no `byte_length` varint.
pub(crate) fn write_stream_meta<W: io::Write>(
    meta: &StreamMeta,
    writer: &mut W,
    byte_length: u32,
    count: Count02,
    family: Family,
) -> MltResult<()> {
    use LogicalEncoding as LE;

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
    let fields = wire_fields(meta.encoding, family, num_values, byte_length)?;
    let code = family.code(fields.logical).ok_or_else(|| {
        MltError::UnsupportedLogicalEncoding(meta.encoding.logical, family.into())
    })?;
    // A blob's count is its byte length, which its length varint already carries.
    let explicit = family != Family::Bytes && count != Count02::Implied(num_values);
    let extension = match family {
        Family::Str(layout) => layout as u8,
        Family::Int(_) | Family::Bool | Family::Float(_) | Family::Vertex | Family::Bytes => 0,
    };
    let enc_byte = if explicit { HAS_EXPLICIT_COUNT } else { 0 }
        | (code << LOGICAL_SHIFT)
        | fields.physical_bits
        | extension;
    writer.write_u8(enc_byte)?;
    if explicit {
        writer.write_varint(num_values)?;
    }
    if fields.writes_length {
        writer.write_varint(byte_length)?;
    }
    if let LE::Float(FloatLogical::Alp(alp)) = meta.encoding.logical {
        writer.write_u8(alp.scale.to_byte())?;
        writer.write_varint(alp.base)?;
    }
    if let LE::Vertex(VertexLogical::MortonDelta(morton)) = meta.encoding.logical {
        writer.write_varint(morton.bits)?;
        writer.write_varint(morton.shift)?;
    }
    Ok(())
}

/// Whether a header starting with `enc_byte` leaves `byte_length` out, for tooling that re-walks one.
/// A byte that does not parse in `family` omits nothing, since parsing the stream already failed on it.
pub(crate) fn omits_byte_length(family: Family, enc_byte: u8) -> bool {
    Encoding02::parse(family, enc_byte).is_ok_and(Encoding02::omits_byte_length)
}

/// Whether `enc_byte` names a raw packed bitmap, the `Bool` family's logical `None` over either raw physical pattern.
pub(crate) fn is_packed_bitmap(enc_byte: u8) -> bool {
    matches!(
        Encoding02::parse(Family::Bool, enc_byte),
        Ok(Encoding02::Bool(LogicalBool::None(_)))
    )
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

    use super::*;
    use crate::test_helpers::parser;

    const INT: StreamCtx02 = StreamCtx02::Property(DataType02::U32);
    const LONG: StreamCtx02 = StreamCtx02::Property(DataType02::U64);
    const BOOL: StreamCtx02 = StreamCtx02::Property(DataType02::Bool);
    const FLOAT: StreamCtx02 = StreamCtx02::Property(DataType02::F64);
    const VERTEX: StreamCtx02 = StreamCtx02::GeomVertices;
    const STR_PLAIN: StreamCtx02 = StreamCtx02::StrData(StrLayout::Plain);
    const STR_FSST_DICT: StreamCtx02 = StreamCtx02::StrData(StrLayout::FsstDict);
    const BLOB: StreamCtx02 = StreamCtx02::StrBlob(DictionaryType::None);

    const INT_FAMILY: Family = Family::Int(WordWidth::W32);
    const FLOAT_FAMILY: Family = Family::Float(WordWidth::W64);

    use PhysicalEncoding as PE;

    const FPF128: PE = PE::FastPFor(FastPForKind::Block128Le);

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
    #[case::int_none(INT_FAMILY, Logical::None, 0b000)]
    #[case::int_delta(INT_FAMILY, Logical::Delta, 0b001)]
    #[case::int_rle(INT_FAMILY, Logical::Rle, 0b010)]
    #[case::int_delta_rle(INT_FAMILY, Logical::DeltaRle, 0b011)]
    #[case::bool_none(Family::Bool, Logical::None, 0b000)]
    #[case::bool_rle(Family::Bool, Logical::Rle, 0b001)]
    #[case::float_none(FLOAT_FAMILY, Logical::None, 0b000)]
    #[case::float_rle(FLOAT_FAMILY, Logical::Rle, 0b001)]
    #[case::float_alp(FLOAT_FAMILY, Logical::Alp, 0b010)]
    #[case::float_dict(FLOAT_FAMILY, Logical::Dict, 0b011)]
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
    fn every_family_fits_the_logical_field() {
        let codes = usize::from(LOGICAL_MASK >> LOGICAL_SHIFT) + 1;
        for family in Family::iter() {
            let members = family.members().len();
            assert!(members <= codes, "{family:?} lists {members} encodings");
        }
    }

    #[rstest]
    #[case::id(DataType02::Id, INT_FAMILY)]
    #[case::long_id(DataType02::LongId, Family::Int(WordWidth::W64))]
    #[case::i8(DataType02::I8, INT_FAMILY)]
    #[case::i64(DataType02::I64, Family::Int(WordWidth::W64))]
    #[case::u64(DataType02::U64, Family::Int(WordWidth::W64))]
    #[case::bool(DataType02::Bool, Family::Bool)]
    #[case::f32(DataType02::F32, Family::Float(WordWidth::W32))]
    #[case::f64(DataType02::F64, FLOAT_FAMILY)]
    fn column_data_type_picks_its_family(#[case] data: DataType02, #[case] family: Family) {
        assert_eq!(StreamCtx02::Property(data).family(), family);
        assert_eq!(StreamCtx02::PropertyDictionary(data).family(), family);
    }

    #[rstest]
    #[case::types(StreamCtx02::GeomTypes, INT_FAMILY)]
    #[case::lengths(StreamCtx02::GeomOffsets(LengthType::Parts), INT_FAMILY)]
    #[case::vertices(StreamCtx02::GeomVertices, Family::Vertex)]
    #[case::vertex_offsets(StreamCtx02::GeomVertexOffsets, INT_FAMILY)]
    #[case::triangle_indices(StreamCtx02::GeomIndices, INT_FAMILY)]
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
        INT_FAMILY,
        StreamType::Length(LengthType::Dictionary)
    )]
    #[case::symbol_lengths(
        StreamCtx02::StrSymbolLengths,
        INT_FAMILY,
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
    #[case::varint_implicit(int(IntLogical::None, PE::VarInt, 5), 5, INT_FAMILY, 0b0000_1000)]
    #[case::varint_explicit(int(IntLogical::None, PE::VarInt, 5), 9, INT_FAMILY, 0b1000_1000)]
    #[case::raw_implicit(int(IntLogical::None, PE::None, 5), 5, INT_FAMILY, 0b0000_0000)]
    #[case::raw_delta(int(IntLogical::Delta, PE::None, 5), 5, INT_FAMILY, 0b0001_0100)]
    #[case::fastpfor128(int(IntLogical::None, FPF128, 5), 5, INT_FAMILY, 0b0000_1100)]
    #[case::delta_varint(int(IntLogical::Delta, PE::VarInt, 5), 5, INT_FAMILY, 0b0001_1000)]
    #[case::rle_implicit(
        int(IntLogical::Rle(rle(5)), PE::VarInt, 5),
        5,
        INT_FAMILY,
        0b0010_0000
    )]
    #[case::delta_rle(
        int(IntLogical::DeltaRle(rle(5)), PE::VarInt, 5),
        5,
        INT_FAMILY,
        0b0011_0000
    )]
    #[case::raw_float(float(FloatLogical::None, PE::None, 5), 5, FLOAT_FAMILY, 0b0000_0000)]
    #[case::raw_bool(boolean(BoolLogical::None, PE::None, 5), 5, Family::Bool, 0b0000_0000)]
    #[case::raw_vertices(
        vertex(VertexLogical::None, PE::None, 6),
        5,
        Family::Vertex,
        0b1000_0000
    )]
    #[case::float_dict_codes_varint(
        float(FloatLogical::Dict, PE::VarInt, 5),
        5,
        FLOAT_FAMILY,
        0b0011_1000
    )]
    #[case::float_dict_codes_raw(
        float(FloatLogical::Dict, PE::None, 5),
        5,
        FLOAT_FAMILY,
        0b0011_0100
    )]
    #[case::float_alp_integers(
        float(FloatLogical::Alp(Alp::new(3, 1, 0).unwrap()), PE::VarInt, 5),
        5,
        FLOAT_FAMILY,
        0b0010_1000
    )]
    #[case::cw_delta_vertices_explicit(
        vertex(VertexLogical::ComponentwiseDelta, PE::VarInt, 8),
        5,
        Family::Vertex,
        0b1010_1000
    )]
    #[case::morton_dict_varint(
        vertex(VertexLogical::MortonDelta(Morton::new(12, 3).unwrap()), PE::VarInt, 5),
        5,
        Family::Vertex,
        0b0011_1000
    )]
    #[case::str_plain_lengths(
        int(IntLogical::None, PE::VarInt, 5),
        5,
        Family::Str(StrLayout::Plain),
        0b0000_1000
    )]
    #[case::str_raw_dict_codes(
        int(IntLogical::None, PE::None, 5),
        5,
        Family::Str(StrLayout::Dict),
        0b0000_0001
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
        let byte_length = family
            .raw_byte_length(meta.num_values)
            .unwrap()
            .unwrap_or(0);
        let mut buf = Vec::new();
        let count = Count02::Implied(implicit_count);
        write_stream_meta(&meta, &mut buf, byte_length, count, family).unwrap();
        assert_eq!(buf[0], expected);
    }

    #[rstest]
    #[case::varint(int(IntLogical::None, PE::VarInt, 5), 5, INT)]
    #[case::varint_explicit(int(IntLogical::None, PE::VarInt, 7), 5, INT)]
    #[case::raw_delta(int(IntLogical::Delta, PE::None, 5), 5, INT)]
    #[case::fastpfor128(int(IntLogical::None, FPF128, 5), 5, INT)]
    #[case::delta_fastpfor128(int(IntLogical::Delta, FPF128, 5), 5, INT)]
    #[case::float_dict_codes_fastpfor128(float(FloatLogical::Dict, FPF128, 5), 5, FLOAT)]
    #[case::delta(int(IntLogical::Delta, PE::VarInt, 5), 5, INT)]
    #[case::rle(int(IntLogical::Rle(rle(5)), PE::VarInt, 5), 5, INT)]
    #[case::delta_rle(int(IntLogical::DeltaRle(rle(9)), PE::VarInt, 9), 5, INT)]
    #[case::float_dict_codes_varint(float(FloatLogical::Dict, PE::VarInt, 5), 5, FLOAT)]
    #[case::float_dict_codes_raw(float(FloatLogical::Dict, PE::None, 5), 5, FLOAT)]
    #[case::float_alp(
        float(FloatLogical::Alp(Alp::new(6, 2, -7).unwrap()), PE::VarInt, 5),
        5,
        FLOAT
    )]
    #[case::float_alp_explicit_count(
        float(FloatLogical::Alp(Alp::new(0, 0, 0).unwrap()), PE::VarInt, 7),
        5,
        FLOAT
    )]
    #[case::cw_delta_vertices(vertex(VertexLogical::ComponentwiseDelta, PE::VarInt, 10), 5, VERTEX)]
    #[case::morton_dict(
        vertex(VertexLogical::MortonDelta(Morton::new(16, 4096).unwrap()), PE::VarInt, 10),
        5,
        VERTEX
    )]
    #[case::morton_dict_fastpfor128(
        vertex(VertexLogical::MortonDelta(Morton::new(0, 0).unwrap()), FPF128, 5),
        5,
        VERTEX
    )]
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
        let count = Count02::Implied(implicit_count);
        write_stream_meta(&meta, &mut buf, byte_length, count, ctx.family()).unwrap();
        buf.extend_from_slice(&payload);

        let (rest, parsed) = parse_stream(&buf, ctx, count, &mut parser()).unwrap();
        assert!(rest.is_empty());
        assert_eq!(parsed.meta.encoding, meta.encoding);
        assert_eq!(parsed.meta.num_values, meta.num_values);
        assert_eq!(parsed.meta.stream_type, ctx.stream_type());
        assert_eq!(parsed.data, payload);
    }

    #[rstest]
    #[case::u32_words(
        int(IntLogical::None, PE::None, 2),
        INT,
        &[1, 0, 0, 0, 2, 0, 0, 0],
        &[0b0000_0000]
    )]
    #[case::u64_words(
        int(IntLogical::None, PE::None, 2),
        LONG,
        &[1, 0, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0],
        &[0b0000_0000]
    )]
    #[case::u64_word_explicit_count(
        int(IntLogical::None, PE::None, 1),
        LONG,
        &[1, 0, 0, 0, 0, 0, 0, 0],
        &[0b1000_0000, 1]
    )]
    #[case::f64_words(
        float(FloatLogical::None, PE::None, 2),
        FLOAT,
        &[0, 0, 0, 0, 0, 0, 0xF0, 0x3F, 0, 0, 0, 0, 0, 0, 0, 0x40],
        &[0b0000_0000]
    )]
    #[case::f32_words(
        float(FloatLogical::None, PE::None, 2),
        StreamCtx02::PropertyDictionary(DataType02::F32),
        &[0, 0, 0x80, 0x3F, 0, 0, 0, 0x40],
        &[0b0000_0000]
    )]
    #[case::bitmap(boolean(BoolLogical::None, PE::None, 9), BOOL, &[0b1010_1010, 1], &[0b1000_0000, 9])]
    #[case::bitmap_of_no_bits(boolean(BoolLogical::None, PE::None, 0), BOOL, &[], &[0b1000_0000, 0])]
    #[case::vertex_words(
        vertex(VertexLogical::None, PE::None, 2),
        VERTEX,
        &[1, 0, 0, 0, 2, 0, 0, 0],
        &[0b0000_0000]
    )]
    #[case::str_dict_codes(
        int(IntLogical::None, PE::None, 2),
        StreamCtx02::StrData(StrLayout::Dict),
        &[1, 0, 0, 0, 2, 0, 0, 0],
        &[0b0000_0001]
    )]
    #[case::empty_words(int(IntLogical::None, PE::None, 0), INT, &[], &[0b1000_0000, 0])]
    fn a_raw_stream_roundtrips_without_a_byte_length(
        #[case] meta: StreamMeta,
        #[case] ctx: StreamCtx02,
        #[case] payload: &[u8],
        #[case] header: &[u8],
    ) {
        let count = Count02::Implied(2);
        let byte_length = u32::try_from(payload.len()).unwrap();
        let mut buf = Vec::new();
        write_stream_meta(&meta, &mut buf, byte_length, count, ctx.family()).unwrap();
        assert_eq!(buf, header);
        buf.extend_from_slice(payload);

        let (rest, parsed) = parse_stream(&buf, ctx, count, &mut parser()).unwrap();
        assert!(rest.is_empty());
        assert_eq!(parsed.meta.encoding, meta.encoding);
        assert_eq!(parsed.meta.num_values, meta.num_values);
        assert_eq!(parsed.data, payload);
    }

    #[test]
    fn a_raw_stream_takes_only_its_derived_length_from_the_input() {
        let buf = [0b0000_0000, 1, 0, 0, 0, 2, 0, 0, 0, 0xFF];
        let (rest, parsed) = parse_stream(&buf, INT, Count02::Implied(2), &mut parser()).unwrap();
        assert_eq!(rest, [0xFF]);
        assert_eq!(parsed.data, [1, 0, 0, 0, 2, 0, 0, 0]);
    }

    #[test]
    fn a_raw_stream_shorter_than_its_derived_length_is_rejected() {
        let buf = [0b0000_0000, 1, 0, 0, 0, 2, 0, 0];
        let err = parse_stream(&buf, INT, Count02::Implied(2), &mut parser()).unwrap_err();
        assert!(matches!(err, MltError::UnableToTake(8)), "{err:?}");
    }

    #[test]
    fn a_raw_stream_whose_derived_length_overflows_is_rejected() {
        let buf = [0b0000_0000];
        let count = Count02::Implied(u32::MAX);
        let err = parse_stream(&buf, LONG, count, &mut parser()).unwrap_err();
        assert!(matches!(err, MltError::IntegerOverflow), "{err:?}");
    }

    #[test]
    fn a_blob_keeps_its_byte_length_where_words_of_the_same_size_drop_it() {
        let words = int(IntLogical::None, PE::None, 2);
        let mut as_words = Vec::new();
        write_stream_meta(&words, &mut as_words, 8, Count02::Implied(2), INT_FAMILY).unwrap();
        assert_eq!(as_words, [0b0000_0000]);

        let blob = int(IntLogical::None, PE::None, 8);
        let mut as_blob = Vec::new();
        write_stream_meta(&blob, &mut as_blob, 8, Count02::Implied(2), Family::Bytes).unwrap();
        assert_eq!(as_blob, [0b0000_0100, 8]);
    }

    #[rstest]
    #[case::u32_words(int(IntLogical::None, PE::None, 2), INT_FAMILY, 7, 8)]
    #[case::u64_words(int(IntLogical::None, PE::None, 2), Family::Int(WordWidth::W64), 8, 16)]
    #[case::f32_words(
        float(FloatLogical::None, PE::None, 3),
        Family::Float(WordWidth::W32),
        24,
        12
    )]
    #[case::bitmap(boolean(BoolLogical::None, PE::None, 9), Family::Bool, 9, 2)]
    #[case::vertex_words(vertex(VertexLogical::None, PE::None, 3), Family::Vertex, 6, 12)]
    fn write_rejects_a_raw_payload_that_is_not_its_count_of_elements(
        #[case] meta: StreamMeta,
        #[case] family: Family,
        #[case] byte_length: u32,
        #[case] expected: usize,
    ) {
        let mut buf = Vec::new();
        let err = write_stream_meta(&meta, &mut buf, byte_length, Count02::Implied(2), family)
            .unwrap_err();
        assert!(
            matches!(err, MltError::InvalidDecodingStreamSize(actual, e) if actual == byte_length.into_usize() && e == expected),
            "{err:?}"
        );
    }

    #[rstest]
    #[case::extension_bit0(INT, 0b0000_1001)]
    #[case::extension_bit1(INT, 0b0000_1010)]
    #[case::extension_on_float(FLOAT, 0b0000_0101)]
    #[case::extension_on_rle(INT, 0b0010_0001)]
    #[case::rle_with_physical(INT, 0b0010_0100)]
    #[case::delta_rle_with_physical(INT, 0b0011_1000)]
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
    #[case::blob_logical_past_table(BLOB, 0b0010_0100)]
    #[case::blob_physical_varint(BLOB, 0b0000_1000)]
    #[case::blob_no_len(BLOB, 0b0000_0000)]
    #[case::blob_front_coded_no_len(BLOB, 0b0001_0000)]
    #[case::delta_no_len(INT, 0b0001_0000)]
    #[case::str_delta_no_len(STR_PLAIN, 0b0001_0000)]
    #[case::float_alp_no_len(FLOAT, 0b0010_0000)]
    #[case::float_dict_no_len(FLOAT, 0b0011_0000)]
    #[case::vertex_delta_no_len(VERTEX, 0b0001_0000)]
    #[case::cw_delta_no_len(VERTEX, 0b0010_0000)]
    #[case::morton_no_len(VERTEX, 0b0011_0000)]
    fn parse_rejects_malformed_encoding_byte(#[case] ctx: StreamCtx02, #[case] enc_byte: u8) {
        let buf = [enc_byte, 0];
        let err = parse_stream(&buf, ctx, Count02::Implied(0), &mut parser()).unwrap_err();
        assert!(
            matches!(err, MltError::ParsingEncodingByte(b) if b == enc_byte),
            "{err:?}"
        );
    }

    #[rstest]
    #[case::float_rle(FLOAT, 0b0001_0000)]
    #[case::bool_rle(BOOL, 0b0001_0000)]
    fn parse_rejects_unimplemented_encoding(#[case] ctx: StreamCtx02, #[case] enc_byte: u8) {
        let buf = [enc_byte, 0];
        let err = parse_stream(&buf, ctx, Count02::Implied(0), &mut parser()).unwrap_err();
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
        let (_, parsed) = parse_stream(&buf, INT, Count02::Implied(1), &mut parser()).unwrap();
        assert_eq!(parsed.meta.encoding.logical, as_int);

        let as_float = parse_stream(&buf, FLOAT, Count02::Implied(1), &mut parser())
            .ok()
            .map(|(_, p)| p.meta.encoding.logical);
        assert_ne!(as_float, Some(as_int));
    }

    #[rstest]
    #[case(DictLayout::Plain, 0b0000_0100)]
    #[case(DictLayout::FrontCoded, 0b0001_0100)]
    fn a_blob_encoding_byte_names_its_dict_layout(
        #[case] layout: DictLayout,
        #[case] enc_byte: u8,
    ) {
        let mut buf = Vec::new();
        write_blob_meta(&mut buf, layout, 0).unwrap();
        assert_eq!(buf[0], enc_byte);
        assert_eq!(DictLayout::from_bits(enc_byte), Some(layout));
    }

    #[test]
    fn logical_code_one_is_delta_for_ints_and_rle_for_floats() {
        let buf = [0b0001_1000, 0];
        let (_, as_int) = parse_stream(&buf, INT, Count02::Implied(0), &mut parser()).unwrap();
        assert_eq!(
            as_int.meta.encoding,
            IntEncoding::new(LogicalEncoding::Int(IntLogical::Delta), PE::VarInt)
        );
        let buf = [0b0001_0000, 0];
        let as_float = parse_stream(&buf, FLOAT, Count02::Implied(0), &mut parser()).unwrap_err();
        assert!(
            matches!(
                as_float,
                MltError::NotImplemented("v2 RLE over a float column")
            ),
            "{as_float:?}"
        );
    }

    #[test]
    fn a_stream_with_neither_count_is_rejected() {
        // VarInt ints with the explicit-count bit clear, so only context could count them.
        let buf = [0b0000_1000, 0];
        let err = parse_stream(&buf, INT, Count02::Explicit, &mut parser()).unwrap_err();
        assert!(
            matches!(err, MltError::StreamWithoutCount(t, b) if t == INT.stream_type() && b == buf[0]),
            "{err:?}"
        );
    }

    #[test]
    fn a_stream_written_against_no_implied_count_carries_its_own() {
        let payload = [1_u8, 2, 3];
        let meta = int(IntLogical::None, PE::VarInt, 5);
        let mut buf = Vec::new();
        write_stream_meta(&meta, &mut buf, 3, Count02::Explicit, INT_FAMILY).unwrap();
        buf.extend_from_slice(&payload);
        assert_eq!(buf, [0b1000_1000, 5, 3, 1, 2, 3]);

        let (rest, parsed) = parse_stream(&buf, INT, Count02::Explicit, &mut parser()).unwrap();
        assert!(rest.is_empty());
        assert_eq!(parsed.meta.num_values, 5);
        assert_eq!(parsed.data, payload);
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
        write_stream_meta(&meta, &mut buf, 3, Count02::Implied(99), Family::Bytes).unwrap();
        buf.extend_from_slice(&payload);
        assert_eq!(buf, [0b0000_0100, 3, 1, 2, 3]);

        let (rest, parsed) = parse_stream(&buf, BLOB, Count02::Implied(99), &mut parser()).unwrap();
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
        let err =
            write_stream_meta(&meta, &mut buf, 0, Count02::Implied(5), INT_FAMILY).unwrap_err();
        assert!(matches!(err, MltError::UnsupportedLogicalEncoding(_, _)));
    }

    #[rstest]
    #[case::cw_delta_on_a_property_column(
        LogicalEncoding::Vertex(VertexLogical::ComponentwiseDelta),
        INT_FAMILY
    )]
    #[case::morton_on_a_property_column(
        LogicalEncoding::Vertex(VertexLogical::Morton(Morton::new(4, 0).unwrap())),
        INT_FAMILY
    )]
    #[case::byte_rle_on_a_bool_column(
        LogicalEncoding::Bool(BoolLogical::ByteRle(RleMeta::Split {
            runs: 1,
            num_rle_values: 1
        })),
        Family::Bool
    )]
    #[case::dict_on_an_int_column(LogicalEncoding::Float(FloatLogical::Dict), INT_FAMILY)]
    #[case::dict_on_a_vertex_stream(LogicalEncoding::Float(FloatLogical::Dict), Family::Vertex)]
    #[case::alp_on_an_int_column(
        LogicalEncoding::Float(FloatLogical::Alp(Alp::new(1, 0, 0).unwrap())),
        INT_FAMILY
    )]
    #[case::alp_on_a_bool_column(
        LogicalEncoding::Float(FloatLogical::Alp(Alp::new(1, 0, 0).unwrap())),
        Family::Bool
    )]
    fn write_rejects_an_encoding_the_family_does_not_list(
        #[case] logical: LogicalEncoding,
        #[case] family: Family,
    ) {
        let meta = meta(logical, PE::None, 5);
        let mut buf = Vec::new();
        let err = write_stream_meta(&meta, &mut buf, 0, Count02::Implied(5), family).unwrap_err();
        assert!(
            matches!(
                err,
                MltError::UnsupportedLogicalEncoding(_, _) | MltError::NotImplemented(_)
            ),
            "{err:?}"
        );
    }
}
