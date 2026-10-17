//! Owned, row-oriented tile model.
//!
//! This is the representation tiles are built in and converted to/from:
//! it is independent of the wire format, and is shared by the encoder, the MVT and
//! `GeoJSON` converters, and the language bindings.
//! The decoder's columnar types live in [`crate::decoder`].

#[cfg(feature = "unstable-v2")]
use std::collections::BTreeMap;
use std::num::NonZeroU32;

use geo_types::{Coord, Geometry, LineString};
#[cfg(feature = "unstable-v2")]
use num_traits::ToPrimitive as _;

use crate::{MltError, MltResult};

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

/// The vertical grid a layer's z coordinates lie on, a power-of-ten step in metres.
///
/// A z of `0` sits at Terrain-RGB's base of -10000 m, so a coordinate's elevation is `-10000 + z * step`.
#[cfg(feature = "unstable-v2")]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ZStep(i8);

#[cfg(feature = "unstable-v2")]
impl ZStep {
    /// The finest step, 10^-3 m.
    pub const MIN_EXPONENT: i8 = -3;
    /// The coarsest step, 10^4 m.
    pub const MAX_EXPONENT: i8 = 4;
    /// The elevation of `z = 0`, in metres.
    pub const BASE_METRES: f64 = -10_000.0;

    /// A step of `10^exponent` metres.
    pub fn new(exponent: i8) -> MltResult<Self> {
        if (Self::MIN_EXPONENT..=Self::MAX_EXPONENT).contains(&exponent) {
            Ok(Self(exponent))
        } else {
            Err(MltError::InvalidZStep(exponent))
        }
    }

    #[must_use]
    pub fn exponent(self) -> i8 {
        self.0
    }

    /// The byte a vertex stream stores this step as, `exponent + 3`.
    pub(crate) fn code(self) -> u8 {
        (self.0 - Self::MIN_EXPONENT).cast_unsigned()
    }

    /// The step a vertex stream's byte names.
    pub(crate) fn from_code(code: u8) -> MltResult<Self> {
        i8::try_from(code)
            .ok()
            .and_then(|c| Self::new(c + Self::MIN_EXPONENT).ok())
            .ok_or(MltError::InvalidZStepCode(code))
    }

    /// The step in metres.
    #[must_use]
    pub fn metres(self) -> f64 {
        if self.0 < 0 {
            1.0 / self.power()
        } else {
            self.power()
        }
    }

    /// The elevation of `z`, in metres.
    ///
    /// A fine step moves the base onto the grid and divides by an exact integer last, so `1_001_234` at `10^-2` reads as `12.34`.
    #[must_use]
    pub fn elevation(self, z: i32) -> f64 {
        let power = self.power();
        if self.0 < 0 {
            (f64::from(z) + Self::BASE_METRES * power) / power
        } else {
            Self::BASE_METRES + f64::from(z) * power
        }
    }

    /// The grid value nearest `elevation` metres, or [`None`] when it falls outside `i32`.
    #[must_use]
    pub fn z(self, elevation: f64) -> Option<i32> {
        let power = self.power();
        let z = if self.0 < 0 {
            elevation * power - Self::BASE_METRES * power
        } else {
            (elevation - Self::BASE_METRES) / power
        };
        z.round().to_i32()
    }

    /// `10^|exponent|`, an integer `f64` holds exactly.
    fn power(self) -> f64 {
        10f64.powi(self.0.unsigned_abs().into())
    }
}

/// The step as the spec's table names it, e.g. `1 dm`.
#[cfg(feature = "unstable-v2")]
impl std::fmt::Display for ZStep {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self.0 {
            -3 => "1 mm",
            -2 => "1 cm",
            -1 => "1 dm",
            0 => "1 m",
            1 => "10 m",
            2 => "100 m",
            3 => "1 km",
            _ => "10 km",
        })
    }
}

/// The kind of column a name belongs to, which a layer's names are unique across.
#[derive(Debug, Clone, Copy, PartialEq, Eq, strum::Display)]
pub enum ColumnRole {
    /// An ordinary column of values, a shared dictionary's children included.
    #[strum(serialize = "property")]
    Property,
    /// A vertex-scoped column of the m-value section.
    #[cfg(feature = "unstable-v2")]
    #[strum(serialize = "m-value")]
    MValue,
    /// A shredded column of maps or lists.
    #[cfg(feature = "unstable-v2")]
    #[strum(serialize = "nested")]
    Nested,
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
    /// Vertex-scoped column names, parallel to `TileFeature::m_values`.
    #[cfg(feature = "unstable-v2")]
    pub(crate) m_value_names: Vec<String>,
    /// Vertex-scoped column types, parallel to `TileFeature::m_values`.
    #[cfg(feature = "unstable-v2")]
    pub(crate) m_value_kinds: Vec<PropKind>,
    /// Nested column names, parallel to `TileFeature::nested`.
    #[cfg(feature = "unstable-v2")]
    pub(crate) nested_names: Vec<String>,
    /// Nested column shapes, parallel to `TileFeature::nested`.
    #[cfg(feature = "unstable-v2")]
    pub(crate) nested_kinds: Vec<NestedKind>,
    /// The grid of every feature's `TileFeature::z`, or [`None`] for a flat layer.
    #[cfg(feature = "unstable-v2")]
    pub(crate) z_step: Option<ZStep>,
    pub(crate) features: Vec<TileFeature>,
}

/// A single map feature in row form.
#[derive(Debug, Clone, PartialEq)]
pub struct TileFeature {
    pub(crate) id: Option<u64>,
    /// Geometry as a [`geo_types`] form
    pub(crate) geometry: Geometry<i32>,
    /// One value per property column, in the same order as
    /// [`TileLayer::property_names`].
    pub(crate) properties: Vec<PropValue>,
    /// One entry per m-value column, in the same order as
    /// [`TileLayer::m_value_names`], each holding one value per vertex of this
    /// feature's geometry or none at all.
    #[cfg(feature = "unstable-v2")]
    pub(crate) m_values: Vec<MValue>,
    /// One value per nested column, in the same order as [`TileLayer::nested_names`].
    #[cfg(feature = "unstable-v2")]
    pub(crate) nested: Vec<NestedValue>,
    /// One z per vertex of the geometry on the layer's [`ZStep`] grid, or none in a flat layer.
    #[cfg(feature = "unstable-v2")]
    pub(crate) z: Vec<i32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PropertyKey(usize);

impl PropertyKey {
    pub(crate) const fn new(index: usize) -> Self {
        Self(index)
    }

    #[must_use]
    pub fn index(self) -> usize {
        self.0
    }
}

/// Handle to one of a layer's m-value columns, as [`PropertyKey`] is to a property column.
#[cfg(feature = "unstable-v2")]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MValueKey(usize);

#[cfg(feature = "unstable-v2")]
impl MValueKey {
    #[must_use]
    pub fn index(self) -> usize {
        self.0
    }
}

/// Handle to one of a layer's nested columns, as [`PropertyKey`] is to a property column.
#[cfg(feature = "unstable-v2")]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NestedKey(usize);

#[cfg(feature = "unstable-v2")]
impl NestedKey {
    #[must_use]
    pub fn index(self) -> usize {
        self.0
    }
}

/// How deep a nested column may go, counting its root as the first level.
#[cfg(feature = "unstable-v2")]
pub(crate) const MAX_NESTED_DEPTH: usize = 8;

/// The shape of one nested column, the union of every value it holds.
#[cfg(feature = "unstable-v2")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NestedKind {
    Leaf(PropKind),
    List(Box<Self>),
    /// String keys, each with its own kind, so a heterogeneous object is one of these.
    Map(BTreeMap<String, Self>),
}

#[cfg(feature = "unstable-v2")]
impl NestedKind {
    /// A map of the given fields, the shape a struct of them shreds into.
    #[must_use]
    pub fn map<K: Into<String>>(fields: impl IntoIterator<Item = (K, Self)>) -> Self {
        Self::Map(fields.into_iter().map(|(k, v)| (k.into(), v)).collect())
    }

    /// A list of `element`.
    #[must_use]
    pub fn list(element: Self) -> Self {
        Self::List(Box::new(element))
    }

    /// How many levels this shape has, counting itself as the first.
    #[must_use]
    pub fn depth(&self) -> usize {
        1 + match self {
            Self::Leaf(_) => 0,
            Self::List(element) => element.depth(),
            Self::Map(fields) => fields.values().map(Self::depth).max().unwrap_or(0),
        }
    }

