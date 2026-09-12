//! In-memory model types specific to tag `0x02` (v2) layers.

use num_enum::TryFromPrimitive;

use crate::{MltError, MltResult};

/// Data type of a v2 property column, the low nibble of the column type byte.
///
/// Unlike v1's [`super::ColumnType`] there are no `Opt` variants - nullability
/// lives in the high nibble, see [`Presence02`] - and geometry is not a column
/// (the layer's geometry section precedes the counted columns).
/// A string column spends one code, its layout living in the extension bits of
/// its leading stream's encoding byte, see
/// [`StrLayout`](super::stream::header02::StrLayout).
#[derive(Debug, Clone, Copy, PartialEq, Eq, TryFromPrimitive)]
#[repr(u8)]
pub(crate) enum DataType02 {
    Id = 0x00,
    LongId = 0x01,
    Bool = 0x02,
    I8 = 0x03,
    U8 = 0x04,
    I32 = 0x05,
    U32 = 0x06,
    I64 = 0x07,
    U64 = 0x08,
    F32 = 0x09,
    F64 = 0x0A,
    Str = 0x0B,
    Struct = 0x0C,
    List = 0x0D,
    Map = 0x0E,
}

impl DataType02 {
    /// Whether the column definition includes a name field.
    /// ID columns use implicit naming, same as v1.
    #[must_use]
    pub(crate) fn has_name(self) -> bool {
        !matches!(self, Self::Id | Self::LongId)
    }
}

/// The interior nodes, the three data types that hold other nodes rather than values.
///
/// Split from [`DataType02`] so a nested column's root, which one of these must be,
/// cannot be a scalar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Interior02 {
    Struct,
    List,
    Map,
}

impl Interior02 {
    /// Whether a node of this kind ends the implied counts, which a lengths stream does.
    #[must_use]
    pub(crate) fn has_lengths(self) -> bool {
        matches!(self, Self::List | Self::Map)
    }
}

impl From<Interior02> for DataType02 {
    fn from(interior: Interior02) -> Self {
        match interior {
            Interior02::Struct => Self::Struct,
            Interior02::List => Self::List,
            Interior02::Map => Self::Map,
        }
    }
}

/// Where a nested node's presence lives, the high nibble of its node type byte.
///
/// A node below a nested column's root cannot use the layer's shared bitfields,
/// which run over features, so only two nibbles are assigned.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NodePresence {
    /// Every value the parent hands this node is present and nothing is stored.
    AllPresent,
    /// A `Bool`-family presence stream follows the node type byte and the field name.
    Stream,
    /// As [`Self::AllPresent`], for a `Str` leaf whose codes index a corpus column.
    SharedAllPresent,
    /// As [`Self::Stream`], for a `Str` leaf whose codes index a corpus column.
    SharedStream,
}

impl NodePresence {
    /// Nibble of [`Self::AllPresent`], already shifted into place.
    const ALL_PRESENT: u8 = 0b0000_0000;

    /// Nibble of [`Self::Stream`], already shifted into place.
    const STREAM: u8 = 0b0001_0000;

    /// Nibble of [`Self::SharedAllPresent`], already shifted into place.
    const SHARED_ALL_PRESENT: u8 = 0b0010_0000;

    /// Nibble of [`Self::SharedStream`], already shifted into place.
    const SHARED_STREAM: u8 = 0b0011_0000;

    /// Nibble bit that says this node's children are shape-coded, already shifted into place.
    ///
    /// Bit `0b0010_0000` is spoken for by a leaf's shared-corpus index, so this sits above it.
    pub(crate) const SHAPES: u8 = 0b0100_0000;

    /// Whether the node's codes index a corpus column rather than its own dictionary.
    #[must_use]
    pub(crate) fn is_shared(self) -> bool {
        matches!(self, Self::SharedAllPresent | Self::SharedStream)
    }

    /// Whether a presence stream follows.
    #[must_use]
    pub(crate) fn has_stream(self) -> bool {
        matches!(self, Self::Stream | Self::SharedStream)
    }

    /// Read a masked nibble, or [`None`] for one this version has no meaning for.
    ///
    /// The shapes bit is a separate axis and is masked off by the caller.
    #[must_use]
    pub(crate) fn parse(nibble: u8) -> Option<Self> {
        match nibble & !Self::SHAPES {
            Self::ALL_PRESENT => Some(Self::AllPresent),
            Self::STREAM => Some(Self::Stream),
            Self::SHARED_ALL_PRESENT => Some(Self::SharedAllPresent),
            Self::SHARED_STREAM => Some(Self::SharedStream),
            _ => None,
        }
    }

    #[must_use]
    fn to_nibble(self) -> u8 {
        match self {
            Self::AllPresent => Self::ALL_PRESENT,
            Self::Stream => Self::STREAM,
            Self::SharedAllPresent => Self::SHARED_ALL_PRESENT,
            Self::SharedStream => Self::SHARED_STREAM,
        }
    }
}

/// What a nested node holds: a leaf's values, or more nodes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NodeKind02 {
    Leaf(ValueType02),
    Struct,
    List,
    Map,
}

impl NodeKind02 {
    /// The interior this kind names, or [`None`] for a leaf.
    #[must_use]
    pub(crate) fn interior(self) -> Option<Interior02> {
        match self {
            Self::Leaf(_) => None,
            Self::Struct => Some(Interior02::Struct),
            Self::List => Some(Interior02::List),
            Self::Map => Some(Interior02::Map),
        }
    }
}

