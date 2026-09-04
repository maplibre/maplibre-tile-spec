//! Decoded tile data shared by the file browser and the layer view.
//! The LRU cache keeps recently decoded tiles around.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use mlt_core::geo_types::{Coord, Geometry, LineString, Polygon};
use mlt_core::geojson::FeatureCollection;
use mlt_core::mvt::mvt_to_feature_collection;
use mlt_core::{Decoder, LendingIterator as _, ParsedLayer, Parser};
use moka::sync::Cache;
use usize_cast::IntoUsize as _;

use crate::ls::is_mlt_extension;

/// Memory budget for decoded tiles.
pub(crate) const CACHE_BYTES: u64 = 256 * 1024 * 1024;

/// One tessellation triangle in tile coordinates.
pub(crate) type Triangle = [Coord<i32>; 3];

/// Triangles of a tessellated feature, one list per polygon part (a plain polygon has one).
pub(crate) type Tessellation = Vec<Vec<Triangle>>;

/// A decoded tile with the tessellation triangles the encoder stored for its polygons.
/// `triangles` is indexed like `fc.features`.
pub(crate) struct ParsedTile {
    pub fc: FeatureCollection,
    pub extent: u32,
    pub triangles: Vec<Option<Tessellation>>,
}

impl ParsedTile {
    pub(crate) fn from_fc(fc: FeatureCollection) -> Self {
        let extent = extent_from_fc(&fc);
        let triangles = vec![None; fc.features.len()];
        Self {
            fc,
            extent,
            triangles,
        }
    }

    pub(crate) fn load(path: &Path) -> anyhow::Result<Self> {
        let buf = fs::read(path)?;
        if is_mlt_extension(path) {
            let layers = Decoder::default().decode_all(Parser::default().parse_layers(&buf)?)?;
            let mut triangles = layer_triangles(&layers)?;
            let fc = FeatureCollection::from_layers(layers)?;
            triangles.resize(fc.features.len(), None);
            Ok(Self {
                extent: extent_from_fc(&fc),
                fc,
                triangles,
            })
        } else {
            Ok(Self::from_fc(mvt_to_feature_collection(buf)?))
        }
    }

    /// Rough number of bytes this tile occupies, used to weigh it in the cache.
    pub(crate) fn memory_estimate(&self) -> usize {
        let triangles: usize = self
            .triangles
            .iter()
            .flatten()
            .flat_map(|parts| parts.iter().map(Vec::len))
            .sum();
        fc_memory_estimate(&self.fc) + triangles * size_of::<Triangle>()
    }
}

/// Rough number of bytes a decoded feature collection occupies.
/// The per-property and per-feature constants stand in for map node, string, and struct overhead.
pub(crate) fn fc_memory_estimate(fc: &FeatureCollection) -> usize {
    let coord = size_of::<Coord<i32>>();
    fc.features
        .iter()
        .map(|f| {
            let verts = geometry_coord_count(&f.geometry) * coord;
            let props: usize = f
                .properties
                .iter()
                .map(|(k, v)| k.len() + v.as_str().map_or(16, str::len) + 48)
                .sum();
            verts + props + 128
        })
        .sum()
}

pub(crate) fn extent_from_fc(fc: &FeatureCollection) -> u32 {
    fc.features
        .first()
        .and_then(|f| {
            f.properties
                .get("_extent")
                .and_then(serde_json::Value::as_u64)
        })
        .map_or(4096, |v| u32::try_from(v).unwrap_or(4096))
}

pub(crate) fn geometry_coord_count(geom: &Geometry<i32>) -> usize {
    match geom {
        Geometry::<i32>::Point(_) => 1,
        Geometry::<i32>::Line(_) | Geometry::<i32>::Rect(_) => 2,
        Geometry::<i32>::Triangle(_) => 3,
        Geometry::<i32>::LineString(ls) => ls.0.len(),
        Geometry::<i32>::Polygon(p) => polygon_coord_count(p),
        Geometry::<i32>::MultiPoint(mp) => mp.0.len(),
        Geometry::<i32>::MultiLineString(mls) => mls.iter().map(|ls| ls.0.len()).sum(),
        Geometry::<i32>::MultiPolygon(mp) => mp.iter().map(polygon_coord_count).sum(),
        Geometry::<i32>::GeometryCollection(gc) => gc.iter().map(geometry_coord_count).sum(),
    }
}

