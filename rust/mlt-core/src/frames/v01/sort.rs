//! Feature reordering for the optimizer.
//!
//! All sorting operates on [`TileLayer01`] — the row-oriented working form
//! that the optimizer uses.  A sort is a single `Vec::sort_by_cached_key`
//! call; there is no permutation machinery, no column-by-column scatter, and
//! no encoded/decoded conversions inside this module.

use crate::geojson::Geom32;
use crate::utils::{hilbert_curve_params, hilbert_sort_key, morton_sort_key};
use crate::v01::{TileFeature, TileLayer01};

/// Controls how features inside a layer are reordered before encoding.
///
/// Reordering features changes their position in every parallel column
/// (geometry, ID, and all properties simultaneously), so the caller must
/// opt in explicitly.
#[derive(Debug, Clone, Copy, PartialEq, Eq, strum::EnumIter, strum::EnumCount)]
pub enum SortStrategy {
    /// Sort features by the Z-order (Morton) curve index of their first vertex.
    ///
    /// Fast to compute.  Spatially close features end up adjacent in the
    /// stream, improving RLE run lengths for location-correlated properties
    /// and CPU cache locality during client-side decoding.
    SpatialMorton,

    /// Sort features by the Hilbert curve index of their first vertex.
    ///
    /// Slower to compute than Morton but achieves superior spatial locality.
    SpatialHilbert,

    /// Sort features by their feature ID in ascending order.
    Id,
}

/// Reorder all features of `layer` according to `strategy`.
///
/// If `strategy` is [`None`] this is a no-op.
/// If the layer has zero or one features the sort is trivially a no-op.
pub(crate) fn reorder_features(layer: &mut TileLayer01, strategy: Option<SortStrategy>) {
    let Some(strategy) = strategy else {
        return;
    };

    if layer.features.len() <= 1 {
        return;
    }

    let curve_params = curve_params_from_features(&layer.features);
    layer
        .features
        .sort_by_cached_key(|f| sort_key(f, strategy, &curve_params));
}

/// Compute the sort key for a single feature under `strategy`.
fn sort_key(f: &TileFeature, strategy: SortStrategy, params: &CurveParams) -> u64 {
    match strategy {
        SortStrategy::SpatialMorton => first_vertex(&f.geometry).map_or(u64::MAX, |(x, y)| {
            u64::from(morton_sort_key(x, y, params.shift, params.num_bits))
        }),
        SortStrategy::SpatialHilbert => first_vertex(&f.geometry).map_or(u64::MAX, |(x, y)| {
            u64::from(hilbert_sort_key(x, y, params.shift, params.num_bits))
        }),
        SortStrategy::Id => f.id.map_or(0, |v| v.saturating_add(1)),
    }
}

/// Parameters derived from the vertex set of a feature collection, used to
/// normalise coordinates before space-filling-curve key computation.
pub(crate) struct CurveParams {
    pub shift: u32,
    pub num_bits: u32,
}

/// Collect all vertex coordinates from `features` and compute the Hilbert/Morton
/// curve parameters (coordinate shift and bit width).
pub(crate) fn curve_params_from_features(features: &[TileFeature]) -> CurveParams {
    // Collect a flat `[x0, y0, x1, y1, …]` vertex array from all features,
    // then reuse the existing `hilbert_curve_params` utility.
    let verts: Vec<i32> = features
        .iter()
        .flat_map(|f| geom_vertices(&f.geometry))
        .collect();

    let (shift, num_bits) = hilbert_curve_params(&verts);
    CurveParams { shift, num_bits }
}

/// Flatten a geometry into its raw vertex sequence as `[x0, y0, x1, y1, …]`.
fn geom_vertices(geom: &Geom32) -> Vec<i32> {
    let mut out = Vec::new();
    push_geom_vertices(geom, &mut out);
    out
}

