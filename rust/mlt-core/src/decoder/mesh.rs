//! Owned read model of a v2 `Meshes` layer, one 3D triangle mesh per feature.

use crate::decoder::MValueScope;
use crate::tile::{Extent, MValue, Mesh, NestedKind, NestedValue, PropKind, PropValue, ZAxis};

/// A decoded `Meshes` layer in row form, which [`crate::Layer::into_meshes`] returns.
#[derive(Debug, Clone, PartialEq)]
pub struct MeshLayer {
    pub(crate) name: String,
    pub(crate) extent: Extent,
    /// How every mesh's vertex z reads as a height.
    pub(crate) z_axis: ZAxis,
    /// Column names, parallel to [`MeshFeature::properties`].
    pub(crate) property_names: Vec<String>,
    pub(crate) property_kinds: Vec<PropKind>,
    /// Column names, parallel to [`MeshFeature::m_values`].
    pub(crate) m_value_names: Vec<String>,
    pub(crate) m_value_kinds: Vec<PropKind>,
    pub(crate) m_value_scopes: Vec<MValueScope>,
    /// Column names, parallel to [`MeshFeature::nested`].
    pub(crate) nested_names: Vec<String>,
    pub(crate) nested_kinds: Vec<NestedKind>,
    pub(crate) features: Vec<MeshFeature>,
}

/// One feature of a [`MeshLayer`].
#[derive(Debug, Clone, PartialEq)]
pub struct MeshFeature {
    pub(crate) id: Option<u64>,
    pub(crate) properties: Vec<PropValue>,
    pub(crate) mesh: Mesh,
    /// One entry per m-value column, holding a value per vertex or per triangle as the column's scope says.
    pub(crate) m_values: Vec<MValue>,
    pub(crate) nested: Vec<NestedValue>,
}

impl MeshLayer {
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[must_use]
    pub fn extent(&self) -> Extent {
        self.extent
    }

    /// How every vertex z reads as a height.
    ///
    /// Relative to ground, a mesh's ground is the terrain at the centre of its `(x, y)` bounding box.
    #[must_use]
    pub fn z_axis(&self) -> ZAxis {
        self.z_axis
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
    pub fn m_value_names(&self) -> &[String] {
        &self.m_value_names
    }

    #[must_use]
    pub fn m_value_kinds(&self) -> &[PropKind] {
        &self.m_value_kinds
    }

    /// Whether each m-value column holds a value per vertex or per triangle.
    #[must_use]
    pub fn m_value_scopes(&self) -> &[MValueScope] {
        &self.m_value_scopes
    }

    #[must_use]
    pub fn nested_names(&self) -> &[String] {
        &self.nested_names
    }

    #[must_use]
    pub fn nested_kinds(&self) -> &[NestedKind] {
        &self.nested_kinds
    }

    #[must_use]
    pub fn features(&self) -> &[MeshFeature] {
        &self.features
    }

    #[must_use]
    pub fn feature_count(&self) -> usize {
        self.features.len()
    }
}

impl MeshFeature {
    #[must_use]
    pub fn id(&self) -> Option<u64> {
        self.id
    }

    /// One value per property column, in the order of [`MeshLayer::property_names`].
    #[must_use]
    pub fn properties(&self) -> &[PropValue] {
        &self.properties
    }

    /// The mesh, its triangle corners indexing its own vertices.
    #[must_use]
    pub fn mesh(&self) -> &Mesh {
        &self.mesh
    }

    /// One value per m-value column, in the order of [`MeshLayer::m_value_names`].
    /// A triangle-scoped value runs over [`Mesh::triangles`] in their order.
    #[must_use]
    pub fn m_values(&self) -> &[MValue] {
        &self.m_values
    }

    /// One value per nested column, in the order of [`MeshLayer::nested_names`].
    #[must_use]
    pub fn nested(&self) -> &[NestedValue] {
        &self.nested
    }
}
