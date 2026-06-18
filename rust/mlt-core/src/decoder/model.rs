use std::fmt;
use std::num::NonZeroU32;

use num_enum::TryFromPrimitive;

use crate::decoder::{Geometry, Id, Property};
use crate::{DecodeState, Lazy, MltError, MltResult, Parsed};

/// Non-zero tile extent.
///
/// Use [`Extent::new`] to validate raw integer input before storing it in
/// owned row or staged layer structures.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Extent(NonZeroU32);

impl Extent {
    pub fn new(value: u32) -> MltResult<Self> {
        NonZeroU32::new(value)
            .map(Self)
            .ok_or(MltError::InvalidExtent(value))
    }

    #[must_use]
    pub fn get(self) -> u32 {
        self.0.get()
    }
}

impl From<Extent> for NonZeroU32 {
    fn from(value: Extent) -> Self {
        value.0
    }
}

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
    pub(crate) name: String,
    pub(crate) extent: Extent,
    /// Column names, parallel to `TileFeature::properties`.
    pub(crate) property_names: Vec<String>,
    /// Column types, parallel to `TileFeature::properties`.
    pub(crate) property_kinds: Vec<PropKind>,
    pub(crate) features: Vec<TileFeature>,
}

/// A single map feature in row form.
#[derive(Debug, Clone, PartialEq)]
pub struct TileFeature {
    pub(crate) id: Option<u64>,
    /// Geometry as a [`geo_types`] form
    pub(crate) geometry: geo_types::Geometry<i32>,
    /// One value per property column, in the same order as
    /// [`TileLayer::property_names`].
    pub(crate) properties: Vec<PropValue>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PropertyKey(usize);

impl PropertyKey {
    #[must_use]
    pub fn index(self) -> usize {
        self.0
    }
}

impl TileLayer {
    pub fn new(name: impl Into<String>, extent: u32) -> MltResult<Self> {
        Self::with_capacity(name, extent, 0)
    }

    pub fn with_capacity(name: impl Into<String>, extent: u32, features: usize) -> MltResult<Self> {
        let name = name.into();
        validate_layer_name(&name)?;
        let extent = Extent::new(extent)?;
        Ok(Self {
            name,
            extent,
            property_names: Vec::new(),
            property_kinds: Vec::new(),
            features: Vec::with_capacity(features),
        })
    }

    pub(crate) fn from_parts(
        name: impl Into<String>,
        extent: u32,
        property_names: Vec<String>,
        features: Vec<TileFeature>,
    ) -> MltResult<Self> {
        let name = name.into();
        validate_layer_name(&name)?;
        let extent = Extent::new(extent)?;
        validate_property_names(&property_names)?;
        let property_kinds = infer_property_kinds(&property_names, &features)?;
        let layer = Self {
            name,
            extent,
            property_names,
            property_kinds,
            features,
        };
        Ok(layer)
    }

    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[must_use]
    pub fn extent(&self) -> Extent {
        self.extent
    }

    #[must_use]
    pub fn property_names(&self) -> &[String] {
        &self.property_names
    }

    #[must_use]
    pub fn features(&self) -> &[TileFeature] {
        &self.features
    }

    #[must_use]
    pub(crate) fn features_mut(&mut self) -> &mut [TileFeature] {
        &mut self.features
    }

    #[must_use]
    pub fn feature_count(&self) -> usize {
        self.features.len()
    }

    pub fn add_property(
        &mut self,
        name: impl Into<String>,
        kind: PropKind,
    ) -> MltResult<PropertyKey> {
        let name = name.into();
        if name.is_empty() {
            return Err(MltError::MissingPropertyName);
        }
        if self.property_names.contains(&name) {
            return Err(MltError::DuplicatePropertyName(name));
        }
        for feature in &mut self.features {
            feature.properties.push(PropValue::null(kind));
        }
        self.property_names.push(name);
        self.property_kinds.push(kind);
        Ok(PropertyKey(self.property_names.len() - 1))
    }

    pub fn push_feature(&mut self, feature: TileFeature) -> MltResult<()> {
        self.validate_feature(&feature)?;
        self.features.push(feature);
        Ok(())
    }

    pub fn builder(name: impl Into<String>, extent: u32) -> MltResult<TileLayerBuilder> {
        Ok(TileLayerBuilder {
            layer: Self::new(name, extent)?,
        })
    }

    fn validate_feature(&self, feature: &TileFeature) -> MltResult<()> {
        let expected = self.property_names.len();
        let actual = feature.properties.len();
        if actual != expected {
            return Err(MltError::PropertyLengthMismatch { expected, actual });
        }
        for (idx, prop) in feature.properties.iter().enumerate() {
            let expected = self.property_kinds[idx];
            let actual = PropKind::from(prop);
            if actual != expected {
                return Err(MltError::PropertyKindMismatch {
                    index: idx,
                    expected,
                    actual,
                });
            }
        }
        Ok(())
    }
}

impl TileFeature {
    #[must_use]
    pub fn new(geometry: geo_types::Geometry<i32>) -> Self {
        Self {
            id: None,
            geometry,
            properties: Vec::new(),
        }
    }

