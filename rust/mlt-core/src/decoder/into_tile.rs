//! Conversion from the decoded columnar form into the row-oriented
//! [`crate::tile`] model.
//!
//! [`ParsedLayer01::into_tile`] materializes one [`TileFeature`] per map
//! feature, owning its geometry and property values outright. That is the form
//! the optimizer, the sorting pipeline, and the converters work in.

#[cfg(feature = "unstable-v2")]
use crate::MltError;
use crate::decoder::{FeatureRef, Layer, Layer01, ParsedLayer, ParsedLayer01, ParsedProperty};
#[cfg(feature = "unstable-v2")]
use crate::decoder::{
    GeoLayout, GeometryValues, Layer02, MValueSpans, MeshFeature, MeshLayer, ParsedLayer02,
    ParsedMValue, ParsedNested,
};
use crate::errors::AsMltError as _;
#[cfg(feature = "unstable-v2")]
use crate::tile::{MValue, Mesh, NestedKind, NestedValue};
use crate::tile::{PropValue, TileFeature, TileLayer};
use crate::{Decoder, Lazy, LendingIterator, MltResult};

/// The parts of a [`TileLayer`] every wire version produces the same way.
struct TileParts {
    name: String,
    extent: u32,
    names: Vec<String>,
    features: Vec<TileFeature>,
}

impl TileParts {
    fn finish(self) -> MltResult<TileLayer> {
        TileLayer::from_parts(self.name, self.extent, self.names, self.features)
    }
}

impl ParsedLayer01<'_> {
    /// Decode and convert into a row-oriented [`TileLayer`], charging every
    /// heap allocation against `dec`.
    pub fn into_tile(self, dec: &mut Decoder) -> MltResult<TileLayer> {
        self.collect_parts(dec)?.finish()
    }

    /// One [`TileFeature`] per map feature, carrying only the columns every version
    /// has. Borrows, so a version that adds its own can fill them in afterwards.
    #[hotpath::measure]
    fn collect_parts(&self, dec: &mut Decoder) -> MltResult<TileParts> {
        let name = self.name().to_string();
        let extent = self.extent().get();
        let names: Vec<String> = self.iterate_prop_names().map(|n| n.to_string()).collect();
        let features = self.rows(dec, |feat, values| TileFeature {
            id: feat.id(),
            geometry: feat.geometry().clone(),
            properties: values,
            #[cfg(feature = "unstable-v2")]
            m_values: Vec::new(),
            #[cfg(feature = "unstable-v2")]
            nested: Vec::new(),
            #[cfg(feature = "unstable-v2")]
            z: Vec::new(),
        })?;
        Ok(TileParts {
            name,
            extent,
            names,
            features,
        })
    }

    /// One row per map feature, which `row` builds from the feature and its property values.
    fn rows<T>(
        &self,
        dec: &mut Decoder,
        mut row: impl FnMut(&FeatureRef<'_, '_>, Vec<PropValue>) -> T,
    ) -> MltResult<Vec<T>> {
        let col_nulls = typed_nulls(&self.properties);
        let mut rows = dec.alloc::<T>(self.feature_count())?;
        let mut feat_iter = self.iter_features();
        while let Some(feat) = feat_iter.next() {
            let feat = feat?;
            let mut values = dec.alloc::<PropValue>(col_nulls.len())?;
            for (col_idx, value) in feat.iter_all_properties().enumerate() {
                values.push(match value {
                    Some(v) => v.into(),
                    None => col_nulls[col_idx].clone(),
                });
            }

            charge_str_props(dec, &values)?;

            rows.push(row(&feat, values));
        }
        Ok(rows)
    }
}

impl Layer01<'_> {
    /// Decode and convert into a row-oriented [`TileLayer`]
    pub fn into_tile(self, dec: &mut Decoder) -> MltResult<TileLayer> {
        self.decode_all(dec)?.into_tile(dec)
    }
}

