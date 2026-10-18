//! A feature's geometry read in place from the layer's decoded buffers.
//!
//! [`GeometryValues::geometry`] validates every offset a feature reaches and hands back a
//! [`GeometryView`]: a `Copy` handle into the layer that implements the [`geo_traits`]
//! traits with `T = i32`, carries z when the layer has a z grid, and never allocates.
//! Its rings are closed the way `GeoJSON` and OGC expect, while the layer stores each
//! ring without its closing vertex; [`GeometryView::vertices`] and
//! [`GeometryView::vertex_range`] give the stored sequence that m-values and z run over.

use std::ops::Range;

use geo_traits::{
    CoordTrait, Dimensions, GeometryTrait, GeometryType as GeoKind, LineStringTrait,
    MultiLineStringTrait, MultiPointTrait, MultiPolygonTrait, PointTrait, PolygonTrait,
    UnimplementedGeometryCollection, UnimplementedLine, UnimplementedRect, UnimplementedTriangle,
};
use usize_cast::IntoUsize as _;

use crate::MltError::{
    GeometryIndexOutOfBounds, GeometryOffsetsNotMonotonic, GeometryOutOfBounds,
    GeometryVertexOutOfBounds, NoGeometryOffsets, NoPartOffsets, NoRingOffsets,
};
use crate::MltResult;
use crate::decoder::{GeometryType, GeometryValues};

/// A half-open index range that is `Copy`, which [`Range`] is not.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Span {
    start: usize,
    end: usize,
}

impl Span {
    fn len(self) -> usize {
        self.end - self.start
    }

    fn is_empty(self) -> bool {
        self.start == self.end
    }

    fn range(self) -> Range<usize> {
        self.start..self.end
    }
}

/// One coordinate of a decoded feature.
///
/// `z` is `Some` exactly when the layer stores a z per vertex; see [`GeometryTrait::dim`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Vertex {
    pub x: i32,
    pub y: i32,
    pub z: Option<i32>,
}

impl CoordTrait for Vertex {
    type T = i32;

    fn dim(&self) -> Dimensions {
        if self.z.is_some() {
            Dimensions::Xyz
        } else {
            Dimensions::Xy
        }
    }

    fn x(&self) -> i32 {
        self.x
    }

    fn y(&self) -> i32 {
        self.y
    }

    fn nth_or_panic(&self, n: usize) -> i32 {
        match (n, self.z) {
            (0, _) => self.x,
            (1, _) => self.y,
            (2, Some(z)) => z,
            _ => panic!(
                "a vertex with {} dimensions has no dimension {n}",
                self.dim().size()
            ),
        }
    }
}

/// The slots of [`GeometryTrait`] every view fills the same way.
macro_rules! view_slots {
    ($a:lifetime) => {
        type T = i32;
        type PointType<'b>
            = PointView<$a>
        where
            Self: 'b;
        type LineStringType<'b>
            = LineView<$a>
        where
            Self: 'b;
        type PolygonType<'b>
            = PolygonView<$a>
        where
            Self: 'b;
        type MultiPointType<'b>
            = MultiPointView<$a>
        where
            Self: 'b;
        type MultiLineStringType<'b>
            = MultiLineStringView<$a>
        where
            Self: 'b;
        type MultiPolygonType<'b>
            = MultiPolygonView<$a>
        where
            Self: 'b;
        type GeometryCollectionType<'b>
            = UnimplementedGeometryCollection<i32>
        where
            Self: 'b;
        type RectType<'b>
            = UnimplementedRect<i32>
        where
            Self: 'b;
        type TriangleType<'b>
            = UnimplementedTriangle<i32>
        where
            Self: 'b;
        type LineType<'b>
            = UnimplementedLine<i32>
        where
            Self: 'b;

        fn dim(&self) -> Dimensions {
            self.values.dim()
        }
    };
}

/// What [`GeometryTrait::as_type`] returns for every view.
type Kind<'s, 'a> = GeoKind<
    's,
    PointView<'a>,
    LineView<'a>,
    PolygonView<'a>,
    MultiPointView<'a>,
    MultiLineStringView<'a>,
    MultiPolygonView<'a>,
    UnimplementedGeometryCollection<i32>,
    UnimplementedRect<i32>,
    UnimplementedTriangle<i32>,
    UnimplementedLine<i32>,