    /// The value a feature that carries nothing for this column holds.
    #[must_use]
    pub fn null_value(&self) -> NestedValue {
        match self {
            Self::Leaf(kind) => NestedValue::Leaf(PropValue::null(*kind)),
            Self::List(_) => NestedValue::List(None),
            Self::Map(_) => NestedValue::Map(None),
        }
    }

    /// Whether `value` is one this shape describes.
    ///
    /// A map may leave a field out, which is how a null field is written.
    #[must_use]
    pub fn accepts(&self, value: &NestedValue) -> bool {
        match (self, value) {
            (Self::Leaf(kind), NestedValue::Leaf(value)) => value.kind() == *kind,
            (Self::List(_), NestedValue::List(None)) | (Self::Map(_), NestedValue::Map(None)) => {
                true
            }
            (Self::List(element), NestedValue::List(Some(items))) => {
                items.iter().all(|item| element.accepts(item))
            }
            (Self::Map(fields), NestedValue::Map(Some(entries))) => entries
                .iter()
                .all(|(key, value)| fields.get(key).is_some_and(|kind| kind.accepts(value))),
            _ => false,
        }
    }

    /// Whether a shape with no fields, which no struct node can hold, sits anywhere in this one.
    fn has_empty_map(&self) -> bool {
        match self {
            Self::Leaf(_) => false,
            Self::List(element) => element.has_empty_map(),
            Self::Map(fields) => fields.is_empty() || fields.values().any(Self::has_empty_map),
        }
    }
}

/// One feature's value for one nested column.
///
/// A map leaves out the keys it has no value for, so a null field is an absent one.
#[cfg(feature = "unstable-v2")]
#[derive(Debug, Clone, PartialEq)]
pub enum NestedValue {
    Leaf(PropValue),
    List(Option<Vec<Self>>),
    Map(Option<BTreeMap<String, Self>>),
}

#[cfg(feature = "unstable-v2")]
impl NestedValue {
    /// A map of the given entries.
    #[must_use]
    pub fn map<K: Into<String>>(entries: impl IntoIterator<Item = (K, Self)>) -> Self {
        Self::Map(Some(
            entries.into_iter().map(|(k, v)| (k.into(), v)).collect(),
        ))
    }

    /// A list of the given items.
    #[must_use]
    pub fn list(items: impl IntoIterator<Item = Self>) -> Self {
        Self::List(Some(items.into_iter().collect()))
    }

    /// Whether this carries nothing at all, which is how a nested value is null.
    #[must_use]
    pub fn is_null(&self) -> bool {
        match self {
            Self::Leaf(value) => value.is_null(),
            Self::List(items) => items.is_none(),
            Self::Map(entries) => entries.is_none(),
        }
    }

    /// The heap bytes this value holds, which the decoder charges its budget for.
    #[must_use]
    pub(crate) fn heap_bytes(&self) -> usize {
        match self {
            Self::Leaf(PropValue::Str(Some(value))) => value.len(),
            Self::Leaf(_) => size_of::<PropValue>(),
            Self::List(items) => items.iter().flatten().map(Self::heap_bytes).sum::<usize>(),
            Self::Map(entries) => entries
                .iter()
                .flatten()
                .map(|(key, value)| key.len() + value.heap_bytes())
                .sum(),
        }
    }