    #[must_use]
    pub fn with_id(geometry: geo_types::Geometry<i32>, id: u64) -> Self {
        Self {
            id: Some(id),
            geometry,
            properties: Vec::new(),
        }
    }

    #[must_use]
    pub fn id(&self) -> Option<u64> {
        self.id
    }

    #[must_use]
    pub fn geometry(&self) -> &geo_types::Geometry<i32> {
        &self.geometry
    }

    #[must_use]
    pub fn properties(&self) -> &[PropValue] {
        &self.properties
    }

    #[must_use]
    pub(crate) fn properties_mut(&mut self) -> &mut [PropValue] {
        &mut self.properties
    }

    pub fn set_property(&mut self, key: PropertyKey, value: PropValue) -> MltResult<()> {
        let Some(prop) = self.properties.get_mut(key.index()) else {
            return Err(MltError::PropertyLengthMismatch {
                expected: key.index() + 1,
                actual: self.properties.len(),
            });
        };
        let expected = PropKind::from(&*prop);
        let actual = PropKind::from(&value);
        if actual != expected {
            return Err(MltError::PropertyKindMismatch {
                index: key.index(),
                expected,
                actual,
            });
        }
        *prop = value;
        Ok(())
    }
}

pub struct TileLayerBuilder {
    layer: TileLayer,
}

impl TileLayerBuilder {
    pub fn add_property(
        &mut self,
        name: impl Into<String>,
        kind: PropKind,
    ) -> MltResult<PropertyKey> {
        self.layer.add_property(name, kind)
    }

    pub fn feature(&mut self, geometry: geo_types::Geometry<i32>) -> TileFeatureBuilder<'_> {
        let properties = self
            .layer
            .property_kinds
            .iter()
            .copied()
            .map(PropValue::null)
            .collect();
        TileFeatureBuilder {
            layer: self,
            feature: TileFeature {
                id: None,
                geometry,
                properties,
            },
        }
    }

    pub fn push_feature(&mut self, feature: TileFeature) -> MltResult<()> {
        self.layer.push_feature(feature)
    }

    #[must_use]
    pub fn finish(self) -> TileLayer {
        self.layer
    }
}

pub struct TileFeatureBuilder<'a> {
    layer: &'a mut TileLayerBuilder,
    feature: TileFeature,
}

impl TileFeatureBuilder<'_> {
    pub fn id(&mut self, id: Option<u64>) -> &mut Self {
        self.feature.id = id;
        self
    }

    pub fn property(&mut self, key: PropertyKey, value: PropValue) -> MltResult<&mut Self> {
        self.feature.set_property(key, value)?;
        Ok(self)
    }

    pub fn finish(self) -> MltResult<()> {
        self.layer.push_feature(self.feature)
    }
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

impl PropValue {
    #[must_use]
    pub fn kind(&self) -> PropKind {
        self.into()
    }

    #[must_use]
    pub fn is_null(&self) -> bool {
        match self {
            Self::Bool(v) => v.is_none(),
            Self::I8(v) => v.is_none(),
            Self::U8(v) => v.is_none(),
            Self::I32(v) => v.is_none(),
            Self::U32(v) => v.is_none(),
            Self::I64(v) => v.is_none(),
            Self::U64(v) => v.is_none(),
            Self::F32(v) => v.is_none(),
            Self::F64(v) => v.is_none(),
            Self::Str(v) => v.is_none(),
        }
    }