fn push_geom_vertices(geom: &Geom32, out: &mut Vec<i32>) {
    match geom {
        Geom32::Point(p) => {
            out.push(p.0.x);
            out.push(p.0.y);
        }
        Geom32::Line(l) => {
            out.extend([l.start.x, l.start.y, l.end.x, l.end.y]);
        }
        Geom32::LineString(ls) => {
            for c in &ls.0 {
                out.push(c.x);
                out.push(c.y);
            }
        }
        Geom32::Polygon(p) => {
            for c in &p.exterior().0 {
                out.push(c.x);
                out.push(c.y);
            }
        }
        Geom32::MultiPoint(mp) => {
            for p in &mp.0 {
                out.push(p.0.x);
                out.push(p.0.y);
            }
        }
        Geom32::MultiLineString(mls) => {
            for ls in &mls.0 {
                for c in &ls.0 {
                    out.push(c.x);
                    out.push(c.y);
                }
            }
        }
        Geom32::MultiPolygon(mp) => {
            for poly in &mp.0 {
                for c in &poly.exterior().0 {
                    out.push(c.x);
                    out.push(c.y);
                }
            }
        }
        Geom32::Triangle(t) => {
            out.extend([t.0.x, t.0.y, t.1.x, t.1.y, t.2.x, t.2.y]);
        }
        Geom32::Rect(r) => {
            let min = r.min();
            let max = r.max();
            out.extend([min.x, min.y, max.x, max.y]);
        }
        Geom32::GeometryCollection(gc) => {
            for g in gc {
                push_geom_vertices(g, out);
            }
        }
    }
}

/// Extract the `(x, y)` coordinate of the first vertex of a geometry.
fn first_vertex(geom: &Geom32) -> Option<(i32, i32)> {
    match geom {
        Geom32::Point(p) => Some((p.0.x, p.0.y)),
        Geom32::Line(l) => Some((l.start.x, l.start.y)),
        Geom32::LineString(ls) => ls.0.first().map(|c| (c.x, c.y)),
        Geom32::Polygon(p) => p.exterior().0.first().map(|c| (c.x, c.y)),
        Geom32::MultiPoint(mp) => mp.0.first().map(|p| (p.0.x, p.0.y)),
        Geom32::MultiLineString(mls) => mls
            .0
            .first()
            .and_then(|ls| ls.0.first().map(|c| (c.x, c.y))),
        Geom32::MultiPolygon(mp) => {
            mp.0.first()
                .and_then(|p| p.exterior().0.first().map(|c| (c.x, c.y)))
        }
        Geom32::Triangle(t) => Some((t.0.x, t.0.y)),
        Geom32::Rect(r) => Some((r.min().x, r.min().y)),
        Geom32::GeometryCollection(gc) => gc.0.first().and_then(first_vertex),
    }
}

/// Return `true` if a spatial sort is likely to reduce compressed size.
///
/// The heuristic: if the vertex bounding box spans more than
/// `SPATIAL_HELP_COVERAGE` of the layer's tile extent on **both** axes, the
/// features are too spread-out for locality clustering to help, so spatial
/// sorting is skipped.
pub(crate) fn spatial_sort_likely_to_help(layer: &TileLayer01) -> bool {
    const SPATIAL_HELP_COVERAGE: f64 = 0.8;

    let extent = f64::from(layer.extent);
    if extent <= 0.0 || layer.features.is_empty() {
        return true;
    }

    let (min_x, max_x, min_y, max_y) = layer
        .features
        .iter()
        .filter_map(|f| first_vertex(&f.geometry))
        .fold(
            (i32::MAX, i32::MIN, i32::MAX, i32::MIN),
            |(min_x, max_x, min_y, max_y), (x, y)| {
                (min_x.min(x), max_x.max(x), min_y.min(y), max_y.max(y))
            },
        );

    if min_x > max_x || min_y > max_y {
        return true;
    }

    let range_x = f64::from(max_x - min_x);
    let range_y = f64::from(max_y - min_y);

    let spread_x = range_x / extent;
    let spread_y = range_y / extent;

    !(spread_x > SPATIAL_HELP_COVERAGE && spread_y > SPATIAL_HELP_COVERAGE)
}