impl From<NodeKind02> for DataType02 {
    fn from(kind: NodeKind02) -> Self {
        match kind {
            NodeKind02::Leaf(values) => values.into(),
            NodeKind02::Struct => Self::Struct,
            NodeKind02::List => Self::List,
            NodeKind02::Map => Self::Map,
        }
    }
}

/// The type byte every node below a nested column's root begins with:
/// [`NodePresence`] in bits 7-4, the data type in bits 3-0.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct NodeType02 {
    pub(crate) presence: NodePresence,
    /// Whether this node's children's structure is coded as one shape id per row.
    pub(crate) shapes: bool,
    pub(crate) data: NodeKind02,
}

impl NodeType02 {
    #[must_use]
    pub(crate) fn new(presence: NodePresence, data: NodeKind02) -> Self {
        Self {
            presence,
            shapes: false,
            data,
        }
    }

    /// The same node type with its children's structure coded as row shapes.
    #[must_use]
    pub(crate) fn shaped(mut self) -> Self {
        self.shapes = true;
        self
    }

    /// Read a wire byte, rejecting the nibbles a node cannot hold.
    pub(crate) fn parse(byte: u8) -> MltResult<Self> {
        let err = || MltError::ParsingColumnType(byte);
        let (nibble, data) = ColumnType02::fields(byte);
        let presence = NodePresence::parse(nibble).ok_or_else(err)?;
        let shapes = nibble & NodePresence::SHAPES != 0;
        let data = DataType02::try_from(data).map_err(|_| err())?;
        let data = match data {
            DataType02::Id | DataType02::LongId => return Err(err()),
            DataType02::Struct => NodeKind02::Struct,
            DataType02::List => NodeKind02::List,
            DataType02::Map => NodeKind02::Map,
            DataType02::Bool => NodeKind02::Leaf(ValueType02::Bool),
            DataType02::I8 => NodeKind02::Leaf(ValueType02::I8),
            DataType02::U8 => NodeKind02::Leaf(ValueType02::U8),
            DataType02::I32 => NodeKind02::Leaf(ValueType02::I32),
            DataType02::U32 => NodeKind02::Leaf(ValueType02::U32),
            DataType02::I64 => NodeKind02::Leaf(ValueType02::I64),
            DataType02::U64 => NodeKind02::Leaf(ValueType02::U64),
            DataType02::F32 => NodeKind02::Leaf(ValueType02::F32),
            DataType02::F64 => NodeKind02::Leaf(ValueType02::F64),
            DataType02::Str => NodeKind02::Leaf(ValueType02::Str),
        };
        // Only a string leaf holds codes, so only it can index a corpus.
        if presence.is_shared() && data != NodeKind02::Leaf(ValueType02::Str) {
            return Err(err());
        }
        Ok(Self {
            presence,
            shapes,
            data,
        })
    }

    #[must_use]
    pub(crate) fn to_byte(self) -> u8 {
        self.presence.to_nibble()
            | if self.shapes { NodePresence::SHAPES } else { 0 }
            | DataType02::from(self.data) as u8
    }
}

/// The type of the values a v2 column holds, the data types that name values.
///
/// Split from [`DataType02`] so a feature id, which is a feature's own rather
/// than one of its values, cannot stand in for a value type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ValueType02 {
    Bool,
    I8,
    U8,
    I32,
    U32,
    I64,
    U64,
    F32,
    F64,
    Str,
}

impl From<ValueType02> for DataType02 {
    fn from(values: ValueType02) -> Self {
        match values {
            ValueType02::Bool => Self::Bool,
            ValueType02::I8 => Self::I8,
            ValueType02::U8 => Self::U8,
            ValueType02::I32 => Self::I32,
            ValueType02::U32 => Self::U32,
            ValueType02::I64 => Self::I64,
            ValueType02::U64 => Self::U64,
            ValueType02::F32 => Self::F32,
            ValueType02::F64 => Self::F64,
            ValueType02::Str => Self::Str,
        }
    }
}

/// The width of a v2 id column, which is all its data type says beyond it being one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum IdWidth02 {
    Id32,
    Id64,
}

impl From<IdWidth02> for DataType02 {
    fn from(width: IdWidth02) -> Self {
        match width {
            IdWidth02::Id32 => Self::Id,
            IdWidth02::Id64 => Self::LongId,
        }
    }
}

/// How a shared dictionary stores its corpus, read from the high nibble of its column type byte.
///
/// A shared-dictionary column has no values of its own, so the nibble that names
/// [`Presence02`] elsewhere names the corpus encoding here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub(crate) enum SharedDictKind {
    /// Two streams: the entry lengths, then the entries.
    Plain = 0b0000_0000,
    /// Four streams: the entry lengths, the symbol lengths, the symbol table, then the corpus.
    Fsst = 0b0001_0000,
    /// As [`Self::Plain`], with no children of its own.
    CorpusPlain = 0b0010_0000,
    /// As [`Self::Fsst`], with no children of its own.
    CorpusFsst = 0b0011_0000,
}

impl SharedDictKind {
    /// Whether the column stores only a corpus, for nodes elsewhere to index.
    #[must_use]
    pub(crate) fn is_corpus_only(self) -> bool {
        matches!(self, Self::CorpusPlain | Self::CorpusFsst)
    }
}