    /// Whether the two are the same kind of value, which is all a feature can check
    /// without the column's shape.
    fn same_shape(&self, other: &Self) -> bool {
        matches!(
            (self, other),
            (Self::Leaf(_), Self::Leaf(_))
                | (Self::List(_), Self::List(_))
                | (Self::Map(_), Self::Map(_))
        )
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
            #[cfg(feature = "unstable-v2")]
            m_value_names: Vec::new(),
            #[cfg(feature = "unstable-v2")]
            m_value_kinds: Vec::new(),
            #[cfg(feature = "unstable-v2")]
            nested_names: Vec::new(),
            #[cfg(feature = "unstable-v2")]
            nested_kinds: Vec::new(),
            #[cfg(feature = "unstable-v2")]
            z_step: None,
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
        validate_unique_names(&property_names, ColumnRole::Property)?;
        let property_kinds = infer_kinds::<PropValue>(&property_names, &features)?;
        let layer = Self {
            name,
            extent,
            property_names,
            property_kinds,
            #[cfg(feature = "unstable-v2")]
            m_value_names: Vec::new(),
            #[cfg(feature = "unstable-v2")]
            m_value_kinds: Vec::new(),
            #[cfg(feature = "unstable-v2")]
            nested_names: Vec::new(),
            #[cfg(feature = "unstable-v2")]
            nested_kinds: Vec::new(),
            #[cfg(feature = "unstable-v2")]
            z_step: None,
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
    pub fn property_kinds(&self) -> &[PropKind] {
        &self.property_kinds
    }

    #[must_use]
    pub fn features(&self) -> &[TileFeature] {
        &self.features
    }

    #[must_use]
    pub fn feature_count(&self) -> usize {
        self.features.len()
    }

    /// Reject a column name any column of this layer already holds.
    fn reject_name_in_use(&self, name: &str, role: ColumnRole) -> MltResult<()> {
        let taken = self
            .property_names
            .iter()
            .map(|n| (n.as_str(), ColumnRole::Property));
        #[cfg(feature = "unstable-v2")]
        let taken = taken
            .chain(
                self.m_value_names
                    .iter()
                    .map(|n| (n.as_str(), ColumnRole::MValue)),
            )
            .chain(
                self.nested_names
                    .iter()
                    .map(|n| (n.as_str(), ColumnRole::Nested)),
            );
        reject_taken_name(name, role, taken)
    }

    pub fn add_property(
        &mut self,
        name: impl Into<String>,
        kind: PropKind,
    ) -> MltResult<PropertyKey> {
        let name = name.into();
        self.reject_name_in_use(&name, ColumnRole::Property)?;
        for feature in &mut self.features {
            feature.properties.push(PropValue::null(kind));
        }
        self.property_names.push(name);
        self.property_kinds.push(kind);
        Ok(PropertyKey(self.property_names.len() - 1))
    }

    #[cfg(feature = "unstable-v2")]
    #[must_use]
    pub fn m_value_names(&self) -> &[String] {
        &self.m_value_names
    }

    #[cfg(feature = "unstable-v2")]
    #[must_use]
    pub fn m_value_kinds(&self) -> &[PropKind] {
        &self.m_value_kinds
    }

    /// Declare a vertex-scoped column, whose values every feature then holds or does not.
    #[cfg(feature = "unstable-v2")]
    pub fn add_m_value(&mut self, name: impl Into<String>, kind: PropKind) -> MltResult<MValueKey> {
        let name = name.into();
        self.reject_name_in_use(&name, ColumnRole::MValue)?;
        for feature in &mut self.features {
            feature.m_values.push(MValue::null(kind));
        }
        self.m_value_names.push(name);
        self.m_value_kinds.push(kind);
        Ok(MValueKey(self.m_value_names.len() - 1))
    }

    /// Name the vertex-scoped columns the features already carry, as
    /// [`Self::from_parts`] names the property columns.
    #[cfg(feature = "unstable-v2")]
    pub(crate) fn with_m_value_names(mut self, names: Vec<String>) -> MltResult<Self> {
        validate_unique_names(&names, ColumnRole::MValue)?;
        for name in &names {
            self.reject_name_in_use(name, ColumnRole::MValue)?;
        }
        self.m_value_kinds = infer_kinds::<MValue>(&names, &self.features)?;
        self.m_value_names = names;
        for feature in &self.features {
            self.validate_m_values(feature)?;
        }
        Ok(self)
    }

    #[cfg(feature = "unstable-v2")]
    #[must_use]
    pub fn nested_names(&self) -> &[String] {
        &self.nested_names
    }

    #[cfg(feature = "unstable-v2")]
    #[must_use]
    pub fn nested_kinds(&self) -> &[NestedKind] {
        &self.nested_kinds
    }

    /// The grid the features' z coordinates lie on, or [`None`] for a flat layer.
    #[cfg(feature = "unstable-v2")]
    #[must_use]
    pub fn z_step(&self) -> Option<ZStep> {
        self.z_step
    }

    /// Give the layer z coordinates on `step`, which every feature pushed after this must carry.
    ///
    /// A layer that already holds features is rejected, since they carry none.
    #[cfg(feature = "unstable-v2")]
    pub fn set_z_step(&mut self, step: ZStep) -> MltResult<()> {
        if !self.features.is_empty() {
            return Err(MltError::ZStepOnNonEmptyLayer(self.name.clone()));
        }
        self.z_step = Some(step);
        Ok(())
    }

    /// Put the features, which already carry their z coordinates, on `step`.
    #[cfg(feature = "unstable-v2")]
    pub(crate) fn with_z_step(mut self, step: Option<ZStep>) -> MltResult<Self> {
        self.z_step = step;
        for feature in &self.features {
            self.validate_z(feature)?;
        }
        Ok(self)
    }

    /// Declare a nested column of the given shape, which every feature then holds a value of.
    ///
    /// A scalar root is an ordinary property column, so [`NestedKind::Leaf`] is rejected here.
    #[cfg(feature = "unstable-v2")]
    pub fn add_nested(
        &mut self,
        name: impl Into<String>,
        kind: NestedKind,
    ) -> MltResult<NestedKey> {
        let name = name.into();
        self.reject_name_in_use(&name, ColumnRole::Nested)?;
        validate_nested_kind(&name, &kind)?;
        for feature in &mut self.features {
            feature.nested.push(kind.null_value());
        }
        self.nested_names.push(name);
        self.nested_kinds.push(kind);
        Ok(NestedKey(self.nested_names.len() - 1))
    }

    /// Name and shape the nested columns the features already carry, as
    /// [`Self::from_parts`] names the property columns.
    #[cfg(feature = "unstable-v2")]
    pub(crate) fn with_nested(
        mut self,
        names: Vec<String>,
        kinds: Vec<NestedKind>,
    ) -> MltResult<Self> {
        validate_unique_names(&names, ColumnRole::Nested)?;
        for name in &names {
            self.reject_name_in_use(name, ColumnRole::Nested)?;
        }
        for (name, kind) in names.iter().zip(&kinds) {
            validate_nested_kind(name, kind)?;
        }
        self.nested_names = names;
        self.nested_kinds = kinds;
        for feature in &self.features {
            self.validate_nested(feature)?;
        }
        Ok(self)
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

    /// A builder for a layer with the name, extent and z step of this one, and no columns or features.
    #[must_use]
    pub fn builder_like(&self) -> TileLayerBuilder {
        let layer = Self {
            name: self.name.clone(),
            extent: self.extent,
            property_names: Vec::new(),
            property_kinds: Vec::new(),
            #[cfg(feature = "unstable-v2")]
            m_value_names: Vec::new(),
            #[cfg(feature = "unstable-v2")]
            m_value_kinds: Vec::new(),
            #[cfg(feature = "unstable-v2")]
            nested_names: Vec::new(),
            #[cfg(feature = "unstable-v2")]
            nested_kinds: Vec::new(),
            #[cfg(feature = "unstable-v2")]
            z_step: self.z_step,
            features: Vec::new(),
        };
        TileLayerBuilder { layer }
    }

    fn validate_feature(&self, feature: &TileFeature) -> MltResult<()> {
        validate_kinds::<PropValue>(&self.property_kinds, feature)?;
        #[cfg(feature = "unstable-v2")]
        self.validate_m_values(feature)?;
        #[cfg(feature = "unstable-v2")]
        self.validate_nested(feature)?;
        #[cfg(feature = "unstable-v2")]
        self.validate_z(feature)?;
        Ok(())
    }

    /// Check a feature's z coordinates against the layer: one per vertex on a z layer, none on a flat one.
    #[cfg(feature = "unstable-v2")]
    fn validate_z(&self, feature: &TileFeature) -> MltResult<()> {
        let expected = if self.z_step.is_some() {
            feature.vertex_count()
        } else {
            0
        };
        if feature.z.len() == expected {
            Ok(())
        } else {
            Err(MltError::ZVertexCountMismatch {
                expected,
                actual: feature.z.len(),
            })
        }
    }

    /// Check a feature's nested values against the layer's columns: one value per
    /// column, of the shape the column declared.
    #[cfg(feature = "unstable-v2")]
    fn validate_nested(&self, feature: &TileFeature) -> MltResult<()> {
        if feature.nested.len() != self.nested_kinds.len() {
            return Err(MltError::NestedColumnCountMismatch {
                expected: self.nested_kinds.len(),
                actual: feature.nested.len(),
            });
        }
        for (index, (value, kind)) in feature.nested.iter().zip(&self.nested_kinds).enumerate() {
            if !kind.accepts(value) {
                return Err(MltError::NestedValueMismatch {
                    index,
                    name: self.nested_names[index].clone(),
                });
            }
        }
        Ok(())
    }

    /// Check a feature's m-values against the layer's columns: one entry per
    /// column, of the column's kind, holding one value per vertex it has.
    #[cfg(feature = "unstable-v2")]
    fn validate_m_values(&self, feature: &TileFeature) -> MltResult<()> {
        validate_kinds::<MValue>(&self.m_value_kinds, feature)?;
        let vertices = feature.vertex_count();
        for (index, m_value) in feature.m_values.iter().enumerate() {
            if let Some(len) = m_value.count()
                && len != vertices
            {
                return Err(MltError::MValueVertexCountMismatch {
                    index,
                    expected: vertices,
                    actual: len,
                });
            }
        }
        Ok(())
    }
}

impl TileFeature {
    #[must_use]
    pub fn new(geometry: Geometry<i32>) -> Self {
        Self {
            id: None,
            geometry,
            properties: Vec::new(),
            #[cfg(feature = "unstable-v2")]
            m_values: Vec::new(),
            #[cfg(feature = "unstable-v2")]
            nested: Vec::new(),
            #[cfg(feature = "unstable-v2")]
            z: Vec::new(),
        }
    }

    #[must_use]
    pub fn with_id(geometry: Geometry<i32>, id: u64) -> Self {
        Self {
            id: Some(id),
            geometry,
            properties: Vec::new(),
            #[cfg(feature = "unstable-v2")]
            m_values: Vec::new(),
            #[cfg(feature = "unstable-v2")]
            nested: Vec::new(),
            #[cfg(feature = "unstable-v2")]
            z: Vec::new(),
        }
    }

    #[must_use]
    pub fn id(&self) -> Option<u64> {
        self.id
    }

    #[must_use]
    pub fn geometry(&self) -> &Geometry<i32> {
        &self.geometry
    }

    #[must_use]
    pub fn properties(&self) -> &[PropValue] {
        &self.properties
    }

    /// How many vertices this feature stores, which is how many values each of
    /// its m-values holds.
    ///
    /// A polygon ring's closing vertex is not stored, so it is not counted.
    #[cfg(feature = "unstable-v2")]
    #[must_use]
    pub fn vertex_count(&self) -> usize {
        stored_vertex_count(&self.geometry)
    }

    #[cfg(feature = "unstable-v2")]
    #[must_use]
    pub fn m_values(&self) -> &[MValue] {
        &self.m_values
    }

    /// One z per vertex, in the order [`Self::vertex_count`] counts them, or none in a flat layer.
    #[cfg(feature = "unstable-v2")]
    #[must_use]
    pub fn z(&self) -> &[i32] {
        &self.z
    }

    /// Set this feature's z coordinates, one per vertex it stores.
    #[cfg(feature = "unstable-v2")]
    pub fn set_z(&mut self, z: Vec<i32>) -> MltResult<()> {
        let expected = self.vertex_count();
        if z.len() != expected {
            return Err(MltError::ZVertexCountMismatch {
                expected,
                actual: z.len(),
            });
        }
        self.z = z;
        Ok(())
    }

    /// Set this feature's values for one m-value column, which must hold one
    /// value per vertex unless the feature has none at all.
    #[cfg(feature = "unstable-v2")]
    pub fn set_m_value(&mut self, key: MValueKey, value: MValue) -> MltResult<()> {
        let vertices = self.vertex_count();
        let Some(slot) = self.m_values.get_mut(key.index()) else {
            return Err(MltError::UnknownMValueKey {
                index: key.index(),
                columns: self.m_values.len(),
            });
        };
        let expected = slot.kind();
        let actual = value.kind();
        if actual != expected {
            return Err(MltError::MValueKindMismatch {
                index: key.index(),
                expected,
                actual,
            });
        }
        if let Some(len) = value.count()
            && len != vertices
        {
            return Err(MltError::MValueVertexCountMismatch {
                index: key.index(),
                expected: vertices,
                actual: len,
            });
        }
        *slot = value;
        Ok(())
    }

    #[cfg(feature = "unstable-v2")]
    #[must_use]
    pub fn nested(&self) -> &[NestedValue] {
        &self.nested
    }

    /// Set this feature's value for one nested column, which must be the kind of
    /// value the column holds.
    ///
    /// Only the layer knows the column's full shape, so that is checked when the
    /// feature is pushed.
    #[cfg(feature = "unstable-v2")]
    pub fn set_nested(&mut self, key: NestedKey, value: NestedValue) -> MltResult<()> {
        let columns = self.nested.len();
        let Some(slot) = self.nested.get_mut(key.index()) else {
            return Err(MltError::UnknownNestedKey {
                index: key.index(),
                columns,
            });
        };
        if !slot.same_shape(&value) {
            return Err(MltError::NestedShapeMismatch { index: key.index() });
        }
        *slot = value;
        Ok(())
    }

    pub fn set_property(&mut self, key: PropertyKey, value: PropValue) -> MltResult<()> {
        let Some(prop) = self.properties.get_mut(key.index()) else {
            return Err(MltError::UnknownProperty {
                index: key.index(),
                count: self.properties.len(),
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

    #[cfg(feature = "unstable-v2")]
    pub fn add_m_value(&mut self, name: impl Into<String>, kind: PropKind) -> MltResult<MValueKey> {
        self.layer.add_m_value(name, kind)
    }

    #[cfg(feature = "unstable-v2")]
    pub fn add_nested(
        &mut self,
        name: impl Into<String>,
        kind: NestedKind,
    ) -> MltResult<NestedKey> {
        self.layer.add_nested(name, kind)
    }

    #[cfg(feature = "unstable-v2")]
    pub fn set_z_step(&mut self, step: ZStep) -> MltResult<()> {
        self.layer.set_z_step(step)
    }

    pub fn feature(&mut self, geometry: Geometry<i32>) -> TileFeatureBuilder<'_> {
        let properties = self
            .layer
            .property_kinds
            .iter()
            .copied()
            .map(PropValue::null)
            .collect();
        #[cfg(feature = "unstable-v2")]
        let m_values = self
            .layer
            .m_value_kinds
            .iter()
            .copied()
            .map(MValue::null)
            .collect();
        #[cfg(feature = "unstable-v2")]
        let nested = self
            .layer
            .nested_kinds
            .iter()
            .map(NestedKind::null_value)
            .collect();
        TileFeatureBuilder {
            layer: self,
            feature: TileFeature {
                id: None,
                geometry,
                properties,
                #[cfg(feature = "unstable-v2")]
                m_values,
                #[cfg(feature = "unstable-v2")]
                nested,
                #[cfg(feature = "unstable-v2")]
                z: Vec::new(),
            },
        }
    }

    /// Declare every property column of `layer`, in its order.
    pub fn add_properties_like(&mut self, layer: &TileLayer) -> MltResult<Vec<PropertyKey>> {
        (layer.property_names.iter().zip(&layer.property_kinds))
            .map(|(name, &kind)| self.add_property(name, kind))
            .collect()
    }

    /// Declare every m-value column of `layer`, in its order.
    #[cfg(feature = "unstable-v2")]
    pub fn add_m_values_like(&mut self, layer: &TileLayer) -> MltResult<Vec<MValueKey>> {
        (layer.m_value_names.iter().zip(&layer.m_value_kinds))
            .map(|(name, &kind)| self.add_m_value(name, kind))
            .collect()
    }

    /// Declare every nested column of `layer`, in its order.
    #[cfg(feature = "unstable-v2")]
    pub fn add_nested_like(&mut self, layer: &TileLayer) -> MltResult<Vec<NestedKey>> {
        (layer.nested_names.iter().zip(&layer.nested_kinds))
            .map(|(name, kind)| self.add_nested(name, kind.clone()))
            .collect()
    }

    /// A feature with the geometry, id and z of `feature`, and no values yet.
    pub fn feature_like(&mut self, feature: &TileFeature) -> TileFeatureBuilder<'_> {
        let mut row = self.feature(feature.geometry.clone());
        row.feature.id = feature.id;
        #[cfg(feature = "unstable-v2")]
        row.feature.z.clone_from(&feature.z);
        row
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

    #[cfg(feature = "unstable-v2")]
    pub fn m_value(&mut self, key: MValueKey, value: MValue) -> MltResult<&mut Self> {
        self.feature.set_m_value(key, value)?;
        Ok(self)
    }

    #[cfg(feature = "unstable-v2")]
    pub fn nested(&mut self, key: NestedKey, value: NestedValue) -> MltResult<&mut Self> {
        self.feature.set_nested(key, value)?;
        Ok(self)
    }

    #[cfg(feature = "unstable-v2")]
    pub fn z(&mut self, z: Vec<i32>) -> MltResult<&mut Self> {
        self.feature.set_z(z)?;
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
}

/// A feature's values for one vertex-scoped column: one per vertex, or none at all.
///
/// The option is around the whole vector because a null is a feature's, not a
/// vertex's: a feature either measures every one of its vertices or none of them.
#[cfg(feature = "unstable-v2")]
#[derive(Debug, Clone, PartialEq)]
pub enum MValue {
    Bool(Option<Vec<bool>>),
    I8(Option<Vec<i8>>),
    U8(Option<Vec<u8>>),
    I32(Option<Vec<i32>>),
    U32(Option<Vec<u32>>),
    I64(Option<Vec<i64>>),
    U64(Option<Vec<u64>>),
    F32(Option<Vec<f32>>),
    F64(Option<Vec<f64>>),
    Str(Option<Vec<String>>),
}

#[cfg(feature = "unstable-v2")]
impl MValue {
    #[must_use]
    pub fn is_null(&self) -> bool {
        self.count().is_none()
    }
}

/// A borrowed, non-null per-feature property value.
///
/// Nullability is lifted to [`ColumnRef`](crate::decoder::ColumnRef): only non-null values appear in
/// [`FeatureRef::iter_properties`](crate::decoder::FeatureRef::iter_properties).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PropValueRef<'a> {
    Bool(bool),
    I8(i8),
    U8(u8),
    I32(i32),
    U32(u32),
    I64(i64),
    U64(u64),
    F32(f32),
    F64(f64),
    Str(&'a str),
}

impl From<bool> for PropValueRef<'_> {
    fn from(v: bool) -> Self {
        Self::Bool(v)
    }
}
impl From<i8> for PropValueRef<'_> {
    fn from(v: i8) -> Self {
        Self::I8(v)
    }
}
impl From<u8> for PropValueRef<'_> {
    fn from(v: u8) -> Self {
        Self::U8(v)
    }
}
impl From<i32> for PropValueRef<'_> {
    fn from(v: i32) -> Self {
        Self::I32(v)
    }
}
impl From<u32> for PropValueRef<'_> {
    fn from(v: u32) -> Self {
        Self::U32(v)
    }
}
impl From<i64> for PropValueRef<'_> {
    fn from(v: i64) -> Self {
        Self::I64(v)
    }
}
impl From<u64> for PropValueRef<'_> {
    fn from(v: u64) -> Self {
        Self::U64(v)
    }
}
impl From<f32> for PropValueRef<'_> {
    fn from(v: f32) -> Self {
        Self::F32(v)
    }
}
impl From<f64> for PropValueRef<'_> {
    fn from(v: f64) -> Self {
        Self::F64(v)
    }
}

