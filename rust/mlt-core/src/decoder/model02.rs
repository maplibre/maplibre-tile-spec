//! In-memory model types specific to tag `0x02` (v2) layers.

use num_enum::TryFromPrimitive;

use crate::{MltError, MltResult};

/// Mask of the column type byte holding the [`DataType02`].
const DATA_TYPE_MASK: u8 = 0x0F;

/// Mask of the column type byte holding the [`Presence02`].
const PRESENCE_MASK: u8 = 0xF0;

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
    //           its own presence bitfield, so the presence nibble is free here
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
/// Variants are already shifted into place, so the byte is matched with
/// `byte & PRESENCE_MASK` rather than shifted down first.
#[derive(Debug, Clone, Copy, PartialEq, Eq, TryFromPrimitive)]
#[repr(u8)]
pub(crate) enum Presence02 {
    /// Every feature has a value and no bitfield is stored.
    AllPresent = 0x00,
    /// A `ceil(feature_count/8)` byte bitfield follows the column name.
    Inline = 0x10,
    // TODO(v2): 0x20..=0xF0: reference to shared presence column `(nibble >> 4) - 2`,
    //           so columns that are null on the same features store one bitfield
    //           between them (not yet implemented).
}

impl Presence02 {
    /// Whether a presence bitfield follows the column name.
    #[must_use]
    pub(crate) fn is_inline(self) -> bool {
        matches!(self, Self::Inline)
    }
}

/// The v2 column type byte: [`Presence02`] in bits 7-4, [`DataType02`] in bits 3-0.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ColumnType02 {
    pub(crate) presence: Presence02,
    pub(crate) data: DataType02,
}

impl ColumnType02 {
    #[must_use]
    pub(crate) fn new(presence: Presence02, data: DataType02) -> Self {
        Self { presence, data }
    }

    /// Split a wire byte into its two masked fields, without validating either.
    /// Values stay in place, so they compare directly against the enum variants.
    #[must_use]
    pub(crate) fn fields(byte: u8) -> (u8, u8) {
        (byte & PRESENCE_MASK, byte & DATA_TYPE_MASK)
    }

    /// Split a wire byte into its two fields, rejecting reserved bit patterns.
    pub(crate) fn parse(byte: u8) -> MltResult<Self> {
        let err = || MltError::ParsingColumnType(byte);
        let (presence, data) = Self::fields(byte);
        let presence = Presence02::try_from(presence).map_err(|_| err())?;
        let data = DataType02::try_from(data).map_err(|_| err())?;
        Ok(Self { presence, data })
    }

    #[must_use]
    pub(crate) fn to_byte(self) -> u8 {
        self.presence as u8 | self.data as u8
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

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case::id(0x00, Presence02::AllPresent, DataType02::Id)]
    #[case::opt_id(0x10, Presence02::Inline, DataType02::Id)]
    #[case::i32(0x05, Presence02::AllPresent, DataType02::I32)]
    #[case::opt_f64(0x1A, Presence02::Inline, DataType02::F64)]
    fn column_type_byte_roundtrip(
        #[case] byte: u8,
        #[case] presence: Presence02,
        #[case] data: DataType02,
    ) {
        let typ = ColumnType02::parse(byte).unwrap();
        assert_eq!(typ, ColumnType02::new(presence, data));
        assert_eq!(typ.to_byte(), byte);
    }

    #[rstest]
    #[case::reserved_data_type(0x0F)]
    #[case::unassigned_data_type(0x0B)]
    #[case::shared_presence_ref(0x25)]
    #[case::reserved_presence(0xF5)]
    fn column_type_byte_rejects_unassigned(#[case] byte: u8) {
        let err = ColumnType02::parse(byte).unwrap_err();
        assert!(matches!(err, MltError::ParsingColumnType(b) if b == byte));
    }
}
