//! The encoder's view of a layer: its features one at a time, however they are stored.

use fast_mvt::{MvtFeatureBuilder, MvtLayerBuilder};
use geo::CoordsIter as _;
use geo_types::{Coord, Geometry};

use crate::decoder::GeometryValues;
use crate::encoder::geometry::coord_count;
#[cfg(feature = "unstable-v2")]
use crate::encoder::geometry::wound_vertex_order;
use crate::tile::{Extent, PropKind, TileLayer};
#[cfg(feature = "unstable-v2")]
use crate::tile::{MValue, NestedKind, NestedValue, ZStep};
use crate::{MltResult, PropValueRef};

/// Everything the encoder reads from a layer, feature by feature.
pub(crate) trait LayerSource {
    fn name(&self) -> &str;
    fn extent(&self) -> Extent;
    fn feature_count(&self) -> usize;
    /// How many property columns there are, which [`Self::property`] indexes.
    fn property_count(&self) -> usize;
    fn property_name(&self, column: usize) -> &str;
    fn property_kinds(&self) -> &[PropKind];
    fn id(&self, feature: usize) -> Option<u64>;
    /// The feature's value in `column`, or [`None`] when it has none there.
    fn property(&self, feature: usize, column: usize) -> Option<PropValueRef<'_>>;
    /// Every coordinate of every feature, in no particular order.
    fn coords(&self) -> impl Iterator<Item = Coord<i32>> + '_;
    fn first_coord(&self, feature: usize) -> Option<Coord<i32>>;
    /// How many coordinates the feature holds, to size the geometry buffers up front.
    fn coord_count(&self, feature: usize) -> usize;
    fn push_geometry(&self, feature: usize, geometry: &mut GeometryValues);
    /// Starts the feature in an MVT `layer`, with its geometry written.
    fn mvt_feature(&self, feature: usize, layer: MvtLayerBuilder) -> MltResult<MvtFeatureBuilder>;

    #[cfg(feature = "unstable-v2")]
    fn v2_layout(&self) -> V2Layout<'_> {
        V2Layout::default()
    }
    #[cfg(feature = "unstable-v2")]
    fn m_value(&self, _feature: usize, _column: usize) -> Option<&MValue> {
        None
    }
    #[cfg(feature = "unstable-v2")]
    fn nested(&self, _feature: usize, _column: usize) -> Option<&NestedValue> {
        None
    }
    /// The feature's z values, one per stored vertex.
    #[cfg(feature = "unstable-v2")]
    fn z(&self, _feature: usize) -> &[i32] {
        &[]
    }
    /// Where each stored vertex of the feature moves to when its rings are wound,
    /// or [`None`] when none does, which the values held per vertex must follow.
    #[cfg(feature = "unstable-v2")]
    fn vertex_order(&self, _feature: usize) -> Option<Vec<usize>> {
        None
    }
}

/// The columns only the v2 wire format has, which [`LayerSource::m_value`] and
/// [`LayerSource::nested`] index.
#[cfg(feature = "unstable-v2")]
#[derive(Default)]
pub(crate) struct V2Layout<'a> {
    pub(crate) m_value_names: &'a [String],
    pub(crate) m_value_kinds: &'a [PropKind],
    pub(crate) nested_names: &'a [String],
    pub(crate) nested_kinds: &'a [NestedKind],
    pub(crate) z_step: Option<ZStep>,
}

/// The order features are staged in: as stored, or a permutation of them.
pub(crate) enum Order {
    Stored(usize),
    Permuted(Vec<u32>),
}

impl Order {
    /// Feature indexes, in staging order.
    pub(crate) fn iter(&self) -> impl Iterator<Item = usize> + Clone + '_ {
        let (stored, permuted) = match self {
            Self::Stored(len) => (0..*len, [].iter()),
            Self::Permuted(permutation) => (0..0, permutation.iter()),
        };
        stored.chain(permuted.map(|&f| f as usize))
    }
}

impl LayerSource for TileLayer {
    fn name(&self) -> &str {
        &self.name
    }

    fn extent(&self) -> Extent {
        self.extent
    }

    fn feature_count(&self) -> usize {
        self.features.len()
    }

    fn property_count(&self) -> usize {
        self.property_names.len()
    }

    fn property_name(&self, column: usize) -> &str {
        &self.property_names[column]
    }

    fn property_kinds(&self) -> &[PropKind] {
        &self.property_kinds
    }

    #[inline]
    fn id(&self, feature: usize) -> Option<u64> {
        self.features[feature].id
    }

    #[inline]
    fn property(&self, feature: usize, column: usize) -> Option<PropValueRef<'_>> {
        self.features[feature]
            .properties
            .get(column)
            .and_then(crate::PropValue::value_ref)
    }

    fn coords(&self) -> impl Iterator<Item = Coord<i32>> + '_ {
        self.features.iter().flat_map(|f| f.geometry.coords_iter())
    }

    #[inline]
    fn first_coord(&self, feature: usize) -> Option<Coord<i32>> {
        first_vertex(&self.features[feature].geometry)
    }

    #[inline]
    fn coord_count(&self, feature: usize) -> usize {
        coord_count(&self.features[feature].geometry)
    }

    fn push_geometry(&self, feature: usize, geometry: &mut GeometryValues) {
        geometry.push_geom(&self.features[feature].geometry);
    }

    fn mvt_feature(&self, feature: usize, layer: MvtLayerBuilder) -> MltResult<MvtFeatureBuilder> {
        Ok(layer.feature(&self.features[feature].geometry)?)
    }

    #[cfg(feature = "unstable-v2")]
    fn v2_layout(&self) -> V2Layout<'_> {
        V2Layout {
            m_value_names: &self.m_value_names,
            m_value_kinds: &self.m_value_kinds,
            nested_names: &self.nested_names,
            nested_kinds: &self.nested_kinds,
            z_step: self.z_step,
        }
    }

    #[cfg(feature = "unstable-v2")]
    fn m_value(&self, feature: usize, column: usize) -> Option<&MValue> {
        self.features[feature].m_values.get(column)
    }

    #[cfg(feature = "unstable-v2")]
    fn nested(&self, feature: usize, column: usize) -> Option<&NestedValue> {
        self.features[feature].nested.get(column)
    }

    #[cfg(feature = "unstable-v2")]
    fn z(&self, feature: usize) -> &[i32] {
        &self.features[feature].z
    }

    #[cfg(feature = "unstable-v2")]
    fn vertex_order(&self, feature: usize) -> Option<Vec<usize>> {
        wound_vertex_order(&self.features[feature].geometry)
    }
}

/// The first coordinate of a geometry, which is what spatial sorts order features by.
fn first_vertex(geom: &Geometry<i32>) -> Option<Coord<i32>> {
    match geom {
        Geometry::<i32>::Point(p) => Some(p.0),
        Geometry::<i32>::Line(l) => Some(l.start),
        Geometry::<i32>::LineString(ls) => ls.0.first().copied(),
        Geometry::<i32>::Polygon(p) => p.exterior().0.first().copied(),
        Geometry::<i32>::MultiPoint(mp) => mp.0.first().map(|p| p.0),
        Geometry::<i32>::MultiLineString(mls) => mls.0.first().and_then(|ls| ls.0.first().copied()),
        Geometry::<i32>::MultiPolygon(mp) => {
            mp.0.first().and_then(|p| p.exterior().0.first().copied())
        }
        Geometry::<i32>::Triangle(t) => Some(t.v1()),
        Geometry::<i32>::Rect(r) => Some(r.min()),
        Geometry::<i32>::GeometryCollection(gc) => gc.0.first().and_then(first_vertex),
    }
}