>;

const UNCHECKED: &str =
    "geo-traits declares the accessor unsafe; this body indexes with bounds checks";

// ───────────────────────────── the layer's side ─────────────────────────────

impl GeometryValues {
    fn dim(&self) -> Dimensions {
        if self.stride() == 3 {
            Dimensions::Xyz
        } else {
            Dimensions::Xy
        }
    }

    /// How many whole vertices the buffer holds.
    fn buffer_vertex_count(&self) -> usize {
        self.vertices.as_ref().map_or(0, Vec::len) / self.stride()
    }

    /// Vertex `index` of the layer, which a validated view only asks for in bounds.
    fn vertex(&self, index: usize) -> Vertex {
        let stride = self.stride();
        let words = self.vertices.as_deref().unwrap_or(&[]);
        let at = index * stride;
        Vertex {
            x: words[at],
            y: words[at + 1],
            z: (stride == 3).then(|| words[at + 2]),
        }
    }

    fn parts(&self) -> &[u32] {
        self.part_offsets.as_deref().unwrap_or(&[])
    }

    fn rings(&self) -> &[u32] {
        self.ring_offsets.as_deref().unwrap_or(&[])
    }

    fn corners(&self) -> &[u32] {
        self.index_buffer.as_deref().unwrap_or(&[])
    }

    /// The vertex range ring `ring` of the layer covers.
    fn ring_span(&self, ring: usize) -> Span {
        let rings = self.rings();
        Span {
            start: rings[ring].into_usize(),
            end: rings[ring + 1].into_usize(),
        }
    }

    /// The range part `part` of the layer covers in the level below it.
    fn part_span(&self, part: usize) -> Span {
        let parts = self.parts();
        Span {
            start: parts[part].into_usize(),
            end: parts[part + 1].into_usize(),
        }
    }

    /// The geometry of feature `index`, validated once so the view is total.
    ///
    /// Every offset the feature reaches is checked here: it exists, each pair is
    /// monotonic, and every vertex it names is in the buffer. The errors are the ones
    /// [`Self::to_geojson`] reports, so a corrupted feature fails the same way on both
    /// paths while its neighbours stay readable.
    pub fn geometry(&self, index: usize) -> MltResult<GeometryView<'_>> {
        let geom_type = *self
            .vector_types
            .get(index)
            .ok_or(GeometryIndexOutOfBounds(index))?;
        let check = Check {
            values: self,
            index,
        };
        let geoms = self.geometry_offsets.as_deref();
        let parts = self.part_offsets.as_deref();
        let rings = self.ring_offsets.as_deref();

        if parts.is_none()
            && let Some(indices) = self.index_buffer.as_deref()
        {
            let tris = self.triangle_offsets.as_deref().unwrap_or(&[]);
            let run = check.pair(tris, index, "triangle_offsets")?;
            let run = Span {
                start: run.start.saturating_mul(3),
                end: run.end.saturating_mul(3),
            };
            let corners = indices.get(run.range()).ok_or(GeometryOutOfBounds {
                index,
                field: "index_buffer",
                idx: run.end,
                len: indices.len(),
            })?;
            for &corner in corners {
                check.vertex(corner.into_usize())?;
            }
            return Ok(GeometryView {
                values: self,
                index,
                vertex_span: None,
                shape: Shape::MultiPolygon(MultiPolygonView {
                    values: self,
                    polygons: Polygons::Triangles(run),
                }),
            });
        }

