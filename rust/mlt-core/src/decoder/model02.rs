//! In-memory model types specific to tag `0x02` (v2) layers.

use num_enum::TryFromPrimitive;

use crate::{MltError, MltResult};

/// Data type of a v2 property column, the low nibble of the column type byte.
///
/// Unlike v1's [`super::ColumnType`] there are no `Opt` variants - nullability
/// lives in the high nibble, see [`Presence02`] - and geometry is not a column
/// (the layer's geometry section precedes the counted columns).
/// Each string encoding variant gets its own flat code instead of v1's runtime
/// `stream_count`.
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
    // TODO(v2): 0x0B..=0x0E: StrPlain / StrDict / StrFsst / StrFsstDict
    //           (not yet implemented)
    // TODO(v2): 0x0F: shared dictionary escape. Each of its sub-columns carries
    //           its own presence nibble, so the column's own nibble is free here
    //           and names the shared dictionary kind instead (plain / FSST / child reference).
}

impl DataType02 {
    /// Whether the column definition includes a name field.
    /// ID columns use implicit naming, same as v1.
    #[must_use]
    pub(crate) fn has_name(self) -> bool {
        !matches!(self, Self::Id | Self::LongId)
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
        let presence = Presence02::parse(presence, shared_count).ok_or_else(err)?;
        let data = DataType02::try_from(data).map_err(|_| err())?;
        Ok(Self { presence, data })
    }

    #[must_use]
    pub(crate) fn to_byte(self) -> u8 {
        self.presence.to_nibble() | self.data as u8
    }
}

/// v2 geometry section layout, the low nibble of the [`LayerLayout`] byte.
///
/// Selects which geometry streams are present and in what fixed order,
/// replacing v1's `stream_count` varint and per-stream `stream_type` bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, TryFromPrimitive)]
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

impl GeoLayout {
    /// Layout for a plain (non-dict, non-tessellated) stream set.
    ///
    /// The stream set is produced by the same topology encoding as v1, where
    /// empty length streams are skipped; every reachable combination maps to a
    /// layout. `ring` without `part` cannot occur structurally.
    pub(crate) fn from_streams(geo: bool, part: bool, ring: bool) -> MltResult<Self> {
        Ok(match (geo, part, ring) {
            (false, false, false) => Self::Points,
            (true, false, false) => Self::MultiPoints,
            (false, true, false) => Self::Lines,
            (true, true, false) => Self::MultiLines,
            (false, true, true) => Self::Polygons,
            (true, true, true) => Self::MultiPolygons,
            (_, false, true) => Err(MltError::NotImplemented(
                "v2 geometry: ring lengths without part lengths",
            ))?,
        })
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

    /// Whether tessellation streams (`TriLengths`, `IndexBuffer`) are present.
    #[must_use]
    pub(crate) fn is_tess(self) -> bool {
        matches!(self, Self::TessPolygons | Self::TessPolygonsWithOutlines)
    }
}

/// The v2 layer layout byte: reserved in bit 7, shared presence bitfield count in
/// bits 6-4, [`GeoLayout`] in bits 3-0.
///
/// It describes the layer as a whole and sits at the layer root, right after the
/// header, so its spare bits are available to sections other than geometry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct LayerLayout {
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
    /// Mask of the byte held in reserve for a future layer-wide flag.
    const RESERVED_MASK: u8 = 0b1000_0000;

    /// Mask of the byte holding the shared presence column count.
    const SHARED_PRESENCE_MASK: u8 = 0b0111_0000;

    /// Mask of the byte holding the [`GeoLayout`].
    const GEO_LAYOUT_MASK: u8 = 0b0000_1111;

    /// Largest shared presence column count the byte can express.
    /// The 8th value is spent on keeping bit 7 free for a future flag.
    pub(crate) const MAX_SHARED_PRESENCE: u8 = Self::SHARED_PRESENCE_MASK >> 4;

