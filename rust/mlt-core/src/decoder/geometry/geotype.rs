use std::ops::Range;

use geo_types::{
    Coord, Geometry, LineString, MultiLineString, MultiPoint, MultiPolygon, Point, Polygon,
};
use usize_cast::IntoUsize as _;

use crate::MltError::{
    GeometryIndexOutOfBounds, GeometryOutOfBounds, GeometryVertexOutOfBounds, IntegerOverflow,
    NoGeometryOffsets, NoPartOffsets, NoRingOffsets,
};
use crate::MltResult;
use crate::decoder::{GeometryType, GeometryValues};
use crate::errors::AsMltError as _;

impl GeometryType {
    #[must_use]
    pub fn is_polygon(self) -> bool {
        matches!(self, Self::Polygon | Self::MultiPolygon)
    }
    #[must_use]
    pub fn is_linestring(self) -> bool {
        matches!(self, Self::LineString | Self::MultiLineString)
    }
    #[must_use]
    pub fn is_multi(self) -> bool {
        matches!(
            self,
            Self::MultiPoint | Self::MultiLineString | Self::MultiPolygon
        )
    }
}

/// An offset level a geometry only needs when it addresses something in it.
fn require<'a>(
    level: Option<&'a [u32]>,
    range: &Range<usize>,
    missing: impl FnOnce() -> crate::MltError,
) -> MltResult<&'a [u32]> {
    match level {
        Some(level) => Ok(level),
        None if range.is_empty() => Ok(&[]),
        None => Err(missing()),
    }
}

impl GeometryValues {
    #[must_use]
    pub fn feature_count(&self) -> usize {
        self.vector_types.len()
    }

    /// Geometry types for each feature, in insertion order.
    #[must_use]
    pub fn vector_types(&self) -> &[GeometryType] {
        &self.vector_types
    }

    /// Cumulative offsets into `part_offsets` for multi-geometry types.
    /// `None` when no multi-geometry features are present.
    #[must_use]
    pub fn geometry_offsets(&self) -> Option<&[u32]> {
        self.geometry_offsets.as_deref()
    }

    /// Cumulative offsets into `ring_offsets` (or directly into `vertices`
    /// for `LineString` layers without rings).
    /// `None` for pure `Point` layers.
    #[must_use]
    pub fn part_offsets(&self) -> Option<&[u32]> {
        self.part_offsets.as_deref()
    }

    /// Cumulative offsets into the vertex buffer (counting whole vertices).
    /// `None` when no ring-level indirection is needed.
    #[must_use]
    pub fn ring_offsets(&self) -> Option<&[u32]> {
        self.ring_offsets.as_deref()
    }

    /// Triangle index buffer produced by Earcut tessellation, three indices per triangle.
    /// Each index names a vertex of the whole layer, so the buffer can go to a GPU as is.
    /// `None` unless the `GeometryValues` was created with [`Self::new_tessellated`].
    #[must_use]
    pub fn index_buffer(&self) -> Option<&[u32]> {
        self.index_buffer.as_deref()
    }

    /// Cumulative triangle counts, one run per polygon feature, starting at `0`.
    /// `None` unless the `GeometryValues` was created with [`Self::new_tessellated`].
    #[must_use]
    pub fn triangle_offsets(&self) -> Option<&[u32]> {
        self.triangle_offsets.as_deref()
    }

    /// The first vertex of every polygon feature, which a v1 triangle index counts from.
    fn polygon_vertex_starts(&self) -> impl Iterator<Item = MltResult<u32>> {
        self.vector_types
            .iter()
            .enumerate()
            .filter(|(_, t)| t.is_polygon())
            .map(|(index, _)| {
                let start = self.vertex_range(index)?.start;
                u32::try_from(start).or_overflow()
            })
    }