        // The vertex span comes from the same lookups the shape made, so a layer the
        // shape reads is one the span reads: notably the encoder's compact staging form,
        // whose single geometries have no `geometry_offsets[index + 1]`.
        let (shape, vertex_span) = match geom_type {
            GeometryType::Point => {
                // Resolve through whichever levels exist: geoms? -> parts? -> rings? -> vertex.
                let idx = geoms.map_or(Ok(index), |g| check.off(g, index, "geometry_offsets"))?;
                let idx = parts.map_or(Ok(idx), |p| check.off(p, idx, "part_offsets"))?;
                let idx = rings.map_or(Ok(idx), |r| check.off(r, idx, "ring_offsets"))?;
                check.vertex(idx)?;
                let shape = Shape::Point(PointView {
                    values: self,
                    vertex: idx,
                });
                (
                    shape,
                    Span {
                        start: idx,
                        end: idx + 1,
                    },
                )
            }
            GeometryType::LineString => {
                let parts = parts.ok_or(NoPartOffsets(index, geom_type))?;
                let part = geoms.map_or(Ok(index), |g| check.off(g, index, "geometry_offsets"))?;
                let span = check.part_vertices(parts, rings, part)?;
                let shape = Shape::LineString(LineView {
                    values: self,
                    corners: Corners::Run { span, ring: false },
                });
                (shape, span)
            }
            GeometryType::Polygon => {
                let parts = parts.ok_or(NoPartOffsets(index, geom_type))?;
                let rings = rings.ok_or(NoRingOffsets(index, geom_type))?;
                let part = geoms.map_or(Ok(index), |g| check.off(g, index, "geometry_offsets"))?;
                let ring_span = check.pair(parts, part, "part_offsets")?;
                check.rings(rings, ring_span)?;
                let shape = Shape::Polygon(PolygonView {
                    values: self,
                    rings: Rings::Offsets(ring_span),
                });
                (shape, check.level(rings, ring_span, "ring_offsets")?)
            }
            GeometryType::MultiPoint => {
                let geoms = geoms.ok_or(NoGeometryOffsets(index, geom_type))?;
                let span = check.pair(geoms, index, "geometry_offsets")?;
                // A point addresses its vertex through every level the layer has.
                let via = match (parts, rings) {
                    (Some(_), Some(_)) => PointVia::PartsThenRings,
                    (Some(_), None) => PointVia::Parts,
                    (None, _) => PointVia::Direct,
                };
                for idx in span.range() {
                    let vertex = match via {
                        PointVia::Direct => idx,
                        PointVia::Parts => check.off(self.parts(), idx, "part_offsets")?,
                        PointVia::PartsThenRings => {
                            let ring = check.off(self.parts(), idx, "part_offsets")?;
                            check.off(self.rings(), ring, "ring_offsets")?
                        }
                    };
                    check.vertex(vertex)?;
                }
                let shape = Shape::MultiPoint(MultiPointView {
                    values: self,
                    points: span,
                    via,
                });
                (shape, check.descend(span, parts, rings)?)
            }
            GeometryType::MultiLineString => {
                let geoms = geoms.ok_or(NoGeometryOffsets(index, geom_type))?;
                let span = check.pair(geoms, index, "geometry_offsets")?;
                // A multi with no sub-geometries never indexes the levels below it,
                // so it does not need them to be present.
                let part_level = require(parts, span, || NoPartOffsets(index, geom_type))?;
                for part in span.range() {
                    check.part_vertices(part_level, rings, part)?;
                }
                let shape = Shape::MultiLineString(MultiLineStringView {
                    values: self,
                    parts: span,
                    via_rings: rings.is_some(),
                });
                (shape, check.descend(span, parts, rings)?)
            }
            GeometryType::MultiPolygon => {
                let geoms = geoms.ok_or(NoGeometryOffsets(index, geom_type))?;
                let span = check.pair(geoms, index, "geometry_offsets")?;
                let part_level = require(parts, span, || NoPartOffsets(index, geom_type))?;
                let ring_level = require(rings, span, || NoRingOffsets(index, geom_type))?;
                for part in span.range() {
                    let ring_span = check.pair(part_level, part, "part_offsets")?;
                    check.rings(ring_level, ring_span)?;
                }
                let shape = Shape::MultiPolygon(MultiPolygonView {
                    values: self,
                    polygons: Polygons::Parts(span),
                });
                (shape, check.descend(span, parts, rings)?)
            }
        };
        Ok(GeometryView {
            values: self,
            index,
            vertex_span: Some(vertex_span),
            shape,
        })
    }
}

/// An offset level a geometry only needs when it addresses something in it.
fn require(
    level: Option<&[u32]>,
    span: Span,
    missing: impl FnOnce() -> crate::MltError,
) -> MltResult<&[u32]> {
    match level {
        Some(level) => Ok(level),
        None if span.is_empty() => Ok(&[]),
        None => Err(missing()),
    }
}