#[cfg(feature = "unstable-v2")]
impl ParsedLayer02<'_> {
    /// Decode and convert into a row-oriented [`TileLayer`], adding the m-value
    /// and nested columns only v2 carries.
    ///
    /// A `Meshes` layer is rejected, since [`Self::into_meshes`] reads it.
    #[hotpath::measure]
    pub fn into_tile(self, dec: &mut Decoder) -> MltResult<TileLayer> {
        if self.layout.geometry == GeoLayout::Meshes {
            return Err(MltError::MeshLayerIntoTile(self.layer.name().to_string()));
        }
        let m_names: Vec<String> = self.m_values.iter().map(|m| m.name().to_string()).collect();
        let nested_names: Vec<String> = self.nested.iter().map(|n| n.name().to_string()).collect();
        let nested_kinds: Vec<NestedKind> = self.nested.iter().map(ParsedNested::kind).collect();
        // One walk per m-value column, stepped alongside the features. Cutting the
        // columns up front instead would cost a span per feature per column, which is
        // memory the tile declares rather than memory it carries.
        let mut m_spans = dec.alloc::<MValueSpans>(self.m_values.len())?;
        m_spans.extend(self.m_values.iter().map(|m| m.spans(&self.layer.geometry)));

        let geometry = &self.layer.geometry;
        let mut parts = self.layer.collect_parts(dec)?;
        for (index, feature) in parts.features.iter_mut().enumerate() {
            feature.m_values = m_values_of(&self.m_values, &mut m_spans, dec)?;
            feature.nested = nested_of(&self.nested, index, dec)?;
            let run = geometry.z_run(index)?;
            let mut z = dec.alloc::<i32>(run.len())?;
            for value in run {
                z.push(value?);
            }
            feature.z = z;
        }
        parts
            .finish()?
            .with_m_value_names(m_names)?
            .with_nested(nested_names, nested_kinds)?
            .with_z_axis(geometry.z_axis())
    }

    /// Decode a `Meshes` layer into a [`MeshLayer`], rejecting any other layout.
    #[hotpath::measure]
    pub fn into_meshes(self, dec: &mut Decoder) -> MltResult<MeshLayer> {
        let name = self.layer.name().to_string();
        if self.layout.geometry != GeoLayout::Meshes {
            return Err(MltError::NotAMeshLayer(name));
        }
        let geometry = &self.layer.geometry;
        let z_axis = geometry.z_axis().ok_or(MltError::MeshVerticesWithoutZ)?;
        let mut m_spans = dec.alloc::<MValueSpans>(self.m_values.len())?;
        m_spans.extend(self.m_values.iter().map(|m| m.spans(geometry)));

        let rows = self.layer.rows(dec, |feat, values| (feat.id(), values))?;
        let mut features = dec.alloc::<MeshFeature>(rows.len())?;
        for (index, (id, properties)) in rows.into_iter().enumerate() {
            features.push(MeshFeature {
                id,
                properties,
                mesh: mesh_of(geometry, index, dec)?,
                m_values: m_values_of(&self.m_values, &mut m_spans, dec)?,
                nested: nested_of(&self.nested, index, dec)?,
            });
        }
        Ok(MeshLayer {
            name,
            extent: self.layer.extent(),
            z_axis,
            property_names: self
                .layer
                .iterate_prop_names()
                .map(|n| n.to_string())
                .collect(),
            property_kinds: typed_nulls(&self.layer.properties)
                .iter()
                .map(PropValue::kind)
                .collect(),
            m_value_names: self.m_values.iter().map(|m| m.name().to_string()).collect(),
            m_value_kinds: self.m_values.iter().map(|m| m.values().kind()).collect(),
            m_value_scopes: self.m_values.iter().map(ParsedMValue::scope).collect(),
            nested_names: self.nested.iter().map(|n| n.name().to_string()).collect(),
            nested_kinds: self.nested.iter().map(ParsedNested::kind).collect(),
            features,
        })
    }
}

#[cfg(feature = "unstable-v2")]
impl Layer02<'_> {
    /// Decode and convert into a row-oriented [`TileLayer`]
    pub fn into_tile(self, dec: &mut Decoder) -> MltResult<TileLayer> {
        self.decode_all(dec)?.into_tile(dec)
    }

    /// Decode a `Meshes` layer into a [`MeshLayer`], rejecting any other layout.
    pub fn into_meshes(self, dec: &mut Decoder) -> MltResult<MeshLayer> {
        self.decode_all(dec)?.into_meshes(dec)
    }
}

impl ParsedLayer<'_> {
    /// Convert into a row-oriented [`TileLayer`] whatever the tag, or `None` for a
    /// tag this build does not know. Keeps every version's columns.
    pub fn into_tile(self, dec: &mut Decoder) -> MltResult<Option<TileLayer>> {
        match self {
            Self::Tag01(l) => l.into_tile(dec).map(Some),
            #[cfg(feature = "unstable-v2")]
            Self::Tag02(l) => l.into_tile(dec).map(Some),
            Self::Unknown(_) => Ok(None),
        }
    }

    /// Convert a `Meshes` layer into a [`MeshLayer`], or `None` for a tag this build
    /// does not know. Any other layer is rejected.
    #[cfg(feature = "unstable-v2")]
    pub fn into_meshes(self, dec: &mut Decoder) -> MltResult<Option<MeshLayer>> {
        match self {
            Self::Tag01(l) => Err(MltError::NotAMeshLayer(l.name().to_string())),
            Self::Tag02(l) => l.into_meshes(dec).map(Some),
            Self::Unknown(_) => Ok(None),
        }
    }
}

impl Layer<'_, Lazy> {
    /// Decode every column, then convert into a row-oriented [`TileLayer`], or
    /// `None` for a tag this build does not know.
    pub fn into_tile(self, dec: &mut Decoder) -> MltResult<Option<TileLayer>> {
        self.decode_all(dec)?.into_tile(dec)
    }

    /// Decode every column, then convert a `Meshes` layer into a [`MeshLayer`], or
    /// `None` for a tag this build does not know. Any other layer is rejected.
    #[cfg(feature = "unstable-v2")]
    pub fn into_meshes(self, dec: &mut Decoder) -> MltResult<Option<MeshLayer>> {
        self.decode_all(dec)?.into_meshes(dec)
    }
}

