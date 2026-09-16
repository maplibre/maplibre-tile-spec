//! Owned, row-oriented tile model.
//!
//! This is the representation tiles are built in and converted to/from:
//! it is independent of the wire format, and is shared by the encoder, the MVT and
//! `GeoJSON` converters, and the language bindings.
//! The decoder's columnar types live in [`crate::decoder`].

#[cfg(feature = "unstable-v2")]
use std::collections::BTreeMap;
use std::num::NonZeroU32;

use geo_types::{Geometry, LineString};

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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PropertyKey(usize);

impl PropertyKey {
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

    fn validate_feature(&self, feature: &TileFeature) -> MltResult<()> {
        validate_kinds::<PropValue>(&self.property_kinds, feature)?;
        #[cfg(feature = "unstable-v2")]
        self.validate_m_values(feature)?;
        #[cfg(feature = "unstable-v2")]
        self.validate_nested(feature)?;
        Ok(())
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

    #[must_use]
    pub(crate) fn properties_mut(&mut self) -> &mut [PropValue] {
        &mut self.properties
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
        }
    };
}

with_kinds!(kind_mappings);

/// How many of a ring's coordinates MLT stores, which is all of them but a closing one.
pub(crate) fn stored_ring_len(ring: &LineString<i32>) -> usize {
    let coords = &ring.0;
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

fn validate_layer_name(name: &str) -> MltResult<()> {
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

    use super::*;

    fn point_feature(properties: Vec<PropValue>) -> TileFeature {
        TileFeature {
            id: None,
            geometry: Geometry::Point(Point::new(0, 0)),
            properties,
            #[cfg(feature = "unstable-v2")]
            m_values: Vec::new(),
            #[cfg(feature = "unstable-v2")]
            nested: Vec::new(),
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
            let mut ls = LineString::new(
                pts.iter()
                    .map(|&(x, y)| geo_types::Coord { x, y })
                    .collect(),
            );
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