impl SharedDictKind {
    /// Read a masked nibble, or [`None`] for one this version has no meaning for.
    #[must_use]
    pub(crate) fn parse(nibble: u8) -> Option<Self> {
        match nibble {
            0b0000_0000 => Some(Self::Plain),
            0b0001_0000 => Some(Self::Fsst),
            0b0010_0000 => Some(Self::CorpusPlain),
            0b0011_0000 => Some(Self::CorpusFsst),
            _ => None,
        }
    }
}

/// Where a v2 column's presence bitfield lives, the high nibble of the column
/// type byte.
///
/// Nibbles `0` and `1` describe a bitfield the column owns, `2..=8` point at one
/// of the layer's shared bitfields, and `9..=15` are reserved.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Presence02 {
    /// Every feature has a value and no bitfield is stored.
    AllPresent,
    /// A `ceil(feature_count/8)` byte bitfield follows the column name.
    Inline,
    /// The column reads the layer's shared presence bitfield at this index, so
    /// columns that are null on the same features store one bitfield between them.
    /// See [`LayerLayout::shared_presence`].
    Shared(u8),
}

impl Presence02 {
    /// Nibble of [`Self::AllPresent`], already shifted into place.
    const ALL_PRESENT: u8 = 0b0000_0000;

    /// Nibble of [`Self::Inline`], already shifted into place.
    const INLINE: u8 = 0b0001_0000;

    /// Nibble of `Shared(0)`; `Shared(i)` is this plus `i << 4`.
    const SHARED_BASE: u8 = 0b0010_0000;

    /// Read a masked presence nibble against a layer that stores `shared_count`
    /// shared bitfields.
    ///
    /// Returns [`None`] for a reserved nibble and for a reference past the last
    /// bitfield the layer declared - both are unreadable, so neither is worth
    /// distinguishing to the caller.
    #[must_use]
    pub(crate) fn parse(nibble: u8, shared_count: u8) -> Option<Self> {
        match nibble {
            Self::ALL_PRESENT => Some(Self::AllPresent),
            Self::INLINE => Some(Self::Inline),
            // Both arms above are below SHARED_BASE, so this cannot underflow.
            _ => {
                let index = (nibble - Self::SHARED_BASE) >> 4;
                (index < shared_count).then_some(Self::Shared(index))
            }
        }
    }

    /// Whether some features may be null, whichever bitfield holds the answer.
    #[must_use]
    pub(crate) fn is_optional(self) -> bool {
        !matches!(self, Self::AllPresent)
    }

    #[must_use]
    fn to_nibble(self) -> u8 {
        match self {
            Self::AllPresent => Self::ALL_PRESENT,
            Self::Inline => Self::INLINE,
            Self::Shared(index) => {
                debug_assert!(index < LayerLayout::MAX_SHARED_PRESENCE);
                Self::SHARED_BASE + (index << 4)
            }
        }
    }
}

/// The v2 column type byte: [`Presence02`] in bits 7-4, [`DataType02`] in bits 3-0.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ColumnType02 {
    pub(crate) presence: Presence02,
    pub(crate) data: DataType02,
}

impl ColumnType02 {
    /// Mask of the byte holding the [`Presence02`].
    const PRESENCE_MASK: u8 = 0b1111_0000;

    /// Mask of the byte holding the [`DataType02`].
    const DATA_TYPE_MASK: u8 = 0b0000_1111;

    #[must_use]
    pub(crate) fn new(presence: Presence02, data: DataType02) -> Self {
        Self { presence, data }
    }

    /// Split a wire byte into its two masked fields, without validating either.
    /// Values stay in place, so they compare directly against the enum variants.
    #[must_use]
    pub(crate) fn fields(byte: u8) -> (u8, u8) {
        (byte & Self::PRESENCE_MASK, byte & Self::DATA_TYPE_MASK)
    }

    /// Split a wire byte into its two fields, rejecting reserved bit patterns.
    ///
    /// `shared_count` is [`LayerLayout::shared_presence`] of the enclosing layer,
    /// so a column pointing past the last shared bitfield is rejected here rather
    /// than resolved to a missing one later.
    pub(crate) fn parse(byte: u8, shared_count: u8) -> MltResult<Self> {
        let err = || MltError::ParsingColumnType(byte);
        let (presence, data) = Self::fields(byte);
        let data = DataType02::try_from(data).map_err(|_| err())?;
        let presence = Presence02::parse(presence, shared_count).ok_or_else(err)?;
        Ok(Self { presence, data })
    }

    /// Read this column type as whichever of the two columns its data type names.
    #[must_use]
    pub(crate) fn split(self) -> ColumnKind02 {
        let values = |values| {
            ColumnKind02::Values(ValuesColumn02 {
                presence: self.presence,
                values,
            })
        };
        match self.data {
            DataType02::Id => ColumnKind02::Id(IdWidth02::Id32),
            DataType02::LongId => ColumnKind02::Id(IdWidth02::Id64),
            DataType02::Bool => values(ValueType02::Bool),
            DataType02::I8 => values(ValueType02::I8),
            DataType02::U8 => values(ValueType02::U8),
            DataType02::I32 => values(ValueType02::I32),
            DataType02::U32 => values(ValueType02::U32),
            DataType02::I64 => values(ValueType02::I64),
            DataType02::U64 => values(ValueType02::U64),
            DataType02::F32 => values(ValueType02::F32),
            DataType02::F64 => values(ValueType02::F64),
            DataType02::Str => values(ValueType02::Str),
            DataType02::Struct => ColumnKind02::Nested(NestedColumn02 {
                presence: self.presence,
                root: Interior02::Struct,
            }),
            DataType02::List => ColumnKind02::Nested(NestedColumn02 {
                presence: self.presence,
                root: Interior02::List,
            }),
            DataType02::Map => ColumnKind02::Nested(NestedColumn02 {
                presence: self.presence,
                root: Interior02::Map,
            }),
        }
    }