    /// Replace each polygon feature's triangle indices with `shift(index, first_vertex)`.
    fn rebase_indices(&mut self, shift: impl Fn(u32, u32) -> Option<u32>) -> MltResult<()> {
        let Some(mut indices) = self.index_buffer.take() else {
            return Ok(());
        };
        let offsets = self.triangle_offsets.as_deref().unwrap_or(&[0]);
        let starts = self
            .polygon_vertex_starts()
            .collect::<MltResult<Vec<_>>>()?;
        if offsets.len() != starts.len() + 1 {
            return Err(GeometryIndexOutOfBounds(offsets.len().saturating_sub(1)));
        }
        for (run, start) in offsets.windows(2).zip(starts) {
            let [from, to] = [run[0], run[1]].map(|t| t.into_usize().saturating_mul(3));
            let slice = indices
                .get_mut(from..to)
                .ok_or(GeometryIndexOutOfBounds(to))?;
            for idx in slice {
                *idx = shift(*idx, start).ok_or(IntegerOverflow)?;
            }
        }
        self.index_buffer = Some(indices);
        Ok(())
    }

    /// Turn v1's per-feature triangle indices into the layer-wide ones this type holds.
    pub(crate) fn rebase_indices_to_layer(&mut self) -> MltResult<()> {
        self.rebase_indices(u32::checked_add)
    }

    /// Turn the layer-wide triangle indices this type holds into v1's per-feature ones.
    pub(crate) fn rebase_indices_to_feature(&mut self) -> MltResult<()> {
        self.rebase_indices(u32::checked_sub)
    }

    /// Flat vertex buffer: `[x0, y0, x1, y1, …]` in tile coordinates.
    #[must_use]
    pub fn vertices(&self) -> Option<&[i32]> {
        self.vertices.as_deref()
    }

