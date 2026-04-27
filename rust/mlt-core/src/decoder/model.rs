use std::fmt;

use num_enum::TryFromPrimitive;

use crate::decoder::{Geometry, Id, Property};
use crate::{DecodeState, Lazy, Parsed};

/// A layer that can be one of the known types, or an unknown.
///
/// The decode-state type parameter `S` mirrors [`Layer01<'a, S>`]:
/// - `Layer<'a>` / `Layer<'a, Lazy>` — freshly parsed; columns may still be raw bytes.
/// - `Layer<'a, Parsed>` — returned by [`Layer::decode_all`]; all columns are decoded. Use `ParsedLayer` alias.
#[non_exhaustive]
pub enum Layer<'a, S: DecodeState = Lazy> {
    /// MVT-compatible layer (tag = 1)
    Tag01(Layer01<'a, S>),
    /// Unknown layer with tag, size, and value
    Unknown(Unknown<'a>),
}
pub type ParsedLayer<'a> = Layer<'a, Parsed>;

impl<'a, S: DecodeState> fmt::Debug for Layer<'a, S>
where
    Layer01<'a, S>: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Tag01(l) => f.debug_tuple("Tag01").field(l).finish(),
            Self::Unknown(u) => f.debug_tuple("Unknown").field(u).finish(),
        }
    }
}

/// Unknown layer data, stored as encoded bytes.
///
/// Returned inside [`Layer::Unknown`] for any layer tag that is not recognized
/// by this version of the library. Consumers can inspect the tag and raw bytes
/// to forward or log the layer without losing data.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Unknown<'a> {
    pub(crate) tag: u8,
    pub(crate) value: &'a [u8],
}

impl<'a> Unknown<'a> {
    /// The raw layer tag identifying this unrecognised layer type.
    #[must_use]
    pub fn tag(&self) -> u32 {
        u32::from(self.tag)
    }

    /// The raw encoded bytes of this layer's body.
    #[must_use]
    pub fn data(&self) -> &'a [u8] {
        self.value
    }
}

/// Column definition
#[derive(Debug, PartialEq)]
pub struct Column<'a> {
    pub(crate) typ: ColumnType,
    pub(crate) name: Option<&'a str>,
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
///   that may be raw or decoded. Use [`Layer01::decode_all`] to transition to `Layer01<Parsed>`.
///
/// - `Layer01<'a, Parsed>` — all columns are fully decoded. The fields `id`, `geometry`, and
///   `properties` hold the parsed types directly, allowing infallible readonly access.
///   There is a `ParsedLayer01<'a>` type alias for this.
pub struct Layer01<'a, S: DecodeState = Lazy> {
    pub name: &'a str,
    pub extent: u32,
    pub(crate) id: Option<Id<'a, S>>,
    pub(crate) geometry: Geometry<'a, S>,
    pub(crate) properties: Vec<Property<'a, S>>,
    #[cfg(fuzzing)]
    pub(crate) layer_order: Vec<crate::decoder::fuzzing::LayerOrdering>,
}

pub type ParsedLayer01<'a> = Layer01<'a, Parsed>;

impl<'a, S> fmt::Debug for Layer01<'a, S>
where
    S: DecodeState,
    Option<Id<'a, S>>: fmt::Debug,
    Geometry<'a, S>: fmt::Debug,
    Vec<Property<'a, S>>: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
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
#[derive(Debug, Clone, PartialEq)]
pub struct TileLayer {
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
    /// Geometry as a [`geo_types`] form
    pub geometry: geo_types::Geometry<i32>,
    /// One value per property column, in the same order as
    /// [`TileLayer::property_names`].
    pub properties: Vec<PropValue>,
}

/// A single typed value for one property of one feature.
///
/// Mirrors the scalar variants of `ParsedProperty` at the per-feature
/// level. `SharedDict` items are flattened: each sub-field becomes its own
/// `PropValue::Str` entry in `TileFeature::properties`, with the
/// corresponding entry in `TileLayer::property_names` set to
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, strum::IntoStaticStr)]
#[strum(serialize_all = "lowercase")]
pub enum PropKind {
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
impl From<&PropValue> for PropKind {
    fn from(prop: &PropValue) -> Self {
        match prop {
            PropValue::Bool(_) => Self::Bool,
            PropValue::I8(_) => Self::I8,
            PropValue::U8(_) => Self::U8,
            PropValue::I32(_) => Self::I32,
            PropValue::U32(_) => Self::U32,
            PropValue::I64(_) => Self::I64,
            PropValue::U64(_) => Self::U64,
            PropValue::F32(_) => Self::F32,
            PropValue::F64(_) => Self::F64,
            PropValue::Str(_) => Self::Str,
        }
    }
}
