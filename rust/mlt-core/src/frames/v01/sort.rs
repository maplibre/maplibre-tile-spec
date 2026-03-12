use std::borrow::Cow;

use crate::MltError;
use crate::decode::FromEncoded as _;
use crate::utils::{hilbert_curve_params, hilbert_sort_key, morton_sort_key};
use crate::v01::{
    DecodedGeometry, DecodedId, DecodedProperty, DecodedStrings, OwnedGeometry, OwnedId,
    OwnedLayer01, OwnedProperty,
};

// ─── Public types ─────────────────────────────────────────────────────────────

/// The space-filling curve used when sorting features spatially.
///
/// Both curves sort features so that spatially adjacent features end up
/// near each other in the stream, improving property RLE runs and client-side
/// cache locality.  Hilbert generally achieves better spatial locality than
/// Morton (Z-order) but is more expensive to compute.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SpaceFillingCurve {
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
/// opt in explicitly.  The default is [`SortStrategy::None`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SortStrategy {
    /// Preserve the original input order - no implicit ordering assumptions made by the map creator are violated.
    #[default]
    None,

    /// Sort features by the space-filling curve index of their first vertex.
    ///
    /// Spatially close features end up adjacent in the stream, which improves
    /// RLE run lengths/ Detlas for properties that correlate with location (e.g.
    /// `class`, `admin_level`, `house_number`) and may improve CPU cache locality during
    /// client-side decoding for some very large tiles.
    Spatial(SpaceFillingCurve),

    /// Sort features by their feature ID in ascending order.
    Id,
}

/// Reorder all columns of `layer` according to `strategy`.
///
/// All columns are decoded in-place before the permutation is applied.
/// If the layer has zero or one features, or if the strategy is
/// [`SortStrategy::None`], this is a no-op.
///
/// Tessellated geometry (`index_buffer` / `triangles` fields) is not yet
/// supported: layers containing either field are left unchanged.
pub(crate) fn reorder_features(
    layer: &mut OwnedLayer01,
    strategy: SortStrategy,
) -> Result<(), MltError> {
    if strategy == SortStrategy::None {
        return Ok(());
    }

    // Everything must be in decoded form before we can permute it.
    ensure_decoded(layer)?;

    let n = geometry_feature_count(&layer.geometry)?;
    if n <= 1 {
        return Ok(());
    }

    // Skip tessellated layers — index_buffer / triangles are not permutable
    // without retessellating.
    if let OwnedGeometry::Decoded(ref g) = layer.geometry
        && (g.index_buffer.is_some() || g.triangles.is_some())
    {
        return Ok(());
    }

    let keys = compute_sort_keys(layer, strategy, n)?;
    let perm = build_permutation(&keys);
    apply_permutation(layer, &perm)
}

// ─── Sort key computation ─────────────────────────────────────────────────────

