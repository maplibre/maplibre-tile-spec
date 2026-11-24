//! Feature reordering for the optimizer

use std::cmp::Ordering;

use geo_types::Coord;
use rustc_hash::FxHashMap;

use crate::codecs::hilbert::{hilbert_curve_params_from_bounds, hilbert_sort_key};
use crate::codecs::morton::morton_sort_key;
use crate::encoder::geometry::{GeometryStager, distance};
use crate::encoder::model::CurveParams;
use crate::encoder::source::{LayerSource, Order};
use crate::encoder::tile::empty_geometry;
use crate::tile::TileLayer;

/// Controls how features inside a layer are reordered before encoding.
///
/// Reordering features changes their position in every parallel column
/// (geometry, ID, and all properties simultaneously), so the caller must
/// opt in explicitly.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, strum::EnumIter, strum::EnumCount)]
pub enum SortStrategy {
    /// Preserve the original feature order - no reordering is applied.
    ///
    /// This is the default.
    #[default]
    Unsorted,

    /// Sort features by the Z-order (Morton) curve index of their first vertex.
    ///
    /// Fast to compute.  Spatially close features end up adjacent in the
    /// stream, improving RLE run lengths for location-correlated properties
    /// and CPU cache locality during client-side decoding.
    ///
    SpatialMorton,

    /// Sort features by the Hilbert curve index of their first vertex.
    ///
    /// Slower to compute than Morton but achieves superior spatial locality.
    SpatialHilbert,

    /// Sort features by their feature ID in ascending order.
    Id,

    /// Start at the origin, then repeatedly take the feature whose first vertex is nearest the last vertex of the previous one.
    ///
    /// Features enter at their stored first vertex and leave at the vertex they store last, rings wound as stored.
    /// Features without a vertex come last.
    Chain,
}

impl TileLayer {
    /// Reorder features by `strategy`, using `params` as the curve normalization
    /// for [`SortStrategy::SpatialMorton`] / [`SortStrategy::SpatialHilbert`].
    ///
    /// `params` is taken as a parameter (rather than recomputed here) so the
    /// same scan feeds the encoder's dictionary builders, see
    /// [`TileLayer::curve_params`].
    ///
    /// [`SortStrategy::Unsorted`] is a no-op; layers with ≤1 feature are
    /// trivially unchanged.
    #[hotpath::measure]
    pub fn sort(&mut self, strategy: SortStrategy, params: CurveParams) {
        if strategy == SortStrategy::Unsorted {
            return;
        }
        let order = sort_order(self, strategy, params, false);
        let mut features: Vec<_> = std::mem::take(&mut self.features)
            .into_iter()
            .map(Some)
            .collect();
        self.features = order
            .iter()
            .map(|f| features[f].take().expect("each feature appears once"))
            .collect();
    }

    /// Hilbert/Morton `CurveParams` for this layer.
    /// Bounds are order-invariant, so the optimizer computes this once per layer
    /// and reuses it across every sort trial and the encoder's dictionary builders.
    #[must_use]
    pub fn curve_params(&self) -> CurveParams {
        curve_params(self)
    }
}

/// The order `strategy` puts the features of `source` in, for features staged with vertex sort when `vertex_sort`.
#[hotpath::measure]
pub(crate) fn sort_order(
    source: &impl LayerSource,
    strategy: SortStrategy,
    params: CurveParams,
    vertex_sort: bool,
) -> Order {
    let len = source.feature_count();
    match strategy {
        SortStrategy::Unsorted => Order::Stored(len),
        SortStrategy::SpatialMorton | SortStrategy::SpatialHilbert => {
            let curve_key = if strategy == SortStrategy::SpatialMorton {
                morton_sort_key
            } else {
                hilbert_sort_key
            };
            Order::Permuted(permutation(len, |f| {
                source
                    .first_coord(f)
                    .map_or(u64::MAX, |c| u64::from(curve_key(c, params)))
            }))
        }
        SortStrategy::Id => Order::Permuted(permutation(len, |f| source.id(f))),
        SortStrategy::Chain => Order::Permuted(chain(source, vertex_sort)),
    }
}

