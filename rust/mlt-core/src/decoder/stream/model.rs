use std::fmt::{Display, Formatter, Result as FmtResult};

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

/// The decimal scaling an ALP column uses: `i = round(v * 10^e / 10^f)`.
/// Chosen before any value is seen, so it carries no frame of reference.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AlpScale {
    /// Decimal exponent the values were scaled by.
    pub(crate) e: u8,
    /// Factor dividing out the trailing zeros `e` introduced, never exceeding `e`.
    pub(crate) f: u8,
}

/// ALP parameters: `v = (base + offset) * 10^f / 10^e`.
/// Written as three header varints, the stream itself holding the unsigned offsets.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Alp {
    pub(crate) scale: AlpScale,
    /// Frame of reference the offsets are measured from, the smallest scaled integer in the column.
    pub(crate) base: i64,
}

#[cfg(feature = "unstable-v2")]
impl AlpScale {
    /// Largest exponent the codes can carry, past which `v * 10^e` leaves the `i64` range.
    /// The codes themselves are bounded tighter, to `2^53 - 1`, by the encoder.
    pub(crate) const MAX_EXPONENT: u8 = 18;

    /// Net power of ten the codes carry, which fixes their magnitude and so their stored size.
    /// The order [`candidates`](crate::codecs::alp::candidates) walks, and so what lets the
    /// encoder stop at the first scale that fits; only the tests holding that invariant name it.
    #[cfg(test)]
    pub(crate) fn net(self) -> u8 {
        self.e - self.f
    }
}

/// Flattened, since the nesting is an encoder concern and these appear in stream labels.
impl std::fmt::Debug for Alp {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(
            f,
            "Alp {{ e: {}, f: {}, base: {} }}",
            self.scale.e, self.scale.f, self.base
        )
    }
}

#[cfg(feature = "unstable-v2")]
impl Alp {
    pub(crate) fn new(e: u8, f: u8, base: i64) -> MltResult<Self> {
        if e <= AlpScale::MAX_EXPONENT && f <= e {
            Ok(Self {
                scale: AlpScale { e, f },
                base,
            })
        } else {
            Err(MltError::InvalidAlpParams(e, f))
        }
    }

    /// Measure a scaled integer from the frame of reference, giving the offset the stream stores.
    /// Wrapping, so a foreign stream whose codes span the whole `i64` range still subtracts
    /// exactly; our own encoder keeps codes within `2^53 - 1`, so the spread fits `u64` easily.
    #[expect(
        clippy::cast_sign_loss,
        reason = "the bit pattern is the point; `code_at` casts it back"
    )]
    pub(crate) fn offset_of(self, code: i64) -> u64 {
        (code as u64).wrapping_sub(self.base as u64)
    }

    /// Put a stored offset back on the frame of reference, inverting [`Self::offset_of`].
    #[expect(
        clippy::cast_possible_wrap,
        clippy::cast_sign_loss,
        reason = "the bit pattern is the point; inverts `offset_of` exactly"
    )]
    pub(crate) fn code_at(self, offset: u64) -> i64 {
        (self.base as u64).wrapping_add(offset) as i64
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
    /// Integers scaled by these parameters.
    /// Only the tag `0x02` codec reads or writes it.
    Alp(Alp),
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
    /// Not true for ALP, whose offsets still need the frame of reference added back.
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
    /// A nested list or map node's lengths, one per present value of the node.
    #[cfg(feature = "unstable-v2")]
    Nested = 0b0000_0111,
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum PhysicalEncoding {
    None,
    /// Preferred, tends to produce the best compression ratio and decoding performance.
    /// But currently limited to 32-bit integer.
    FastPFor(FastPForKind),
    /// Can produce better results in combination with a heavyweight compression scheme like `Gzip`.
    /// Simple compression scheme where the encoding is easier to implement compared to `FastPfor`.
    VarInt,
    /// Every value in the same number of bits, which a leading payload byte names.
    /// Beats `VarInt` on the short, small-alphabet streams a dictionary's codes form,
    /// where `FastPFor`'s 128-value blocks are too coarse to pay for themselves.
    #[cfg(feature = "unstable-v2")]
    BitPacked,
}

/// The `FastPFor` block size and word order a wire version codes its integer streams with.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum FastPForKind {
    /// 256-value blocks over big-endian words
    Block256Be,
    /// 128-value blocks over little-endian words
    #[cfg(feature = "unstable-v2")]
    Block128Le,
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

// Display impls the annotated dump's JSON carries instead of serde derives on this
// vocabulary, so the wire enums do not become frozen public JSON API.

impl Display for StreamType {
    /// `present`, `data`, `data[fsst]`, `offset[string]`, `length[rings]`.
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            Self::Present => f.write_str("present"),
            Self::Data(dict) => {
                let name = match dict {
                    DictionaryType::None => return f.write_str("data"),
                    DictionaryType::Single => "single",
                    DictionaryType::Shared => "shared",
                    DictionaryType::Vertex => "vertex",
                    DictionaryType::Morton => "morton",
                    DictionaryType::Fsst => "fsst",
                };
                write!(f, "data[{name}]")
            }
            Self::Offset(offset) => {
                let name = match offset {
                    OffsetType::Vertex => "vertex",
                    OffsetType::Index => "index",
                    OffsetType::String => "string",
                    OffsetType::Key => "key",
                };
                write!(f, "offset[{name}]")
            }
            Self::Length(length) => {
                let name = match length {
                    LengthType::VarBinary => "var-binary",
                    LengthType::Geometries => "geometries",
                    LengthType::Parts => "parts",
                    LengthType::Rings => "rings",
                    LengthType::Triangles => "triangles",
                    LengthType::Symbol => "symbol",
                    LengthType::Dictionary => "dictionary",
                    #[cfg(feature = "unstable-v2")]
                    LengthType::Nested => "nested",
                };
                write!(f, "length[{name}]")
            }
        }
    }
}