/// Compute one sort key per feature.  The key type is `u64` so that both
/// curve codes (u32) and raw IDs (u64) can share the same return type.
fn compute_sort_keys(
    layer: &OwnedLayer01,
    strategy: SortStrategy,
    n: usize,
) -> Result<Vec<u64>, MltError> {
    match strategy {
        SortStrategy::None => unreachable!("None is filtered before calling this"),

        SortStrategy::Spatial(curve) => {
            let geom = match &layer.geometry {
                OwnedGeometry::Decoded(g) => g,
                OwnedGeometry::Encoded(_) => return Err(MltError::NotDecoded("geometry")),
            };
            Ok(spatial_sort_keys(geom, n, curve)
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
fn spatial_sort_keys(decoded: &DecodedGeometry, n: usize, curve: SpaceFillingCurve) -> Vec<u32> {
    match curve {
        SpaceFillingCurve::Morton => {
            let (x_shift, y_shift) = morton_coordinate_shifts(decoded);
            (0..n)
                .map(|i| {
                    first_vertex(i, decoded)
                        .map_or(u32::MAX, |(x, y)| morton_sort_key(x, y, x_shift, y_shift))
                })
                .collect()
        }

        SpaceFillingCurve::Hilbert => {
            let verts = decoded.vertices.as_deref().unwrap_or(&[]);
            let (shift, num_bits) = hilbert_curve_params(verts);
            (0..n)
                .map(|i| {
                    first_vertex(i, decoded)
                        .map_or(u32::MAX, |(x, y)| hilbert_sort_key(x, y, shift, num_bits))
                })
                .collect()
        }
    }
}

/// Compute per-axis non-negative shifts for Morton coding.
///
/// Returns `(x_shift, y_shift)` where each shift equals `min.unsigned_abs()`
/// when the axis minimum is negative, and `0` otherwise.
fn morton_coordinate_shifts(decoded: &DecodedGeometry) -> (u32, u32) {
    let Some(verts) = decoded.vertices.as_deref() else {
        return (0, 0);
    };
    let min_x = verts.iter().copied().step_by(2).min().unwrap_or(0);
    let min_y = verts.iter().copied().skip(1).step_by(2).min().unwrap_or(0);
    let x_shift = if min_x < 0 { min_x.unsigned_abs() } else { 0 };
    let y_shift = if min_y < 0 { min_y.unsigned_abs() } else { 0 };
    (x_shift, y_shift)
}

/// Extract the `(x, y)` coordinate of the first vertex for feature `i`.
///
/// Navigates the offset hierarchy present in `decoded` to find the correct
/// position in the flat vertex buffer.  Returns `None` if there are no
/// vertices or the index is out of range.
fn first_vertex(i: usize, decoded: &DecodedGeometry) -> Option<(i32, i32)> {
    let verts = decoded.vertices.as_deref()?;

    let vtx_pair_idx = match (
        decoded.geometry_offsets.as_deref(),
        decoded.part_offsets.as_deref(),
        decoded.ring_offsets.as_deref(),
    ) {
        // Points: 1:1 with vertex pairs.
        (None, None, None) => i,

        // LineStrings / mixed Point+LineString:
        // part_offsets[i] = start vertex pair for feature i.
        (None, Some(parts), None) => *parts.get(i)? as usize,

        // Polygons / mixed with rings:
        // part_offsets[i] = start ring for feature i;
        // ring_offsets[ring] = start vertex pair for that ring.
        (None, Some(parts), Some(rings)) => {
            let ring_start = *parts.get(i)? as usize;
            *rings.get(ring_start)? as usize
        }

        // MultiPoint: geometry_offsets[i] = start vertex pair.
        (Some(geoms), None, None) => *geoms.get(i)? as usize,

        // Multi + parts: geometry_offsets[i] = start sub-geom;
        //                part_offsets[sub-geom] = start vertex pair.
        (Some(geoms), Some(parts), None) => {
            let geom_start = *geoms.get(i)? as usize;
            *parts.get(geom_start)? as usize
        }

        // Full hierarchy: geometry → sub-geom → ring → vertex.
        (Some(geoms), Some(parts), Some(rings)) => {
            let geom_start = *geoms.get(i)? as usize;
            let part_start = *parts.get(geom_start)? as usize;
            *rings.get(part_start)? as usize
        }

        // Any other combination is unexpected.
        _ => return None,
    };

    let x = *verts.get(2 * vtx_pair_idx)?;
    let y = *verts.get(2 * vtx_pair_idx + 1)?;
    Some((x, y))
}

// ─── Permutation builder ──────────────────────────────────────────────────────

/// Build a permutation such that `perm[new_position] = old_position`.
///
/// Uses a stable sort so that features with equal keys retain their original
/// relative order (important for z-fighting stability).
fn build_permutation<K: Ord>(keys: &[K]) -> Vec<usize> {
    let mut indices: Vec<usize> = (0..keys.len()).collect();
    indices.sort_by(|&a, &b| keys[a].cmp(&keys[b]));
    indices
}

// ─── Permutation application ──────────────────────────────────────────────────

#[allow(clippy::unnecessary_wraps)]
fn apply_permutation(layer: &mut OwnedLayer01, perm: &[usize]) -> Result<(), MltError> {
    if let OwnedGeometry::Decoded(geom) = &mut layer.geometry {
        permute_geometry(geom, perm);
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

// ─── Decode-in-place ──────────────────────────────────────────────────────────

/// Decode all columns of `layer` in-place so that permutation can be applied
/// to the plain decoded values.
///
/// Each column is a no-op if it is already in the `Decoded` variant.
pub(crate) fn ensure_decoded(layer: &mut OwnedLayer01) -> Result<(), MltError> {
    if let OwnedGeometry::Encoded(e) = &layer.geometry {
        let dec = DecodedGeometry::from_encoded(borrowme::borrow(e))?;
        layer.geometry = OwnedGeometry::Decoded(dec);
    }

    if let OwnedId::Encoded(e) = &layer.id {
        let dec = Option::<DecodedId>::from_encoded(e.as_ref().map(borrowme::borrow))?;
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

// ─── Geometry permutation ─────────────────────────────────────────────────────

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
fn permute_geometry(decoded: &mut DecodedGeometry, perm: &[usize]) {
    let n = perm.len();

    let old_types = std::mem::take(&mut decoded.vector_types);
    decoded.vector_types = perm.iter().map(|&i| old_types[i]).collect();

    let old_geom_offs = decoded.geometry_offsets.take();
    let old_part_offs = decoded.part_offsets.take();
    let old_ring_offs = decoded.ring_offsets.take();
    let Some(old_verts) = decoded.vertices.take() else {
        decoded.geometry_offsets = old_geom_offs;
        decoded.part_offsets = old_part_offs;
        decoded.ring_offsets = old_ring_offs;
        return;
    };

    match (old_geom_offs, old_part_offs, old_ring_offs) {
        // ── Case 1: Points ────────────────────────────────────────────────
        (None, None, None) => {
            decoded.vertices = Some(
                perm.iter()
                    .flat_map(|&i| [old_verts[2 * i], old_verts[2 * i + 1]])
                    .collect(),
            );
        }

        // ── Case 2: LineStrings (part_offsets → vertex pairs) ─────────────
        (None, Some(old_parts), None) => {
            let mut new_parts = Vec::with_capacity(n + 1);
            let mut new_verts = Vec::new();
            new_parts.push(0u32);
            for &i in perm {
                let vs = old_parts[i] as usize;
                let ve = old_parts[i + 1] as usize;
                #[expect(
                    clippy::cast_possible_truncation,
                    reason = "offset differences originate as u32 values"
                )]
                new_parts.push(*new_parts.last().unwrap() + (ve - vs) as u32);
                new_verts.extend_from_slice(&old_verts[2 * vs..2 * ve]);
            }
            decoded.part_offsets = Some(new_parts);
            decoded.vertices = Some(new_verts);
        }

        // ── Case 3: Polygons (part_offsets → ring_offsets → vtx) ──────────
        (None, Some(old_parts), Some(old_rings)) => {
            let mut new_parts = Vec::with_capacity(n + 1);
            let mut new_rings = Vec::new();
            let mut new_verts = Vec::new();
            new_parts.push(0u32);
            new_rings.push(0u32);
            for &i in perm {
                let rs = old_parts[i] as usize;
                let re = old_parts[i + 1] as usize;
                #[expect(
                    clippy::cast_possible_truncation,
                    reason = "offset differences originate as u32 values"
                )]
                new_parts.push(*new_parts.last().unwrap() + (re - rs) as u32);
                for r in rs..re {
                    let vs = old_rings[r] as usize;
                    let ve = old_rings[r + 1] as usize;
                    #[expect(
                        clippy::cast_possible_truncation,
                        reason = "offset differences originate as u32 values"
                    )]
                    new_rings.push(*new_rings.last().unwrap() + (ve - vs) as u32);
                    new_verts.extend_from_slice(&old_verts[2 * vs..2 * ve]);
                }
            }
            decoded.part_offsets = Some(new_parts);
            decoded.ring_offsets = Some(new_rings);
            decoded.vertices = Some(new_verts);
        }

        // ── Case 4: MultiPoints (geometry_offsets → vertex pairs) ─────────
        (Some(old_geoms), None, None) => {
            let mut new_geoms = Vec::with_capacity(n + 1);
            let mut new_verts = Vec::new();
            new_geoms.push(0u32);
            for &i in perm {
                let vs = old_geoms[i] as usize;
                let ve = old_geoms[i + 1] as usize;
                #[expect(
                    clippy::cast_possible_truncation,
                    reason = "offset differences originate as u32 values"
                )]
                new_geoms.push(*new_geoms.last().unwrap() + (ve - vs) as u32);
                new_verts.extend_from_slice(&old_verts[2 * vs..2 * ve]);
            }
            decoded.geometry_offsets = Some(new_geoms);
            decoded.vertices = Some(new_verts);
        }

        // ── Case 5: MultiLines (geom_offs → part_offs → vertex pairs) ─────
        (Some(old_geoms), Some(old_parts), None) => {
            let total_geoms = *old_geoms.last().unwrap_or(&0) as usize;
            let mut new_geoms = Vec::with_capacity(n + 1);
            let mut new_parts = Vec::with_capacity(total_geoms + 1);
            let mut new_verts = Vec::new();
            new_geoms.push(0u32);
            new_parts.push(0u32);
            for &i in perm {
                let gs = old_geoms[i] as usize;
                let ge = old_geoms[i + 1] as usize;
                #[expect(
                    clippy::cast_possible_truncation,
                    reason = "offset differences originate as u32 values"
                )]
                new_geoms.push(*new_geoms.last().unwrap() + (ge - gs) as u32);
                for g in gs..ge {
                    let vs = old_parts[g] as usize;
                    let ve = old_parts[g + 1] as usize;
                    #[expect(
                        clippy::cast_possible_truncation,
                        reason = "offset differences originate as u32 values"
                    )]
                    new_parts.push(*new_parts.last().unwrap() + (ve - vs) as u32);
                    new_verts.extend_from_slice(&old_verts[2 * vs..2 * ve]);
                }
            }
            decoded.geometry_offsets = Some(new_geoms);
            decoded.part_offsets = Some(new_parts);
            decoded.vertices = Some(new_verts);
        }

        // ── Case 6: MultiPolygons (geom → part → ring → vertex pairs) ─────
        (Some(old_geoms), Some(old_parts), Some(old_rings)) => {
            let total_geoms = *old_geoms.last().unwrap_or(&0) as usize;
            let total_parts = *old_parts.last().unwrap_or(&0) as usize;
            let mut new_geoms = Vec::with_capacity(n + 1);
            let mut new_parts = Vec::with_capacity(total_geoms + 1);
            let mut new_rings = Vec::with_capacity(total_parts + 1);
            let mut new_verts = Vec::new();
            new_geoms.push(0u32);
            new_parts.push(0u32);
            new_rings.push(0u32);
            for &i in perm {
                let gs = old_geoms[i] as usize;
                let ge = old_geoms[i + 1] as usize;
                #[expect(
                    clippy::cast_possible_truncation,
                    reason = "offset differences originate as u32 values"
                )]
                new_geoms.push(*new_geoms.last().unwrap() + (ge - gs) as u32);
                for g in gs..ge {
                    let ps = old_parts[g] as usize;
                    let pe = old_parts[g + 1] as usize;
                    #[expect(
                        clippy::cast_possible_truncation,
                        reason = "offset differences originate as u32 values"
                    )]
                    new_parts.push(*new_parts.last().unwrap() + (pe - ps) as u32);
                    for r in ps..pe {
                        let vs = old_rings[r] as usize;
                        let ve = old_rings[r + 1] as usize;
                        #[expect(
                            clippy::cast_possible_truncation,
                            reason = "offset differences originate as u32 values"
                        )]
                        new_rings.push(*new_rings.last().unwrap() + (ve - vs) as u32);
                        new_verts.extend_from_slice(&old_verts[2 * vs..2 * ve]);
                    }
                }
            }
            decoded.geometry_offsets = Some(new_geoms);
            decoded.part_offsets = Some(new_parts);
            decoded.ring_offsets = Some(new_rings);
            decoded.vertices = Some(new_verts);
        }

        // Unexpected combination — restore everything unchanged.
        (old_geom_offs, old_part_offs, old_ring_offs) => {
            decoded.geometry_offsets = old_geom_offs;
            decoded.part_offsets = old_part_offs;
            decoded.ring_offsets = old_ring_offs;
            decoded.vertices = Some(old_verts);
        }
    }
}

// ─── ID permutation ───────────────────────────────────────────────────────────

fn permute_id(id: &mut DecodedId, perm: &[usize]) {
    let old = id.0.clone();
    id.0 = perm.iter().map(|&i| old[i]).collect();
}

// ─── Property permutation ─────────────────────────────────────────────────────

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

// ─── Helper ───────────────────────────────────────────────────────────────────

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

    let min_x = vertices.iter().copied().step_by(2).min().unwrap_or(0);
    let max_x = vertices.iter().copied().step_by(2).max().unwrap_or(0);
    let min_y = vertices
        .iter()
        .copied()
        .skip(1)
        .step_by(2)
        .min()
        .unwrap_or(0);
    let max_y = vertices
        .iter()
        .copied()
        .skip(1)
        .step_by(2)
        .max()
        .unwrap_or(0);

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
