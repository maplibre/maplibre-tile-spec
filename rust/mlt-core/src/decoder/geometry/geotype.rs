use std::ops::Range;

use geo_types::{Coord, LineString, MultiLineString, MultiPoint, MultiPolygon, Point, Polygon};

use crate::MltError::{
    GeometryIndexOutOfBounds, GeometryOutOfBounds, GeometryVertexOutOfBounds, NoGeometryOffsets,
    NoPartOffsets, NoRingOffsets,
};
use crate::MltResult;
use crate::decoder::{GeometryType, GeometryValues};
use crate::geojson::{Coord32, Geom32};
use crate::utils::AsUsize as _;

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

impl GeometryValues {
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

    /// Triangle index buffer produced by Earcut tessellation.
    /// `None` unless the `GeometryValues` was created with [`Self::new_tessellated`].
    #[must_use]
    pub fn index_buffer(&self) -> Option<&[u32]> {
        self.index_buffer.as_deref()
    }

    /// Per-feature triangle counts produced by Earcut tessellation.
    /// `None` unless the `GeometryValues` was created with [`Self::new_tessellated`].
    #[must_use]
    pub fn triangles(&self) -> Option<&[u32]> {
        self.triangles.as_deref()
    }

    /// Flat vertex buffer: `[x0, y0, x1, y1, …]` in tile coordinates.
    #[must_use]
    pub fn vertices(&self) -> Option<&[i32]> {
        self.vertices.as_deref()
    }

    /// Build a `GeoJSON` geometry for a single feature at index `i`.
    /// Polygon and `MultiPolygon` rings are closed per `GeoJSON` spec
    /// (MLT omits the closing vertex).
    pub fn to_geojson(&self, index: usize) -> MltResult<Geom32> {
        let verts = self.vertices.as_deref().unwrap_or(&[]);
        let geoms = self.geometry_offsets.as_deref();
        let parts = self.part_offsets.as_deref();
        let rings = self.ring_offsets.as_deref();

        let off = |s: &[u32], idx: usize, field: &'static str| -> MltResult<usize> {
            s.get(idx)
                .map(|&v| v.as_usize())
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

        let vert = |idx: usize| -> MltResult<Coord32> {
            verts
                .get(idx * 2..idx * 2 + 2)
                .map(|s| Coord { x: s[0], y: s[1] })
                .ok_or(GeometryVertexOutOfBounds {
                    index,
                    vertex: idx,
                    count: verts.len() / 2,
                })
        };
        let line = |r: Range<usize>| -> MltResult<LineString<i32>> { r.map(&vert).collect() };
        let closed_ring = |r: Range<usize>| -> MltResult<LineString<i32>> {
            let first = r.start;
            let mut coords: Vec<Coord32> = r.map(&vert).collect::<Result<_, _>>()?;
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

        match geom_type {
            GeometryType::Point => {
                // Resolve through hierarchy: geoms? -> parts? -> rings? -> vertex
                let idx = geoms.map_or(Ok(index), |g| geom_off(g, index))?;
                let idx = parts.map_or(Ok(idx), |p| part_off(p, idx))?;
                let idx = rings.map_or(Ok(idx), |r| ring_off(r, idx))?;
                Ok(Geom32::Point(Point(vert(idx)?)))
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
                line(vert_range).map(Geom32::LineString)
            }
            GeometryType::Polygon => {
                let parts = parts.ok_or(NoPartOffsets(index, geom_type))?;
                let rings = rings.ok_or(NoRingOffsets(index, geom_type))?;
                let idx = geoms
                    .map(|geom| geom_off(geom, index))
                    .transpose()?
                    .unwrap_or(index);
                poly_from_rings(part_range(parts, idx)?, rings).map(Geom32::Polygon)
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
                Ok(Geom32::MultiPoint(MultiPoint(
                    coords?.into_iter().map(Point).collect(),
                )))
            }
            GeometryType::MultiLineString => {
                let geoms = geoms.ok_or(NoGeometryOffsets(index, geom_type))?;
                let parts = parts.ok_or(NoPartOffsets(index, geom_type))?;
                let geom_rng = geom_range(geoms, index)?;
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
                Ok(Geom32::MultiLineString(MultiLineString(lines?)))
            }
            GeometryType::MultiPolygon => {
                let geoms = geoms.ok_or(NoGeometryOffsets(index, geom_type))?;
                let parts = parts.ok_or(NoPartOffsets(index, geom_type))?;
                let rings = rings.ok_or(NoRingOffsets(index, geom_type))?;
                let polys: Vec<_> = geom_range(geoms, index)?
                    .map(|idx| poly_from_rings(part_range(parts, idx)?, rings))
                    .collect::<Result<_, _>>()?;
                Ok(Geom32::MultiPolygon(MultiPolygon(polys)))
            }
        }
    }
}