/// The features of `source` in the order a walk from the origin visits them, always stepping to the feature whose first vertex is nearest the vertex stored last.
/// Features that start at one position wait in index order behind a single site.
fn chain(source: &impl LayerSource, vertex_sort: bool) -> Vec<u32> {
    let len = source.feature_count();
    let mut site_of = FxHashMap::<(i32, i32), usize>::default();
    let mut sites = Vec::new();
    let mut queues: Vec<Vec<usize>> = Vec::new();
    let mut without = Vec::new();
    for feature in 0..len {
        let Some(start) = source.first_coord(feature) else {
            without.push(feature);
            continue;
        };
        let site = *site_of.entry((start.x, start.y)).or_insert_with(|| {
            sites.push(start);
            queues.push(Vec::new());
            sites.len() - 1
        });
        queues[site].push(feature);
    }
    let mut order = Vec::with_capacity(len);
    let mut tree = SiteTree::new(&sites);
    let mut next = vec![0_usize; sites.len()];
    let mut values = empty_geometry(source, false);
    let mut stager = GeometryStager::new(&mut values).vertex_sorted(vertex_sort);
    let mut at = Coord { x: 0, y: 0 };
    loop {
        // A site at `at` is the only one that near.
        let continued = site_of
            .get(&(at.x, at.y))
            .copied()
            .filter(|&site| next[site] < queues[site].len());
        let Some(site) =
            continued.or_else(|| tree.nearest(&sites, at, |site| queues[site][next[site]]))
        else {
            break;
        };
        let feature = queues[site][next[site]];
        next[site] += 1;
        if next[site] == queues[site].len() {
            tree.remove(site);
        }
        order.push(feature);
        source.push_geometry(feature, &mut stager);
        stager.finish_feature();
        at = stager.last_vertex();
    }
    order.extend(without);
    order
        .into_iter()
        .map(|f| u32::try_from(f).expect("feature count fits in u32"))
        .collect()
}

/// Sites in a 2-d tree whose subtrees count the sites not yet removed, so a search skips the emptied ones.
/// Each subtree is a range of `slots` with its root in the middle.
struct SiteTree {
    /// The site at each slot.
    slots: Vec<u32>,
    /// The slot of each site.
    slot_of: Vec<u32>,
    /// The bounding box of the subtree rooted at each slot.
    min: Vec<Coord<i32>>,
    max: Vec<Coord<i32>>,
    /// Whether the subtree rooted at each slot splits on x rather than y.
    split_x: Vec<bool>,
    /// How many sites of the subtree rooted at each slot are not removed.
    live: Vec<u32>,
    /// Whether the site at each slot is removed.
    removed: Vec<bool>,
}

impl SiteTree {
    fn new(sites: &[Coord<i32>]) -> Self {
        let len = sites.len();
        let mut tree = Self {
            slots: (0..len)
                .map(|s| u32::try_from(s).expect("site count fits in u32"))
                .collect(),
            slot_of: vec![0; len],
            min: vec![Coord::default(); len],
            max: vec![Coord::default(); len],
            split_x: vec![false; len],
            live: vec![0; len],
            removed: vec![false; len],
        };
        tree.build(sites, 0, len);
        for (slot, &site) in tree.slots.iter().enumerate() {
            tree.slot_of[site as usize] = u32::try_from(slot).expect("site count fits in u32");
        }
        tree
    }

    /// Arranges `slots[lo..hi]` into a subtree.
    fn build(&mut self, sites: &[Coord<i32>], lo: usize, hi: usize) {
        if lo >= hi {
            return;
        }
        let at = |slot: &u32| sites[*slot as usize];
        let (min, max) = self.slots[lo..hi].iter().map(at).fold(
            (at(&self.slots[lo]), at(&self.slots[lo])),
            |(min, max), c| {
                (
                    Coord {
                        x: min.x.min(c.x),
                        y: min.y.min(c.y),
                    },
                    Coord {
                        x: max.x.max(c.x),
                        y: max.y.max(c.y),
                    },
                )
            },
        );
        let split_x = i64::from(max.x) - i64::from(min.x) >= i64::from(max.y) - i64::from(min.y);
        let mid = lo + (hi - lo) / 2;
        self.slots[lo..hi].select_nth_unstable_by_key(mid - lo, |&s| {
            let c = sites[s as usize];
            if split_x { (c.x, c.y) } else { (c.y, c.x) }
        });
        self.build(sites, lo, mid);
        self.build(sites, mid + 1, hi);
        self.min[mid] = min;
        self.max[mid] = max;
        self.split_x[mid] = split_x;
        self.live[mid] = u32::try_from(hi - lo).expect("site count fits in u32");
    }

