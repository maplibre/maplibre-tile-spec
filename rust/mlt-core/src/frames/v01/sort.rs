use std::borrow::Cow;

use geo_types::{Coord, LineString, MultiLineString, MultiPoint, MultiPolygon, Point, Polygon};

use crate::geojson::{Coord32, Geom32};
use crate::utils::{hilbert_curve_params, hilbert_sort_key, morton_sort_key};
use crate::v01::{
    DecodedGeometry, DecodedId, DecodedProperty, DecodedStrings, GeometryType, OwnedGeometry,
    OwnedId, OwnedLayer01, OwnedProperty,
};
use crate::{DecodeInto as _, MltError};

/// The space-filling curve used when sorting features spatially.
///
/// Both curves sort features so that spatially adjacent features end up
/// near each other in the stream, improving property RLE runs and client-side
/// cache locality.  Hilbert generally achieves better spatial locality than
/// Morton (Z-order) but is more expensive to compute.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum SpaceFillingCurve {
    /// Z-order (Morton) curve.  Fast to compute; good locality.
    #[default]
    Morton,
    /// Hilbert curve.  Slower to compute; superior locality.
    Hilbert,
}

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

/// Reorder all columns of `layer` according to `strategy`.
///
/// All columns are decoded in-place before the permutation is applied.
/// If the layer has zero or one features, or if `strategy` is [`None`],
/// this is a no-op.
///
/// Tessellated geometry (`index_buffer` / `triangles` fields) is not yet
/// supported: layers containing either field are left unchanged.
pub(crate) fn reorder_features(
    layer: &mut OwnedLayer01,
    strategy: Option<SortStrategy>,
    allow_id_regeneration: bool,
) -> Result<(), MltError> {
    if strategy.is_none() && !allow_id_regeneration {
        return Ok(());
    }

    // Everything must be in decoded form before we can permute it.
    ensure_decoded(layer)?;

    let n = geometry_feature_count(&layer.geometry)?;
    if n <= 1 {
        // Still might want to regenerate IDs for single feature (to 0).
        if allow_id_regeneration {
            regenerate_ids(layer, n);
        }
        return Ok(());
    }

    let geoms = if let OwnedGeometry::Decoded(ref g) = layer.geometry {
        extract_geometries(g)?
    } else {
        vec![]
    };

    if let Some(strategy) = strategy {
        // Skip tessellated layers — index_buffer / triangles are not permutable
        // without retessellating.
        if let OwnedGeometry::Decoded(ref g) = layer.geometry
            && (g.index_buffer.is_some() || g.triangles.is_some())
        {
            if allow_id_regeneration {
                regenerate_ids(layer, n);
            }
            return Ok(());
        }

        let keys = compute_sort_keys(layer, strategy, n, &geoms)?;
        let perm = build_permutation(&keys);
        apply_permutation(layer, &perm, &geoms)?;
    }

    if allow_id_regeneration {
        regenerate_ids(layer, n);
    }

    Ok(())
}

