use num_enum::TryFromPrimitive;

use crate::geojson::Geom32;
use crate::v01::{Geometry, Id, Property};
use crate::{DecodeState, Lazy, Parsed};

/// Column definition
#[derive(Debug, PartialEq)]
pub struct Column<'a> {
    pub(crate) typ: ColumnType,
    pub(crate) name: Option<&'a str>,
    pub(crate) children: Vec<Self>,
}

/// Owned variant of [`Column`].
#[derive(Debug, PartialEq, Clone)]
pub struct OwnedColumn {
    pub(crate) typ: ColumnType,
    pub(crate) name: Option<String>,
    pub(crate) children: Vec<Self>,
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

/// Representation of an MLT feature table layer with tag `0x01` during decoding.
///
/// The type parameter `S` controls how columns are stored:
///
/// - `Layer01<'a>` / `Layer01<'a, Lazy>` (default) — columns are [`LazyParsed`](crate::LazyParsed) enums
///   that may be raw or decoded. Use `decode_id`, `decode_geometry`, `decode_properties` for
///   selective in-place decoding, or [`Layer01::decode_all`] to transition to `Layer01<Parsed>`.
///
/// - `Layer01<'a, Parsed>` — all columns are fully decoded. The fields `id`, `geometry`, and
///   `properties` hold the parsed types directly, allowing infallible readonly access.
///   There is a `ParsedLayer01<'a>` type alias for this.
pub struct Layer01<'a, S: DecodeState = Lazy> {
    pub name: &'a str,
    pub extent: u32,
    pub id: Option<Id<'a, S>>,
    pub geometry: Geometry<'a, S>,
    pub(crate) properties: Vec<Property<'a, S>>,
    #[cfg(fuzzing)]
    pub(crate) layer_order: Vec<crate::frames::v01::fuzzing::LayerOrdering>,
}

pub type ParsedLayer01<'a> = Layer01<'a, Parsed>;

impl<'a, S> std::fmt::Debug for Layer01<'a, S>
where
    S: DecodeState,
    Option<Id<'a, S>>: std::fmt::Debug,
    Geometry<'a, S>: std::fmt::Debug,
    Vec<Property<'a, S>>: std::fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut s = f.debug_struct("Layer01");
        s.field("name", &self.name)
            .field("extent", &self.extent)
            .field("id", &self.id)
            .field("geometry", &self.geometry)
            .field("properties", &self.properties);
        #[cfg(fuzzing)]
        s.field("layer_order", &self.layer_order);
        s.finish()
    }
}

impl<'a, S> Clone for Layer01<'a, S>
where
    S: DecodeState,
    Option<Id<'a, S>>: Clone,
    Geometry<'a, S>: Clone,
    Vec<Property<'a, S>>: Clone,
{
    fn clone(&self) -> Self {
        Self {
            name: self.name,
            extent: self.extent,
            id: self.id.clone(),
            geometry: self.geometry.clone(),
            properties: self.properties.clone(),
            #[cfg(fuzzing)]
            layer_order: self.layer_order.clone(),
        }
    }
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
    pub geometry: Geom32,
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