impl Display for LogicalEncoding {
    /// The value kind, then its encoding: `int/rle`, `float/alp`, `vertex/morton-delta`.
    ///
    /// Run parameters are left out, they belong to the region's own annotation.
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        let (kind, enc) = match self {
            Self::Int(int) => (
                "int",
                match int {
                    IntLogical::None => "none",
                    IntLogical::Delta => "delta",
                    IntLogical::Rle(_) => "rle",
                    IntLogical::DeltaRle(_) => "delta-rle",
                },
            ),
            Self::Bool(b) => (
                "bool",
                match b {
                    BoolLogical::None => "none",
                    BoolLogical::ByteRle(_) => "byte-rle",
                },
            ),
            Self::Float(float) => (
                "float",
                match float {
                    FloatLogical::None => "none",
                    FloatLogical::Dict => "dict",
                    FloatLogical::Alp(_) => "alp",
                },
            ),
            Self::Vertex(vertex) => (
                "vertex",
                match vertex {
                    VertexLogical::None => "none",
                    VertexLogical::Delta => "delta",
                    VertexLogical::ComponentwiseDelta => "componentwise-delta",
                    VertexLogical::Morton(_) => "morton",
                    VertexLogical::MortonDelta(_) => "morton-delta",
                    VertexLogical::MortonRle(_) => "morton-rle",
                },
            ),
        };
        write!(f, "{kind}/{enc}")
    }
}