impl<'a> From<&'a str> for PropValueRef<'a> {
    fn from(v: &'a str) -> Self {
        Self::Str(v)
    }
}

macro_rules! kind_mappings {
    (
        scalar { $($sv:ident),* $(,)? }
        string { $($gv:ident),* $(,)? }
    ) => {
        /// The data type of one property or m-value column.
        #[derive(Debug, Clone, Copy, PartialEq, Eq, strum::IntoStaticStr)]
        #[strum(serialize_all = "lowercase")]
        pub enum PropKind {
            $($sv,)*
            $($gv,)*
        }

        impl From<&PropValue> for PropKind {
            fn from(prop: &PropValue) -> Self {
                match prop {
                    $(PropValue::$sv(_) => Self::$sv,)*
                    $(PropValue::$gv(_) => Self::$gv,)*
                }
            }
        }

        impl PropValue {
            #[must_use]
            pub fn is_null(&self) -> bool {
                match self {
                    $(Self::$sv(v) => v.is_none(),)*
                    $(Self::$gv(v) => v.is_none(),)*
                }
            }

            /// The null value of `kind`.
            #[must_use]
            pub fn null(kind: PropKind) -> Self {
                match kind {
                    $(PropKind::$sv => Self::$sv(None),)*
                    $(PropKind::$gv => Self::$gv(None),)*
                }
            }

            /// The value, borrowed, or [`None`] for a null.
            #[inline]
            #[must_use]
            pub fn value_ref(&self) -> Option<PropValueRef<'_>> {
                match self {
                    $(Self::$sv(v) => v.map(PropValueRef::$sv),)*
                    $(Self::$gv(v) => v.as_deref().map(PropValueRef::$gv),)*
                }
            }
        }

        impl PropValueRef<'_> {
            /// The [`PropKind`] of this value.
            #[inline]
            #[must_use]
            pub fn kind(self) -> PropKind {
                match self {
                    $(Self::$sv(_) => PropKind::$sv,)*
                    $(Self::$gv(_) => PropKind::$gv,)*
                }
            }
        }

        impl From<PropValueRef<'_>> for PropValue {
            fn from(value: PropValueRef<'_>) -> Self {
                match value {
                    $(PropValueRef::$sv(v) => Self::$sv(Some(v)),)*
                    $(PropValueRef::$gv(v) => Self::$gv(Some(v.to_owned())),)*
                }
            }
        }

        #[cfg(feature = "unstable-v2")]
        impl MValue {
            #[must_use]
            pub fn kind(&self) -> PropKind {
                match self {
                    $(Self::$sv(_) => PropKind::$sv,)*
                    $(Self::$gv(_) => PropKind::$gv,)*
                }
            }

            /// How many values this holds, or [`None`] when the feature carries none.
            #[must_use]
            pub fn count(&self) -> Option<usize> {
                match self {
                    $(Self::$sv(v) => v.as_ref().map(Vec::len),)*
                    $(Self::$gv(v) => v.as_ref().map(Vec::len),)*
                }
            }

            /// The empty value of `kind`, which is what a feature with no values holds.
            #[must_use]
            pub fn null(kind: PropKind) -> Self {
                match kind {
                    $(PropKind::$sv => Self::$sv(None),)*
                    $(PropKind::$gv => Self::$gv(None),)*
                }
            }

            /// Value `i` becomes the one at `order[i]`, unless the counts differ.
            pub(crate) fn reorder(&mut self, order: &[usize]) {
                fn run<T: Clone>(values: &mut Option<Vec<T>>, order: &[usize]) {
                    if let Some(v) = values
                        && v.len() == order.len()
                    {
                        *v = order.iter().map(|&i| v[i].clone()).collect();
                    }
                }
                match self {
                    $(Self::$sv(v) => run(v, order),)*
                    $(Self::$gv(v) => run(v, order),)*
                }
            }
        }
    };
}

