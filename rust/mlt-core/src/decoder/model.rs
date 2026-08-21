//! Version-agnostic decode-side layer types.
//!
//! [`Layer01`] is the in-memory columnar form for *both* tag `0x01` and tag
//! `0x02` layers - the two differ only in wire format, which lives in
//! [`super::model01`] and `model02`.

use std::fmt;

use crate::decoder::{Geometry, GeometryValues, Id, Property};
use crate::tile::Extent;
use crate::{DecodeState, Lazy, Parsed};

/// A layer that can be one of the known types, or an unknown.
///
/// The decode-state type parameter `S` mirrors [`Layer01<'a, S>`]:
/// - `Layer<'a>` / `Layer<'a, Lazy>` - freshly parsed; columns may still be raw bytes.
/// - `Layer<'a, Parsed>` - returned by [`Layer::decode_all`]; all columns are decoded. Use `ParsedLayer` alias.
#[non_exhaustive]
pub enum Layer<'a, S: DecodeState = Lazy> {
    /// MVT-compatible layer (tag = 1)
    Tag01(Layer01<'a, S>),
    /// Experimental v2 layer (tag = 2).
    ///
    /// Parsed into the same in-memory columnar representation as `Tag01` but with an more compact wire format.
    #[cfg(feature = "unstable-v2")]
    Tag02(Layer01<'a, S>),
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
            #[cfg(feature = "unstable-v2")]
            Self::Tag02(l) => f.debug_tuple("Tag02").field(l).finish(),
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

/// Representation of an MLT feature table layer during decoding.
///
/// Used for both tag `0x01` and tag `0x02` layers - the name is historical.
///
/// The type parameter `S` controls how columns are stored:
///
/// - `Layer01<'a>` / `Layer01<'a, Lazy>` (default) - columns are `LazyParsed` enums
///   that may be raw or decoded. Use [`Layer01::decode_all`] to transition to `Layer01<Parsed>`.
///
/// - `Layer01<'a, Parsed>` - all columns are fully decoded. The fields `id`, `geometry`, and
///   `properties` hold the parsed types directly, allowing infallible readonly access.
///   There is a `ParsedLayer01<'a>` type alias for this.
pub struct Layer01<'a, S: DecodeState = Lazy> {
    pub(crate) name: &'a str,
    pub(crate) extent: Extent,
    pub(crate) id: Option<Id<'a, S>>,
    pub(crate) geometry: Geometry<'a, S>,
    pub(crate) properties: Vec<Property<'a, S>>,
    #[cfg(fuzzing)]
    pub(crate) layer_order: Vec<crate::decoder::fuzzing::LayerOrdering>,
}

pub type ParsedLayer01<'a> = Layer01<'a, Parsed>;

impl<'a, S: DecodeState> Layer01<'a, S> {
    #[must_use]
    pub fn name(&self) -> &'a str {
        self.name
    }

    #[must_use]
    pub fn extent(&self) -> Extent {
        self.extent
    }
}

impl ParsedLayer01<'_> {
    /// Returns the decoded geometry buffer for this layer.
    ///
    /// Provides access to the columnar geometry arrays (vertex buffer, offset arrays, geometry
    /// types) for advanced use cases such as building typed arrays for WebAssembly or
    /// performing spatial indexing. For iterating feature geometries as `geo_types` values,
    /// prefer [`iter_features`](Self::iter_features) instead.
    #[must_use]
    pub fn geometry_values(&self) -> &GeometryValues {
        &self.geometry
    }

    #[must_use]
    pub fn feature_count(&self) -> usize {
        self.geometry.vector_types.len()
    }
}

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