fn extract_geometries(decoded: &DecodedGeometry) -> Result<Vec<Geom32>, MltError> {
    let n = decoded.vector_types.len();
    let mut result = Vec::with_capacity(n);

    let verts = decoded.vertices.as_deref().unwrap_or(&[]);
    let geoms = decoded.geometry_offsets.as_deref();
    let parts = decoded.part_offsets.as_deref();
    let rings = decoded.ring_offsets.as_deref();

    let mut geom_ptr = 0;
    let mut part_ptr = 0;
    let mut vtx_ptr = 0;

    let get_off = |s: &[u32], idx: usize, field: &'static str| -> Result<usize, MltError> {
        s.get(idx)
            .map(|&v| v as usize)
            .ok_or(MltError::GeometryOutOfBounds {
                index: 0,
                field,
                idx,
                len: s.len(),
            })
    };

    let get_vert = |idx: usize| -> Result<Coord32, MltError> {
        verts
            .get(idx * 2..idx * 2 + 2)
            .map(|s| Coord { x: s[0], y: s[1] })
            .ok_or(MltError::GeometryVertexOutOfBounds {
                index: 0,
                vertex: idx,
                count: verts.len() / 2,
            })
    };

    for i in 0..n {
        let geom_type = decoded.vector_types[i];
        let geom = match geom_type {
            GeometryType::Point => {
                if geoms.is_some() {
                    geom_ptr += 1;
                }
                let res = Geom32::Point(Point(get_vert(vtx_ptr)?));
                vtx_ptr += 1;
                res
            }
            GeometryType::LineString => {
                let part_count = if let Some(g) = geoms {
                    let start = get_off(g, geom_ptr, "geometry_offsets")?;
                    let end = get_off(g, geom_ptr + 1, "geometry_offsets")?;
                    geom_ptr += 1;
                    end - start
                } else {
                    1
                };

                let mut all_ls_verts = Vec::new();
                for _ in 0..part_count {
                    let vtx_count = if let Some(r) = rings {
                        let p = parts.ok_or(MltError::NoPartOffsets(i, geom_type))?;
                        let r_idx = get_off(p, part_ptr, "part_offsets")?;
                        part_ptr += 1;
                        let v_start = get_off(r, r_idx, "ring_offsets")?;
                        let v_end = get_off(r, r_idx + 1, "ring_offsets")?;
                        v_end - v_start
                    } else {
                        let p = parts.ok_or(MltError::NoPartOffsets(i, geom_type))?;
                        let v_start = get_off(p, part_ptr, "part_offsets")?;
                        let v_end = get_off(p, part_ptr + 1, "part_offsets")?;
                        part_ptr += 1;
                        v_end - v_start
                    };
                    for _ in 0..vtx_count {
                        all_ls_verts.push(get_vert(vtx_ptr)?);
                        vtx_ptr += 1;
                    }
                }
                Geom32::LineString(LineString(all_ls_verts))
            }
            GeometryType::Polygon => {
                let part_count = if let Some(g) = geoms {
                    let start = get_off(g, geom_ptr, "geometry_offsets")?;
                    let end = get_off(g, geom_ptr + 1, "geometry_offsets")?;
                    geom_ptr += 1;
                    end - start
                } else {
                    1
                };

                let mut polys = Vec::with_capacity(part_count);
                for _ in 0..part_count {
                    let p = parts.ok_or(MltError::NoPartOffsets(i, geom_type))?;
                    let r = rings.ok_or(MltError::NoRingOffsets(i, geom_type))?;
                    let r_start = get_off(p, part_ptr, "part_offsets")?;
                    let r_end = get_off(p, part_ptr + 1, "part_offsets")?;
                    part_ptr += 1;

                    let mut polygon_rings = Vec::with_capacity(r_end - r_start);
                    for r_idx in r_start..r_end {
                        let v_start = get_off(r, r_idx, "ring_offsets")?;
                        let v_end = get_off(r, r_idx + 1, "ring_offsets")?;
                        let mut coords = Vec::with_capacity(v_end - v_start + 1);
                        for _ in v_start..v_end {
                            coords.push(get_vert(vtx_ptr)?);
                            vtx_ptr += 1;
                        }
                        if let Some(&first) = coords.first() {
                            coords.push(first);
                        }
                        polygon_rings.push(LineString(coords));
                    }
                    let mut iter = polygon_rings.into_iter();
                    polys.push(Polygon::new(
                        iter.next().unwrap_or_else(|| LineString(vec![])),
                        iter.collect(),
                    ));
                }
                Geom32::Polygon(
                    polys
                        .into_iter()
                        .next()
                        .unwrap_or_else(|| Polygon::new(LineString(vec![]), vec![])),
                )
            }
            GeometryType::MultiPoint => {
                let g = geoms.ok_or(MltError::NoGeometryOffsets(i, geom_type))?;
                let v_start = get_off(g, geom_ptr, "geometry_offsets")?;
                let v_end = get_off(g, geom_ptr + 1, "geometry_offsets")?;
                geom_ptr += 1;
                let mut points = Vec::with_capacity(v_end - v_start);
                for _ in v_start..v_end {
                    points.push(Point(get_vert(vtx_ptr)?));
                    vtx_ptr += 1;
                }
                Geom32::MultiPoint(MultiPoint(points))
            }
            GeometryType::MultiLineString => {
                let g = geoms.ok_or(MltError::NoGeometryOffsets(i, geom_type))?;
                let p_start = get_off(g, geom_ptr, "geometry_offsets")?;
                let p_end = get_off(g, geom_ptr + 1, "geometry_offsets")?;
                geom_ptr += 1;
                let mut lines = Vec::with_capacity(p_end - p_start);
                for p_idx in p_start..p_end {
                    let vtx_count = if let Some(r) = rings {
                        let p = parts.ok_or(MltError::NoPartOffsets(i, geom_type))?;
                        let r_idx = get_off(p, p_idx, "part_offsets")?;
                        let v_start = get_off(r, r_idx, "ring_offsets")?;
                        let v_end = get_off(r, r_idx + 1, "ring_offsets")?;
                        v_end - v_start
                    } else {
                        let p = parts.ok_or(MltError::NoPartOffsets(i, geom_type))?;
                        let v_start = get_off(p, p_idx, "part_offsets")?;
                        let v_end = get_off(p, p_idx + 1, "part_offsets")?;
                        v_end - v_start
                    };
                    let mut coords = Vec::with_capacity(vtx_count);
                    for _ in 0..vtx_count {
                        coords.push(get_vert(vtx_ptr)?);
                        vtx_ptr += 1;
                    }
                    lines.push(LineString(coords));
                }
                part_ptr = p_end;
                Geom32::MultiLineString(MultiLineString(lines))
            }
            GeometryType::MultiPolygon => {
                let g = geoms.ok_or(MltError::NoGeometryOffsets(i, geom_type))?;
                let p_start = get_off(g, geom_ptr, "geometry_offsets")?;
                let p_end = get_off(g, geom_ptr + 1, "geometry_offsets")?;
                geom_ptr += 1;
                let mut polys = Vec::with_capacity(p_end - p_start);
                for p_idx in p_start..p_end {
                    let p = parts.ok_or(MltError::NoPartOffsets(i, geom_type))?;
                    let r = rings.ok_or(MltError::NoRingOffsets(i, geom_type))?;
                    let r_start = get_off(p, p_idx, "part_offsets")?;
                    let r_end = get_off(p, p_idx + 1, "part_offsets")?;
                    let mut polygon_rings = Vec::with_capacity(r_end - r_start);
                    for r_idx in r_start..r_end {
                        let v_start = get_off(r, r_idx, "ring_offsets")?;
                        let v_end = get_off(r, r_idx + 1, "ring_offsets")?;
                        let mut coords = Vec::with_capacity(v_end - v_start + 1);
                        for _ in v_start..v_end {
                            coords.push(get_vert(vtx_ptr)?);
                            vtx_ptr += 1;
                        }
                        if let Some(&first) = coords.first() {
                            coords.push(first);
                        }
                        polygon_rings.push(LineString(coords));
                    }
                    let mut iter = polygon_rings.into_iter();
                    polys.push(Polygon::new(
                        iter.next().unwrap_or_else(|| LineString(vec![])),
                        iter.collect(),
                    ));
                }
                part_ptr = p_end;
                Geom32::MultiPolygon(MultiPolygon(polys))
            }
        };
        result.push(geom);
    }

    Ok(result)
}