#[cfg(test)]
mod tests {
    use geo_types::{Coord, Geometry as GeoGeom, LineString, Point, Polygon};

    use super::*;
    use crate::geojson::Geom32;
    use crate::v01::{
        Geometry, GeometryEncoder, GeometryValues, IntEncoder, RawGeometry, TileFeature,
        TileLayer01,
    };

    // ── geometry test helpers ──────────────────────────────────────────────────

    fn pt(x: i32, y: i32) -> Geom32 {
        GeoGeom::Point(Point::new(x, y))
    }

    fn ls(coords: &[(i32, i32)]) -> Geom32 {
        GeoGeom::LineString(LineString::new(
            coords.iter().map(|&(x, y)| Coord { x, y }).collect(),
        ))
    }

    fn poly_square(x0: i32, y0: i32, side: i32) -> Geom32 {
        let ring = LineString::new(vec![
            Coord { x: x0, y: y0 },
            Coord {
                x: x0 + side,
                y: y0,
            },
            Coord {
                x: x0 + side,
                y: y0 + side,
            },
            Coord {
                x: x0,
                y: y0 + side,
            },
            Coord { x: x0, y: y0 },
        ]);
        GeoGeom::Polygon(Polygon::new(ring, vec![]))
    }

    /// Encode + serialize + parse + decode a `GeometryValues` (round-trip).
    fn roundtrip_geom(decoded: &GeometryValues) -> GeometryValues {
        let encoded = decoded
            .clone()
            .encode(GeometryEncoder::all(IntEncoder::varint()))
            .expect("encode failed");

        let mut buf = Vec::new();
        encoded.write_to(&mut buf).expect("serialize failed");

        let (remaining, parsed) = RawGeometry::from_bytes(&buf).expect("parse failed");
        assert!(
            remaining.is_empty(),
            "unexpected trailing bytes after parse"
        );

        Geometry::Raw(parsed)
            .into_parsed(&mut crate::Decoder::default())
            .expect("decode failed")
    }

    /// Build the canonical (dense, wire-decoded) form of an ordered geometry sequence.
    fn canonical(geoms: &[Geom32]) -> GeometryValues {
        let mut decoded = GeometryValues::default();
        for g in geoms {
            decoded.push_geom(g);
        }
        roundtrip_geom(&decoded)
    }

    /// Build a `TileLayer01` from `geoms` and `ids`, apply `reorder_features`,
    /// and return it.
    fn layer_after_sort(geoms: &[Geom32], ids: &[u64], strategy: SortStrategy) -> TileLayer01 {
        let features: Vec<TileFeature> = geoms
            .iter()
            .zip(ids.iter())
            .map(|(g, &id)| TileFeature {
                id: Some(id),
                geometry: g.clone(),
                properties: vec![],
            })
            .collect();

        let mut layer = TileLayer01 {
            name: "test".to_string(),
            extent: 4096,
            property_names: vec![],
            features,
        };

        reorder_features(&mut layer, Some(strategy));
        layer
    }

    /// Sort, then encode+decode the result and compare to `canonical(expected)`.
    fn assert_sort_roundtrip(
        geoms: &[Geom32],
        ids: &[u64],
        strategy: SortStrategy,
        expected: &[Geom32],
    ) {
        let layer = layer_after_sort(geoms, ids, strategy);

        let mut sorted_decoded = GeometryValues::default();
        for f in &layer.features {
            sorted_decoded.push_geom(&f.geometry);
        }

        let after_roundtrip = roundtrip_geom(&sorted_decoded);
        let expected_canonical = canonical(expected);

        assert_eq!(
            after_roundtrip, expected_canonical,
            "\nsorted geometry did not match expected after encode→decode round-trip\
             \nvector_types after sort: {:?}\
             \nvector_types expected:   {:?}",
            sorted_decoded.vector_types, expected_canonical.vector_types,
        );
    }