    /// The site nearest `at` that is not removed, the one with the lowest `tie` of equally near ones.
    fn nearest<T: Ord + Copy>(
        &self,
        sites: &[Coord<i32>],
        at: Coord<i32>,
        tie: impl Fn(usize) -> T,
    ) -> Option<usize> {
        let mut best = None;
        self.search(sites, at, &tie, 0, self.slots.len(), &mut best);
        best.map(|(_, _, site)| site)
    }

    fn search<T: Ord + Copy>(
        &self,
        sites: &[Coord<i32>],
        at: Coord<i32>,
        tie: &impl Fn(usize) -> T,
        lo: usize,
        hi: usize,
        best: &mut Option<(u64, T, usize)>,
    ) {
        if lo >= hi {
            return;
        }
        let mid = lo + (hi - lo) / 2;
        if self.live[mid] == 0 {
            return;
        }
        let clamped = Coord {
            x: at.x.clamp(self.min[mid].x, self.max[mid].x),
            y: at.y.clamp(self.min[mid].y, self.max[mid].y),
        };
        // Equally near sites may still win on `tie`.
        if best.is_some_and(|(d, _, _)| distance(at, clamped) > d) {
            return;
        }
        let site = self.slots[mid] as usize;
        if !self.removed[mid] {
            let key = (distance(at, sites[site]), tie(site));
            if best.is_none_or(|(d, t, _)| key < (d, t)) {
                *best = Some((key.0, key.1, site));
            }
        }
        let below = if self.split_x[mid] {
            at.x < sites[site].x
        } else {
            at.y < sites[site].y
        };
        if below {
            self.search(sites, at, tie, lo, mid, best);
            self.search(sites, at, tie, mid + 1, hi, best);
        } else {
            self.search(sites, at, tie, mid + 1, hi, best);
            self.search(sites, at, tie, lo, mid, best);
        }
    }

    /// Removes `site`, which must not be removed yet.
    fn remove(&mut self, site: usize) {
        let slot = self.slot_of[site] as usize;
        self.removed[slot] = true;
        let (mut lo, mut hi) = (0, self.slots.len());
        loop {
            let mid = lo + (hi - lo) / 2;
            self.live[mid] -= 1;
            match slot.cmp(&mid) {
                Ordering::Less => hi = mid,
                Ordering::Greater => lo = mid + 1,
                Ordering::Equal => return,
            }
        }
    }
}

/// Feature indexes ordered by `key`, with ties in stored order, as a stable sort leaves them.
fn permutation<K: Ord>(len: usize, key: impl Fn(usize) -> K) -> Vec<u32> {
    let mut keyed: Vec<(K, u32)> = (0..len)
        .map(|f| (key(f), u32::try_from(f).expect("feature count fits in u32")))
        .collect();
    keyed.sort_unstable();
    keyed.into_iter().map(|(_, f)| f).collect()
}

/// Hilbert/Morton `CurveParams` covering every coordinate of `source`.
#[hotpath::measure]
pub(crate) fn curve_params(source: &impl LayerSource) -> CurveParams {
    let (min_val, max_val) = source.coords().fold((i32::MAX, i32::MIN), |(min, max), c| {
        (min.min(c.x).min(c.y), max.max(c.x).max(c.y))
    });
    hilbert_curve_params_from_bounds(min_val, max_val)
}

#[cfg(test)]
mod tests {
    use geo_types::{
        Coord, Geometry as GeoGeom, Geometry, GeometryCollection, Line, LineString,
        MultiLineString, MultiPoint, MultiPolygon, Point, Polygon, Rect, Triangle,
    };
    use rstest::rstest;

    use super::sort_order;
    use crate::decoder::{GeometryType, GeometryValues, RawGeometry};
    use crate::encoder::model::CurveParams;
    use crate::encoder::source::Order;
    use crate::encoder::{
        Codecs, Encoder, EncoderConfig, ExplicitEncoder, IntEncoder, Network, Profile,
        SortStrategy, stage_tile,
    };
    use crate::test_helpers::{assert_empty, dec, into_layer01, parser};
    use crate::tile::{TileFeature, TileLayer};
    use crate::{Layer, LazyParsed};

    fn pt(x: i32, y: i32) -> Geometry<i32> {
        GeoGeom::Point(Point::new(x, y))
    }

    fn ls(coords: &[(i32, i32)]) -> Geometry<i32> {
        GeoGeom::LineString(LineString::new(
            coords.iter().map(|&(x, y)| Coord { x, y }).collect(),
        ))
    }

