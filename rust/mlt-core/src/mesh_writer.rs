//! A layer of 3D meshes written one feature at a time, then encoded as a v2 `Meshes` layer.

use fast_mvt::{MvtFeatureBuilder, MvtLayerBuilder};
use geo_types::Coord;

use crate::decoder::{GeometryValues, MValueScope};
use crate::encoder::{EncoderConfig, LayerSource, StagedValues, V2Layout, encode_layer};
use crate::layer_writer::{Columns, item_range, len32};
use crate::tile::{
    Extent, MValueKey, Mesh, PropKind, PropertyKey, ZAxis, mesh_fault, validate_layer_name,
};
use crate::{MltError, MltResult, PropValueRef};

/// A layer of meshes written feature by feature, straight from borrowed values.
///
/// Names and string values are borrowed for `'a`.
/// [`MeshLayerWriter::reset`] keeps the buffers for the next layer.
/// A [`PropertyKey`] or [`MValueKey`] means nothing after [`MeshLayerWriter::reset`].
///
/// ```
/// use mlt_core::encoder::{EncoderConfig, Network, Profile, WireVersion};
/// use mlt_core::{Mesh, MeshLayerWriter, MValueScope, PropKind, ZStep};
///
/// let mut layer = MeshLayerWriter::new("landmarks", 4096, ZStep::new(-2)?)?;
/// let name = layer.add_property("name", PropKind::Str)?;
/// let part = layer.add_m_value("part", PropKind::Str, MValueScope::Triangle)?;
/// let mesh = Mesh::new(vec![[0, 0, 0], [0, 10, 0], [10, 0, 500]], vec![[0, 1, 2]])?;
/// let mut feature = layer.feature(&mesh);
/// feature.id(Some(7)).property(name, "Kiosk")?.m_values(part, ["roof"])?;
/// feature.finish()?;
/// let cfg = EncoderConfig::from_profile(Profile::Native(Network::Cellular4G))
///     .with_wire_version(WireVersion::V02);
/// let mlt = layer.encode(cfg)?;
/// # Ok::<(), mlt_core::MltError>(())
/// ```
#[derive(Debug, Clone)]
pub struct MeshLayerWriter<'a> {
    name: &'a str,
    extent: Extent,
    z_axis: ZAxis,
    columns: Columns<'a>,
    meshes: Meshes,
}

/// Every feature's mesh, each level holding the cumulative end of each feature's run.
#[derive(Debug, Clone, Default)]
struct Meshes {
    vertices: Vec<[i32; 3]>,
    vertex_ends: Vec<u32>,
    triangles: Vec<[u32; 3]>,
    triangle_ends: Vec<u32>,
}

impl Meshes {
    fn clear(&mut self) {
        self.vertices.clear();
        self.vertex_ends.clear();
        self.triangles.clear();
        self.triangle_ends.clear();
    }

    fn push(&mut self, mesh: &Mesh) -> MltResult<()> {
        self.vertices.extend_from_slice(mesh.vertices());
        self.triangles.extend_from_slice(mesh.triangles());
        self.vertex_ends.push(len32(self.vertices.len())?);
        self.triangle_ends.push(len32(self.triangles.len())?);
        Ok(())
    }

    fn vertices(&self, feature: usize) -> &[[i32; 3]] {
        &self.vertices[item_range(&self.vertex_ends, feature)]
    }

    fn triangles(&self, feature: usize) -> &[[u32; 3]] {
        &self.triangles[item_range(&self.triangle_ends, feature)]
    }
}

impl<'a> MeshLayerWriter<'a> {
    /// An empty layer whose z read on `z_axis`.
    ///
    /// Relative to ground, a mesh's ground is the terrain at the centre of its `(x, y)` bounding box.
    /// A mesh on a slope should reach below `z = 0` down to the lowest terrain under it, so it does not float.
    pub fn new(name: &'a str, extent: u32, z_axis: impl Into<ZAxis>) -> MltResult<Self> {
        validate_layer_name(name)?;
        Ok(Self {
            name,
            extent: Extent::new(extent)?,
            z_axis: z_axis.into(),
            columns: Columns::default(),
            meshes: Meshes::default(),
        })
    }

    /// Starts over as an empty layer, keeping the buffers of this one.
    pub fn reset(&mut self, name: &'a str, extent: u32, z_axis: impl Into<ZAxis>) -> MltResult<()> {
        validate_layer_name(name)?;
        self.extent = Extent::new(extent)?;
        self.name = name;
        self.z_axis = z_axis.into();
        self.columns.clear();
        self.meshes.clear();
        Ok(())
    }

    /// Adds a property column, which the features written so far have no value in.
    pub fn add_property(&mut self, name: &'a str, kind: PropKind) -> MltResult<PropertyKey> {
        self.columns.add_property(name, kind)
    }

    /// Adds an m-value column, holding one value per vertex or per triangle as `scope` says,
    /// which the features written so far are null in.
    pub fn add_m_value(
        &mut self,
        name: &'a str,
        kind: PropKind,
        scope: MValueScope,
    ) -> MltResult<MValueKey> {
        self.columns.add_m_value(name, kind, scope)
    }