/// The checks [`GeometryValues::geometry`] runs, naming feature `index` in every error.
struct Check<'a> {
    values: &'a GeometryValues,
    index: usize,
}

impl Check<'_> {
    fn off(&self, level: &[u32], idx: usize, field: &'static str) -> MltResult<usize> {
        level
            .get(idx)
            .map(|&v| v.into_usize())
            .ok_or(GeometryOutOfBounds {
                index: self.index,
                field,
                idx,
                len: level.len(),
            })
    }

    /// Entries `idx` and `idx + 1` of `level`, which must not run backwards.
    fn pair(&self, level: &[u32], idx: usize, field: &'static str) -> MltResult<Span> {
        let start = self.off(level, idx, field)?;
        let end = self.off(level, idx + 1, field)?;
        if start > end {
            return Err(GeometryOffsetsNotMonotonic {
                index: self.index,
                field,
                idx,
            });
        }
        Ok(Span { start, end })
    }

    /// The run of the level below that `span` of `level` covers, as the ends of `span`
    /// name it. Its entries exist in any layer whose offsets are complete.
    fn level(&self, level: &[u32], span: Span, field: &'static str) -> MltResult<Span> {
        let start = self.off(level, span.start, field)?;
        let end = self.off(level, span.end, field)?;
        if start > end {
            return Err(GeometryOffsetsNotMonotonic {
                index: self.index,
                field,
                idx: span.start,
            });
        }
        Ok(Span { start, end })
    }

    /// The vertices a multi's `span` of geometries covers, through each level the layer has,
    /// as [`GeometryValues::vertex_range`] descends them.
    fn descend(&self, span: Span, parts: Option<&[u32]>, rings: Option<&[u32]>) -> MltResult<Span> {
        let mut span = span;
        if let Some(parts) = parts {
            span = self.level(parts, span, "part_offsets")?;
        }
        if let Some(rings) = rings {
            span = self.level(rings, span, "ring_offsets")?;
        }
        Ok(span)
    }

    fn vertex(&self, vertex: usize) -> MltResult<()> {
        let count = self.values.buffer_vertex_count();
        if vertex < count {
            Ok(())
        } else {
            Err(GeometryVertexOutOfBounds {
                index: self.index,
                vertex,
                count,
            })
        }
    }

    /// Every vertex of `span` is in the buffer, reporting the first that is not.
    fn vertex_span(&self, span: Span) -> MltResult<()> {
        let count = self.values.buffer_vertex_count();
        if span.end > count {
            return Err(GeometryVertexOutOfBounds {
                index: self.index,
                vertex: span.start.max(count),
                count,
            });
        }
        Ok(())
    }

    /// The vertices of part `part`: a ring's when the layer has rings, else its own run.
    fn part_vertices(&self, parts: &[u32], rings: Option<&[u32]>, part: usize) -> MltResult<Span> {
        let span = match rings {
            Some(rings) => self.pair(
                rings,
                self.off(parts, part, "part_offsets")?,
                "ring_offsets",
            )?,
            None => self.pair(parts, part, "part_offsets")?,
        };
        self.vertex_span(span)?;
        Ok(span)
    }

    /// Every ring of `span` has a monotonic range of vertices in the buffer.
    fn rings(&self, rings: &[u32], span: Span) -> MltResult<()> {
        for ring in span.range() {
            let vertices = self.pair(rings, ring, "ring_offsets")?;
            self.vertex_span(vertices)?;
        }
        Ok(())
    }
}

// ───────────────────────────── the views ─────────────────────────────

/// The geometry of one feature, borrowed from the [`GeometryValues`] it was decoded into.
///
/// Made by [`GeometryValues::geometry`], which validated it, so every method here and on
/// the views it hands out is total. It is `Copy` and holds no allocation.
#[derive(Clone, Copy, Debug)]
pub struct GeometryView<'a> {
    values: &'a GeometryValues,
    index: usize,
    /// The stored vertices of the feature, or `None` for a triangles-only feature,
    /// whose corners are not a contiguous run.
    vertex_span: Option<Span>,
    shape: Shape<'a>,
}