fn regenerate_ids(layer: &mut OwnedLayer01, n: usize) {
    let new_ids = (0..n as u64).map(Some).collect();
    layer.id = OwnedId::Decoded(Some(DecodedId(new_ids)));
}

/// Compute one sort key per feature.  The key type is `u64` so that both
/// curve codes (u32) and raw IDs (u64) can share the same return type.
fn compute_sort_keys(
    layer: &OwnedLayer01,
    strategy: SortStrategy,
    n: usize,
    geoms: &[Geom32],
) -> Result<Vec<u64>, MltError> {
    match strategy {
        SortStrategy::SpatialMorton | SortStrategy::SpatialHilbert => {
            let curve = match strategy {
                SortStrategy::SpatialMorton => SpaceFillingCurve::Morton,
                SortStrategy::SpatialHilbert => SpaceFillingCurve::Hilbert,
                _ => unreachable!("only morton and hilbert sort strategies are supported"),
            };
            let geom_data = match &layer.geometry {
                OwnedGeometry::Decoded(g) => g,
                OwnedGeometry::Encoded(_) => return Err(MltError::NotDecoded("geometry")),
            };
            Ok(spatial_sort_keys(geom_data, n, curve, geoms)
                .into_iter()
                .map(u64::from)
                .collect())
        }

        SortStrategy::Id => {
            let ids = match &layer.id {
                OwnedId::Decoded(Some(d)) => d,
                OwnedId::Decoded(None) => {
                    // No ID column — produce a stable identity permutation.
                    return Ok((0..n as u64).collect());
                }
                OwnedId::Encoded(_) => return Err(MltError::NotDecoded("id")),
            };
            // Null IDs sort last (u64::MAX).
            Ok(ids.0.iter().map(|id| id.unwrap_or(u64::MAX)).collect())
        }
    }
}

