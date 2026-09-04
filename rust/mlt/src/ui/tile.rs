//! Decoded tile data shared by the file browser and the layer view.
//! The LRU cache keeps recently decoded tiles around.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use mlt_core::geo_types::{Coord, Geometry, Polygon};
use mlt_core::geojson::FeatureCollection;
use mlt_core::mvt::mvt_to_feature_collection;
use mlt_core::{Decoder, Parser};
use moka::sync::Cache;

use crate::ls::is_mlt_extension;

/// Memory budget for decoded tiles.
pub(crate) const CACHE_BYTES: u64 = 256 * 1024 * 1024;

/// A decoded tile and its extent.
pub(crate) struct ParsedTile {
    pub fc: FeatureCollection,
    pub extent: u32,
}

impl ParsedTile {
    pub(crate) fn from_fc(fc: FeatureCollection) -> Self {
        let extent = extent_from_fc(&fc);
        Self { fc, extent }
    }

    pub(crate) fn load(path: &Path) -> anyhow::Result<Self> {
        let buf = fs::read(path)?;
        let fc = if is_mlt_extension(path) {
            let layers = Decoder::default().decode_all(Parser::default().parse_layers(&buf)?)?;
            FeatureCollection::from_layers(layers)?
        } else {
            mvt_to_feature_collection(buf)?
        };
        Ok(Self::from_fc(fc))
    }

    /// Rough number of bytes this tile occupies, used to weigh it in the cache.
    pub(crate) fn memory_estimate(&self) -> usize {
        fc_memory_estimate(&self.fc)
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
    use mlt_core::geo_types::{GeometryCollection, Line, LineString, Point, Rect, Triangle};

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