with_kinds!(kind_mappings);

impl PropKind {
    fn is_signed_int(self) -> bool {
        matches!(self, Self::I8 | Self::I32 | Self::I64)
    }

    fn is_unsigned_int(self) -> bool {
        matches!(self, Self::U8 | Self::U32 | Self::U64)
    }

    fn is_float(self) -> bool {
        matches!(self, Self::F32 | Self::F64)
    }

    /// The kind of a column holding values of both kinds.
    ///
    /// Equal kinds stay as they are, signed integers widen to `I64`, unsigned integers to `U64`,
    /// signed mixed with unsigned to `I64`, floats to `F64`, integers mixed with floats to `F64`,
    /// and any other mix to `Str`.
    #[must_use]
    pub fn merge(self, other: Self) -> Self {
        let int = |k: Self| k.is_signed_int() || k.is_unsigned_int();
        if self == other {
            self
        } else if int(self) && int(other) {
            if self.is_unsigned_int() && other.is_unsigned_int() {
                Self::U64
            } else {
                Self::I64
            }
        } else if (self.is_float() || int(self)) && (other.is_float() || int(other)) {
            Self::F64
        } else {
            Self::Str
        }
    }
}

impl PropValueRef<'_> {
    fn as_int(self) -> Option<i128> {
        match self {
            Self::I8(v) => Some(i128::from(v)),
            Self::U8(v) => Some(i128::from(v)),
            Self::I32(v) => Some(i128::from(v)),
            Self::U32(v) => Some(i128::from(v)),
            Self::I64(v) => Some(i128::from(v)),
            Self::U64(v) => Some(i128::from(v)),
            Self::Bool(_) | Self::F32(_) | Self::F64(_) | Self::Str(_) => None,
        }
    }

    /// `self` as a value of column kind `kind`.
    ///
    /// A matching kind is kept, an integer goes to another integer kind when it is in range,
    /// a float or integer goes to `F64` as an `as f64` cast, and anything goes to `Str` as its text.
    /// Every other conversion, and an out-of-range integer, gives the null of `kind`.
    #[must_use]
    pub fn coerce(self, kind: PropKind) -> PropValue {
        if self.kind() == kind {
            return self.into();
        }
        let int = self.as_int();
        #[expect(
            clippy::cast_precision_loss,
            reason = "integers in a float column are taken as floats"
        )]
        match kind {
            PropKind::I8 => PropValue::I8(int.and_then(|v| i8::try_from(v).ok())),
            PropKind::U8 => PropValue::U8(int.and_then(|v| u8::try_from(v).ok())),
            PropKind::I32 => PropValue::I32(int.and_then(|v| i32::try_from(v).ok())),
            PropKind::U32 => PropValue::U32(int.and_then(|v| u32::try_from(v).ok())),
            PropKind::I64 => PropValue::I64(int.and_then(|v| i64::try_from(v).ok())),
            PropKind::U64 => PropValue::U64(int.and_then(|v| u64::try_from(v).ok())),
            PropKind::F64 => PropValue::F64(match self {
                Self::F32(v) => Some(f64::from(v)),
                Self::F64(v) => Some(v),
                Self::Bool(_) | Self::Str(_) => None,
                Self::I8(_)
                | Self::U8(_)
                | Self::I32(_)
                | Self::U32(_)
                | Self::I64(_)
                | Self::U64(_) => int.map(|v| v as f64),
            }),
            PropKind::Str => PropValue::Str(Some(match self {
                Self::Bool(v) => v.to_string(),
                Self::I8(v) => v.to_string(),
                Self::U8(v) => v.to_string(),
                Self::I32(v) => v.to_string(),
                Self::U32(v) => v.to_string(),
                Self::I64(v) => v.to_string(),
                Self::U64(v) => v.to_string(),
                Self::F32(v) => v.to_string(),
                Self::F64(v) => v.to_string(),
                Self::Str(v) => v.to_string(),
            })),
            PropKind::Bool | PropKind::F32 => PropValue::null(kind),
        }
    }
}

/// How many of a ring's coordinates MLT stores, which is all of them but a closing one.
pub(crate) fn stored_ring_len(ring: &LineString<i32>) -> usize {
    stored_len(&ring.0)
}

/// [`stored_ring_len`] of a ring given as its coordinates.
pub(crate) fn stored_len(coords: &[Coord<i32>]) -> usize {
    if coords.len() > 1 && coords.last() == coords.first() {
        coords.len() - 1
    } else {
        coords.len()
    }
}

/// How many vertices a geometry contributes to the layer's vertex sequence, which
/// is what an m-value column holds one value per.
#[cfg(feature = "unstable-v2")]
pub(crate) fn stored_vertex_count(geom: &Geometry<i32>) -> usize {
    let polygon = |poly: &geo_types::Polygon<i32>| {
        std::iter::once(poly.exterior())
            .chain(poly.interiors())
            .map(stored_ring_len)
            .sum::<usize>()
    };
    match geom {
        Geometry::Point(_) => 1,
        Geometry::Line(_) => 2,
        Geometry::LineString(ls) => ls.0.len(),
        Geometry::Polygon(p) => polygon(p),
        Geometry::MultiPoint(mp) => mp.0.len(),
        Geometry::MultiLineString(mls) => mls.iter().map(|ls| ls.0.len()).sum(),
        Geometry::MultiPolygon(mp) => mp.iter().map(polygon).sum(),
        Geometry::Triangle(t) => polygon(&t.to_polygon()),
        Geometry::Rect(r) => polygon(&r.to_polygon()),
        Geometry::GeometryCollection(gc) => gc.iter().map(stored_vertex_count).sum(),
    }
}