/// Return one sort key per feature using the first vertex of each feature as
/// the representative point.  Features without a vertex receive `u32::MAX`
/// so they sort to the end.
fn spatial_sort_keys(
    decoded: &DecodedGeometry,
    n: usize,
    curve: SpaceFillingCurve,
    geoms: &[Geom32],
) -> Vec<u32> {
    let verts = decoded.vertices.as_deref().unwrap_or(&[]);
    let (shift, num_bits) = hilbert_curve_params(verts);

    match curve {
        SpaceFillingCurve::Morton => (0..n)
            .map(|i| {
                first_vertex(&geoms[i])
                    .map_or(u32::MAX, |(x, y)| morton_sort_key(x, y, shift, num_bits))
            })
            .collect(),

        SpaceFillingCurve::Hilbert => (0..n)
            .map(|i| {
                first_vertex(&geoms[i])
                    .map_or(u32::MAX, |(x, y)| hilbert_sort_key(x, y, shift, num_bits))
            })
            .collect(),
    }
}

/// Extract the `(x, y)` coordinate of the first vertex for feature `i`.
fn first_vertex(geom: &Geom32) -> Option<(i32, i32)> {
    match geom {
        Geom32::Point(p) => Some((p.0.x, p.0.y)),
        Geom32::Line(l) => Some((l.start.x, l.start.y)),
        Geom32::LineString(ls) => ls.0.first().map(|c| (c.x, c.y)),
        Geom32::Polygon(p) => p.exterior().0.first().map(|c| (c.x, c.y)),
        Geom32::MultiPoint(mp) => mp.0.first().map(|p| (p.0.x, p.0.y)),
        Geom32::MultiLineString(mls) => mls.0.first().and_then(|ls| ls.0.first().map(|c| (c.x, c.y))),
        Geom32::MultiPolygon(mp) => mp
            .0
            .first()
            .and_then(|p| p.exterior().0.first().map(|c| (c.x, c.y))),
        Geom32::Triangle(t) => Some((t.0.x, t.0.y)),
        Geom32::Rect(r) => Some((r.min().x, r.min().y)),
        Geom32::GeometryCollection(gc) => gc.0.first().and_then(first_vertex),
    }
}

/// Build a permutation such that `perm[new_position] = old_position`.
///
/// Uses a stable sort so that features with equal keys retain their original
/// relative order (important for z-fighting stability).
fn build_permutation<K: Ord>(keys: &[K]) -> Vec<usize> {
    let mut indices: Vec<usize> = (0..keys.len()).collect();
    indices.sort_by(|&a, &b| keys[a].cmp(&keys[b]));
    indices
}

#[allow(clippy::unnecessary_wraps)]
fn apply_permutation(
    layer: &mut OwnedLayer01,
    perm: &[usize],
    geoms: &[Geom32],
) -> Result<(), MltError> {
    if let OwnedGeometry::Decoded(geom) = &mut layer.geometry {
        permute_geometry(geom, perm, geoms);
    }
    if let OwnedId::Decoded(Some(id)) = &mut layer.id {
        permute_id(id, perm);
    }
    for prop in &mut layer.properties {
        if let OwnedProperty::Decoded(dp) = prop {
            permute_property(dp, perm);
        }
    }
    Ok(())
}

