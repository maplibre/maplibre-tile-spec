use borrowme::borrowme;
use num_enum::TryFromPrimitive;

/// Logical encoding technique used for a column, as stored in the tile
#[borrowme]
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

/// Metadata for RLE decoding
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RleMeta {
    pub runs: u32,
    pub num_rle_values: u32,
}

/// Metadata for Morton decoding
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MortonMeta {
    pub num_bits: u32,
    pub coordinate_shift: u32,
}

/// How should the stream be interpreted at the logical level (second pass of decoding)
#[derive(Clone, Copy, PartialEq)]
pub enum LogicalEncoding {
    None,
    Delta,
    DeltaRle(RleMeta),
    ComponentwiseDelta,
    Rle(RleMeta),
    Morton(MortonMeta),
    MortonDelta(MortonMeta),
    MortonRle(MortonMeta),
    PseudoDecimal,
}

/// Representation of decoded stream data
#[derive(Debug, PartialEq)]
pub enum LogicalData {
    VecU32(Vec<u32>),
    VecU64(Vec<u64>),
}

/// Representation of a decoded value
#[derive(Debug, PartialEq)]
pub struct LogicalValue {
    pub(crate) meta: StreamMeta,
    pub(crate) data: LogicalData,
}

// Physical encoding types

/// Dictionary type used for a column, as stored in the tile
#[borrowme]
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
#[borrowme]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, TryFromPrimitive)]
#[repr(u8)]
pub enum OffsetType {
    Vertex = 0,
    Index = 1,
    String = 2,
    Key = 3,
}

/// Length type used for a column, as stored in the tile
#[borrowme]
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
#[borrowme]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, TryFromPrimitive)]
#[repr(u8)]
pub enum PhysicalEncoding {
    None = 0,
    /// Preferred, tends to produce the best compression ratio and decoding performance.
    /// But currently limited to 32-bit integer.
    FastPFOR = 1,
    /// Can produce better results in combination with a heavyweight compression scheme like `Gzip`.
    /// Simple compression scheme where the encoding is easier to implement compared to `FastPfor`.
    VarInt = 2,
}

// Stream types

#[derive(Clone, Copy, PartialEq)]
pub struct IntEncoding {
    pub logical: LogicalEncoding,
    pub physical: PhysicalEncoding,
}

/// Metadata about an encoded stream
#[derive(Clone, Copy, PartialEq)]
pub struct StreamMeta {
    pub stream_type: StreamType,
    pub encoding: IntEncoding,
    pub num_values: u32,
}

/// Representation of an encoded stream
#[borrowme]
#[derive(Debug, PartialEq, Clone)]
pub struct Stream<'a> {
    pub meta: StreamMeta,
    pub data: StreamData<'a>,
}

#[borrowme::borrowme]
#[derive(PartialEq, Clone)]
pub enum StreamData<'a> {
    VarInt(#[borrowme(borrow_with=Vec::as_slice)] &'a [u8]),
    Encoded(#[borrowme(borrow_with=Vec::as_slice)] &'a [u8]),
}