/// Check a nested column's shape: an interior root, no empty field set, and no
/// more levels than the wire allows.
#[cfg(feature = "unstable-v2")]
fn validate_nested_kind(name: &str, kind: &NestedKind) -> MltResult<()> {
    if matches!(kind, NestedKind::Leaf(_)) {
        return Err(MltError::NestedRootIsLeaf(name.to_string()));
    }
    if kind.has_empty_map() {
        return Err(MltError::EmptyStructNode);
    }
    let depth = kind.depth();
    if depth > MAX_NESTED_DEPTH {
        return Err(MltError::NestedTooDeep(depth));
    }
    Ok(())
}

pub(crate) fn validate_layer_name(name: &str) -> MltResult<()> {
    if name.is_empty() {
        Err(MltError::MissingLayerName)
    } else {
        Ok(())
    }
}

/// Reject a column name one of `taken` already holds, naming the kind of column that holds it.
///
/// A layer has one namespace of column names, so this is the only rule every named column obeys.
pub(crate) fn reject_taken_name<'a>(
    name: &str,
    role: ColumnRole,
    taken: impl IntoIterator<Item = (&'a str, ColumnRole)>,
) -> MltResult<()> {
    for (seen, taken_by) in taken {
        if seen == name {
            return Err(MltError::DuplicateColumnName {
                name: name.to_string(),
                role,
                taken_by,
            });
        }
    }
    Ok(())
}

/// Reject a name repeated within one list of column names.
fn validate_unique_names(names: &[String], role: ColumnRole) -> MltResult<()> {
    // Linear scan, not a HashSet: column counts are small, so this skips a per-layer alloc.
    // Empty names are allowed; real MVT tiles contain them.
    for (i, name) in names.iter().enumerate() {
        reject_taken_name(name, role, names[..i].iter().map(|n| (n.as_str(), role)))?;
    }
    Ok(())
}

/// One feature's values for one kind of column, so the checks over them are written once.
trait FeatureColumn: Sized {
    /// This kind of column's values, one per column of the layer.
    fn of(feature: &TileFeature) -> &[Self];
    fn column_kind(&self) -> PropKind;
    /// A feature holding a different number of values than the layer has columns.
    fn count_mismatch(expected: usize, actual: usize) -> MltError;
    /// A value of a different kind than its column.
    fn kind_mismatch(index: usize, expected: PropKind, actual: PropKind) -> MltError;
}

impl FeatureColumn for PropValue {
    fn of(feature: &TileFeature) -> &[Self] {
        &feature.properties
    }

    fn column_kind(&self) -> PropKind {
        PropKind::from(self)
    }

    fn count_mismatch(expected: usize, actual: usize) -> MltError {
        MltError::PropertyLengthMismatch { expected, actual }
    }

    fn kind_mismatch(index: usize, expected: PropKind, actual: PropKind) -> MltError {
        MltError::PropertyKindMismatch {
            index,
            expected,
            actual,
        }
    }
}

#[cfg(feature = "unstable-v2")]
impl FeatureColumn for MValue {
    fn of(feature: &TileFeature) -> &[Self] {
        &feature.m_values
    }

    fn column_kind(&self) -> PropKind {
        self.kind()
    }

    fn count_mismatch(expected: usize, actual: usize) -> MltError {
        MltError::MValueColumnCountMismatch { expected, actual }
    }

    fn kind_mismatch(index: usize, expected: PropKind, actual: PropKind) -> MltError {
        MltError::MValueKindMismatch {
            index,
            expected,
            actual,
        }
    }
}

/// The kind of each column, taken from the features that carry values for it.
///
/// A column no feature carries a value for is [`PropKind::Str`].
fn infer_kinds<T: FeatureColumn>(
    names: &[String],
    features: &[TileFeature],
) -> MltResult<Vec<PropKind>> {
    let mut kinds = vec![None; names.len()];
    for feature in features {
        let values = T::of(feature);
        if values.len() != names.len() {
            return Err(T::count_mismatch(names.len(), values.len()));
        }
        for (index, value) in values.iter().enumerate() {
            let actual = value.column_kind();
            match kinds[index] {
                Some(expected) if expected != actual => {
                    return Err(T::kind_mismatch(index, expected, actual));
                }
                None => kinds[index] = Some(actual),
                _ => {}
            }
        }
    }
    Ok(kinds
        .into_iter()
        .map(|kind| kind.unwrap_or(PropKind::Str))
        .collect())
}