/// Decode all columns of `layer` in-place so that permutation can be applied
/// to the plain decoded values.
///
/// Each column is a no-op if it is already in the `Decoded` variant.
pub(crate) fn ensure_decoded(layer: &mut OwnedLayer01) -> Result<(), MltError> {
    if let OwnedGeometry::Encoded(e) = &layer.geometry {
        let dec = borrowme::borrow(e).decode_into()?;
        layer.geometry = OwnedGeometry::Decoded(dec);
    }

    if let OwnedId::Encoded(e) = &layer.id {
        let dec = e.as_ref().map(borrowme::borrow).decode_into()?;
        layer.id = OwnedId::Decoded(dec);
    }

    for prop in &mut layer.properties {
        if matches!(prop, OwnedProperty::Encoded(_)) {
            let decoded_ref = borrowme::borrow(prop as &OwnedProperty).decode()?;
            let decoded_owned = borrowme::ToOwned::to_owned(&decoded_ref);
            *prop = OwnedProperty::Decoded(decoded_owned);
        }
    }

    Ok(())
}

/// Apply `perm` to all arrays inside `decoded` so that the feature at new
/// position `k` is the feature that was at old position `perm[k]`.
///
/// Handles all six offset-array combinations that can appear after decoding:
///
/// | geometry_offsets | part_offsets | ring_offsets | Geometry kind         |
/// |------------------|--------------|--------------|-----------------------|
/// | None             | None         | None         | Points                |
/// | None             | Some         | None         | LineStrings / mixed   |
/// | None             | Some         | Some         | Polygons / mixed      |
/// | Some             | None         | None         | MultiPoints           |
/// | Some             | Some         | None         | MultiLines / mixed    |
/// | Some             | Some         | Some         | MultiPolygons / mixed |
fn permute_geometry(decoded: &mut DecodedGeometry, perm: &[usize], geoms: &[Geom32]) {
    let mut new_decoded = DecodedGeometry::default();
    for &i in perm {
        new_decoded.push_geom(&geoms[i]);
    }
    *decoded = new_decoded;
}

fn permute_id(id: &mut DecodedId, perm: &[usize]) {
    let old = id.0.clone();
    id.0 = perm.iter().map(|&i| old[i]).collect();
}

fn permute_property(prop: &mut DecodedProperty<'_>, perm: &[usize]) {
    match prop {
        DecodedProperty::Bool(s) => permute_vec(&mut s.values, perm),
        DecodedProperty::I8(s) => permute_vec(&mut s.values, perm),
        DecodedProperty::U8(s) => permute_vec(&mut s.values, perm),
        DecodedProperty::I32(s) => permute_vec(&mut s.values, perm),
        DecodedProperty::U32(s) => permute_vec(&mut s.values, perm),
        DecodedProperty::I64(s) => permute_vec(&mut s.values, perm),
        DecodedProperty::U64(s) => permute_vec(&mut s.values, perm),
        DecodedProperty::F32(s) => permute_vec(&mut s.values, perm),
        DecodedProperty::F64(s) => permute_vec(&mut s.values, perm),
        DecodedProperty::Str(s) => permute_strings(s, perm),
        DecodedProperty::SharedDict(sd) => {
            for item in &mut sd.items {
                permute_vec(&mut item.ranges, perm);
            }
        }
    }
}

fn permute_vec<T: Copy>(values: &mut Vec<T>, perm: &[usize]) {
    let old = values.clone();
    *values = perm.iter().map(|&i| old[i]).collect();
}

/// Permute a `DecodedStrings` column in-place.
///
/// Materialises the per-feature strings, reorders them, then rebuilds the
/// `lengths` + `data` encoding.  The column name is preserved.
fn permute_strings(s: &mut DecodedStrings<'_>, perm: &[usize]) {
    let old: Vec<Option<String>> = s.materialize();
    let permuted: Vec<Option<String>> = perm.iter().map(|&i| old[i].clone()).collect();
    let name = s.name.clone();
    let rebuilt = DecodedStrings::from(permuted);
    s.lengths = rebuilt.lengths;
    s.data = Cow::Owned(rebuilt.data.into_owned());
    s.name = name;
}