    fn poly_square(x0: i32, y0: i32, side: i32) -> Geometry<i32> {
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
        let mut enc = Encoder::new(EncoderConfig::from_profile(Profile::Native(
            Network::Cellular4G,
        )));
        let mut codecs = Codecs::default();
        decoded
            .clone()
            .write_to(&mut enc, &mut codecs)
            .expect("encode failed");
        let buf = enc.data().to_vec();

        let parsed = assert_empty(RawGeometry::from_bytes(&buf, &mut parser()));
        let mut d = dec();
        let result = LazyParsed::Raw(parsed)
            .into_parsed(&mut d)
            .expect("decode failed");
        assert!(
            d.consumed() > 0,
            "decoder should consume bytes after decode"
        );
        result
    }

    /// Build the canonical (dense, wire-decoded) form of an ordered geometry sequence.
    fn canonical(geoms: &[Geometry<i32>]) -> GeometryValues {
        let mut decoded = GeometryValues::default();
        for g in geoms {
            decoded.push_geom(g);
        }
        roundtrip_geom(&decoded)
    }

    /// Build a `TileLayer` from `geoms` and `ids`, apply `reorder_features`,
    /// and return it.
    fn layer_after_sort(geoms: &[Geometry<i32>], ids: &[u64], strategy: SortStrategy) -> TileLayer {
        let features: Vec<TileFeature> = geoms
            .iter()
            .zip(ids.iter())
            .map(|(g, &id)| TileFeature {
                id: Some(id),
                geometry: g.clone(),
                properties: vec![],
                #[cfg(feature = "unstable-v2")]
                m_values: vec![],
                #[cfg(feature = "unstable-v2")]
                nested: vec![],
                #[cfg(feature = "unstable-v2")]
                z: Vec::new(),
                #[cfg(feature = "unstable-v2")]
                triangles: None,
            })
            .collect();

        let mut layer = TileLayer::from_parts("test", 4096, vec![], features).unwrap();

        let params = layer.curve_params();
        layer.sort(strategy, params);
        layer
    }