    /// Starts a feature holding `mesh`.
    /// It joins the layer when [`MeshFeatureWriter::finish`] succeeds, and is discarded otherwise.
    pub fn feature<'w>(&'w mut self, mesh: &'w Mesh) -> MeshFeatureWriter<'w, 'a> {
        MeshFeatureWriter {
            layer: self,
            mesh,
            id: None,
            finished: false,
        }
    }

    #[must_use]
    pub fn feature_count(&self) -> usize {
        self.columns.feature_count()
    }

    /// Encodes the layer as MLT, choosing every encoding as [`LayerWriter::encode`] does.
    ///
    /// [`LayerWriter::encode`]: crate::LayerWriter::encode
    pub fn encode(&self, cfg: EncoderConfig) -> MltResult<Vec<u8>> {
        encode_layer(self, cfg)
    }
}

/// A feature being written into a [`MeshLayerWriter`].
#[must_use = "call .finish() to add the feature to the layer"]
pub struct MeshFeatureWriter<'w, 'a> {
    layer: &'w mut MeshLayerWriter<'a>,
    mesh: &'w Mesh,
    id: Option<u64>,
    finished: bool,
}

impl<'a> MeshFeatureWriter<'_, 'a> {
    pub fn id(&mut self, id: Option<u64>) -> &mut Self {
        self.id = id;
        self
    }

    /// Sets the feature's value in the `key` column, which must be of the value's kind.
    pub fn property(
        &mut self,
        key: PropertyKey,
        value: impl Into<PropValueRef<'a>>,
    ) -> MltResult<&mut Self> {
        self.layer.columns.set_property(key, value.into())?;
        Ok(self)
    }

    /// Sets the feature's values in the `key` m-value column, one per vertex or per triangle of
    /// its mesh as the column's scope says, in the order the mesh lists them.
    ///
    /// A feature that sets none is null in the column, as is one whose values are rejected.
    pub fn m_values(
        &mut self,
        key: MValueKey,
        values: impl IntoIterator<Item = impl Into<PropValueRef<'a>>>,
    ) -> MltResult<&mut Self> {
        self.layer.columns.set_m_values(key, values)?;
        Ok(self)
    }

    /// Adds the feature to the layer, or discards it if its m-values do not fit its mesh.
    pub fn finish(mut self) -> MltResult<()> {
        let (vertices, triangles) = (self.mesh.vertices().len(), self.mesh.triangles().len());
        let layer = &mut *self.layer;
        layer.columns.check_m_values(|scope| match scope {
            MValueScope::Vertex => vertices,
            MValueScope::Triangle => triangles,
        })?;
        let columns = &layer.columns;
        if let Some(fault) = mesh_fault(self.mesh.vertices(), self.mesh.triangles(), |a, b| {
            columns.same_vertex_values(a, b)
        }) {
            return Err(MltError::InvalidMesh {
                feature: columns.feature_count(),
                fault,
            });
        }
        layer.columns.finish_feature(self.id)?;
        layer.meshes.push(self.mesh)?;
        self.finished = true;
        Ok(())
    }
}

impl Drop for MeshFeatureWriter<'_, '_> {
    fn drop(&mut self) {
        if !self.finished {
            self.layer.columns.discard_feature();
        }
    }
}

impl LayerSource for MeshLayerWriter<'_> {
    fn name(&self) -> &str {
        self.name
    }

    fn extent(&self) -> Extent {
        self.extent
    }

    fn feature_count(&self) -> usize {
        Self::feature_count(self)
    }

    fn property_count(&self) -> usize {
        self.columns.property_count()
    }

    fn property_name(&self, column: usize) -> &str {
        self.columns.property_name(column)
    }

    fn property_kinds(&self) -> &[PropKind] {
        self.columns.property_kinds()
    }

    fn id(&self, feature: usize) -> Option<u64> {
        self.columns.id(feature)
    }

    fn property(&self, feature: usize, column: usize) -> Option<PropValueRef<'_>> {
        self.columns.property(feature, column)
    }

    fn coords(&self) -> impl Iterator<Item = Coord<i32>> + '_ {
        self.meshes.vertices.iter().map(|&[x, y, _]| Coord { x, y })
    }

    fn first_coord(&self, feature: usize) -> Option<Coord<i32>> {
        let &[x, y, _] = self.meshes.vertices(feature).first()?;
        Some(Coord { x, y })
    }

    fn coord_count(&self, feature: usize) -> usize {
        self.meshes.vertices(feature).len()
    }

    fn push_geometry(&self, feature: usize, values: &mut GeometryValues) {
        values.push_mesh_parts(
            self.meshes.vertices(feature),
            self.meshes.triangles(feature),
        );
    }

    fn polygons(
        &self,
        _feature: usize,
    ) -> impl Iterator<Item = impl Iterator<Item = &[Coord<i32>]> + '_> + '_ {
        std::iter::empty::<std::iter::Empty<&[Coord<i32>]>>()
    }

    fn mvt_feature(
        &self,
        _feature: usize,
        _layer: MvtLayerBuilder,
    ) -> MltResult<MvtFeatureBuilder> {
        unreachable!("a MeshLayerWriter has no MVT export, since MVT holds no mesh")
    }

    fn v2_layout(&self) -> V2Layout<'_> {
        V2Layout {
            nested_names: &[],
            nested_kinds: &[],
            z_axis: Some(self.z_axis),
            meshes: true,
        }
    }

    fn m_value_kinds(&self) -> &[PropKind] {
        self.columns.m_value_kinds()
    }

    fn m_value_name(&self, column: usize) -> &str {
        self.columns.m_value_name(column)
    }

    fn m_value_scope(&self, column: usize) -> MValueScope {
        self.columns.m_value_scope(column)
    }

    fn push_m_values(&self, feature: usize, column: usize, values: &mut StagedValues) -> bool {
        self.columns.push_m_values(feature, column, None, values)
    }
}