#[derive(Clone, Copy, Debug)]
enum Shape<'a> {
    Point(PointView<'a>),
    LineString(LineView<'a>),
    Polygon(PolygonView<'a>),
    MultiPoint(MultiPointView<'a>),
    MultiLineString(MultiLineStringView<'a>),
    MultiPolygon(MultiPolygonView<'a>),
}

impl<'a> GeometryView<'a> {
    /// The feature's type in the layer's own terms.
    #[must_use]
    pub fn geometry_type(&self) -> GeometryType {
        self.values.vector_types[self.index]
    }

    /// Which feature of the layer this is.
    #[must_use]
    pub fn index(&self) -> usize {
        self.index
    }

    /// The range of the layer's vertex sequence the feature owns, closing vertices
    /// excluded, which m-values and z run over; see [`GeometryValues::vertex_range`].
    ///
    /// `None` for a feature stored as triangles alone, whose corners index the buffer
    /// in no particular order.
    #[must_use]
    pub fn vertex_range(&self) -> Option<Range<usize>> {
        self.vertex_span.map(Span::range)
    }

    /// The feature's stored vertices in storage order: the vertices of
    /// [`Self::vertex_range`], or every triangle's three corners for a triangles-only
    /// feature. A ring's closing vertex is not among them.
    #[must_use]
    pub fn vertices(&self) -> Vertices<'a> {
        let seq = match (self.vertex_span, &self.shape) {
            (Some(span), _) => VertexSeq::Run(span.range()),
            (
                None,
                Shape::MultiPolygon(MultiPolygonView {
                    polygons: Polygons::Triangles(run),
                    ..
                }),
            ) => VertexSeq::Corners(self.values.corners()[run.range()].iter()),
            (None, _) => VertexSeq::Run(0..0),
        };
        Vertices {
            values: self.values,
            seq,
        }
    }
}

impl<'a> GeometryTrait for GeometryView<'a> {
    view_slots!('a);

    fn as_type(&self) -> Kind<'_, 'a> {
        match &self.shape {
            Shape::Point(p) => GeoKind::Point(p),
            Shape::LineString(l) => GeoKind::LineString(l),
            Shape::Polygon(p) => GeoKind::Polygon(p),
            Shape::MultiPoint(m) => GeoKind::MultiPoint(m),
            Shape::MultiLineString(m) => GeoKind::MultiLineString(m),
            Shape::MultiPolygon(m) => GeoKind::MultiPolygon(m),
        }
    }
}

/// The stored vertices of a feature, see [`GeometryView::vertices`].
#[derive(Clone, Debug)]
pub struct Vertices<'a> {
    values: &'a GeometryValues,
    seq: VertexSeq<'a>,
}

#[derive(Clone, Debug)]
enum VertexSeq<'a> {
    Run(Range<usize>),
    Corners(std::slice::Iter<'a, u32>),
}

impl Iterator for Vertices<'_> {
    type Item = Vertex;

    fn next(&mut self) -> Option<Vertex> {
        let index = match &mut self.seq {
            VertexSeq::Run(run) => run.next()?,
            VertexSeq::Corners(corners) => corners.next()?.into_usize(),
        };
        Some(self.values.vertex(index))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        match &self.seq {
            VertexSeq::Run(run) => run.size_hint(),
            VertexSeq::Corners(corners) => corners.size_hint(),
        }
    }
}

impl ExactSizeIterator for Vertices<'_> {}

/// A point feature, or one point of a multi-point.
#[derive(Clone, Copy, Debug)]
pub struct PointView<'a> {
    values: &'a GeometryValues,
    vertex: usize,
}

impl<'a> GeometryTrait for PointView<'a> {
    view_slots!('a);

    fn as_type(&self) -> Kind<'_, 'a> {
        GeoKind::Point(self)
    }
}

impl PointTrait for PointView<'_> {
    type CoordType<'b>
        = Vertex
    where
        Self: 'b;

    fn coord(&self) -> Option<Vertex> {
        Some(self.values.vertex(self.vertex))
    }
}

/// Where a line's coordinates come from.
#[derive(Clone, Copy, Debug)]
enum Corners {
    /// A run of stored vertices, closed when it is a ring.
    Run { span: Span, ring: bool },
    /// The three corners of a triangle, named by the index buffer from `first`.
    Triangle { first: usize },
}

/// A line string, or a ring of a polygon.
///
/// A ring is closed: its last coordinate repeats its first, which the layer does not store.
#[derive(Clone, Copy, Debug)]
pub struct LineView<'a> {
    values: &'a GeometryValues,
    corners: Corners,
}

impl LineView<'_> {
    /// Whether this is a polygon ring, which the view closes.
    #[must_use]
    pub fn is_ring(&self) -> bool {
        match self.corners {
            Corners::Run { ring, .. } => ring,
            Corners::Triangle { .. } => true,
        }
    }

    /// The stored vertices of this part, closing vertex excluded, or `None` for the
    /// ring of a triangle, whose corners are not a contiguous run.
    #[must_use]
    pub fn vertex_range(&self) -> Option<Range<usize>> {
        match self.corners {
            Corners::Run { span, .. } => Some(span.range()),
            Corners::Triangle { .. } => None,
        }
    }

    /// How many vertices the layer stores for this part.
    fn stored(&self) -> usize {
        match self.corners {
            Corners::Run { span, .. } => span.len(),
            Corners::Triangle { .. } => 3,
        }
    }

    /// The layer vertex behind stored coordinate `i`.
    fn stored_vertex(&self, i: usize) -> usize {
        match self.corners {
            Corners::Run { span, .. } => span.start + i,
            Corners::Triangle { first } => self.values.corners()[first + i].into_usize(),
        }
    }
}