pub(crate) fn polygon_coord_count(poly: &Polygon<i32>) -> usize {
    poly.exterior().0.len() + poly.interiors().iter().map(|r| r.0.len()).sum::<usize>()
}

/// Polygon vertices in the order the encoder tessellated them, plus where each part starts.
/// Rings drop their closing vertex, and anything that is not a polygon yields `None`.
fn tessellation_vertices(geom: &Geometry<i32>) -> Option<(Vec<Coord<i32>>, Vec<usize>)> {
    fn push_ring(out: &mut Vec<Coord<i32>>, ring: &LineString<i32>) {
        let coords = &ring.0;
        let closed = coords.len() > 1 && coords.first() == coords.last();
        let n = if closed {
            coords.len() - 1
        } else {
            coords.len()
        };
        out.extend_from_slice(&coords[..n]);
    }
    fn push_polygon(out: &mut Vec<Coord<i32>>, starts: &mut Vec<usize>, poly: &Polygon<i32>) {
        starts.push(out.len());
        push_ring(out, poly.exterior());
        for ring in poly.interiors() {
            push_ring(out, ring);
        }
    }
    let mut out = Vec::new();
    let mut starts = Vec::new();
    match geom {
        Geometry::<i32>::Polygon(poly) => push_polygon(&mut out, &mut starts, poly),
        Geometry::<i32>::MultiPolygon(mp) => {
            for poly in mp {
                push_polygon(&mut out, &mut starts, poly);
            }
        }
        Geometry::<i32>::Point(_)
        | Geometry::<i32>::Line(_)
        | Geometry::<i32>::LineString(_)
        | Geometry::<i32>::MultiPoint(_)
        | Geometry::<i32>::MultiLineString(_)
        | Geometry::<i32>::GeometryCollection(_)
        | Geometry::<i32>::Rect(_)
        | Geometry::<i32>::Triangle(_) => return None,
    }
    Some((out, starts))
}