impl Display for PhysicalEncoding {
    /// `none`, `varint`, `fastpfor[256be]`, `bit-packed`.
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            Self::None => f.write_str("none"),
            Self::VarInt => f.write_str("varint"),
            Self::FastPFor(kind) => {
                let name = match kind {
                    FastPForKind::Block256Be => "256be",
                    #[cfg(feature = "unstable-v2")]
                    FastPForKind::Block128Le => "128le",
                };
                write!(f, "fastpfor[{name}]")
            }
            #[cfg(feature = "unstable-v2")]
            Self::BitPacked => f.write_str("bit-packed"),
        }
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    fn morton() -> Morton {
        Morton { bits: 4, shift: 0 }
    }

    fn alp() -> Alp {
        Alp {
            scale: AlpScale { e: 3, f: 1 },
            base: -5,
        }
    }

    fn rle() -> RleMeta {
        RleMeta::Split {
            runs: 2,
            num_rle_values: 5,
        }
    }

    #[rstest]
    #[case::present(StreamType::Present, "present")]
    #[case::data_none(StreamType::Data(DictionaryType::None), "data")]
    #[case::data_single(StreamType::Data(DictionaryType::Single), "data[single]")]
    #[case::data_shared(StreamType::Data(DictionaryType::Shared), "data[shared]")]
    #[case::data_vertex(StreamType::Data(DictionaryType::Vertex), "data[vertex]")]
    #[case::data_morton(StreamType::Data(DictionaryType::Morton), "data[morton]")]
    #[case::data_fsst(StreamType::Data(DictionaryType::Fsst), "data[fsst]")]
    #[case::offset_vertex(StreamType::Offset(OffsetType::Vertex), "offset[vertex]")]
    #[case::offset_index(StreamType::Offset(OffsetType::Index), "offset[index]")]
    #[case::offset_string(StreamType::Offset(OffsetType::String), "offset[string]")]
    #[case::offset_key(StreamType::Offset(OffsetType::Key), "offset[key]")]
    #[case::length_var_binary(StreamType::Length(LengthType::VarBinary), "length[var-binary]")]
    #[case::length_geometries(StreamType::Length(LengthType::Geometries), "length[geometries]")]
    #[case::length_parts(StreamType::Length(LengthType::Parts), "length[parts]")]
    #[case::length_rings(StreamType::Length(LengthType::Rings), "length[rings]")]
    #[case::length_triangles(StreamType::Length(LengthType::Triangles), "length[triangles]")]
    #[case::length_symbol(StreamType::Length(LengthType::Symbol), "length[symbol]")]
    #[case::length_dictionary(StreamType::Length(LengthType::Dictionary), "length[dictionary]")]
    fn every_stream_type_renders_its_wire_label(
        #[case] stream_type: StreamType,
        #[case] expected: &str,
    ) {
        assert_eq!(stream_type.to_string(), expected);
    }

    #[cfg(feature = "unstable-v2")]
    #[test]
    fn a_nested_length_stream_renders_its_wire_label() {
        assert_eq!(
            StreamType::Length(LengthType::Nested).to_string(),
            "length[nested]"
        );
    }

    #[rstest]
    #[case::int_none(LogicalEncoding::Int(IntLogical::None), "int/none")]
    #[case::int_delta(LogicalEncoding::Int(IntLogical::Delta), "int/delta")]
    #[case::int_rle(LogicalEncoding::Int(IntLogical::Rle(rle())), "int/rle")]
    #[case::int_delta_rle(LogicalEncoding::Int(IntLogical::DeltaRle(rle())), "int/delta-rle")]
    #[case::bool_none(LogicalEncoding::Bool(BoolLogical::None), "bool/none")]
    #[case::bool_byte_rle(LogicalEncoding::Bool(BoolLogical::ByteRle(rle())), "bool/byte-rle")]
    #[case::float_none(LogicalEncoding::Float(FloatLogical::None), "float/none")]
    #[case::float_dict(LogicalEncoding::Float(FloatLogical::Dict), "float/dict")]
    #[case::float_alp(LogicalEncoding::Float(FloatLogical::Alp(alp())), "float/alp")]
    #[case::vertex_none(LogicalEncoding::Vertex(VertexLogical::None), "vertex/none")]
    #[case::vertex_delta(LogicalEncoding::Vertex(VertexLogical::Delta), "vertex/delta")]
    #[case::vertex_componentwise_delta(
        LogicalEncoding::Vertex(VertexLogical::ComponentwiseDelta),
        "vertex/componentwise-delta"
    )]
    #[case::vertex_morton(LogicalEncoding::Vertex(VertexLogical::Morton(morton())), "vertex/morton")]
    #[case::vertex_morton_delta(
        LogicalEncoding::Vertex(VertexLogical::MortonDelta(morton())),
        "vertex/morton-delta"
    )]
    #[case::vertex_morton_rle(
        LogicalEncoding::Vertex(VertexLogical::MortonRle(morton())),
        "vertex/morton-rle"
    )]
    fn every_logical_encoding_renders_kind_then_encoding(
        #[case] encoding: LogicalEncoding,
        #[case] expected: &str,
    ) {
        assert_eq!(encoding.to_string(), expected);
    }

    #[rstest]
    #[case::none(PhysicalEncoding::None, "none")]
    #[case::varint(PhysicalEncoding::VarInt, "varint")]
    #[case::fastpfor_256be(
        PhysicalEncoding::FastPFor(FastPForKind::Block256Be),
        "fastpfor[256be]"
    )]
    fn every_physical_encoding_renders_its_wire_label(
        #[case] encoding: PhysicalEncoding,
        #[case] expected: &str,
    ) {
        assert_eq!(encoding.to_string(), expected);
    }

    #[cfg(feature = "unstable-v2")]
    #[rstest]
    #[case::fastpfor_128le(
        PhysicalEncoding::FastPFor(FastPForKind::Block128Le),
        "fastpfor[128le]"
    )]
    #[case::bit_packed(PhysicalEncoding::BitPacked, "bit-packed")]
    fn every_v2_physical_encoding_renders_its_wire_label(
        #[case] encoding: PhysicalEncoding,
        #[case] expected: &str,
    ) {
        assert_eq!(encoding.to_string(), expected);
    }

    #[rstest]
    #[case::int(LogicalEncoding::Int(IntLogical::None), ValueKind::Int)]
    #[case::bool(LogicalEncoding::Bool(BoolLogical::None), ValueKind::Bool)]
    #[case::float(LogicalEncoding::Float(FloatLogical::None), ValueKind::Float)]
    #[case::vertex(LogicalEncoding::Vertex(VertexLogical::None), ValueKind::Vertex)]
    fn the_identity_encoding_of_a_kind_reports_that_kind_back(
        #[case] expected: LogicalEncoding,
        #[case] kind: ValueKind,
    ) {
        let none = LogicalEncoding::none(kind);
        assert_eq!(none, expected);
        assert_eq!(none.kind(), kind);
        assert!(none.is_identity());
    }
}