impl<'a> GeometryTrait for LineView<'a> {
    view_slots!('a);

    fn as_type(&self) -> Kind<'_, 'a> {
        GeoKind::LineString(self)
    }
}

impl LineStringTrait for LineView<'_> {
    type CoordType<'b>
        = Vertex
    where
        Self: 'b;

    fn num_coords(&self) -> usize {
        let stored = self.stored();
        if self.is_ring() && stored > 0 {
            stored + 1
        } else {
            stored
        }
    }

    #[expect(unsafe_code, reason = "see UNCHECKED")]
    unsafe fn coord_unchecked(&self, i: usize) -> Vertex {
        let _ = UNCHECKED;
        let i = if i == self.stored() { 0 } else { i };
        self.values.vertex(self.stored_vertex(i))
    }
}

/// Where a polygon's rings come from.
#[derive(Clone, Copy, Debug)]
enum Rings {
    /// A run of the layer's rings, exterior first.
    Offsets(Span),
    /// One triangle of the index buffer, from `first`, as a single ring.
    Triangle { first: usize },
}

/// A polygon feature, one polygon of a multi-polygon, or one triangle of a
/// triangles-only feature.
#[derive(Clone, Copy, Debug)]
pub struct PolygonView<'a> {
    values: &'a GeometryValues,
    rings: Rings,
}

impl<'a> PolygonView<'a> {
    fn ring(&self, i: usize) -> LineView<'a> {
        let corners = match self.rings {
            Rings::Offsets(span) => Corners::Run {
                span: self.values.ring_span(span.start + i),
                ring: true,
            },
            Rings::Triangle { first } => Corners::Triangle { first },
        };
        LineView {
            values: self.values,
            corners,
        }
    }

    fn ring_count(&self) -> usize {
        match self.rings {
            Rings::Offsets(span) => span.len(),
            Rings::Triangle { .. } => 1,
        }
    }
}

impl<'a> GeometryTrait for PolygonView<'a> {
    view_slots!('a);

    fn as_type(&self) -> Kind<'_, 'a> {
        GeoKind::Polygon(self)
    }
}

impl<'a> PolygonTrait for PolygonView<'a> {
    type RingType<'b>
        = LineView<'a>
    where
        Self: 'b;

    /// `None` for a polygon with no rings or an empty exterior, as for a `geo_types`
    /// polygon whose exterior has no coordinates.
    fn exterior(&self) -> Option<LineView<'a>> {
        if self.ring_count() == 0 {
            return None;
        }
        let exterior = self.ring(0);
        (exterior.stored() > 0).then_some(exterior)
    }

    fn num_interiors(&self) -> usize {
        self.ring_count().saturating_sub(1)
    }

    #[expect(unsafe_code, reason = "see UNCHECKED")]
    unsafe fn interior_unchecked(&self, i: usize) -> LineView<'a> {
        self.ring(i + 1)
    }
}