    /// Sort, then encode+decode the result and compare to `canonical(expected)`.
    fn assert_sort_roundtrip(
        geoms: &[Geometry<i32>],
        ids: &[u64],
        strategy: SortStrategy,
        expected: &[Geometry<i32>],
    ) {
        let layer = layer_after_sort(geoms, ids, strategy);

        let mut sorted_decoded = GeometryValues::default();
        for f in layer.features() {
            sorted_decoded.push_geom(f.geometry());
        }

        let after_roundtrip = roundtrip_geom(&sorted_decoded);
        let expected_canonical = canonical(expected);

        assert_eq!(
            after_roundtrip, expected_canonical,
            "\nsorted geometry did not match expected after encode->decode round-trip\
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

    #[test]
    fn chain_sort_starts_nearest_the_origin_and_follows_the_previous_end() {
        let layer = layer_after_sort(
            &[
                pt(100, 100),
                ls(&[(10, 0), (10, 50)]),
                pt(12, 52),
                pt(1, 1),
                pt(90, 95),
            ],
            &[1, 2, 3, 4, 5],
            SortStrategy::Chain,
        );

        let ids: Vec<Option<u64>> = layer.features().iter().map(TileFeature::id).collect();
        assert_eq!(ids, vec![Some(4), Some(2), Some(3), Some(5), Some(1)]);
    }

    #[test]
    fn chain_sort_takes_features_sharing_a_start_in_index_order() {
        let layer = layer_after_sort(
            &[pt(5, 5), ls(&[(5, 5), (9, 9)]), pt(5, 5), pt(9, 9)],
            &[1, 2, 3, 4],
            SortStrategy::Chain,
        );

        let ids: Vec<Option<u64>> = layer.features().iter().map(TileFeature::id).collect();
        assert_eq!(ids, vec![Some(1), Some(2), Some(4), Some(3)]);
    }

    #[test]
    fn chain_sort_under_vertex_sort_follows_the_multi_point_end_it_stores() {
        let layer = layer_after_sort(
            &[
                GeoGeom::MultiPoint(MultiPoint(vec![
                    Point::new(0, 0),
                    Point::new(100, 0),
                    Point::new(1, 0),
                ])),
                pt(2, 0),
                pt(101, 0),
            ],
            &[1, 2, 3],
            SortStrategy::Unsorted,
        );
        let params = layer.curve_params();

        let Order::Permuted(order) = sort_order(&layer, SortStrategy::Chain, params, true) else {
            panic!("chain sort permutes the features");
        };
        assert_eq!(order, vec![0, 2, 1]);
    }

    #[test]
    fn chain_sort_leaves_a_wound_polygon_at_its_stored_last_vertex() {
        let layer = layer_after_sort(
            &[
                GeoGeom::Polygon(Polygon::new(
                    LineString::from(vec![(0, 0), (0, 10), (10, 10), (10, 0), (0, 0)]),
                    vec![],
                )),
                pt(3, 0),
                pt(1, 10),
            ],
            &[1, 2, 3],
            SortStrategy::Chain,
        );

        let ids: Vec<Option<u64>> = layer.features().iter().map(TileFeature::id).collect();
        assert_eq!(ids, vec![Some(1), Some(3), Some(2)]);
    }

    #[test]
    fn chain_sort_leaves_a_closed_polygon_at_its_last_stored_vertex() {
        let layer = layer_after_sort(
            &[poly_square(0, 0, 10), pt(3, 0), pt(0, 12)],
            &[1, 2, 3],
            SortStrategy::Chain,
        );

        let ids: Vec<Option<u64>> = layer.features().iter().map(TileFeature::id).collect();
        assert_eq!(ids, vec![Some(1), Some(3), Some(2)]);
    }

    #[test]
    fn chain_sort_matches_a_brute_force_walk_over_scattered_lines() {
        let lines: Vec<[(i32, i32); 2]> = (0..3000_u32)
            .map(|i| {
                let x = i32::try_from(i.wrapping_mul(2_654_435_761) % 64).unwrap();
                let y = i32::try_from(i.wrapping_mul(40_503) % 32).unwrap();
                let dx = i32::try_from(i % 7).unwrap() - 3;
                [(x * 8, y * 8), (x * 8 + dx * 9, y * 8 + 5)]
            })
            .collect();
        let geoms: Vec<_> = lines.iter().map(|line| ls(line)).collect();
        let ids: Vec<u64> = (0..geoms.len() as u64).collect();

        let layer = layer_after_sort(&geoms, &ids, SortStrategy::Chain);

        let mut left: Vec<usize> = (0..lines.len()).collect();
        let mut at = (0_i32, 0_i32);
        let mut expected = Vec::new();
        while let Some(pos) = (0..left.len()).min_by_key(|&pos| {
            let (x, y) = lines[left[pos]][0];
            ((x - at.0).abs() + (y - at.1).abs(), left[pos])
        }) {
            let line = left.remove(pos);
            expected.push(Some(line as u64));
            at = lines[line][1];
        }
        let actual: Vec<Option<u64>> = layer.features().iter().map(TileFeature::id).collect();
        assert_eq!(actual, expected);
    }

    #[test]
    fn chain_sort_breaks_ties_by_stored_index() {
        let layer = layer_after_sort(
            &[pt(5, 0), pt(0, 5), pt(0, 5)],
            &[1, 2, 3],
            SortStrategy::Chain,
        );

        let ids: Vec<Option<u64>> = layer.features().iter().map(TileFeature::id).collect();
        assert_eq!(ids, vec![Some(1), Some(2), Some(3)]);
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

        let ids: Vec<Option<u64>> = layer.features().iter().map(TileFeature::id).collect();
        assert_eq!(ids, vec![Some(1u64), Some(2), Some(3)]);

        // Verify geometry types match expected order
        let geom_types: Vec<&str> = layer
            .features()
            .iter()
            .map(|f| GeometryType::try_from(f.geometry()).unwrap().into())
            .collect();
        assert_eq!(geom_types, vec!["LineString", "Point", "Point"]);
    }

    /// Build row-oriented tile layer from geometries and IDs (one feature per geometry).
    fn build_tile_layer(geoms: &[Geometry<i32>], ids: &[Option<u64>]) -> TileLayer {
        assert_eq!(geoms.len(), ids.len());
        TileLayer::from_parts(
            "test",
            4096,
            vec![],
            geoms
                .iter()
                .zip(ids.iter())
                .map(|(g, &id)| TileFeature {
                    id,
                    geometry: g.clone(),
                    properties: vec![],
                    #[cfg(feature = "unstable-v2")]
                    m_values: vec![],
                    #[cfg(feature = "unstable-v2")]
                    nested: vec![],
                    #[cfg(feature = "unstable-v2")]
                    z: Vec::new(),
                    #[cfg(feature = "unstable-v2")]
                    triangles: None,
                })
                .collect(),
        )
        .unwrap()
    }

    /// Encode the layer with a given sort strategy, decode it back, and return the `TileLayer`.
    /// This tests the full encode->decode roundtrip, verifying that sorting was applied.
    fn sort_encode_decode(tile: &TileLayer, sort: SortStrategy) -> TileLayer {
        let enc_cfg = EncoderConfig::from_profile(Profile::Native(Network::Cellular4G));
        let enc = Encoder::with_explicit(enc_cfg, ExplicitEncoder::for_id(IntEncoder::varint()));
        let mut codecs = Codecs::default();
        let enc = stage_tile(tile, sort, false, false)
            .encode_into(enc, &mut codecs)
            .expect("encode failed");

        // Serialize to bytes and reparse to get a `Layer01`.
        let buf = enc.into_layer_bytes().expect("into_layer_bytes failed");

        let mut p = parser();
        let layer_back = assert_empty(Layer::from_bytes(&buf, &mut p));
        assert!(p.reserved() > 0, "parser should reserve bytes after parse");

        let layer01 = into_layer01(layer_back);

        let mut d = dec();
        let tile = layer01.into_tile(&mut d).expect("decode after sort failed");
        assert!(
            d.consumed() > 0,
            "decoder should consume bytes after decode"
        );
        tile
    }

    fn encode_decode(tile: &TileLayer, cfg: EncoderConfig) -> TileLayer {
        let buf = tile.encode(cfg).expect("encode failed");
        let layer = assert_empty(Layer::from_bytes(&buf, &mut parser()));
        into_layer01(layer)
            .into_tile(&mut dec())
            .expect("decode failed")
    }

    fn scattered_points(count: u32) -> Vec<Geometry<i32>> {
        (0..count)
            .map(|i| {
                let x = i32::try_from(i.wrapping_mul(2_654_435_761) % 4096).unwrap();
                let y = i32::try_from(i.wrapping_mul(40_503) % 4096).unwrap();
                pt(x, y)
            })
            .collect()
    }

    #[test]
    fn chain_sort_encodes_scattered_points_in_the_chain_order() {
        let geoms = scattered_points(600);
        let ids = vec![None; geoms.len()];
        let mut expected = build_tile_layer(&geoms, &ids);
        let params = expected.curve_params();
        expected.sort(SortStrategy::Chain, params);

        let decoded = encode_decode(
            &build_tile_layer(&geoms, &ids),
            EncoderConfig::from_profile(Profile::Native(Network::Cellular4G)).with_chain_sort(true),
        );

        assert_eq!(
            vertices_from_source(&decoded),
            vertices_from_source(&expected)
        );
    }

    #[test]
    fn default_config_keeps_the_source_order() {
        let geoms = scattered_points(600);
        let ids: Vec<Option<u64>> = (0..600).rev().map(Some).collect();
        let decoded = encode_decode(
            &build_tile_layer(&geoms, &ids),
            EncoderConfig::from_profile(Profile::Native(Network::Cellular4G)),
        );

        let decoded_ids: Vec<Option<u64>> =
            decoded.features().iter().map(TileFeature::id).collect();
        assert_eq!(decoded_ids, ids);
    }

    #[test]
    fn hilbert_sort_reaches_a_layer_spread_over_the_whole_extent() {
        let geoms = scattered_points(600);
        let ids = vec![None; geoms.len()];
        let mut expected = build_tile_layer(&geoms, &ids);
        let params = expected.curve_params();
        expected.sort(SortStrategy::SpatialHilbert, params);

        let decoded = encode_decode(
            &build_tile_layer(&geoms, &ids),
            EncoderConfig::from_profile(Profile::Native(Network::Cellular4G))
                .with_spatial_hilbert_sort(true),
        );

        assert_eq!(
            vertices_from_source(&decoded),
            vertices_from_source(&expected)
        );
    }

    /// Rebuild a flat vertex buffer from the feature geometries in source order.
    fn vertices_from_source(source: &TileLayer) -> Vec<i32> {
        let mut geom = GeometryValues::default();
        for f in source.features() {
            geom.push_geom(f.geometry());
        }
        geom.vertices().unwrap_or_default().to_vec()
    }

    #[test]
    fn test_shared_morton_shift() {
        // P1 at (0, -10), P2 at (-10, 0).
        // With shared shift = 10:
        // P1 shifted: (10, 0) -> interleave(10, 0) = 68
        // P2 shifted: (0, 10) -> interleave(0, 10) = 136
        // P1 (key 68) < P2 (key 136), so expected order: [P1(0,-10), P2(-10,0)].

        let tile = build_tile_layer(&[pt(0, -10), pt(-10, 0)], &[Some(1), Some(2)]);
        let source = sort_encode_decode(&tile, SortStrategy::SpatialMorton);

        let verts = vertices_from_source(&source);
        assert_eq!(verts, vec![0, -10, -10, 0]);
    }

    #[test]
    fn test_id_sort_nulls_first() {
        let tile = build_tile_layer(&[pt(2, 2), pt(1, 1), pt(0, 0)], &[Some(10), None, Some(5)]);
        let source = sort_encode_decode(&tile, SortStrategy::Id);

        let ids: Vec<Option<u64>> = source.features().iter().map(TileFeature::id).collect();
        // Expected order: [None, Some(5), Some(10)]
        assert_eq!(ids, vec![None, Some(5), Some(10)]);

        let verts = vertices_from_source(&source);
        // Corresponding verts: [pt(1,1), pt(0,0), pt(2,2)] -> [1,1, 0,0, 2,2]
        assert_eq!(verts, vec![1, 1, 0, 0, 2, 2]);
    }

    #[test]
    fn test_mixed_geometry_morton_sort() {
        // [Point(2,0), LineString(0,0 -> 0,5), Point(1,0)]
        // Morton keys (assuming shift 0):
        // P1(2,0) -> 4
        // LS(0,0) -> 0
        // P2(1,0) -> 1
        // Expected order: [LS, P2, P1]

        let tile = build_tile_layer(
            &[pt(2, 0), ls(&[(0, 0), (0, 5)]), pt(1, 0)],
            &[Some(1), Some(2), Some(3)],
        );
        let source = sort_encode_decode(&tile, SortStrategy::SpatialMorton);

        let types: Vec<_> = source
            .features()
            .iter()
            .map(|f| GeometryType::try_from(f.geometry()).unwrap())
            .collect();

        assert_eq!(
            types,
            vec![
                GeometryType::LineString,
                GeometryType::Point,
                GeometryType::Point
            ]
        );

        let verts = vertices_from_source(&source);
        // Expected vertices: LS(0,0,0,5), P2(1,0), P1(2,0)
        assert_eq!(verts, vec![0, 0, 0, 5, 1, 0, 2, 0]);
    }

    #[test]
    fn id_sort_separates_the_two_largest_ids() {
        let layer = layer_after_sort(
            &[pt(0, 0), pt(1, 1)],
            &[u64::MAX, u64::MAX - 1],
            SortStrategy::Id,
        );

        let ids: Vec<Option<u64>> = layer.features().iter().map(TileFeature::id).collect();
        assert_eq!(ids, vec![Some(u64::MAX - 1), Some(u64::MAX)]);
    }

    #[test]
    fn id_sort_puts_the_missing_id_before_zero() {
        let layer = build_tile_layer(&[pt(0, 0), pt(1, 1)], &[Some(0), None]);
        let mut layer = layer;
        layer.sort(SortStrategy::Id, CurveParams { shift: 0, bits: 1 });

        let ids: Vec<Option<u64>> = layer.features().iter().map(TileFeature::id).collect();
        assert_eq!(ids, vec![None, Some(0)]);
    }

    #[test]
    fn spatial_sort_reads_the_first_vertex_of_every_geometry_kind() {
        let at = |k: i32| Coord { x: k, y: k };
        let geoms = [
            GeoGeom::Rect(Rect::new(at(0), at(20))),
            GeoGeom::Triangle(Triangle::new(at(1), at(11), at(21))),
            GeoGeom::MultiPolygon(MultiPolygon(vec![Polygon::new(
                LineString(vec![at(2), at(12), at(22), at(2)]),
                vec![],
            )])),
            GeoGeom::MultiLineString(MultiLineString(vec![LineString(vec![at(3), at(13)])])),
            GeoGeom::MultiPoint(MultiPoint(vec![Point::from(at(4)), Point::from(at(14))])),
            GeoGeom::Polygon(Polygon::new(
                LineString(vec![at(5), at(15), at(25), at(5)]),
                vec![],
            )),
            GeoGeom::LineString(LineString(vec![at(6), at(16)])),
            GeoGeom::Line(Line::new(at(7), at(17))),
            GeoGeom::Point(Point::from(at(8))),
            GeoGeom::GeometryCollection(GeometryCollection(vec![GeoGeom::Point(Point::from(at(
                9,
            )))])),
            GeoGeom::LineString(LineString(vec![])),
        ];
        let ids: Vec<Option<u64>> = (0..u64::try_from(geoms.len()).unwrap()).map(Some).collect();

        let mut layer = build_tile_layer(&geoms, &ids);
        let params = layer.curve_params();
        // Morton keys interleave the bits of x and y
        // This means the diagonal they rise with the coordinate and the sorted order is the vertex order.
        layer.sort(SortStrategy::SpatialMorton, params);

        let sorted: Vec<Option<u64>> = layer.features().iter().map(TileFeature::id).collect();
        assert_eq!(sorted, ids);
    }

    #[test]
    fn curve_params_of_a_vertexless_layer_is_the_degenerate_grid() {
        let layer = build_tile_layer(&[GeoGeom::LineString(LineString(vec![]))], &[None]);
        assert_eq!(layer.curve_params(), CurveParams { shift: 0, bits: 1 });
    }

    #[rstest]
    #[case::morton(SortStrategy::SpatialMorton, &[0, 1, 2, 3])]
    #[case::hilbert(SortStrategy::SpatialHilbert, &[0, 2, 3, 1])]
    fn spatial_sort_of_the_unit_square_follows_the_chosen_curve(
        #[case] strategy: SortStrategy,
        #[case] expected: &[u64],
    ) {
        let geoms = [pt(0, 0), pt(1, 0), pt(0, 1), pt(1, 1)];
        let ids: Vec<Option<u64>> = (0..4).map(Some).collect();

        let mut layer = build_tile_layer(&geoms, &ids);
        let params = layer.curve_params();
        assert_eq!(params, CurveParams { shift: 0, bits: 1 });
        layer.sort(strategy, params);

        let sorted: Vec<Option<u64>> = layer.features().iter().map(TileFeature::id).collect();
        assert_eq!(
            sorted,
            expected.iter().copied().map(Some).collect::<Vec<_>>()
        );
    }

    #[test]
    fn unsorted_leaves_the_feature_order_alone() {
        let geoms = [pt(9, 9), pt(0, 0), pt(5, 5)];
        let ids = [Some(3u64), Some(1), Some(2)];

        let mut layer = build_tile_layer(&geoms, &ids);
        let params = layer.curve_params();
        layer.sort(SortStrategy::Unsorted, params);

        let sorted: Vec<Option<u64>> = layer.features().iter().map(TileFeature::id).collect();
        assert_eq!(sorted, ids.to_vec());
    }

    #[test]
    fn every_vertexless_geometry_kind_sorts_after_the_one_feature_with_a_vertex() {
        let empty_ring = || Polygon::new(LineString(vec![]), vec![]);
        let geoms = [
            GeoGeom::LineString(LineString(vec![])),
            GeoGeom::Polygon(empty_ring()),
            GeoGeom::MultiPoint(MultiPoint(vec![])),
            GeoGeom::MultiLineString(MultiLineString(vec![])),
            GeoGeom::MultiLineString(MultiLineString(vec![LineString(vec![])])),
            GeoGeom::MultiPolygon(MultiPolygon(vec![])),
            GeoGeom::MultiPolygon(MultiPolygon(vec![empty_ring()])),
            GeoGeom::GeometryCollection(GeometryCollection(vec![])),
            GeoGeom::GeometryCollection(GeometryCollection(vec![GeoGeom::LineString(LineString(
                vec![],
            ))])),
            pt(1, 1),
        ];
        let ids: Vec<Option<u64>> = (0..u64::try_from(geoms.len()).unwrap()).map(Some).collect();

        let mut layer = build_tile_layer(&geoms, &ids);
        let params = layer.curve_params();
        layer.sort(SortStrategy::SpatialMorton, params);

        let sorted: Vec<Option<u64>> = layer.features().iter().map(TileFeature::id).collect();
        assert_eq!(
            sorted,
            vec![
                Some(9),
                Some(0),
                Some(1),
                Some(2),
                Some(3),
                Some(4),
                Some(5),
                Some(6),
                Some(7),
                Some(8)
            ]
        );
    }
}