    // ── pure Points ──────────────────────────────────────────────────────────

    #[test]
    fn pure_points_id_sort_roundtrip() {
        assert_sort_roundtrip(
            &[pt(0, 0), pt(1, 1), pt(2, 2)],
            &[3, 2, 1],
            SortStrategy::Id,
            &[pt(2, 2), pt(1, 1), pt(0, 0)],
        );
    }

    // ── pure LineStrings ─────────────────────────────────────────────────────

    #[test]
    fn pure_linestrings_id_sort_roundtrip() {
        assert_sort_roundtrip(
            &[ls(&[(0, 0), (0, 10)]), ls(&[(5, 5), (10, 10)])],
            &[2, 1],
            SortStrategy::Id,
            &[ls(&[(5, 5), (10, 10)]), ls(&[(0, 0), (0, 10)])],
        );
    }

    // ── [Point, LineString, Point] ────────────────────────────────────────────

    #[test]
    fn point_line_point_id_sort_to_line_point_point_roundtrip() {
        assert_sort_roundtrip(
            &[pt(0, 0), ls(&[(1, 0), (1, 5)]), pt(5, 5)],
            &[3, 1, 2],
            SortStrategy::Id,
            &[ls(&[(1, 0), (1, 5)]), pt(5, 5), pt(0, 0)],
        );
    }

    #[test]
    fn point_line_point_id_sort_to_point_point_line_roundtrip() {
        assert_sort_roundtrip(
            &[pt(0, 0), ls(&[(1, 0), (1, 5)]), pt(5, 5)],
            &[1, 3, 2],
            SortStrategy::Id,
            &[pt(0, 0), pt(5, 5), ls(&[(1, 0), (1, 5)])],
        );
    }

    // ── [Point, Polygon, Point] ───────────────────────────────────────────────

    #[test]
    fn point_polygon_point_id_sort_roundtrip() {
        assert_sort_roundtrip(
            &[pt(0, 0), poly_square(10, 10, 5), pt(5, 5)],
            &[2, 1, 3],
            SortStrategy::Id,
            &[poly_square(10, 10, 5), pt(0, 0), pt(5, 5)],
        );
    }

    // ── spatial Morton sort ───────────────────────────────────────────────────

    #[test]
    fn point_line_point_morton_sort_roundtrip() {
        assert_sort_roundtrip(
            &[pt(2, 0), ls(&[(0, 0), (0, 5)]), pt(1, 0)],
            &[1, 2, 3],
            SortStrategy::SpatialMorton,
            &[ls(&[(0, 0), (0, 5)]), pt(1, 0), pt(2, 0)],
        );
    }

    // ── already-sorted is identity ────────────────────────────────────────────

    #[test]
    fn id_sort_already_sorted_is_identity_roundtrip() {
        let geoms = &[pt(0, 0), ls(&[(1, 0), (1, 5)]), pt(5, 5)];
        assert_sort_roundtrip(geoms, &[1, 2, 3], SortStrategy::Id, geoms);
    }

    // ── ID column co-permuted with geometry ───────────────────────────────────

    #[test]
    fn id_column_co_permuted_with_geometry() {
        let layer = layer_after_sort(
            &[pt(0, 0), ls(&[(1, 0), (1, 5)]), pt(5, 5)],
            &[3, 1, 2],
            SortStrategy::Id,
        );

        let ids: Vec<Option<u64>> = layer.features.iter().map(|f| f.id).collect();
        assert_eq!(ids, vec![Some(1u64), Some(2), Some(3)]);

        // Verify geometry types match expected order
        let geom_types: Vec<&str> = layer
            .features
            .iter()
            .map(|f| match &f.geometry {
                GeoGeom::Point(_) => "Point",
                GeoGeom::LineString(_) => "LineString",
                _ => "Other",
            })
            .collect();
        assert_eq!(geom_types, vec!["LineString", "Point", "Point"]);
    }
}