/// The mesh of feature `index`, its corners shifted to name the feature's own vertices.
#[cfg(feature = "unstable-v2")]
fn mesh_of(geometry: &GeometryValues, index: usize, dec: &mut Decoder) -> MltResult<Mesh> {
    let out_of_bounds = |field, idx, len| MltError::GeometryOutOfBounds {
        index,
        field,
        idx,
        len,
    };
    let run = geometry.vertex_range(index)?;
    let xyz = geometry.vertices.as_deref().unwrap_or_default();
    let coords = xyz
        .get(run.start * 3..run.end * 3)
        .ok_or_else(|| out_of_bounds("vertices", run.end * 3, xyz.len()))?;
    let mut vertices = dec.alloc::<[i32; 3]>(run.len())?;
    vertices.extend_from_slice(coords.as_chunks::<3>().0);

    let tris = geometry.triangle_range(index)?;
    let corners = geometry.index_buffer.as_deref().unwrap_or_default();
    let corners = corners
        .get(tris.start * 3..tris.end * 3)
        .ok_or_else(|| out_of_bounds("index_buffer", tris.end * 3, corners.len()))?;
    let first = u32::try_from(run.start).or_overflow()?;
    let mut triangles = dec.alloc::<[u32; 3]>(tris.len())?;
    // A corner below the run wraps past every vertex, which `Mesh::new` rejects.
    triangles.extend(
        corners
            .as_chunks::<3>()
            .0
            .iter()
            .map(|t| t.map(|corner| corner.wrapping_sub(first))),
    );
    Mesh::new(vertices, triangles)
}

/// The m-values of the next feature, one per column, charged against `dec`.
///
/// Every column's walk is stepped once per feature, which is what pairs a feature
/// with its run.
#[cfg(feature = "unstable-v2")]
fn m_values_of(
    columns: &[ParsedMValue<'_>],
    spans: &mut [MValueSpans<'_>],
    dec: &mut Decoder,
) -> MltResult<Vec<MValue>> {
    let mut values = dec.alloc::<MValue>(columns.len())?;
    for (column, spans) in columns.iter().zip(spans) {
        let span = spans.next().transpose()?.flatten();
        if let Some(span) = &span {
            dec.consume(u32::try_from(column.values().row_bytes(span)).or_overflow()?)?;
        }
        values.push(column.values().row(column.name(), span)?);
    }
    Ok(values)
}

/// The nested values of feature `index`, one per column, charged against `dec`.
#[cfg(feature = "unstable-v2")]
fn nested_of(
    columns: &[ParsedNested<'_>],
    index: usize,
    dec: &mut Decoder,
) -> MltResult<Vec<NestedValue>> {
    let mut values = dec.alloc::<NestedValue>(columns.len())?;
    for column in columns {
        let value = column.row(index)?;
        dec.consume(u32::try_from(value.heap_bytes()).or_overflow()?)?;
        values.push(value);
    }
    Ok(values)
}

/// Build a flat list of typed null [`PropValue`]s, one per logical column position
/// as yielded by [`crate::FeatureRef::iter_all_properties`].
///
/// Each scalar column contributes one entry with its specific null variant (e.g.
/// `PropValue::Bool(None)`).  A `SharedDict` column expands to one `PropValue::Str(None)`
/// entry per sub-item.
fn typed_nulls(properties: &[ParsedProperty<'_>]) -> Vec<PropValue> {
    use ParsedProperty as PP;
    use PropValue as PV;
    let mut nulls = Vec::new();
    for prop in properties {
        match prop {
            PP::Bool(_) => nulls.push(PV::Bool(None)),
            PP::I8(_) => nulls.push(PV::I8(None)),
            PP::U8(_) => nulls.push(PV::U8(None)),
            PP::I32(_) => nulls.push(PV::I32(None)),
            PP::U32(_) => nulls.push(PV::U32(None)),
            PP::I64(_) => nulls.push(PV::I64(None)),
            PP::U64(_) => nulls.push(PV::U64(None)),
            PP::F32(_) => nulls.push(PV::F32(None)),
            PP::F64(_) => nulls.push(PV::F64(None)),
            PP::Str(_) => nulls.push(PV::Str(None)),
            PP::SharedDict(d) => {
                for _ in &d.items {
                    nulls.push(PV::Str(None));
                }
            }
        }
    }
    nulls
}

/// Charge `dec` for the bytes of the owned strings inside `PropValue::Str`.
fn charge_str_props(dec: &mut Decoder, props: &[PropValue]) -> MltResult<()> {
    let str_bytes = props
        .iter()
        .filter_map(|p| {
            if let PropValue::Str(Some(s)) = p {
                Some(s.len())
            } else {
                None
            }
        })
        .try_fold(0u32, |acc, n| {
            acc.checked_add(u32::try_from(n).or_overflow()?)
                .or_overflow()
        })?;
    if str_bytes > 0 {
        dec.consume(str_bytes)?;
    }
    Ok(())
}