/// Check a feature's values for one kind of column: one per column, of the column's kind.
fn validate_kinds<T: FeatureColumn>(kinds: &[PropKind], feature: &TileFeature) -> MltResult<()> {
    let values = T::of(feature);
    if values.len() != kinds.len() {
        return Err(T::count_mismatch(kinds.len(), values.len()));
    }
    for (index, (value, expected)) in values.iter().zip(kinds).enumerate() {
        let actual = value.column_kind();
        if actual != *expected {
            return Err(T::kind_mismatch(index, *expected, actual));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use geo_types::{Geometry, Point};
    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case::same_kind(PropKind::I32, PropKind::I32, PropKind::I32)]
    #[case::signed_widths(PropKind::I8, PropKind::I32, PropKind::I64)]
    #[case::unsigned_widths(PropKind::U8, PropKind::U32, PropKind::U64)]
    #[case::signed_with_unsigned(PropKind::I64, PropKind::U64, PropKind::I64)]
    #[case::small_signed_with_unsigned(PropKind::I8, PropKind::U8, PropKind::I64)]
    #[case::floats(PropKind::F32, PropKind::F64, PropKind::F64)]
    #[case::int_with_float(PropKind::U8, PropKind::F32, PropKind::F64)]
    #[case::bool_with_int(PropKind::Bool, PropKind::I64, PropKind::Str)]
    #[case::str_with_float(PropKind::Str, PropKind::F64, PropKind::Str)]
    fn merge_kinds(#[case] a: PropKind, #[case] b: PropKind, #[case] expected: PropKind) {
        assert_eq!(a.merge(b), expected);
        assert_eq!(b.merge(a), expected);
    }

    #[rstest]
    #[case::same_kind(PropValueRef::I32(-3), PropKind::I32, PropValue::I32(Some(-3)))]
    #[case::widen_signed(PropValueRef::I8(-3), PropKind::I64, PropValue::I64(Some(-3)))]
    #[case::unsigned_into_signed(PropValueRef::U64(7), PropKind::I64, PropValue::I64(Some(7)))]
    #[case::u64_above_i64_max(PropValueRef::U64(u64::MAX), PropKind::I64, PropValue::I64(None))]
    #[case::negative_into_u64(PropValueRef::I64(-1), PropKind::U64, PropValue::U64(None))]
    #[case::narrowing_out_of_range(PropValueRef::I32(300), PropKind::U8, PropValue::U8(None))]
    #[case::f32_into_f64(PropValueRef::F32(0.5), PropKind::F64, PropValue::F64(Some(0.5)))]
    #[case::int_into_f64(PropValueRef::I64(-2), PropKind::F64, PropValue::F64(Some(-2.0)))]
    #[case::u64_into_f64(PropValueRef::U64(1 << 63), PropKind::F64, PropValue::F64(Some(9.223_372_036_854_776e18)))]
    #[case::bool_into_str(PropValueRef::Bool(true), PropKind::Str, PropValue::Str(Some("true".into())))]
    #[case::int_into_str(PropValueRef::I64(-12), PropKind::Str, PropValue::Str(Some("-12".into())))]
    #[case::f32_into_str(PropValueRef::F32(1.5), PropKind::Str, PropValue::Str(Some("1.5".into())))]
    #[case::str_into_str(PropValueRef::Str("a"), PropKind::Str, PropValue::Str(Some("a".into())))]
    #[case::str_into_int(PropValueRef::Str("1"), PropKind::I64, PropValue::I64(None))]
    #[case::f64_into_f32(PropValueRef::F64(1.0), PropKind::F32, PropValue::F32(None))]
    #[case::u8_into_i8(PropValueRef::U8(127), PropKind::I8, PropValue::I8(Some(127)))]
    #[case::u32_into_i32(PropValueRef::U32(7), PropKind::I32, PropValue::I32(Some(7)))]
    #[case::i32_into_u32(PropValueRef::I32(7), PropKind::U32, PropValue::U32(Some(7)))]
    #[case::i8_into_f64(PropValueRef::I8(-1), PropKind::F64, PropValue::F64(Some(-1.0)))]
    #[case::u8_into_f64(PropValueRef::U8(1), PropKind::F64, PropValue::F64(Some(1.0)))]
    #[case::i32_into_f64(PropValueRef::I32(-3), PropKind::F64, PropValue::F64(Some(-3.0)))]
    #[case::u32_into_f64(PropValueRef::U32(3), PropKind::F64, PropValue::F64(Some(3.0)))]
    #[case::bool_into_f64(PropValueRef::Bool(true), PropKind::F64, PropValue::F64(None))]
    #[case::str_into_f64(PropValueRef::Str("1.5"), PropKind::F64, PropValue::F64(None))]
    #[case::i8_into_str(PropValueRef::I8(-8), PropKind::Str, PropValue::Str(Some("-8".into())))]
    #[case::u8_into_str(PropValueRef::U8(8), PropKind::Str, PropValue::Str(Some("8".into())))]
    #[case::i32_into_str(PropValueRef::I32(-32), PropKind::Str, PropValue::Str(Some("-32".into())))]
    #[case::u32_into_str(PropValueRef::U32(32), PropKind::Str, PropValue::Str(Some("32".into())))]
    #[case::u64_into_str(PropValueRef::U64(64), PropKind::Str, PropValue::Str(Some("64".into())))]
    #[case::f64_into_str(PropValueRef::F64(0.25), PropKind::Str, PropValue::Str(Some("0.25".into())))]
    fn coerce_to_kind(
        #[case] value: PropValueRef<'_>,
        #[case] kind: PropKind,
        #[case] expected: PropValue,
    ) {
        assert_eq!(value.coerce(kind), expected);
    }

    #[rstest]
    #[case::bool(PropValueRef::Bool(true), PropKind::Bool, PropValue::Bool(Some(true)))]
    #[case::u8(PropValueRef::U8(1), PropKind::U8, PropValue::U8(Some(1)))]
    #[case::str(PropValueRef::Str("x"), PropKind::Str, PropValue::Str(Some("x".into())))]
    fn ref_into_owned_keeps_kind(
        #[case] value: PropValueRef<'_>,
        #[case] kind: PropKind,
        #[case] expected: PropValue,
    ) {
        assert_eq!(value.kind(), kind);
        assert_eq!(PropValue::from(value), expected);
    }

    fn point_feature(properties: Vec<PropValue>) -> TileFeature {
        TileFeature {
            id: None,
            geometry: Geometry::Point(Point::new(0, 0)),
            properties,
            #[cfg(feature = "unstable-v2")]
            m_values: Vec::new(),
            #[cfg(feature = "unstable-v2")]
            nested: Vec::new(),
            #[cfg(feature = "unstable-v2")]
            z: Vec::new(),
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
        assert_eq!(
            layer
                .add_property("name", PropKind::Str)
                .unwrap_err()
                .to_string(),
            "duplicate column name name: the property column repeats the property column"
        );
    }

    #[test]
    fn from_parts_allows_empty_property_name() {
        // Real MVT tiles contain empty keys.
        assert!(TileLayer::from_parts("layer", 4096, vec![String::new()], vec![]).is_ok());
    }

    #[test]
    fn from_parts_rejects_duplicate_property_name() {
        assert_eq!(
            TileLayer::from_parts("layer", 4096, vec!["dup".into(), "dup".into()], vec![])
                .unwrap_err()
                .to_string(),
            "duplicate column name dup: the property column repeats the property column"
        );
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

    #[cfg(feature = "unstable-v2")]
    #[test]
    fn push_feature_validates_m_value_count() {
        let mut layer = TileLayer::new("layer", 4096).unwrap();
        layer.add_m_value("m", PropKind::I32).unwrap();
        assert!(matches!(
            layer.push_feature(point_feature(vec![])),
            Err(MltError::MValueColumnCountMismatch {
                expected: 1,
                actual: 0
            })
        ));
    }

    #[test]
    fn with_id_is_new_with_that_id() {
        let geometry = Geometry::Point(Point::new(1, 2));
        let mut expected = TileFeature::new(geometry.clone());
        expected.id = Some(7);
        assert_eq!(TileFeature::with_id(geometry, 7), expected);
    }

    #[cfg(feature = "unstable-v2")]
    #[test]
    fn push_feature_validates_one_value_per_vertex() {
        let mut layer = TileLayer::new("layer", 4096).unwrap();
        layer.add_m_value("m", PropKind::I32).unwrap();
        let mut feature = point_feature(vec![]);
        feature.m_values = vec![MValue::I32(Some(vec![1, 2]))];
        assert!(matches!(
            layer.push_feature(feature),
            Err(MltError::MValueVertexCountMismatch {
                index: 0,
                expected: 1,
                actual: 2,
            })
        ));
    }

    #[cfg(feature = "unstable-v2")]
    #[test]
    fn a_polygon_does_not_count_its_closing_vertices() {
        let ring = |pts: &[(i32, i32)]| {
            let mut ls = LineString::new(pts.iter().map(|&(x, y)| Coord { x, y }).collect());
            ls.close();
            ls
        };
        let poly = geo_types::Polygon::new(
            ring(&[(0, 0), (0, 4), (4, 4), (4, 0)]),
            vec![ring(&[(1, 1), (1, 2), (2, 2)])],
        );
        let feature = TileFeature::new(Geometry::Polygon(poly));
        assert_eq!(feature.vertex_count(), 7);
    }

    #[cfg(feature = "unstable-v2")]
    #[test]
    fn a_shape_counts_its_root_as_the_first_level() {
        let kind = NestedKind::list(NestedKind::map([("a", NestedKind::Leaf(PropKind::I32))]));
        assert_eq!(kind.depth(), 3);
    }

    #[cfg(feature = "unstable-v2")]
    #[test]
    fn a_map_may_leave_out_a_field_but_not_add_one() {
        let kind = NestedKind::map([
            ("a", NestedKind::Leaf(PropKind::I32)),
            ("b", NestedKind::Leaf(PropKind::Str)),
        ]);
        assert!(kind.accepts(&NestedValue::map([(
            "a",
            NestedValue::Leaf(PropValue::I32(Some(1)))
        )])));
        assert!(!kind.accepts(&NestedValue::map([(
            "c",
            NestedValue::Leaf(PropValue::I32(Some(1)))
        )])));
        assert!(!kind.accepts(&NestedValue::map([(
            "a",
            NestedValue::Leaf(PropValue::Str(Some("x".into())))
        )])));
    }

    #[cfg(feature = "unstable-v2")]
    fn list_kind() -> NestedKind {
        NestedKind::list(NestedKind::Leaf(PropKind::I32))
    }

    #[cfg(feature = "unstable-v2")]
    #[test]
    fn add_m_value_rejects_a_name_a_property_has() {
        let mut layer = TileLayer::new("layer", 4096).unwrap();
        layer.add_property("n", PropKind::I32).unwrap();
        assert_eq!(
            layer
                .add_m_value("n", PropKind::I32)
                .unwrap_err()
                .to_string(),
            "duplicate column name n: the m-value column repeats the property column"
        );
    }

    #[cfg(feature = "unstable-v2")]
    #[test]
    fn add_m_value_rejects_a_name_a_nested_column_has() {
        let mut layer = TileLayer::new("layer", 4096).unwrap();
        layer.add_nested("n", list_kind()).unwrap();
        assert_eq!(
            layer
                .add_m_value("n", PropKind::I32)
                .unwrap_err()
                .to_string(),
            "duplicate column name n: the m-value column repeats the nested column"
        );
    }

    #[cfg(feature = "unstable-v2")]
    #[test]
    fn add_m_value_rejects_duplicate_names() {
        let mut layer = TileLayer::new("layer", 4096).unwrap();
        layer.add_m_value("n", PropKind::I32).unwrap();
        assert_eq!(
            layer
                .add_m_value("n", PropKind::I32)
                .unwrap_err()
                .to_string(),
            "duplicate column name n: the m-value column repeats the m-value column"
        );
    }

    #[cfg(feature = "unstable-v2")]
    #[test]
    fn add_property_rejects_a_name_an_m_value_has() {
        let mut layer = TileLayer::new("layer", 4096).unwrap();
        layer.add_m_value("n", PropKind::I32).unwrap();
        assert_eq!(
            layer
                .add_property("n", PropKind::I32)
                .unwrap_err()
                .to_string(),
            "duplicate column name n: the property column repeats the m-value column"
        );
    }

    #[cfg(feature = "unstable-v2")]
    #[test]
    fn add_property_rejects_a_name_a_nested_column_has() {
        let mut layer = TileLayer::new("layer", 4096).unwrap();
        layer.add_nested("n", list_kind()).unwrap();
        assert_eq!(
            layer
                .add_property("n", PropKind::I32)
                .unwrap_err()
                .to_string(),
            "duplicate column name n: the property column repeats the nested column"
        );
    }

    #[cfg(feature = "unstable-v2")]
    #[test]
    fn add_nested_rejects_a_name_a_property_has() {
        let mut layer = TileLayer::new("layer", 4096).unwrap();
        layer.add_property("n", PropKind::I32).unwrap();
        assert_eq!(
            layer.add_nested("n", list_kind()).unwrap_err().to_string(),
            "duplicate column name n: the nested column repeats the property column"
        );
    }

    #[cfg(feature = "unstable-v2")]
    #[test]
    fn add_nested_rejects_a_name_an_m_value_already_has() {
        let mut layer = TileLayer::new("layer", 4096).unwrap();
        layer.add_m_value("n", PropKind::I32).unwrap();
        assert_eq!(
            layer.add_nested("n", list_kind()).unwrap_err().to_string(),
            "duplicate column name n: the nested column repeats the m-value column"
        );
    }

    #[cfg(feature = "unstable-v2")]
    #[test]
    fn add_nested_rejects_duplicate_names() {
        let mut layer = TileLayer::new("layer", 4096).unwrap();
        layer.add_nested("n", list_kind()).unwrap();
        assert_eq!(
            layer.add_nested("n", list_kind()).unwrap_err().to_string(),
            "duplicate column name n: the nested column repeats the nested column"
        );
    }

    #[cfg(feature = "unstable-v2")]
    #[test]
    fn set_nested_rejects_a_value_of_another_shape() {
        let mut layer = TileLayer::new("layer", 4096).unwrap();
        let key = layer
            .add_nested("n", NestedKind::list(NestedKind::Leaf(PropKind::I32)))
            .unwrap();
        let mut feature = point_feature(vec![]);
        feature.nested = vec![NestedValue::List(None)];
        assert_eq!(
            feature
                .set_nested(key, NestedValue::Map(None))
                .unwrap_err()
                .to_string(),
            "nested column 0 was given a value of another shape"
        );
    }

    #[test]
    fn from_parts_rejects_a_feature_with_another_property_count() {
        let features = vec![point_feature(vec![PropValue::I32(Some(1))])];
        assert!(matches!(
            TileLayer::from_parts("layer", 4096, vec!["a".into(), "b".into()], features),
            Err(MltError::PropertyLengthMismatch {
                expected: 2,
                actual: 1
            })
        ));
    }

    #[test]
    fn set_property_rejects_an_unknown_key() {
        let mut layer = TileLayer::new("layer", 4096).unwrap();
        let key = layer.add_property("a", PropKind::I32).unwrap();
        let mut feature = point_feature(vec![]);
        assert!(matches!(
            feature.set_property(key, PropValue::I32(Some(1))),
            Err(MltError::UnknownProperty { index: 0, count: 0 })
        ));
    }

    #[test]
    fn set_property_rejects_a_value_of_another_kind() {
        let mut layer = TileLayer::new("layer", 4096).unwrap();
        let key = layer.add_property("a", PropKind::I32).unwrap();
        let mut feature = point_feature(vec![PropValue::I32(None)]);
        assert!(matches!(
            feature.set_property(key, PropValue::Str(Some("x".into()))),
            Err(MltError::PropertyKindMismatch {
                index: 0,
                expected: PropKind::I32,
                actual: PropKind::Str,
            })
        ));
    }

    #[cfg(feature = "unstable-v2")]
    #[rstest]
    #[case::millimetre(-3, "1 mm")]
    #[case::centimetre(-2, "1 cm")]
    #[case::decimetre(-1, "1 dm")]
    #[case::metre(0, "1 m")]
    #[case::ten_metres(1, "10 m")]
    #[case::hundred_metres(2, "100 m")]
    #[case::kilometre(3, "1 km")]
    #[case::ten_kilometres(4, "10 km")]
    fn a_z_step_reads_as_the_spec_names_it(#[case] exponent: i8, #[case] name: &str) {
        assert_eq!(ZStep::new(exponent).unwrap().to_string(), name);
    }

    #[cfg(feature = "unstable-v2")]
    #[rstest]
    #[case::line(Geometry::Line(geo_types::Line::new((0, 0), (1, 1))), 2)]
    #[case::triangle(Geometry::Triangle(geo_types::Triangle::new((0, 0).into(), (1, 0).into(), (0, 1).into())), 3)]
    #[case::rect(Geometry::Rect(geo_types::Rect::new((0, 0), (2, 2))), 4)]
    #[case::collection(
        Geometry::GeometryCollection(geo_types::GeometryCollection(vec![
            Geometry::Point(Point::new(0, 0)),
            Geometry::Line(geo_types::Line::new((0, 0), (1, 1))),
        ])),
        3
    )]
    fn a_geometry_counts_the_vertices_it_stores(
        #[case] geometry: Geometry<i32>,
        #[case] vertices: usize,
    ) {
        assert_eq!(TileFeature::new(geometry).vertex_count(), vertices);
    }

    #[cfg(feature = "unstable-v2")]
    #[test]
    fn an_m_value_with_no_values_is_null() {
        assert!(MValue::I32(None).is_null());
        assert!(!MValue::I32(Some(vec![])).is_null());
    }

    #[cfg(feature = "unstable-v2")]
    #[test]
    fn add_m_value_gives_existing_features_a_null() {
        let mut layer = TileLayer::new("layer", 4096).unwrap();
        layer.push_feature(point_feature(vec![])).unwrap();
        layer.add_m_value("m", PropKind::I32).unwrap();
        assert_eq!(layer.features()[0].m_values(), [MValue::I32(None)]);
    }

    #[cfg(feature = "unstable-v2")]
    #[test]
    fn add_nested_gives_existing_features_a_null() {
        let mut layer = TileLayer::new("layer", 4096).unwrap();
        layer.push_feature(point_feature(vec![])).unwrap();
        layer.add_nested("n", list_kind()).unwrap();
        assert_eq!(layer.features()[0].nested(), [NestedValue::List(None)]);
    }

    #[cfg(feature = "unstable-v2")]
    #[test]
    fn push_feature_validates_m_value_kind() {
        let mut layer = TileLayer::new("layer", 4096).unwrap();
        layer.add_m_value("m", PropKind::I32).unwrap();
        let mut feature = point_feature(vec![]);
        feature.m_values = vec![MValue::Str(None)];
        assert!(matches!(
            layer.push_feature(feature),
            Err(MltError::MValueKindMismatch {
                index: 0,
                expected: PropKind::I32,
                actual: PropKind::Str,
            })
        ));
    }

    #[cfg(feature = "unstable-v2")]
    #[test]
    fn push_feature_validates_nested_count() {
        let mut layer = TileLayer::new("layer", 4096).unwrap();
        layer.add_nested("n", list_kind()).unwrap();
        assert!(matches!(
            layer.push_feature(point_feature(vec![])),
            Err(MltError::NestedColumnCountMismatch {
                expected: 1,
                actual: 0
            })
        ));
    }

    #[cfg(feature = "unstable-v2")]
    #[test]
    fn push_feature_validates_nested_shape() {
        let mut layer = TileLayer::new("layer", 4096).unwrap();
        layer.add_nested("n", list_kind()).unwrap();
        let mut feature = point_feature(vec![]);
        feature.nested = vec![NestedValue::Map(None)];
        assert_eq!(
            layer.push_feature(feature).unwrap_err().to_string(),
            "nested column 0 (n) was given a value of another shape"
        );
    }

    #[cfg(feature = "unstable-v2")]
    #[test]
    fn set_m_value_rejects_an_unknown_key() {
        let mut layer = TileLayer::new("layer", 4096).unwrap();
        let key = layer.add_m_value("m", PropKind::I32).unwrap();
        let mut feature = point_feature(vec![]);
        assert!(matches!(
            feature.set_m_value(key, MValue::I32(Some(vec![1]))),
            Err(MltError::UnknownMValueKey {
                index: 0,
                columns: 0
            })
        ));
    }

    #[cfg(feature = "unstable-v2")]
    #[test]
    fn set_nested_rejects_an_unknown_key() {
        let mut layer = TileLayer::new("layer", 4096).unwrap();
        let key = layer.add_nested("n", list_kind()).unwrap();
        let mut feature = point_feature(vec![]);
        assert!(matches!(
            feature.set_nested(key, NestedValue::List(None)),
            Err(MltError::UnknownNestedKey {
                index: 0,
                columns: 0
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
