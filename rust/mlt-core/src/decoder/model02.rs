//! In-memory model types specific to tag `0x02` (v2) layers.
//!
//! The numeric code assignments below are **provisional** — the v2 spec
//! (`docs/migrating-to-v2.md`) intentionally defers them. Scalar and ID codes
//! mirror v1 for clarity; string codes (28–35), shared-dict codes (36–39), and
//! shared-presence variants (`base | 0x80`) will be added with those features.

use num_enum::TryFromPrimitive;

/// Column data type of a v2 property column, as stored in the tile.
///
/// Unlike v1's [`super::ColumnType`], geometry is not a column (the layer's
/// geometry section precedes the counted columns), and each string encoding
/// variant will get its own flat code instead of a runtime `stream_count`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, TryFromPrimitive)]
#[repr(u8)]
pub(crate) enum ColumnType02 {
    Id = 0,
    OptId = 1,
    LongId = 2,
    OptLongId = 3,
    Bool = 10,
    OptBool = 11,
    I8 = 12,
    OptI8 = 13,
    U8 = 14,
    OptU8 = 15,
    I32 = 16,
    OptI32 = 17,
    U32 = 18,
    OptU32 = 19,
    I64 = 20,
    OptI64 = 21,
    U64 = 22,
    OptU64 = 23,
    F32 = 24,
    OptF32 = 25,
    F64 = 26,
    OptF64 = 27,
    // 28..=35: StrPlain / OptStrPlain / StrDict / OptStrDict / StrFsst /
    //          OptStrFsst / StrFsstDict / OptStrFsstDict   (not yet implemented)
    // 36..=39: SharedDictPlain / SharedDictFsst / SharedDictChildRef /
    //          OptSharedDictChildRef                        (not yet implemented)
}

impl ColumnType02 {
    /// Whether the column definition includes a name field.
    /// ID columns use implicit naming, same as v1.
    #[must_use]
    pub(crate) fn has_name(self) -> bool {
        !matches!(
            self,
            Self::Id | Self::OptId | Self::LongId | Self::OptLongId
        )
    }

    /// Whether a presence bitfield follows the column name.
    #[must_use]
    pub(crate) fn is_optional(self) -> bool {
        (self as u8) & 1 != 0
    }
}

/// v2 geometry section layout, the first byte of the geometry section.
///
/// Selects which geometry streams are present and in what fixed order,
/// replacing v1's `stream_count` varint and per-stream `stream_type` bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, TryFromPrimitive)]
#[repr(u8)]
pub(crate) enum GeoLayout {
    /// `Types`, `Vertices`
    Points = 0,
    /// `Types`, `VertexData` (dict), `VertexOffsets`
    PointsDict = 1,
    /// `Types`, `GeoLengths`, `Vertices`
    MultiPoints = 2,
    /// `Types`, `GeoLengths`, `VertexData` (dict), `VertexOffsets`
    MultiPointsDict = 3,
    /// `Types`, `PartLengths`, `Vertices`
    Lines = 4,
    /// `Types`, `PartLengths`, `VertexData` (dict), `VertexOffsets`
    LinesDict = 5,
    /// `Types`, `GeoLengths`, `PartLengths`, `Vertices`
    MultiLines = 6,
    /// `Types`, `GeoLengths`, `PartLengths`, `VertexData` (dict), `VertexOffsets`
    MultiLinesDict = 7,
    /// `Types`, `PartLengths`, `RingLengths`, `Vertices`
    Polygons = 8,
    /// `Types`, `PartLengths`, `RingLengths`, `VertexData` (dict), `VertexOffsets`
    PolygonsDict = 9,
    /// `Types`, `GeoLengths`, `PartLengths`, `RingLengths`, `Vertices`
    MultiPolygons = 10,
    /// `Types`, `GeoLengths`, `PartLengths`, `RingLengths`, `VertexData` (dict), `VertexOffsets`
    MultiPolygonsDict = 11,
    /// `Types`, `TriLengths`, `IndexBuffer`, `Vertices`
    TessPolygons = 12,
    /// `Types`, `GeoLengths`, `PartLengths`, `RingLengths`, `TriLengths`, `IndexBuffer`, `Vertices`
    TessPolygonsWithOutlines = 13,
}

impl GeoLayout {
    /// Layout for a plain (non-dict, non-tessellated) stream set.
    ///
    /// The stream set is produced by the same topology encoding as v1, where
    /// empty length streams are skipped; every reachable combination maps to a
    /// layout. `ring` without `part` cannot occur structurally.
    pub(crate) fn from_streams(geo: bool, part: bool, ring: bool) -> crate::MltResult<Self> {
        Ok(match (geo, part, ring) {
            (false, false, false) => Self::Points,
            (true, false, false) => Self::MultiPoints,
            (false, true, false) => Self::Lines,
            (true, true, false) => Self::MultiLines,
            (false, true, true) => Self::Polygons,
            (true, true, true) => Self::MultiPolygons,
            (_, false, true) => Err(crate::MltError::NotImplemented(
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