    #[must_use]
    pub(crate) fn to_byte(self) -> u8 {
        self.presence.to_nibble() | self.data as u8
    }
}

/// What a v2 column type byte names: a feature's id, or one column of its values.
///
/// The two read different fields after the byte - an id column has no name of its
/// own - so nothing but this split can name what follows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ColumnKind02 {
    Id(IdWidth02),
    Values(ValuesColumn02),
    /// The root of a nested column, whose body is a tree of nodes rather than a stream set.
    Nested(NestedColumn02),
}

/// A nested column's root: where its presence bitfield lives, and which interior it is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct NestedColumn02 {
    pub(crate) presence: Presence02,
    pub(crate) root: Interior02,
}

/// A v2 column of values: where its presence bitfield lives, and what its values are.
///
/// Holding one is proof the column is not an id, so the streams and containers a
/// column of values reads never have to consider one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ValuesColumn02 {
    pub(crate) presence: Presence02,
    pub(crate) values: ValueType02,
}

impl ValuesColumn02 {
    /// Read the type byte of an [m-value column](super::root02), which holds neither
    /// a feature id nor a shared dictionary.
    ///
    /// An id belongs to a feature rather than a vertex, and a shared dictionary
    /// introduces counted columns, which the m-value section does not hold.
    pub(crate) fn parse_m_value(byte: u8, shared_count: u8) -> MltResult<Self> {
        match ColumnType02::parse(byte, shared_count)?.split() {
            ColumnKind02::Values(column) => Ok(column),
            ColumnKind02::Id(_) | ColumnKind02::Nested(_) => Err(MltError::ParsingColumnType(byte)),
        }
    }
}

impl From<ValuesColumn02> for ColumnType02 {
    fn from(column: ValuesColumn02) -> Self {
        Self::new(column.presence, column.values.into())
    }
}

/// A v2 column type byte, read as whichever of the two column shapes its data type nibble names.
///
/// The two shapes read the high nibble differently, so nothing but this split can name it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Column02 {
    /// A column of values: [`Presence02`] in the high nibble, [`DataType02`] in the low.
    Values(ColumnType02),
    /// A shared dictionary and the columns indexing it: [`SharedDictKind`] in the high nibble.
    SharedDict(SharedDictKind),
}

impl Column02 {
    /// Data type nibble of [`Self::SharedDict`], the one value no [`DataType02`] takes.
    pub(crate) const SHARED_DICT: u8 = 0x0F;

    /// Read a wire byte in the terms of the shape its data type nibble names.
    ///
    /// `shared_count` is [`LayerLayout::shared_presence`] of the enclosing layer.
    pub(crate) fn parse(byte: u8, shared_count: u8) -> MltResult<Self> {
        let err = || MltError::ParsingColumnType(byte);
        let (high, data) = ColumnType02::fields(byte);
        if data == Self::SHARED_DICT {
            return SharedDictKind::parse(high)
                .map(Self::SharedDict)
                .ok_or_else(err);
        }
        ColumnType02::parse(byte, shared_count).map(Self::Values)
    }
}

/// v2 geometry section layout, the low nibble of the [`LayerLayout`] byte.
///
/// Selects which geometry streams are present and in what fixed order,
/// replacing v1's `stream_count` varint and per-stream `stream_type` bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, TryFromPrimitive, strum::IntoStaticStr)]
#[repr(u8)]
pub(crate) enum GeoLayout {
    /// `Types`, `Vertices`
    Points = 0x00,
    /// `Types`, `VertexData` (dict), `VertexOffsets`
    PointsDict = 0x01,
    /// `Types`, `GeoLengths`, `Vertices`
    MultiPoints = 0x02,
    /// `Types`, `GeoLengths`, `VertexData` (dict), `VertexOffsets`
    MultiPointsDict = 0x03,
    /// `Types`, `PartLengths`, `Vertices`
    Lines = 0x04,
    /// `Types`, `PartLengths`, `VertexData` (dict), `VertexOffsets`
    LinesDict = 0x05,
    /// `Types`, `GeoLengths`, `PartLengths`, `Vertices`
    MultiLines = 0x06,
    /// `Types`, `GeoLengths`, `PartLengths`, `VertexData` (dict), `VertexOffsets`
    MultiLinesDict = 0x07,
    /// `Types`, `PartLengths`, `RingLengths`, `Vertices`
    Polygons = 0x08,
    /// `Types`, `PartLengths`, `RingLengths`, `VertexData` (dict), `VertexOffsets`
    PolygonsDict = 0x09,
    /// `Types`, `GeoLengths`, `PartLengths`, `RingLengths`, `Vertices`
    MultiPolygons = 0x0A,
    /// `Types`, `GeoLengths`, `PartLengths`, `RingLengths`, `VertexData` (dict), `VertexOffsets`
    MultiPolygonsDict = 0x0B,
    /// `Types`, `TriLengths`, `IndexBuffer`, `Vertices`
    TessPolygons = 0x0C,
    /// `Types`, `GeoLengths`, `PartLengths`, `RingLengths`, `TriLengths`, `IndexBuffer`, `Vertices`
    TessPolygonsWithOutlines = 0x0D,
}

/// How a v2 geometry section stores its vertices, which [`GeoLayout`] names alongside the topology.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum VertexStorage {
    /// One entry per vertex, in a single stream.
    Plain,
    /// The distinct vertices once, then one index per vertex into them.
    Dict,
}