    #[must_use]
    pub(crate) fn new(geometry: GeoLayout, shared_presence: u8) -> Self {
        debug_assert!(shared_presence <= Self::MAX_SHARED_PRESENCE);
        Self {
            shared_presence,
            geometry,
        }
    }

    /// Split a wire byte into its three fields, without validating any of them.
    /// The reserved bit stays in place, the other two are shifted down.
    #[must_use]
    pub(crate) fn fields(byte: u8) -> (u8, u8, u8) {
        (
            byte & Self::RESERVED_MASK,
            (byte & Self::SHARED_PRESENCE_MASK) >> 4,
            byte & Self::GEO_LAYOUT_MASK,
        )
    }

    /// Split a wire byte into its three fields, rejecting reserved bit patterns.
    pub(crate) fn parse(byte: u8) -> MltResult<Self> {
        let (reserved, shared_presence, geometry) = Self::fields(byte);
        if reserved != 0 {
            return Err(MltError::ParsingLayerLayout(byte));
        }
        let geometry =
            GeoLayout::try_from(geometry).map_err(|_| MltError::ParsingGeoLayout(geometry))?;
        Ok(Self {
            shared_presence,
            geometry,
        })
    }

    #[must_use]
    pub(crate) fn to_byte(self) -> u8 {
        debug_assert!(self.shared_presence <= Self::MAX_SHARED_PRESENCE);
        (self.shared_presence << 4) | self.geometry as u8
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    /// Shared bitfield count of a layer that declares as many as the byte allows.
    const ALL_SHARED: u8 = LayerLayout::MAX_SHARED_PRESENCE;

    #[rstest]
    #[case::id(0b0000_0000, Presence02::AllPresent, DataType02::Id)]
    #[case::opt_id(0b0001_0000, Presence02::Inline, DataType02::Id)]
    #[case::i32(0b0000_0101, Presence02::AllPresent, DataType02::I32)]
    #[case::opt_f64(0b0001_1010, Presence02::Inline, DataType02::F64)]
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
    #[case::reserved_data_type(0b0000_1111, ALL_SHARED)]
    #[case::unassigned_data_type(0b0000_1011, ALL_SHARED)]
    #[case::reserved_presence(0b1001_0101, ALL_SHARED)]
    #[case::reserved_presence_top(0b1111_0101, ALL_SHARED)]
    #[case::shared_ref_without_shared_columns(0b0010_0101, 0)]
    #[case::shared_ref_past_declared_count(0b0100_0101, 1)]
    fn column_type_byte_rejects_unassigned(#[case] byte: u8, #[case] shared_count: u8) {
        let err = ColumnType02::parse(byte, shared_count).unwrap_err();
        assert!(matches!(err, MltError::ParsingColumnType(b) if b == byte));
    }

    #[rstest]
    #[case::points(0b0000_0000, 0, GeoLayout::Points)]
    #[case::multi_polygons(0b0000_1010, 0, GeoLayout::MultiPolygons)]
    #[case::one_shared_presence(0b0001_0100, 1, GeoLayout::Lines)]
    #[case::max_shared_presence(0b0111_0000, 7, GeoLayout::Points)]
    fn layer_layout_byte_roundtrip(
        #[case] byte: u8,
        #[case] shared_presence: u8,
        #[case] geometry: GeoLayout,
    ) {
        let layout = LayerLayout::parse(byte).unwrap();
        assert_eq!(layout, LayerLayout::new(geometry, shared_presence));
        assert_eq!(layout.to_byte(), byte);
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

    #[rstest]
    #[case::reserved_bit(0b1000_0000)]
    #[case::reserved_bit_with_shared(0b1111_0000)]
    fn layer_layout_byte_rejects_reserved_bit(#[case] byte: u8) {
        let err = LayerLayout::parse(byte).unwrap_err();
        assert!(matches!(err, MltError::ParsingLayerLayout(b) if b == byte));
    }
}
