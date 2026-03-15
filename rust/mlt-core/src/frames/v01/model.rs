use geo_types::Geometry as GeoGeometry;
use num_enum::TryFromPrimitive;

use crate::v01::{
    EncodedGeometry, EncodedId, EncodedProperty, Geometry, GeometryValues, Id, IdValues, Property,
    StagedProperty,
};

/// Column definition
#[derive(Debug, PartialEq)]
pub struct Column<'a> {
    pub typ: ColumnType,
    pub name: Option<&'a str>,
    pub children: Vec<Column<'a>>,
}

/// Owned variant of [`Column`].
#[derive(Debug, PartialEq, Clone)]
pub struct OwnedColumn {
    pub typ: ColumnType,
    pub name: Option<String>,
    pub children: Vec<OwnedColumn>,
}

/// Column data type, as stored in the tile
#[derive(Debug, Clone, Copy, PartialEq, TryFromPrimitive)]
#[repr(u8)]
pub enum ColumnType {
    Id = 0,
    OptId = 1,
    LongId = 2,
    OptLongId = 3,
    Geometry = 4,
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
    Str = 28,
    OptStr = 29,
    SharedDict = 30,
}

/// Representation of a feature table layer encoded as MLT tag `0x01`
#[derive(Debug, PartialEq)]
pub struct Layer01<'a> {
    pub name: &'a str,
    pub extent: u32,
    pub id: Option<Id<'a>>,
    pub geometry: Geometry<'a>,
    pub properties: Vec<Property<'a>>,
    #[cfg(fuzzing)]
    pub layer_order: Vec<crate::frames::v01::fuzzing::LayerOrdering>,
}

/// Columnar layer data being prepared for encoding (stage 2 of the encoding pipeline).
///
/// Holds fully-owned columnar data. Constructed directly (synthetics, benches) or
/// converted from [`TileLayer01`].
/// Consumed by encoding to produce [`EncodedLayer01`].
#[derive(Debug, PartialEq, Clone)]
#[cfg_attr(all(not(test), feature = "arbitrary"), derive(arbitrary::Arbitrary))]
pub struct StagedLayer01 {
    pub name: String,
    pub extent: u32,
    pub id: Option<IdValues>,
    pub geometry: GeometryValues,
    pub properties: Vec<StagedProperty>,
}

/// Wire-ready layer data (stage 3 of the encoding pipeline).
///
/// Produced by encoding a [`StagedLayer01`]. Can be serialised directly to bytes
/// via [`EncodedLayer01::write_to`].
#[derive(Debug, PartialEq, Clone)]
pub struct EncodedLayer01 {
    pub name: String,
    pub extent: u32,
    pub id: Option<EncodedId>,
    pub geometry: EncodedGeometry,
    pub properties: Vec<EncodedProperty>,
    #[cfg(fuzzing)]
    pub layer_order: Vec<crate::frames::v01::fuzzing::LayerOrdering>,
}

/// Row-oriented working form for the optimizer.
///
/// All features are stored as a flat [`Vec<TileFeature>`] so that sorting is
/// a single `sort_by_cached_key` call.  The `property_names` vec is parallel
/// to every `TileFeature::properties` slice in this layer.
#[derive(Debug, Clone)]
pub struct TileLayer01 {
    pub name: String,
    pub extent: u32,
    /// Column names, parallel to `TileFeature::properties`.
    pub property_names: Vec<String>,
    pub features: Vec<TileFeature>,
}

/// A single map feature in row form.
#[derive(Debug, Clone, PartialEq)]
pub struct TileFeature {
    pub id: Option<u64>,
    /// Geometry in `geo_types` / `Geom32` form.
    pub geometry: GeoGeometry<i32>,
    /// One value per property column, in the same order as
    /// [`TileLayer01::property_names`].
    pub properties: Vec<PropValue>,
}

/// A single typed value for one property of one feature.
///
/// Mirrors the scalar variants of `ParsedProperty` at the per-feature
/// level. `SharedDict` items are flattened: each sub-field becomes its own
/// `PropValue::Str` entry in `TileFeature::properties`, with the
/// corresponding entry in `TileLayer01::property_names` set to
/// `"prefix:suffix"`.
#[derive(Debug, Clone, PartialEq)]
pub enum PropValue {
    Bool(Option<bool>),
    I8(Option<i8>),
    U8(Option<u8>),
    I32(Option<i32>),
    U32(Option<u32>),
    I64(Option<i64>),
    U64(Option<u64>),
    F32(Option<f32>),
    F64(Option<f64>),
    Str(Option<String>),
}