/// The nesting of a v2 geometry section below the geometry level, carrying the streams that name it.
///
/// A ring level with no part level above it is not a shape the geometry model can
/// produce, so it is not a shape this type can hold.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Topology {
    /// No nesting: a geometry addresses its vertices directly.
    Flat,
    /// One part per geometry, each addressing its vertices directly.
    Parts(Vec<u32>),
    /// One part per geometry, each addressing a run of rings.
    PartsAndRings { parts: Vec<u32>, rings: Vec<u32> },
}

impl Topology {
    /// The part and ring length streams, in wire order.
    pub(crate) fn streams(&self) -> (Option<&[u32]>, Option<&[u32]>) {
        match self {
            Self::Flat => (None, None),
            Self::Parts(parts) => (Some(parts), None),
            Self::PartsAndRings { parts, rings } => (Some(parts), Some(rings)),
        }
    }

    /// Widen to the full part-and-ring nesting, padding a level this layer never used.
    #[must_use]
    pub(crate) fn with_rings(self) -> Self {
        match self {
            Self::Flat => Self::PartsAndRings {
                parts: Vec::new(),
                rings: Vec::new(),
            },
            Self::Parts(parts) => Self::PartsAndRings {
                parts,
                rings: Vec::new(),
            },
            rings @ Self::PartsAndRings { .. } => rings,
        }
    }
}

impl GeoLayout {
    /// Layout for a stream set with plain or dictionary vertices.
    ///
    /// `has_geo` is independent of the nesting below it: any [`Topology`] can be
    /// addressed either per feature or per sub-geometry.
    #[must_use]
    pub(crate) fn from_topology(
        topology: &Topology,
        has_geo: bool,
        vertices: VertexStorage,
    ) -> Self {
        use VertexStorage as V;
        match (topology, has_geo, vertices) {
            (Topology::Flat, false, V::Plain) => Self::Points,
            (Topology::Flat, false, V::Dict) => Self::PointsDict,
            (Topology::Flat, true, V::Plain) => Self::MultiPoints,
            (Topology::Flat, true, V::Dict) => Self::MultiPointsDict,
            (Topology::Parts(_), false, V::Plain) => Self::Lines,
            (Topology::Parts(_), false, V::Dict) => Self::LinesDict,
            (Topology::Parts(_), true, V::Plain) => Self::MultiLines,
            (Topology::Parts(_), true, V::Dict) => Self::MultiLinesDict,
            (Topology::PartsAndRings { .. }, false, V::Plain) => Self::Polygons,
            (Topology::PartsAndRings { .. }, false, V::Dict) => Self::PolygonsDict,
            (Topology::PartsAndRings { .. }, true, V::Plain) => Self::MultiPolygons,
            (Topology::PartsAndRings { .. }, true, V::Dict) => Self::MultiPolygonsDict,
        }
    }

    /// Layout for a tessellated stream set, which keeps its vertices plain.
    ///
    /// Tessellation pairs with the full outline topology or with none of it, so a
    /// layer holding only part of it pads the streams it is missing with empty ones.
    #[must_use]
    pub(crate) fn tessellated(has_outlines: bool) -> Self {
        if has_outlines {
            Self::TessPolygonsWithOutlines
        } else {
            Self::TessPolygons
        }
    }

    #[must_use]
    pub(crate) fn has_geo_lengths(self) -> bool {
        matches!(
            self,
            Self::MultiPoints
                | Self::MultiPointsDict
                | Self::MultiLines
                | Self::MultiLinesDict
                | Self::MultiPolygons
                | Self::MultiPolygonsDict
                | Self::TessPolygonsWithOutlines
        )
    }

    #[must_use]
    pub(crate) fn has_part_lengths(self) -> bool {
        matches!(
            self,
            Self::Lines
                | Self::LinesDict
                | Self::MultiLines
                | Self::MultiLinesDict
                | Self::Polygons
                | Self::PolygonsDict
                | Self::MultiPolygons
                | Self::MultiPolygonsDict
                | Self::TessPolygonsWithOutlines
        )
    }

    #[must_use]
    pub(crate) fn has_ring_lengths(self) -> bool {
        matches!(
            self,
            Self::Polygons
                | Self::PolygonsDict
                | Self::MultiPolygons
                | Self::MultiPolygonsDict
                | Self::TessPolygonsWithOutlines
        )
    }

    /// Whether vertex data is stored as a dictionary + offsets pair.
    #[must_use]
    pub(crate) fn is_dict(self) -> bool {
        matches!(
            self,
            Self::PointsDict
                | Self::MultiPointsDict
                | Self::LinesDict
                | Self::MultiLinesDict
                | Self::PolygonsDict
                | Self::MultiPolygonsDict
        )
    }

    /// Whether the topology gives every feature's vertex count, which an
    /// [m-value column](super::root02) needs to read its values back per feature.
    ///
    /// A point layer holds one vertex per feature, so a vertex-scoped column there
    /// is a property column. A tessellated layer without outlines has no per-feature
    /// vertex count at all.
    #[must_use]
    pub(crate) fn allows_m_values(self) -> bool {
        !matches!(self, Self::Points | Self::PointsDict | Self::TessPolygons)
    }

    /// Whether tessellation streams (`TriLengths`, `IndexBuffer`) are present.
    #[must_use]
    pub(crate) fn is_tess(self) -> bool {
        matches!(self, Self::TessPolygons | Self::TessPolygonsWithOutlines)
    }
}

