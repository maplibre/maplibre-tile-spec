use derive_debug::Dbg;
use num_enum::TryFromPrimitive;

use crate::utils::formatter::{bytes_dbg, compact_dbg};
use crate::{MltError, MltResult};

/// Logical encoding technique used for a column, as stored in the tile
///
/// Variants are already shifted into the primary logical field of the v1 encoding byte (bits 7-5),
/// so that field is matched with a mask rather than shifted down first.
/// The secondary field (bits 4-2) holds the same patterns three bits lower.
#[derive(Debug, Clone, Copy, PartialEq, TryFromPrimitive)]
#[repr(u8)]
pub enum LogicalTechnique {
    None = 0b0000_0000,
    Delta = 0b0010_0000,
    ComponentwiseDelta = 0b0100_0000,
    Rle = 0b0110_0000,
    Morton = 0b1000_0000,
}

/// The combinations of the two [`LogicalTechnique`] fields that are legal on the wire
///
/// Each variant is the whole logical part of the v1 encoding byte, i.e. the primary field
/// (bits 7-5) or-ed with the secondary field (bits 4-2).
/// Any other pairing of the two fields is rejected while parsing.
#[derive(Debug, Clone, Copy, PartialEq, TryFromPrimitive)]
#[repr(u8)]
pub enum LogicalCombination {
    None = 0b0000_0000,
    Delta = 0b0010_0000,
    DeltaRle = 0b0010_1100,
    ComponentwiseDelta = 0b0100_0000,
    Rle = 0b0110_0000,
    Morton = 0b1000_0000,
    MortonDelta = 0b1000_0100,
    MortonRle = 0b1000_1100,
}

/// Which RLE stream layout the encoder should produce.
///
/// A data-less selector chosen up front by the wire format (see
/// [`WireVersion::rle_layout`](crate::encoder::WireVersion)); the realized
/// per-stream metadata is [`RleMeta`], whose variants mirror these.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RleLayout {
    /// Tag `0x01`: all run lengths first, then all values.
    Split,
    /// Tag `0x02`: `(run_length, value)` pairs. Requires the `unstable-v2` feature.
    #[cfg(feature = "unstable-v2")]
    Interleaved,
}

/// Metadata for RLE decoding, one variant per [`RleLayout`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RleMeta {
    /// Tag `0x01`: physically-decoded words are `[run_len × runs][value × runs]`.
    /// `runs` is the split point; `num_rle_values` is the expanded element count.
    Split { runs: u32, num_rle_values: u32 },
    /// Tag `0x02`: physically-decoded words are `(run_len, value)` pairs. The run
    /// count is derived from the data length, so only the expanded element count
    /// (`num_rle_values`, from the stream's count context) is carried.
    /// Requires the `unstable-v2` feature.
    #[cfg(feature = "unstable-v2")]
    Interleaved { num_rle_values: u32 },
}

impl RleMeta {
    /// The total expanded element count, common to both layouts.
    #[cfg(feature = "unstable-v2")]
    #[must_use]
    pub(crate) fn num_rle_values(self) -> u32 {
        match self {
            Self::Split { num_rle_values, .. } => num_rle_values,
            #[cfg(feature = "unstable-v2")]
            Self::Interleaved { num_rle_values } => num_rle_values,
        }
    }
}

/// Metadata for Morton decoding
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Morton {
    /// Number of bits used
    pub(crate) bits: u32,
    /// Coordinate shift
    pub(crate) shift: u32,
}

impl Morton {
    pub fn new(bits: u32, shift: u32) -> MltResult<Self> {
        if bits <= 16 {
            Ok(Self { bits, shift })
        } else {
            Err(MltError::InvalidMortonBits(bits))
        }
    }
}

/// What kind of values a stream holds, which fixes the encodings it can name.
/// Neither wire format stores it: v1 reads it from the column type, v2 from the stream's context.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValueKind {
    Int,
    Bool,
    Float,
    Vertex,
}

/// Logical encoding of a stream of integer values.
/// Covers the id columns, the integer property columns, and the geometry length and offset streams.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum IntLogical {
    None,
    Delta,
    Rle(RleMeta),
    DeltaRle(RleMeta),
}

/// Logical encoding of a bool column's data stream or a presence bitfield.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BoolLogical {
    /// A raw packed bitmap, one bit per value.
    None,
    /// A byte-RLE compressed bitmap.
    /// Its run parameters come from the stream's context rather than from its header.
    ByteRle(RleMeta),
}

/// Logical encoding of a float column's data stream.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FloatLogical {
    /// Fixed-width little-endian values, one per element.
    None,
    /// A decimal split across two integer streams, which v1 can name but no decoder here implements.
    /// One code per element, with the distinct values following as a second stream.
    /// Only the tag `0x02` codec reads or writes it.
    Dict,
}

/// Logical encoding of a geometry vertex stream, whose values are interleaved coordinate pairs.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum VertexLogical {
    None,
    Delta,
    ComponentwiseDelta,
    Morton(Morton),
    MortonDelta(Morton),
    MortonRle(Morton),
}

/// How should the stream be interpreted at the logical level (second pass of decoding)
///
/// Split per [`ValueKind`] so a stream can name only the encodings its values can have.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LogicalEncoding {
    Int(IntLogical),
    Bool(BoolLogical),
    Float(FloatLogical),
    Vertex(VertexLogical),
}