/// Return `true` if a spatial sort is likely to reduce compressed size.
///
/// The heuristic: if the vertex bounding box spans more than
/// `SPATIAL_HELP_COVERAGE` of the layer's tile extent on **both** axes, the
/// features are too spread-out for locality clustering to help, so spatial
/// sorting is skipped.
///
/// If the geometry is not yet decoded, or has no vertices, the function
/// conservatively returns `true` (attempt the trial anyway).
pub(crate) fn spatial_sort_likely_to_help(layer: &OwnedLayer01) -> bool {
    /// Skip spatial sort when both axes span more than this fraction of extent.
    const SPATIAL_HELP_COVERAGE: f64 = 0.8;

    let vertices = match &layer.geometry {
        OwnedGeometry::Decoded(g) => match &g.vertices {
            Some(v) if v.len() >= 4 => v,
            _ => return true, // no vertices — be conservative
        },
        OwnedGeometry::Encoded(_) => return true, // not decoded — be conservative
    };

    let extent = f64::from(layer.extent);
    if extent <= 0.0 {
        return true;
    }

    let (min_x, max_x, min_y, max_y) = vertices.chunks_exact(2).fold(
        (i32::MAX, i32::MIN, i32::MAX, i32::MIN),
        |(min_x, max_x, min_y, max_y), chunk| {
            let x = chunk[0];
            let y = chunk[1];
            (min_x.min(x), max_x.max(x), min_y.min(y), max_y.max(y))
        },
    );

    let range_x = f64::from(max_x - min_x);
    let range_y = f64::from(max_y - min_y);

    let spread_x = range_x / extent;
    let spread_y = range_y / extent;

    // If both axes are highly spread, spatial sort is unlikely to cluster.
    !(spread_x > SPATIAL_HELP_COVERAGE && spread_y > SPATIAL_HELP_COVERAGE)
}

pub(crate) fn geometry_feature_count(geom: &OwnedGeometry) -> Result<usize, MltError> {
    match geom {
        OwnedGeometry::Decoded(g) => Ok(g.vector_types.len()),
        OwnedGeometry::Encoded(_) => Err(MltError::NotDecoded("geometry")),
    }
}

#[cfg(test)]
mod tests {
    use geo_types::{Coord, Geometry as GeoGeom, LineString, Point, Polygon};

    use super::*;
    use crate::geojson::Geom32;
    use crate::optimizer::ManualOptimisation as _;
    use crate::v01::{
        DecodedGeometry, DecodedId, EncodedGeometry, Geometry, GeometryEncoder, GeometryType,
        IntEncoder, OwnedGeometry, OwnedId, OwnedLayer01,
    };

    // ── helpers ──────────────────────────────────────────────────────────────

    /// Encode + serialize + parse + decode a `DecodedGeometry`.
    ///
    /// This is the canonical round-trip: it exercises the full wire path and
    /// converts sparse `push_*` offset arrays into the dense form that the
    /// decoder always produces.
    fn roundtrip_geom(decoded: &DecodedGeometry) -> DecodedGeometry {
        let mut geom = OwnedGeometry::Decoded(decoded.clone());
        geom.manual_optimisation(GeometryEncoder::all(IntEncoder::varint()))
            .expect("encode failed");

        let mut buf = Vec::new();
        geom.write_to(&mut buf).expect("serialize failed");

        let (remaining, parsed) = EncodedGeometry::parse(&buf).expect("parse failed");
        assert!(
            remaining.is_empty(),
            "unexpected trailing bytes after parse"
        );

        Geometry::Encoded(parsed).decode().expect("decode failed")
    }

    /// Build the canonical (dense, wire-decoded) form of an ordered geometry
    /// sequence.  This is the reference representation used in assertions.
    fn canonical(geoms: &[Geom32]) -> DecodedGeometry {
        let mut decoded = DecodedGeometry::default();
        for g in geoms {
            decoded.push_geom(g);
        }
        roundtrip_geom(&decoded)
    }