/// The v2 layer layout byte: an m-value flag in bit 7, shared presence bitfield
/// count in bits 6-4, [`GeoLayout`] in bits 3-0.
///
/// It describes the layer as a whole and sits at the layer root, right after the
/// header, so its spare bits are available to sections other than geometry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct LayerLayout {
    /// Whether an m-value section ends the layer body, after the counted columns.
    pub(crate) m_values: bool,
    /// How many shared presence bitfields the layer stores, at most
    /// [`Self::MAX_SHARED_PRESENCE`].
    ///
    /// The bitfields themselves follow this byte immediately, before the geometry
    /// section: one `ceil(feature_count/8)` byte LSB-first bitfield each, in index
    /// order. Columns read one through their [`Presence02::Shared`] nibble, so a
    /// set of columns that are null on the same features pays for one bitfield
    /// rather than one each.
    pub(crate) shared_presence: u8,
    pub(crate) geometry: GeoLayout,
}

impl LayerLayout {
    /// Mask of the bit saying an [m-value section](super::root02) ends the body.
    const M_VALUES_MASK: u8 = 0b1000_0000;

    /// Mask of the byte holding the shared presence column count.
    const SHARED_PRESENCE_MASK: u8 = 0b0111_0000;

    /// Mask of the byte holding the [`GeoLayout`].
    const GEO_LAYOUT_MASK: u8 = 0b0000_1111;

    /// Largest shared presence column count the byte can express.
    /// The 8th value is spent on the m-value flag in bit 7.
    pub(crate) const MAX_SHARED_PRESENCE: u8 = Self::SHARED_PRESENCE_MASK >> 4;

    #[must_use]
    pub(crate) fn new(geometry: GeoLayout, shared_presence: u8, m_values: bool) -> Self {
        debug_assert!(shared_presence <= Self::MAX_SHARED_PRESENCE);
        Self {
            m_values,
            shared_presence,
            geometry,
        }
    }

    /// Split a wire byte into its three fields, without validating any of them.
    /// The m-value flag stays in place, the other two are shifted down.
    #[must_use]
    pub(crate) fn fields(byte: u8) -> (u8, u8, u8) {
        (
            byte & Self::M_VALUES_MASK,
            (byte & Self::SHARED_PRESENCE_MASK) >> 4,
            byte & Self::GEO_LAYOUT_MASK,
        )
    }

    /// Split a wire byte into its three fields, rejecting reserved bit patterns.
    pub(crate) fn parse(byte: u8) -> MltResult<Self> {
        let (m_values, shared_presence, geometry) = Self::fields(byte);
        let geometry =
            GeoLayout::try_from(geometry).map_err(|_| MltError::ParsingGeoLayout(geometry))?;
        Ok(Self {
            m_values: m_values != 0,
            shared_presence,
            geometry,
        })
    }