impl LogicalEncoding {
    /// The kind of values this encoding belongs to.
    #[must_use]
    pub fn kind(self) -> ValueKind {
        match self {
            Self::Int(_) => ValueKind::Int,
            Self::Bool(_) => ValueKind::Bool,
            Self::Float(_) => ValueKind::Float,
            Self::Vertex(_) => ValueKind::Vertex,
        }
    }

    /// The identity encoding for `kind`, i.e. values stored as they are.
    #[must_use]
    pub(crate) fn none(kind: ValueKind) -> Self {
        match kind {
            ValueKind::Int => Self::Int(IntLogical::None),
            ValueKind::Bool => Self::Bool(BoolLogical::None),
            ValueKind::Float => Self::Float(FloatLogical::None),
            ValueKind::Vertex => Self::Vertex(VertexLogical::None),
        }
    }

    /// Whether the stream's own logical pass is a no-op, so the physical words are already the output.
    /// True for a float dictionary's codes, which the column turns back into floats.
    #[must_use]
    pub(crate) fn is_identity(self) -> bool {
        matches!(
            self,
            Self::Int(IntLogical::None)
                | Self::Bool(BoolLogical::None)
                | Self::Float(FloatLogical::None | FloatLogical::Dict)
                | Self::Vertex(VertexLogical::None)
        )
    }
}

/// Carries the stream metadata needed to perform the logical decode pass.
///
/// Construct with [`LogicalValue::new`] after the physical decode pass fills a
/// `&[u32]` or `&[u64]` buffer, then call the appropriate `decode_*` method,
/// passing that slice as `data`.
#[derive(Debug, PartialEq)]
pub struct LogicalValue {
    pub(crate) meta: StreamMeta,
}

// Physical encoding types

/// Dictionary type used for a column, as stored in the tile
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, TryFromPrimitive)]
#[repr(u8)]
pub enum DictionaryType {
    None = 0b0000_0000,
    Single = 0b0000_0001,
    Shared = 0b0000_0010,
    Vertex = 0b0000_0011,
    Morton = 0b0000_0100,
    Fsst = 0b0000_0101,
}

/// Offset type used for a column, as stored in the tile
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, TryFromPrimitive)]
#[repr(u8)]
pub enum OffsetType {
    Vertex = 0b0000_0000,
    Index = 0b0000_0001,
    String = 0b0000_0010,
    Key = 0b0000_0011,
}

/// Length type used for a column, as stored in the tile
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, TryFromPrimitive)]
#[repr(u8)]
pub enum LengthType {
    VarBinary = 0b0000_0000,
    Geometries = 0b0000_0001,
    Parts = 0b0000_0010,
    Rings = 0b0000_0011,
    Triangles = 0b0000_0100,
    Symbol = 0b0000_0101,
    Dictionary = 0b0000_0110,
}

/// How should the stream be interpreted at the physical level (first pass of decoding)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum StreamType {
    Present,
    Data(DictionaryType),
    Offset(OffsetType),
    Length(LengthType),
}

/// Physical encoding used for a column, as stored in the tile
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, TryFromPrimitive)]
#[repr(u8)]
pub enum PhysicalEncoding {
    None = 0b0000_0000,
    /// Preferred, tends to produce the best compression ratio and decoding performance.
    /// But currently limited to 32-bit integer.
    FastPFor256 = 0b0000_0001,
    /// Can produce better results in combination with a heavyweight compression scheme like `Gzip`.
    /// Simple compression scheme where the encoding is easier to implement compared to `FastPfor`.
    VarInt = 0b0000_0010,
}

// RawStream types

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct IntEncoding {
    pub logical: LogicalEncoding,
    pub physical: PhysicalEncoding,
}

impl IntEncoding {
    #[must_use]
    pub(crate) const fn new(logical: LogicalEncoding, physical: PhysicalEncoding) -> Self {
        Self { logical, physical }
    }

    #[must_use]
    pub(crate) fn none(kind: ValueKind) -> Self {
        Self::new(LogicalEncoding::none(kind), PhysicalEncoding::None)
    }
}

/// Metadata about an encoded stream
#[derive(Clone, Copy, Dbg, PartialEq)]
pub struct StreamMeta {
    #[dbg(formatter = "compact_dbg")]
    pub stream_type: StreamType,
    #[dbg(formatter = "compact_dbg")]
    pub encoding: IntEncoding,
    pub(crate) num_values: u32,
}

impl StreamMeta {
    #[inline]
    pub(crate) fn new(stream_type: StreamType, encoding: IntEncoding, num_values: u32) -> Self {
        Self {
            stream_type,
            encoding,
            num_values,
        }
    }

    #[inline]
    pub(crate) fn new2(
        stream_type: StreamType,
        logical: LogicalEncoding,
        physical: PhysicalEncoding,
        num_values: usize,
    ) -> MltResult<Self> {
        let enc = IntEncoding::new(logical, physical);
        Ok(Self::new(stream_type, enc, u32::try_from(num_values)?))
    }

    #[inline]
    pub(crate) fn new_none(
        stream_type: StreamType,
        kind: ValueKind,
        num_values: usize,
    ) -> MltResult<Self> {
        let enc = IntEncoding::none(kind);
        Ok(Self::new(stream_type, enc, u32::try_from(num_values)?))
    }
}

/// Representation of an encoded stream
#[derive(Clone, Dbg, PartialEq)]
pub struct RawStream<'a> {
    pub meta: StreamMeta,
    #[dbg(formatter = "bytes_dbg")]
    pub(crate) data: &'a [u8],
}

impl<'a> RawStream<'a> {
    #[must_use]
    pub(crate) fn new(meta: StreamMeta, data: &'a [u8]) -> Self {
        Self { meta, data }
    }
}