/// Per-feature triangles from the tessellation streams, in [`FeatureCollection::from_layers`] order.
fn layer_triangles(layers: &[ParsedLayer<'_>]) -> anyhow::Result<Vec<Option<Tessellation>>> {
    let mut out = Vec::new();
    for layer in layers {
        let Some(layer) = layer.as_layer01() else {
            continue;
        };
        let values = layer.geometry_values();
        let (Some(counts), Some(indices)) = (values.triangles(), values.index_buffer()) else {
            out.extend(std::iter::repeat_n(None, layer.feature_count()));
            continue;
        };
        let mut counts = counts.iter();
        let mut next_index = 0usize;
        let mut features = layer.iter_features();
        while let Some(feat) = features.next() {
            let feat = feat?;
            let Some((verts, starts)) = tessellation_vertices(feat.geometry()) else {
                out.push(None);
                continue;
            };
            let Some(count) = counts.next() else {
                out.push(None);
                continue;
            };
            let end = next_index + count.into_usize() * 3;
            let mut parts: Tessellation = vec![Vec::new(); starts.len()];
            let feature_indices = indices.get(next_index..end).unwrap_or(&[]);
            for t in feature_indices.as_chunks::<3>().0 {
                let idx = [t[0].into_usize(), t[1].into_usize(), t[2].into_usize()];
                let Some(tri) = idx
                    .iter()
                    .map(|&i| verts.get(i).copied())
                    .collect::<Option<Vec<_>>>()
                else {
                    continue;
                };
                let part = starts.partition_point(|&s| s <= idx[0]).saturating_sub(1);
                if let Some(slot) = parts.get_mut(part) {
                    slot.push([tri[0], tri[1], tri[2]]);
                }
            }
            next_index = end;
            out.push(Some(parts));
        }
    }
    Ok(out)
}

/// Memory-bounded LRU cache of decoded tiles keyed by path.
#[derive(Clone)]
pub(crate) struct TileCache(Cache<PathBuf, Arc<ParsedTile>>);

impl TileCache {
    pub(crate) fn new(max_bytes: u64) -> Self {
        Self(
            Cache::builder()
                .max_capacity(max_bytes)
                .weigher(|_, tile: &Arc<ParsedTile>| {
                    u32::try_from(tile.memory_estimate()).unwrap_or(u32::MAX)
                })
                .build(),
        )
    }

    pub(crate) fn get(&self, path: &Path) -> Option<Arc<ParsedTile>> {
        self.0.get(path)
    }

    /// Return the cached tile for `path`, decoding and caching it first if needed.
    pub(crate) fn load(&self, path: &Path) -> anyhow::Result<Arc<ParsedTile>> {
        if let Some(tile) = self.0.get(path) {
            return Ok(tile);
        }
        let tile = Arc::new(ParsedTile::load(path)?);
        self.0.insert(path.to_path_buf(), Arc::clone(&tile));
        Ok(tile)
    }
}

impl Default for TileCache {
    fn default() -> Self {
        Self::new(CACHE_BYTES)
    }
}

#[cfg(test)]
mod tests {
    use mlt_core::geo_types::{
        GeometryCollection, Line, LineString, MultiPolygon, Point, Rect, Triangle,
    };

    use super::*;

    fn c(x: i32, y: i32) -> Coord<i32> {
        Coord { x, y }
    }

    impl TileCache {
        pub(crate) fn cached_paths(&self) -> Vec<PathBuf> {
            self.0.run_pending_tasks();
            let mut paths: Vec<PathBuf> = self.0.iter().map(|(k, _)| (*k).clone()).collect();
            paths.sort();
            paths
        }
    }

    #[test]
    fn tessellation_vertices_drop_closing_points_and_mark_part_starts() {
        let closed = LineString(vec![c(0, 0), c(4, 0), c(4, 4), c(0, 4), c(0, 0)]);
        let open = LineString(vec![c(1, 1), c(2, 1), c(2, 2)]);
        let poly = Polygon::new(closed.clone(), vec![open.clone()]);
        let (verts, starts) = tessellation_vertices(&Geometry::Polygon(poly.clone())).unwrap();
        assert_eq!(verts.len(), 7, "four exterior and three interior vertices");
        assert_eq!(starts, vec![0]);

        let multi = MultiPolygon(vec![poly, Polygon::new(open, vec![])]);
        let (verts, starts) = tessellation_vertices(&Geometry::MultiPolygon(multi)).unwrap();
        assert_eq!(verts.len(), 10);
        assert_eq!(starts, vec![0, 7]);

        assert!(tessellation_vertices(&Geometry::Point(Point(c(1, 1)))).is_none());
        assert!(tessellation_vertices(&Geometry::LineString(closed)).is_none());
    }

    #[test]
    fn coord_counts_cover_every_geometry_kind() {
        let ring = LineString(vec![c(0, 0), c(4, 0), c(4, 4)]);
        assert_eq!(geometry_coord_count(&Geometry::Point(Point(c(1, 1)))), 1);
        assert_eq!(
            geometry_coord_count(&Geometry::Line(Line::new(c(0, 0), c(1, 1)))),
            2
        );
        assert_eq!(
            geometry_coord_count(&Geometry::Rect(Rect::new(c(0, 0), c(1, 1)))),
            2
        );
        assert_eq!(
            geometry_coord_count(&Geometry::Triangle(Triangle::new(
                c(0, 0),
                c(1, 0),
                c(0, 1)
            ))),
            3
        );
        assert_eq!(geometry_coord_count(&Geometry::LineString(ring.clone())), 3);
        assert_eq!(
            geometry_coord_count(&Geometry::Polygon(Polygon::new(
                ring.clone(),
                vec![ring.clone()]
            ))),
            8,
            "geo closes both rings"
        );
        let gc = GeometryCollection(vec![
            Geometry::LineString(ring),
            Geometry::Point(Point(c(0, 0))),
        ]);
        assert_eq!(geometry_coord_count(&Geometry::GeometryCollection(gc)), 4);
    }

    #[test]
    fn extent_falls_back_when_the_property_is_missing_or_huge() {
        let fc = FeatureCollection {
            features: Vec::new(),
            ty: "FeatureCollection".into(),
        };
        assert_eq!(extent_from_fc(&fc), 4096);
    }
}