    #[must_use]
    pub(crate) fn to_byte(self) -> u8 {
        debug_assert!(self.shared_presence <= Self::MAX_SHARED_PRESENCE);
        let m_values = if self.m_values {
            Self::M_VALUES_MASK
        } else {
            0
        };
        m_values | (self.shared_presence << 4) | self.geometry as u8
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    /// Shared bitfield count of a layer that declares as many as the byte allows.
    const ALL_SHARED: u8 = LayerLayout::MAX_SHARED_PRESENCE;

    fn parts_and_rings() -> Topology {
        Topology::PartsAndRings {
            parts: vec![1],
            rings: vec![4],
        }
    }

    #[rstest]
    #[case::id(0b0000_0000, Presence02::AllPresent, DataType02::Id)]
    #[case::opt_id(0b0001_0000, Presence02::Inline, DataType02::Id)]
    #[case::i32(0b0000_0101, Presence02::AllPresent, DataType02::I32)]
    #[case::opt_f64(0b0001_1010, Presence02::Inline, DataType02::F64)]
    #[case::str(0b0000_1011, Presence02::AllPresent, DataType02::Str)]
    #[case::first_shared(0b0010_0101, Presence02::Shared(0), DataType02::I32)]
    #[case::last_shared(0b1000_1010, Presence02::Shared(6), DataType02::F64)]
    fn column_type_byte_roundtrip(
        #[case] byte: u8,
        #[case] presence: Presence02,
        #[case] data: DataType02,
    ) {
        let typ = ColumnType02::parse(byte, ALL_SHARED).unwrap();
        assert_eq!(typ, ColumnType02::new(presence, data));
        assert_eq!(typ.to_byte(), byte);
    }

    #[rstest]
    #[case::values(
        0b0001_0101,
        Column02::Values(ColumnType02::new(Presence02::Inline, DataType02::I32))
    )]
    #[case::plain_shared_dict(0b0000_1111, Column02::SharedDict(SharedDictKind::Plain))]
    #[case::fsst_shared_dict(0b0001_1111, Column02::SharedDict(SharedDictKind::Fsst))]
    #[case::plain_corpus(0b0010_1111, Column02::SharedDict(SharedDictKind::CorpusPlain))]
    #[case::fsst_corpus(0b0011_1111, Column02::SharedDict(SharedDictKind::CorpusFsst))]
    fn column_byte_names_the_shape_it_holds(#[case] byte: u8, #[case] column: Column02) {
        assert_eq!(Column02::parse(byte, ALL_SHARED).unwrap(), column);
    }

    #[rstest]
    #[case::reserved_shared_dict_kind(0b0100_1111)]
    #[case::reserved_presence_over_a_nested_root(0b1001_1100)]
    fn column_byte_rejects_unassigned(#[case] byte: u8) {
        let err = Column02::parse(byte, ALL_SHARED).unwrap_err();
        assert!(matches!(err, MltError::ParsingColumnType(b) if b == byte));
    }

    #[rstest]
    #[case::shared_dict_is_not_a_data_type(0b0000_1111, ALL_SHARED)]
    #[case::reserved_presence_over_a_nested_root(0b1001_1100, ALL_SHARED)]
    #[case::reserved_presence(0b1001_0101, ALL_SHARED)]
    #[case::reserved_presence_top(0b1111_0101, ALL_SHARED)]
    #[case::shared_ref_without_shared_columns(0b0010_0101, 0)]
    #[case::shared_ref_past_declared_count(0b0100_0101, 1)]
    fn column_type_byte_rejects_unassigned(#[case] byte: u8, #[case] shared_count: u8) {
        let err = ColumnType02::parse(byte, shared_count).unwrap_err();
        assert!(matches!(err, MltError::ParsingColumnType(b) if b == byte));
    }

    #[rstest]
    #[case::points(Topology::Flat, false, VertexStorage::Plain, GeoLayout::Points)]
    #[case::points_dict(Topology::Flat, false, VertexStorage::Dict, GeoLayout::PointsDict)]
    #[case::multi_points(Topology::Flat, true, VertexStorage::Plain, GeoLayout::MultiPoints)]
    #[case::lines_dict(Topology::Parts(vec![2]), false, VertexStorage::Dict, GeoLayout::LinesDict)]
    #[case::multi_lines(Topology::Parts(vec![2]), true, VertexStorage::Plain, GeoLayout::MultiLines)]
    #[case::polygons(parts_and_rings(), false, VertexStorage::Plain, GeoLayout::Polygons)]
    #[case::multi_polygons_dict(
        parts_and_rings(),
        true,
        VertexStorage::Dict,
        GeoLayout::MultiPolygonsDict
    )]
    fn a_stream_set_picks_its_layout(
        #[case] topology: Topology,
        #[case] has_geo: bool,
        #[case] vertices: VertexStorage,
        #[case] expected: GeoLayout,
    ) {
        let layout = GeoLayout::from_topology(&topology, has_geo, vertices);
        let (parts, rings) = topology.streams();
        assert_eq!(layout, expected);
        assert_eq!(layout.has_geo_lengths(), has_geo);
        assert_eq!(layout.has_part_lengths(), parts.is_some());
        assert_eq!(layout.has_ring_lengths(), rings.is_some());
        assert_eq!(layout.is_dict(), vertices == VertexStorage::Dict);
        assert!(!layout.is_tess());
    }

    #[rstest]
    #[case::tess(false, GeoLayout::TessPolygons)]
    #[case::tess_with_outlines(true, GeoLayout::TessPolygonsWithOutlines)]
    fn a_tessellated_stream_set_picks_its_layout(
        #[case] has_outlines: bool,
        #[case] expected: GeoLayout,
    ) {
        let layout = GeoLayout::tessellated(has_outlines);
        assert_eq!(layout, expected);
        assert!(layout.is_tess());
        assert!(!layout.is_dict());
        assert_eq!(layout.has_geo_lengths(), has_outlines);
        assert_eq!(layout.has_part_lengths(), has_outlines);
        assert_eq!(layout.has_ring_lengths(), has_outlines);
    }

    #[rstest]
    #[case::flat(Topology::Flat)]
    #[case::parts(Topology::Parts(vec![2]))]
    #[case::parts_and_rings(parts_and_rings())]
    fn a_topology_widened_to_rings_keeps_both_streams(#[case] topology: Topology) {
        let widened = topology.with_rings();
        let (parts, rings) = widened.streams();
        assert!(parts.is_some() && rings.is_some());
    }

    #[rstest]
    #[case::points(0b0000_0000, 0, GeoLayout::Points, false)]
    #[case::multi_polygons(0b0000_1010, 0, GeoLayout::MultiPolygons, false)]
    #[case::one_shared_presence(0b0001_0100, 1, GeoLayout::Lines, false)]
    #[case::max_shared_presence(0b0111_0000, 7, GeoLayout::Points, false)]
    #[case::m_values(0b1000_0100, 0, GeoLayout::Lines, true)]
    #[case::m_values_with_shared(0b1010_1000, 2, GeoLayout::Polygons, true)]
    #[case::m_values_with_max_shared(0b1111_0110, 7, GeoLayout::MultiLines, true)]
    fn layer_layout_byte_roundtrip(
        #[case] byte: u8,
        #[case] shared_presence: u8,
        #[case] geometry: GeoLayout,
        #[case] m_values: bool,
    ) {
        let layout = LayerLayout::parse(byte).unwrap();
        assert_eq!(
            layout,
            LayerLayout::new(geometry, shared_presence, m_values)
        );
        assert_eq!(layout.to_byte(), byte);
    }

    #[rstest]
    #[case::bool(0b0000_0010, ValueType02::Bool)]
    #[case::opt_i32(0b0001_0101, ValueType02::I32)]
    #[case::shared_f64(0b0010_1010, ValueType02::F64)]
    #[case::str(0b0000_1011, ValueType02::Str)]
    fn m_value_type_byte_roundtrip(#[case] byte: u8, #[case] values: ValueType02) {
        let column = ValuesColumn02::parse_m_value(byte, ALL_SHARED).unwrap();
        assert_eq!(column.values, values);
        assert_eq!(ColumnType02::from(column).to_byte(), byte);
    }

    #[rstest]
    #[case::id(0b0000_0000)]
    #[case::opt_id(0b0001_0000)]
    #[case::long_id(0b0000_0001)]
    #[case::shared_dict(0b0000_1111)]
    #[case::struct_root(0b0000_1100)]
    #[case::list_root(0b0000_1101)]
    #[case::map_root(0b0000_1110)]
    #[case::reserved_presence(0b1001_0101)]
    fn m_value_type_byte_rejects_what_a_vertex_cannot_hold(#[case] byte: u8) {
        let err = ValuesColumn02::parse_m_value(byte, ALL_SHARED).unwrap_err();
        assert!(matches!(err, MltError::ParsingColumnType(b) if b == byte));
    }

    #[rstest]
    #[case::points(GeoLayout::Points, false)]
    #[case::points_dict(GeoLayout::PointsDict, false)]
    #[case::multi_points(GeoLayout::MultiPoints, true)]
    #[case::lines(GeoLayout::Lines, true)]
    #[case::multi_polygons_dict(GeoLayout::MultiPolygonsDict, true)]
    #[case::tess_polygons(GeoLayout::TessPolygons, false)]
    #[case::tess_polygons_with_outlines(GeoLayout::TessPolygonsWithOutlines, true)]
    fn only_a_layout_with_per_feature_vertex_counts_takes_m_values(
        #[case] layout: GeoLayout,
        #[case] allowed: bool,
    ) {
        assert_eq!(layout.allows_m_values(), allowed);
    }

    #[rstest]
    #[case::i32_leaf(
        0b0000_0101,
        NodePresence::AllPresent,
        NodeKind02::Leaf(ValueType02::I32)
    )]
    #[case::optional_str_leaf(
        0b0001_1011,
        NodePresence::Stream,
        NodeKind02::Leaf(ValueType02::Str)
    )]
    #[case::struct_node(0b0000_1100, NodePresence::AllPresent, NodeKind02::Struct)]
    #[case::optional_list_node(0b0001_1101, NodePresence::Stream, NodeKind02::List)]
    #[case::map_node(0b0000_1110, NodePresence::AllPresent, NodeKind02::Map)]
    #[case::shared_str_leaf(
        0b0010_1011,
        NodePresence::SharedAllPresent,
        NodeKind02::Leaf(ValueType02::Str)
    )]
    #[case::optional_shared_str_leaf(
        0b0011_1011,
        NodePresence::SharedStream,
        NodeKind02::Leaf(ValueType02::Str)
    )]
    fn node_type_byte_roundtrip(
        #[case] byte: u8,
        #[case] presence: NodePresence,
        #[case] data: NodeKind02,
    ) {
        let typ = NodeType02::parse(byte).unwrap();
        assert_eq!(typ, NodeType02::new(presence, data));
        assert_eq!(typ.to_byte(), byte);
    }

    #[rstest]
    #[case::shaped_struct(0b0100_1100, NodePresence::AllPresent, NodeKind02::Struct)]
    #[case::shaped_map_over_nulls(0b0101_1110, NodePresence::Stream, NodeKind02::Map)]
    fn the_shapes_bit_reads_beside_a_nodes_own_presence(
        #[case] byte: u8,
        #[case] presence: NodePresence,
        #[case] data: NodeKind02,
    ) {
        let typ = NodeType02::parse(byte).unwrap();
        assert_eq!(typ, NodeType02::new(presence, data).shaped());
        assert!(typ.shapes);
        assert_eq!(typ.to_byte(), byte);
    }

    #[rstest]
    #[case::id_is_a_features_own(0b0000_0000)]
    #[case::long_id_is_a_features_own(0b0000_0001)]
    #[case::shared_dict_introduces_columns(0b0000_1111)]
    #[case::reserved_node_presence(0b1000_0101)]
    #[case::reserved_node_presence_top(0b1111_0101)]
    #[case::shared_i32_leaf(0b0010_0101)]
    #[case::shared_struct_node(0b0011_1100)]
    fn node_type_byte_rejects_what_a_node_cannot_hold(#[case] byte: u8) {
        let err = NodeType02::parse(byte).unwrap_err();
        assert!(matches!(err, MltError::ParsingColumnType(b) if b == byte));
    }

    #[rstest]
    #[case::struct_root(0b0000_1100, Interior02::Struct)]
    #[case::list_root(0b0001_1101, Interior02::List)]
    #[case::map_root(0b0010_1110, Interior02::Map)]
    fn a_nested_root_reads_as_the_interior_its_nibble_names(
        #[case] byte: u8,
        #[case] root: Interior02,
    ) {
        let typ = ColumnType02::parse(byte, ALL_SHARED).unwrap();
        let ColumnKind02::Nested(column) = typ.split() else {
            panic!("expected a nested root")
        };
        assert_eq!(column.root, root);
        assert_eq!(typ.to_byte(), byte);
    }

    #[rstest]
    #[case::unassigned_geo_layout(0b0000_1110)]
    #[case::unassigned_geo_layout_with_shared(0b0010_1111)]
    fn layer_layout_byte_rejects_unassigned_geo_layout(#[case] byte: u8) {
        let err = LayerLayout::parse(byte).unwrap_err();
        assert!(
            matches!(err, MltError::ParsingGeoLayout(b) if b == byte & LayerLayout::GEO_LAYOUT_MASK)
        );
    }
}