    #[must_use]
    pub fn null(kind: PropKind) -> Self {
        match kind {
            PropKind::Bool => Self::Bool(None),
            PropKind::I8 => Self::I8(None),
            PropKind::U8 => Self::U8(None),
            PropKind::I32 => Self::I32(None),
            PropKind::U32 => Self::U32(None),
            PropKind::I64 => Self::I64(None),
            PropKind::U64 => Self::U64(None),
            PropKind::F32 => Self::F32(None),
            PropKind::F64 => Self::F64(None),
            PropKind::Str => Self::Str(None),
        }
    }
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

fn validate_layer_name(name: &str) -> MltResult<()> {
    if name.is_empty() {
        Err(MltError::MissingLayerName)
    } else {
        Ok(())
    }
}

fn validate_property_names(names: &[String]) -> MltResult<()> {
    // Column counts are small, so a linear scan avoids the per-layer HashSet allocation.
    for (i, name) in names.iter().enumerate() {
        if name.is_empty() {
            return Err(MltError::MissingPropertyName);
        }
        if names[..i].iter().any(|n| n == name) {
            return Err(MltError::DuplicatePropertyName(name.clone()));
        }
    }
    Ok(())
}

fn infer_property_kinds(names: &[String], features: &[TileFeature]) -> MltResult<Vec<PropKind>> {
    let mut kinds = vec![None; names.len()];
    for feature in features {
        let expected = names.len();
        let actual = feature.properties.len();
        if actual != expected {
            return Err(MltError::PropertyLengthMismatch { expected, actual });
        }
        for (idx, prop) in feature.properties.iter().enumerate() {
            let actual = PropKind::from(prop);
            match kinds[idx] {
                Some(expected) if expected != actual => {
                    return Err(MltError::PropertyKindMismatch {
                        index: idx,
                        expected,
                        actual,
                    });
                }
                None => kinds[idx] = Some(actual),
                _ => {}
            }
        }
    }
    Ok(kinds
        .into_iter()
        .map(|kind| kind.unwrap_or(PropKind::Str))
        .collect())
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

#[cfg(test)]
mod tests {
    use geo_types::{Geometry, Point};

    use super::*;

    fn point_feature(properties: Vec<PropValue>) -> TileFeature {
        TileFeature {
            id: None,
            geometry: Geometry::Point(Point::new(0, 0)),
            properties,
        }
    }

    #[test]
    fn tile_layer_constructor_rejects_empty_name() {
        assert!(matches!(
            TileLayer::new("", 4096),
            Err(MltError::MissingLayerName)
        ));
    }

    #[test]
    fn tile_layer_constructor_rejects_zero_extent() {
        assert!(matches!(
            TileLayer::new("layer", 0),
            Err(MltError::InvalidExtent(0))
        ));
    }

    #[test]
    fn add_property_rejects_duplicate_names() {
        let mut layer = TileLayer::new("layer", 4096).unwrap();
        layer.add_property("name", PropKind::Str).unwrap();
        assert!(matches!(
            layer.add_property("name", PropKind::Str),
            Err(MltError::DuplicatePropertyName(name)) if name == "name"
        ));
    }

    #[test]
    fn add_property_rejects_empty_name() {
        let mut layer = TileLayer::new("layer", 4096).unwrap();
        assert!(matches!(
            layer.add_property("", PropKind::Str),
            Err(MltError::MissingPropertyName)
        ));
    }

    #[test]
    fn from_parts_rejects_empty_property_name() {
        assert!(matches!(
            TileLayer::from_parts("layer", 4096, vec![String::new()], vec![]),
            Err(MltError::MissingPropertyName)
        ));
    }

    #[test]
    fn from_parts_rejects_duplicate_property_name() {
        assert!(matches!(
            TileLayer::from_parts("layer", 4096, vec!["dup".into(), "dup".into()], vec![]),
            Err(MltError::DuplicatePropertyName(name)) if name == "dup"
        ));
    }

    #[test]
    fn push_feature_validates_property_count() {
        let mut layer = TileLayer::new("layer", 4096).unwrap();
        layer.add_property("name", PropKind::Str).unwrap();
        assert!(matches!(
            layer.push_feature(point_feature(vec![])),
            Err(MltError::PropertyLengthMismatch {
                expected: 1,
                actual: 0
            })
        ));
    }

    #[test]
    fn push_feature_validates_property_kind() {
        let mut layer = TileLayer::new("layer", 4096).unwrap();
        layer.add_property("flag", PropKind::Bool).unwrap();
        layer
            .push_feature(point_feature(vec![PropValue::Bool(Some(true))]))
            .unwrap();
        assert!(matches!(
            layer.push_feature(point_feature(vec![PropValue::I32(Some(1))])),
            Err(MltError::PropertyKindMismatch {
                index: 0,
                expected: PropKind::Bool,
                actual: PropKind::I32,
            })
        ));
    }

    #[test]
    fn declared_property_kind_is_enforced_for_first_feature() {
        let mut layer = TileLayer::new("layer", 4096).unwrap();
        layer.add_property("flag", PropKind::Bool).unwrap();
        assert!(matches!(
            layer.push_feature(point_feature(vec![PropValue::I32(Some(1))])),
            Err(MltError::PropertyKindMismatch {
                index: 0,
                expected: PropKind::Bool,
                actual: PropKind::I32,
            })
        ));
    }

    #[test]
    fn builder_uses_declared_property_kind_for_defaults() {
        let mut builder = TileLayer::builder("layer", 4096).unwrap();
        let flag = builder.add_property("flag", PropKind::Bool).unwrap();
        let mut feature = builder.feature(Geometry::Point(Point::new(0, 0)));
        feature.property(flag, PropValue::Bool(Some(true))).unwrap();
        feature.finish().unwrap();
        let layer = builder.finish();

        assert_eq!(
            layer.features()[0].properties()[0],
            PropValue::Bool(Some(true))
        );
    }
}