    /// The range of the layer's vertex sequence that feature `index` owns.
    ///
    /// The sequence is every vertex of every feature in feature order, which is
    /// what the vertex buffer holds and what an m-value column runs over.
    /// A ring's closing vertex is not stored, so it is not in the range either.
    pub fn vertex_range(&self, index: usize) -> MltResult<Range<usize>> {
        let entry = |level: &[u32], idx: usize, field: &'static str| -> MltResult<usize> {
            level
                .get(idx)
                .map(|&v| v.into_usize())
                .ok_or(GeometryOutOfBounds {
                    index,
                    field,
                    idx,
                    len: level.len(),
                })
        };
        // Every level addresses a contiguous run of the level below it, so descending
        // a range through them lands on the vertices without caring which shape it is.
        let mut range = match self.geometry_offsets.as_deref() {
            Some(geoms) => {
                entry(geoms, index, "geometry_offsets")?
                    ..entry(geoms, index + 1, "geometry_offsets")?
            }
            None => index..index + 1,
        };
        for (level, field) in [
            (self.part_offsets.as_deref(), "part_offsets"),
            (self.ring_offsets.as_deref(), "ring_offsets"),
        ] {
            if let Some(level) = level {
                range = entry(level, range.start, field)?..entry(level, range.end, field)?;
            }
        }
        Ok(range)
    }

    /// How many vertices feature `index` owns, see [`Self::vertex_range`].
    pub fn vertex_count(&self, index: usize) -> MltResult<usize> {
        let range = self.vertex_range(index)?;
        Ok(range.end.saturating_sub(range.start))
    }

    /// Build a `GeoJSON` geometry for a single feature at index `i`.
    /// Polygon and `MultiPolygon` rings are closed per `GeoJSON` spec
    /// (MLT omits the closing vertex).
    pub fn to_geojson(&self, index: usize) -> MltResult<Geometry<i32>> {
        let verts = self.vertices.as_deref().unwrap_or(&[]);
        let geoms = self.geometry_offsets.as_deref();
        let parts = self.part_offsets.as_deref();
        let rings = self.ring_offsets.as_deref();

        let off = |s: &[u32], idx: usize, field: &'static str| -> MltResult<usize> {
            s.get(idx)
                .map(|&v| v.into_usize())
                .ok_or(GeometryOutOfBounds {
                    index,
                    field,
                    idx,
                    len: s.len(),
                })
        };
        let off_pair = |s: &[u32], idx: usize, field: &'static str| -> MltResult<Range<usize>> {
            Ok(off(s, idx, field)?..off(s, idx + 1, field)?)
        };

        let geom_off = |s: &[u32], i: usize| off(s, i, "geometry_offsets");
        let part_off = |s: &[u32], i: usize| off(s, i, "part_offsets");
        let ring_off = |s: &[u32], i: usize| off(s, i, "ring_offsets");
        let geom_range = |s: &[u32], i: usize| off_pair(s, i, "geometry_offsets");
        let part_range = |s: &[u32], i: usize| off_pair(s, i, "part_offsets");
        let ring_range = |s: &[u32], i: usize| off_pair(s, i, "ring_offsets");

        let vert = |idx: usize| -> MltResult<Coord<i32>> {
            idx.checked_mul(2)
                .and_then(|w| verts.get(w..w.checked_add(2)?))
                .map(|s| Coord { x: s[0], y: s[1] })
                .ok_or(GeometryVertexOutOfBounds {
                    index,
                    vertex: idx,
                    count: verts.len() / 2,
                })
        };
        let line = |r: Range<usize>| -> MltResult<LineString<i32>> { r.map(&vert).collect() };
        let closed_ring = |r: Range<usize>| -> MltResult<LineString<i32>> {
            if r.is_empty() {
                return Ok(LineString(vec![]));
            }
            let first = r.start;
            let mut coords: Vec<Coord<i32>> = r.map(&vert).collect::<Result<_, _>>()?;
            coords.push(vert(first)?);
            Ok(LineString(coords))
        };
        let poly_from_rings = |part_rng: Range<usize>, r: &[u32]| -> MltResult<Polygon<i32>> {
            let mut rings = part_rng
                .map(|idx| closed_ring(ring_range(r, idx)?))
                .collect::<Result<Vec<_>, _>>()?
                .into_iter();
            Ok(Polygon::new(
                rings.next().unwrap_or_else(|| LineString(vec![])),
                rings.collect(),
            ))
        };

        let geom_type = *self
            .vector_types
            .get(index)
            .ok_or(GeometryIndexOutOfBounds(index))?;

        if parts.is_none()
            && let Some(indices) = self.index_buffer.as_deref()
        {
            let tris = self.triangle_offsets.as_deref().unwrap_or(&[]);
            let run = off_pair(tris, index, "triangle_offsets")?;
            let run = run.start.saturating_mul(3)..run.end.saturating_mul(3);
            let len = indices.len();
            let corners = indices.get(run.clone()).ok_or(GeometryOutOfBounds {
                index,
                field: "index_buffer",
                idx: run.end,
                len,
            })?;
            let triangles = corners
                .as_chunks::<3>()
                .0
                .iter()
                .map(|tri| {
                    let [a, b, c] = tri.map(|i| vert(i.into_usize()));
                    let (a, b, c) = (a?, b?, c?);
                    Ok(Polygon::new(LineString(vec![a, b, c, a]), vec![]))
                })
                .collect::<MltResult<_>>()?;
            return Ok(Geometry::<i32>::MultiPolygon(MultiPolygon(triangles)));
        }

        match geom_type {
            GeometryType::Point => {
                // Resolve through hierarchy: geoms? -> parts? -> rings? -> vertex
                let idx = geoms.map_or(Ok(index), |g| geom_off(g, index))?;
                let idx = parts.map_or(Ok(idx), |p| part_off(p, idx))?;
                let idx = rings.map_or(Ok(idx), |r| ring_off(r, idx))?;
                Ok(Geometry::<i32>::Point(Point(vert(idx)?)))
            }
            GeometryType::LineString => {
                let parts = parts.ok_or(NoPartOffsets(index, geom_type))?;
                // Get part index: use geoms[index] if present, else index directly
                let part_idx = geoms.map_or(Ok(index), |geom| geom_off(geom, index))?;
                // With rings: parts[part_idx] gives ring index, use ring_offsets for vertex range
                // Without rings: use part_offsets directly for vertex range
                let vert_range = match rings {
                    Some(ring) => ring_range(ring, part_off(parts, part_idx)?)?,
                    None => part_range(parts, part_idx)?,
                };
                line(vert_range).map(Geometry::<i32>::LineString)
            }
            GeometryType::Polygon => {
                let parts = parts.ok_or(NoPartOffsets(index, geom_type))?;
                let rings = rings.ok_or(NoRingOffsets(index, geom_type))?;
                let idx = geoms
                    .map(|geom| geom_off(geom, index))
                    .transpose()?
                    .unwrap_or(index);
                poly_from_rings(part_range(parts, idx)?, rings).map(Geometry::<i32>::Polygon)
            }
            GeometryType::MultiPoint => {
                let geoms = geoms.ok_or(NoGeometryOffsets(index, geom_type))?;
                let geom_rng = geom_range(geoms, index)?;
                // Resolve vertex index through parts?->rings? hierarchy
                // When ring_offsets exist (polygon geometry present), geometry_offsets indexes
                // into part_offsets which indexes into ring_offsets for vertex indices.
                // When only part_offsets exist, geometry_offsets indexes into part_offsets
                // which gives direct vertex indices.
                // When neither exist, geometry_offsets gives direct vertex indices.
                let coords: Result<Vec<_>, _> = match (parts, rings) {
                    (Some(parts), Some(rings)) => geom_rng
                        .map(|idx| vert(ring_off(rings, part_off(parts, idx)?)?))
                        .collect(),
                    (Some(part), None) => geom_rng.map(|idx| vert(part_off(part, idx)?)).collect(),
                    (None, _) => geom_rng.map(&vert).collect(),
                };
                Ok(Geometry::<i32>::MultiPoint(MultiPoint(
                    coords?.into_iter().map(Point).collect(),
                )))
            }
            GeometryType::MultiLineString => {
                let geoms = geoms.ok_or(NoGeometryOffsets(index, geom_type))?;
                let geom_rng = geom_range(geoms, index)?;
                // A multi with no sub-geometries never indexes the levels below it,
                // so it does not need them to be present.
                let parts = require(parts, &geom_rng, || NoPartOffsets(index, geom_type))?;
                // geometry_offsets indexes into part_offsets for each linestring.
                // When ring_offsets exist (polygon geometry present), part_offsets indexes
                // into ring_offsets for vertex ranges. Otherwise, part_offsets directly
                // gives vertex ranges.
                let lines: Result<Vec<_>, _> = match rings {
                    Some(ring) => geom_rng
                        .map(|idx| line(ring_range(ring, part_off(parts, idx)?)?))
                        .collect(),
                    None => geom_rng.map(|idx| line(part_range(parts, idx)?)).collect(),
                };
                Ok(Geometry::<i32>::MultiLineString(MultiLineString(lines?)))
            }
            GeometryType::MultiPolygon => {
                let geoms = geoms.ok_or(NoGeometryOffsets(index, geom_type))?;
                let geom_rng = geom_range(geoms, index)?;
                // A multi with no sub-geometries never indexes the levels below it,
                // so it does not need them to be present.
                let parts = require(parts, &geom_rng, || NoPartOffsets(index, geom_type))?;
                let rings = require(rings, &geom_rng, || NoRingOffsets(index, geom_type))?;
                let polys: Vec<_> = geom_rng
                    .map(|idx| poly_from_rings(part_range(parts, idx)?, rings))
                    .collect::<Result<_, _>>()?;
                Ok(Geometry::<i32>::MultiPolygon(MultiPolygon(polys)))
            }
        }
    }
}
