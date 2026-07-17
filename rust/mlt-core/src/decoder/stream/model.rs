use derive_debug::Dbg;
use num_enum::TryFromPrimitive;

use crate::utils::formatter::{bytes_dbg, compact_dbg};
use crate::{MltError, MltResult};

/// Logical encoding technique used for a column, as stored in the tile
#[derive(Debug, Clone, Copy, PartialEq, TryFromPrimitive)]
#[repr(u8)]
pub enum LogicalTechnique {
    None = 0,
    Delta = 1,
    ComponentwiseDelta = 2,
    Rle = 3,
    Morton = 4,
    PseudoDecimal = 5,
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
    /// Tag `0x02`: `(run_length, value)` pairs.
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
    Interleaved { num_rle_values: u32 },
}

impl RleMeta {
    /// The total expanded element count, common to both layouts.
    #[must_use]
    pub(crate) fn num_rle_values(self) -> u32 {
        match self {
            Self::Split { num_rle_values, .. } | Self::Interleaved { num_rle_values } => {
                num_rle_values
            }
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

/// How should the stream be interpreted at the logical level (second pass of decoding)
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LogicalEncoding {
    None,
    Delta,
    DeltaRle(RleMeta),
    ComponentwiseDelta,
    Rle(RleMeta),
    Morton(Morton),
    MortonDelta(Morton),
    MortonRle(Morton),
    PseudoDecimal,
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
    None = 0,
    Single = 1,
    Shared = 2,
    Vertex = 3,
    Morton = 4,
    Fsst = 5,
}

/// Offset type used for a column, as stored in the tile
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, TryFromPrimitive)]
#[repr(u8)]
pub enum OffsetType {
    Vertex = 0,
    Index = 1,
    String = 2,
    Key = 3,
}

/// Length type used for a column, as stored in the tile
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, TryFromPrimitive)]
#[repr(u8)]
pub enum LengthType {
    VarBinary = 0,
    Geometries = 1,
    Parts = 2,
    Rings = 3,
    Triangles = 4,
    Symbol = 5,
    Dictionary = 6,
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
    None = 0,
    /// Preferred, tends to produce the best compression ratio and decoding performance.
    /// But currently limited to 32-bit integer.
    FastPFor256 = 1,
    /// Can produce better results in combination with a heavyweight compression scheme like `Gzip`.
    /// Simple compression scheme where the encoding is easier to implement compared to `FastPfor`.
    VarInt = 2,
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
    pub(crate) const fn none() -> Self {
        Self::new(LogicalEncoding::None, PhysicalEncoding::None)
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
    pub(crate) fn new_none(stream_type: StreamType, num_values: usize) -> MltResult<Self> {
        let enc = IntEncoding::none();
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