    /// Build a layer with `push_*` geometry (sparse offsets, no pre-canonicalization),
    /// apply `reorder_features`, and return the resulting `DecodedGeometry`.
    ///
    /// Does NOT call `roundtrip_geom` before sorting.  The geometry handed to
    /// `reorder_features` is exactly what `push_*` produces — the same state
    /// that application code would see before any encoding step.
    fn layer_after_sort(geoms: &[Geom32], ids: &[u64], strategy: SortStrategy) -> OwnedLayer01 {
        let mut decoded_geom = DecodedGeometry::default();
        for g in geoms {
            decoded_geom.push_geom(g);
        }

        let mut layer = OwnedLayer01 {
            name: "test".to_string(),
            extent: 4096,
            id: OwnedId::Decoded(Some(DecodedId(ids.iter().map(|&id| Some(id)).collect()))),
            geometry: OwnedGeometry::Decoded(decoded_geom),
            properties: vec![],
        };

        reorder_features(&mut layer, Some(strategy), false).expect("reorder_features failed");
        layer
    }

    /// Sort, then encode+decode the result and compare to `canonical(expected)`.
    ///
    /// This is the core round-trip assertion: the geometry that comes out of
    /// `reorder_features` must survive a full wire encode/decode cycle and
    /// equal the expected canonical representation.
    fn assert_sort_roundtrip(
        geoms: &[Geom32],
        ids: &[u64],
        strategy: SortStrategy,
        expected: &[Geom32],
    ) {
        let layer = layer_after_sort(geoms, ids, strategy);

        let sorted_geom = match layer.geometry {
            OwnedGeometry::Decoded(g) => g,
            _ => panic!("geometry was not decoded after reorder_features"),
        };

        let after_roundtrip = roundtrip_geom(&sorted_geom);
        let expected_canonical = canonical(expected);

        assert_eq!(
            after_roundtrip, expected_canonical,
            "\nsorted geometry did not match expected after encode→decode round-trip\
             \nvector_types after sort: {:?}\
             \nvector_types expected:   {:?}",
            sorted_geom.vector_types, expected_canonical.vector_types,
        );
    }

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
            Coord { x: x0, y: y0 }, // closed
        ]);
        GeoGeom::Polygon(Polygon::new(ring, vec![]))
    }

    // ── pure Points ──────────────────────────────────────────────────────────

    /// IDs [3, 2, 1] fully reverse three points.
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

    /// IDs [2, 1] swap two linestrings.
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

    /// IDs [3, 1, 2] → permutation [1, 2, 0] → [LineString, Point, Point].
    #[test]
    fn point_line_point_id_sort_to_line_point_point_roundtrip() {
        assert_sort_roundtrip(
            &[pt(0, 0), ls(&[(1, 0), (1, 5)]), pt(5, 5)],
            &[3, 1, 2],
            SortStrategy::Id,
            &[ls(&[(1, 0), (1, 5)]), pt(5, 5), pt(0, 0)],
        );
    }

    /// IDs [1, 3, 2] → permutation [0, 2, 1] → [Point, Point, LineString].
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

    /// IDs [2, 1, 3] → permutation [1, 0, 2] → [Polygon, Point, Point].
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

    /// Coordinates chosen so Morton keys are unambiguous:
    ///   Point(2,0)           → Morton 4
    ///   LineString first (0,0) → Morton 0
    ///   Point(1,0)           → Morton 1
    /// Expected order after sort: [LineString, Point(1,0), Point(2,0)].
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

    /// Verifies that IDs and vector_types are both reordered consistently.
    #[test]
    fn id_column_co_permuted_with_geometry() {
        let layer = layer_after_sort(
            &[pt(0, 0), ls(&[(1, 0), (1, 5)]), pt(5, 5)],
            &[3, 1, 2],
            SortStrategy::Id,
        );

        let ids = match &layer.id {
            OwnedId::Decoded(Some(d)) => d.0.clone(),
            _ => panic!("expected decoded IDs after sort"),
        };
        assert_eq!(ids, vec![Some(1u64), Some(2), Some(3)]);

        let types = match &layer.geometry {
            OwnedGeometry::Decoded(g) => g.vector_types.clone(),
            _ => panic!("expected decoded geometry after sort"),
        };
        assert_eq!(
            types,
            vec![
                GeometryType::LineString,
                GeometryType::Point,
                GeometryType::Point
            ],
        );
    }
}