/// How a multi-point reaches each point's vertex.
#[derive(Clone, Copy, Debug)]
enum PointVia {
    /// `geometry_offsets` name vertices directly.
    Direct,
    /// `geometry_offsets` name parts, which name vertices.
    Parts,
    /// `geometry_offsets` name parts, which name rings, which name vertices.
    PartsThenRings,
}

/// A multi-point feature.
#[derive(Clone, Copy, Debug)]
pub struct MultiPointView<'a> {
    values: &'a GeometryValues,
    points: Span,
    via: PointVia,
}

impl<'a> GeometryTrait for MultiPointView<'a> {
    view_slots!('a);

    fn as_type(&self) -> Kind<'_, 'a> {
        GeoKind::MultiPoint(self)
    }
}

impl<'a> MultiPointTrait for MultiPointView<'a> {
    type InnerPointType<'b>
        = PointView<'a>
    where
        Self: 'b;

    fn num_points(&self) -> usize {
        self.points.len()
    }

    #[expect(unsafe_code, reason = "see UNCHECKED")]
    unsafe fn point_unchecked(&self, i: usize) -> PointView<'a> {
        let idx = self.points.start + i;
        let vertex = match self.via {
            PointVia::Direct => idx,
            PointVia::Parts => self.values.parts()[idx].into_usize(),
            PointVia::PartsThenRings => {
                let ring = self.values.parts()[idx].into_usize();
                self.values.rings()[ring].into_usize()
            }
        };
        PointView {
            values: self.values,
            vertex,
        }
    }
}

/// A multi-line-string feature.
#[derive(Clone, Copy, Debug)]
pub struct MultiLineStringView<'a> {
    values: &'a GeometryValues,
    parts: Span,
    /// Whether a part names a ring that holds the vertices, as in a layer that also
    /// has polygons, rather than the vertices themselves.
    via_rings: bool,
}

impl<'a> GeometryTrait for MultiLineStringView<'a> {
    view_slots!('a);

    fn as_type(&self) -> Kind<'_, 'a> {
        GeoKind::MultiLineString(self)
    }
}

impl<'a> MultiLineStringTrait for MultiLineStringView<'a> {
    type InnerLineStringType<'b>
        = LineView<'a>
    where
        Self: 'b;

    fn num_line_strings(&self) -> usize {
        self.parts.len()
    }

    #[expect(unsafe_code, reason = "see UNCHECKED")]
    unsafe fn line_string_unchecked(&self, i: usize) -> LineView<'a> {
        let part = self.parts.start + i;
        let span = if self.via_rings {
            self.values
                .ring_span(self.values.parts()[part].into_usize())
        } else {
            self.values.part_span(part)
        };
        LineView {
            values: self.values,
            corners: Corners::Run { span, ring: false },
        }
    }
}

/// Where a multi-polygon's polygons come from.
#[derive(Clone, Copy, Debug)]
enum Polygons {
    /// A run of parts, each a run of rings.
    Parts(Span),
    /// A run of the index buffer, three corners per polygon.
    Triangles(Span),
}

/// A multi-polygon feature, or a triangles-only feature as one polygon per triangle.
#[derive(Clone, Copy, Debug)]
pub struct MultiPolygonView<'a> {
    values: &'a GeometryValues,
    polygons: Polygons,
}

impl<'a> GeometryTrait for MultiPolygonView<'a> {
    view_slots!('a);

    fn as_type(&self) -> Kind<'_, 'a> {
        GeoKind::MultiPolygon(self)
    }
}

impl<'a> MultiPolygonTrait for MultiPolygonView<'a> {
    type InnerPolygonType<'b>
        = PolygonView<'a>
    where
        Self: 'b;

    fn num_polygons(&self) -> usize {
        match self.polygons {
            Polygons::Parts(span) => span.len(),
            Polygons::Triangles(span) => span.len() / 3,
        }
    }

    #[expect(unsafe_code, reason = "see UNCHECKED")]
    unsafe fn polygon_unchecked(&self, i: usize) -> PolygonView<'a> {
        let rings = match self.polygons {
            Polygons::Parts(span) => Rings::Offsets(self.values.part_span(span.start + i)),
            Polygons::Triangles(span) => Rings::Triangle {
                first: span.start + 3 * i,
            },
        };
        PolygonView {
            values: self.values,
            rings,
        }
    }
}
