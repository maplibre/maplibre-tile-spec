use std::collections::BTreeSet;

use super::model::VertexBufferType;
use crate::MltResult;
use crate::codecs::morton::{encode_morton, morton_deltas, z_order_params};
use crate::codecs::zigzag::encode_componentwise_delta_vec2s;
use crate::decoder::GeometryType::{LineString, Point, Polygon};
use crate::decoder::{
    ColumnType, DictionaryType, GeometryType, GeometryValues, LengthType, LogicalEncoding,
    MortonMeta, OffsetType, StreamType,
};
use crate::encoder::Encoder;
use crate::encoder::model::ColumnKind::Geometry;
use crate::encoder::stream::{write_precomputed_u32, write_u32_stream};
use crate::errors::AsMltError as _;
use crate::utils::AsUsize as _;

/// Compute `ZOrderCurve` parameters from the vertex value range.
///
/// Returns `(num_bits, coordinate_shift)` matching Java's `SpaceFillingCurve`.
/// Build a sorted unique Morton dictionary and per-vertex offset indices from a flat
/// `[x0, y0, x1, y1, …]` vertex slice.
///
/// Returns `(sorted_unique_codes, per_vertex_offsets)`.
fn build_morton_dict(vertices: &[i32], meta: MortonMeta) -> MltResult<(Vec<u32>, Vec<u32>)> {
    let codes: Vec<u32> = vertices
        .chunks_exact(2)
        .map(|c| encode_morton(c[0], c[1], meta))
        .collect::<Result<_, _>>()?;

    let dict: Vec<u32> = codes
        .iter()
        .copied()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();

    let offsets: Vec<u32> = codes
        .iter()
        .map(|&code| u32::try_from(dict.partition_point(|&c| c < code)).or_overflow())
        .collect::<Result<_, _>>()?;

    Ok((dict, offsets))
}

/// Push consecutive offset-differences from `offsets` onto `lengths`.
///
/// Expects a slice of `n + 1` elements and produces `n` lengths,
/// one per consecutive pair: `offsets[i + 1] - offsets[i]`.
#[inline]
fn extend_offsets(lengths: &mut Vec<u32>, offsets: &[u32]) -> usize {
    lengths.extend(offsets.windows(2).map(|w| w[1] - w[0]));
    offsets.len() - 1
}

/// Convert geometry offsets to length stream for encoding.
/// This is the inverse of `decode_root_length_stream`.
///
/// The offset array can be either:
/// - Sparse: entries only for geometries that need them (types > `buffer_id`), N+1 entries for N matching geoms
/// - Dense (normalized): N+1 entries for N geometry types, indexed by geometry position
///
/// If dense `(len == geom_types.len() + 1)`, use geometry index directly.
/// If sparse, use sequential indexing for matching geometry types.
fn encode_root_length_stream(
    geom_types: &[GeometryType],
    geom_offsets: &[u32],
    buffer_id: GeometryType,
) -> Vec<u32> {
    if geom_offsets.len() == geom_types.len() + 1 {
        // Dense: zip by position, then filter out non-contributing types.
        geom_types
            .iter()
            .zip(geom_offsets.windows(2))
            .filter(|&(&t, _)| t > buffer_id)
            .map(|(_, w)| w[1] - w[0])
            .collect()
    } else {
        // Sparse: filter types first, then zip with consecutive offset pairs.
        geom_types
            .iter()
            .filter(|&&t| t > buffer_id)
            .zip(geom_offsets.windows(2))
            .map(|(_, w)| w[1] - w[0])
            .collect()
    }
}

/// Convert part offsets to length stream for level 1 encoding.
fn encode_level1_length_stream(
    geom_types: &[GeometryType],
    geom_offsets: &[u32],
    part_offsets: &[u32],
    is_line_string_present: bool,
) -> Vec<u32> {
    let mut lengths = Vec::new();
    let mut part_idx = 0;

    for (i, &geom_type) in geom_types.iter().enumerate() {
        if geom_type.is_polygon() || (is_line_string_present && geom_type.is_linestring()) {
            let n = (geom_offsets[i + 1] - geom_offsets[i]).as_usize();
            part_idx += extend_offsets(&mut lengths, &part_offsets[part_idx..=part_idx + n]);
        }
        // Note: Point/MultiPoint don't have entries in the sparse part_offsets used
        // at this call site, so part_idx must not advance for non-length types here.
    }

    lengths
}

/// Compute ring vertex-count lengths for the no-geometry-offsets + has-ring-offsets case.
///
/// In this branch `part_offsets` is a **dense** N+1 array (one slot per geometry,
/// including Points) and `ring_offsets` holds the vertex offsets for every slot.
/// Using the geometry index directly as the ring-slot index avoids the
/// running-counter misalignment that `encode_level1_length_stream` would produce
/// when non-length types (Points) occupy slots that a sparse counter skips.
fn encode_ring_lengths_for_mixed(
    geom_types: &[GeometryType],
    part_offsets: &[u32],
    ring_offsets: &[u32],
    has_line_string: bool,
) -> Vec<u32> {
    let mut lengths = Vec::new();
    for (i, &geom_type) in geom_types.iter().enumerate() {
        if geom_type.is_polygon() || (has_line_string && geom_type.is_linestring()) {
            let s = part_offsets[i].as_usize();
            let e = part_offsets[i + 1].as_usize();
            extend_offsets(&mut lengths, &ring_offsets[s..=e]);
        }
    }
    lengths
}

/// Convert ring offsets to length stream for level 2 encoding.
/// This is the inverse of `decode_level2_length_stream`.
///
/// The `geom_offsets` array is expected to be an N+1 element array for N geometries.
/// The `part_offsets` array tracks ring counts cumulatively.
fn encode_level2_length_stream(
    geom_types: &[GeometryType],
    geom_offsets: &[u32],
    part_offsets: &[u32],
    ring_offsets: &[u32],
) -> Vec<u32> {
    let mut lengths = Vec::new();
    let mut part_idx = 0;
    let mut ring_idx = 0;

    for (i, &geom_type) in geom_types.iter().enumerate() {
        let count = (geom_offsets[i + 1] - geom_offsets[i]).as_usize();

        // Only Polygon and MultiPolygon have ring data in level 2
        // LineStrings with Polygon present add their vertex counts directly to ring_offsets,
        // but they don't have parts (ring count per linestring is always 1 implicitly)
        if geom_type.is_polygon() {
            // Polygon/MultiPolygon: iterate through sub-polygons, each has parts (ring counts)
            for _ in 0..count {
                let n = (part_offsets[part_idx + 1] - part_offsets[part_idx]).as_usize();
                ring_idx += extend_offsets(&mut lengths, &ring_offsets[ring_idx..=ring_idx + n]);
                part_idx += 1;
            }
        } else if geom_type.is_linestring() {
            // LineStrings contribute to ring_offsets directly (vertex counts)
            ring_idx += extend_offsets(&mut lengths, &ring_offsets[ring_idx..=ring_idx + count]);
        }
        // Note: Point/MultiPoint don't contribute to ring_offsets
    }

    lengths
}

/// Convert part offsets without ring buffer to length stream.
///
/// This path is reached only when `ring_offsets` is absent, which means no Polygon/MultiPolygon
/// types are present (they always create `ring_offsets`).  Only LineString/MultiLineString
/// contribute vertex-count lengths here; Point/MultiPoint use an implicit count of 1 in the
/// decoder and produce no entry in this stream.
fn encode_level1_without_ring_buffer_length_stream(
    geom_types: &[GeometryType],
    geom_offsets: &[u32],
    part_offsets: &[u32],
) -> Vec<u32> {
    let mut lengths = Vec::new();
    let mut part_idx = 0;

    for (i, &geom_type) in geom_types.iter().enumerate() {
        if geom_type.is_linestring() {
            let n = (geom_offsets[i + 1] - geom_offsets[i]).as_usize();
            part_idx += extend_offsets(&mut lengths, &part_offsets[part_idx..=part_idx + n]);
        }
        // Point/MultiPoint don't contribute to part_offsets; part_idx must not advance.
    }

    lengths
}

/// Normalize `geom_offsets` for mixed geometry types.
fn normalize_geometry_offsets(vector_types: &[GeometryType], geom_offsets: &[u32]) -> Vec<u32> {
    let mut normalized = Vec::with_capacity(vector_types.len() + 1);
    let mut offset = 0_u32;
    let mut sparse_idx = 0_usize; // Index into sparse geom_offsets

    for &geom_type in vector_types {
        normalized.push(offset);

        if geom_type.is_multi() {
            // Multi* types get their count from the sparse array
            if sparse_idx + 1 < geom_offsets.len() {
                let start = geom_offsets[sparse_idx];
                let end = geom_offsets[sparse_idx + 1];
                offset += end - start;
                sparse_idx += 1;
            }
        } else {
            // Non-Multi types have implicit count of 1
            offset += 1;
        }
    }

    normalized.push(offset);
    normalized
}

/// Normalize `part_offsets` for ring-based indexing (Polygon mixed with `Point`/`LineString`).
///
/// Called only when `geom_offsets` is absent (no Multi\* types) and `ring_offsets` is
/// present.  In this context `part_offsets` is a compact polygon-only array; this function
/// expands it to a dense per-geometry array so that `encode_ring_lengths_for_mixed` can index
/// directly by geometry position.
///
/// Each slot in the output holds the first index into `ring_offsets` for that geometry:
/// - `Point`: no contribution — slot range is empty (`ring_idx` unchanged).
/// - `LineString`: contributes 1 slot (vertex count) — slot range is 1.
/// - `Polygon`: contributes `ring_count` slots — slot range equals its ring count.
fn normalize_part_offsets_for_rings(
    vector_types: &[GeometryType],
    part_offsets: &[u32],
    ring_offsets: &[u32],
) -> Vec<u32> {
    let mut normalized = Vec::with_capacity(vector_types.len() + 1);
    let mut ring_idx = 0_u32;
    let mut part_idx = 0_usize;

    for &geom_type in vector_types {
        normalized.push(ring_idx);

        if geom_type == Point {
            // Point has no vertex-count slot in ring_offsets.
        } else if geom_type.is_linestring() {
            // Each LineString occupies exactly one slot in ring_offsets.
            ring_idx += 1;
        } else if geom_type.is_polygon() && part_idx + 1 < part_offsets.len() {
            // Polygon occupies ring_count slots (one vertex-count per ring).
            let ring_count = part_offsets[part_idx + 1] - part_offsets[part_idx];
            ring_idx += ring_count;
            part_idx += 1;
        }
        // No Multi* types can appear here (they always produce geom_offsets).
    }

    // ring_idx must equal ring_offsets.len() - 1 for well-formed data.
    debug_assert_eq!(
        ring_idx as usize,
        ring_offsets.len().saturating_sub(1),
        "ring index mismatch after normalization"
    );
    normalized.push(ring_idx);
    normalized
}

/// Choose between Vec2 componentwise-delta and Morton dictionary encoding.
///
/// Morton is only selected when:
/// - The coordinate range fits within 16 bits per axis (required by the spec), and
/// - The uniqueness ratio is below the threshold, meaning enough vertices are
///   repeated that the dictionary overhead is worthwhile.
///
/// Returns the chosen [`VertexBufferType`] together with the pre-computed [`MortonMeta`]
/// when Morton is selected, so the caller can reuse it without a second range scan.
pub fn select_vertex_strategy(vertices: &[i32]) -> (VertexBufferType, Option<MortonMeta>) {
    const MORTON_UNIQUENESS_THRESHOLD: f64 = 0.5;

    let total = vertices.len() / 2;
    if total == 0 {
        return (VertexBufferType::Vec2, None);
    }
    let Ok(meta) = z_order_params(vertices) else {
        return (VertexBufferType::Vec2, None);
    };

    let unique_count = vertices
        .chunks_exact(2)
        .map(|c| (c[0], c[1]))
        .collect::<std::collections::HashSet<_>>()
        .len();

    #[expect(clippy::cast_precision_loss)]
    let uniqueness_ratio = unique_count as f64 / total as f64;

    if uniqueness_ratio < MORTON_UNIQUENESS_THRESHOLD {
        (VertexBufferType::Morton, Some(meta))
    } else {
        (VertexBufferType::Vec2, None)
    }
}

/// Write a geometry `u32` stream: [`Encoder::override_int_enc`] when explicit mode is active,
/// otherwise try all pruned candidates and keep the shortest.
///
/// Returns `1` if the stream was written, `0` if it was skipped.  Empty streams are skipped
/// unless [`Encoder::force_stream`] returns `true` for `geo_stream_name`.
fn write_geo_u32_stream(
    data: &[u32],
    stream_type: StreamType,
    stream_name: &'static str,
    enc: &mut Encoder,
) -> MltResult<u8> {
    Ok(
        if data.is_empty() && !enc.force_stream(Geometry, stream_name) {
            0
        } else {
            write_u32_stream(data, stream_type, Geometry, stream_name, "", enc)?;
            1
        },
    )
}

/// Like [`write_geo_u32_stream`] but for pre-logically-encoded data: delegates to
/// [`write_precomputed_u32`] instead of [`write_u32_stream`].
///
/// Returns `1` if the stream was written, `0` if skipped (empty + no force).
fn write_geo_precomputed_stream(
    data: &[u32],
    stream_type: StreamType,
    logical: LogicalEncoding,
    stream_name: &'static str,
    enc: &mut Encoder,
) -> MltResult<u8> {
    Ok(
        if data.is_empty() && !enc.force_stream(Geometry, stream_name) {
            0
        } else {
            write_precomputed_u32(data, stream_type, logical, Geometry, stream_name, enc)?;
            1
        },
    )
}

impl GeometryValues {
    /// Write the geometry column to `enc`.
    pub fn write_to(self, enc: &mut Encoder) -> MltResult<()> {
        let Self {
            vector_types,
            geometry_offsets,
            part_offsets,
            ring_offsets,
            index_buffer,
            triangles,
            vertices,
        } = self;

        // Flatten every Option<Vec> → Vec  (empty == not present).
        // triangles: None means no tessellation; Some([]) can't occur in practice (each
        // push_geom appends a count), so empty == absent is safe here too.
        // vertices: None means no coordinate data (e.g. empty layer).
        let geom_offsets = geometry_offsets.unwrap_or_default();
        let part_offsets = part_offsets.unwrap_or_default();
        let ring_offsets = ring_offsets.unwrap_or_default();
        let index_buffer = index_buffer.unwrap_or_default();
        let triangles = triangles.unwrap_or_default();
        let vertices = vertices.unwrap_or_default();

        // Select vertex encoding strategy; auto-selection pre-computes MortonMeta so we don't
        // have to scan the vertex range twice.
        let (vertex_buffer_type, auto_morton_meta) = match enc.override_vertex_buffer_type() {
            Some(vbt) => (vbt, None),
            None => select_vertex_strategy(&vertices),
        };

        let meta: Vec<u32> = vector_types.iter().map(|t| *t as u32).collect();

        let part_offsets = if geom_offsets.is_empty()
            && !ring_offsets.is_empty()
            && !part_offsets.is_empty()
            && part_offsets.len() != vector_types.len() + 1
        {
            // Normalize part_offsets when there are no geometry offsets but ring offsets exist.
            normalize_part_offsets_for_rings(&vector_types, &part_offsets, &ring_offsets)
        } else {
            part_offsets
        };

        // Write column type to meta; reserve exactly 1 byte for stream count
        // (geometry never exceeds ~8 streams, always fits in a single varint byte).
        ColumnType::Geometry.write_to(&mut enc.meta)?;
        let stream_count_pos = enc.data.len();
        enc.data.push(0); // placeholder — patched below
        let mut n: u8 = 0;

        // Meta stream — always written, even for a zero-feature layer.
        let typ = StreamType::Length(LengthType::VarBinary);
        write_u32_stream(&meta, typ, Geometry, "meta", "", enc)?;
        n += 1;

        // Topology: compute each length stream and write it immediately.
        if !geom_offsets.is_empty() {
            let geom_offsets = if geom_offsets.len() == vector_types.len() + 1 {
                geom_offsets
            } else {
                normalize_geometry_offsets(&vector_types, &geom_offsets)
            };
            let data = encode_root_length_stream(&vector_types, &geom_offsets, Polygon);
            let typ = StreamType::Length(LengthType::Geometries);
            n += write_geo_u32_stream(&data, typ, "geometries", enc)?;

            // part_offsets is intentionally kept sparse here (polygon-only cumulative
            // ring counts). encode_level1/2_length_stream navigate it with a running
            // part_idx counter that advances only for Polygon/LineString types, which
            // matches the sparse layout. Densifying via normalize_part_offsets_for_rings
            // would insert Point slots and corrupt the counter arithmetic.
            if !part_offsets.is_empty() {
                if ring_offsets.is_empty() {
                    // geom → parts only (no rings).
                    let data = encode_level1_without_ring_buffer_length_stream(
                        &vector_types,
                        &geom_offsets,
                        &part_offsets,
                    );
                    let typ = StreamType::Length(LengthType::Parts);
                    n += write_geo_u32_stream(&data, typ, "no_rings", enc)?;
                } else {
                    // Full topology: geom → parts → rings.
                    // LineStrings contribute to rings here, not to parts.
                    let data = encode_level1_length_stream(
                        &vector_types,
                        &geom_offsets,
                        &part_offsets,
                        false,
                    );
                    let typ = StreamType::Length(LengthType::Parts);
                    n += write_geo_u32_stream(&data, typ, "rings", enc)?;

                    let data = encode_level2_length_stream(
                        &vector_types,
                        &geom_offsets,
                        &part_offsets,
                        &ring_offsets,
                    );
                    let typ = StreamType::Length(LengthType::Rings);
                    n += write_geo_u32_stream(&data, typ, "rings2", enc)?;
                }
            }
        } else if !part_offsets.is_empty() {
            if ring_offsets.is_empty() {
                let data = encode_root_length_stream(&vector_types, &part_offsets, Point);
                let typ = StreamType::Length(LengthType::Parts);
                n += write_geo_u32_stream(&data, typ, "no_rings", enc)?;
            } else {
                // No Multi* types; parts → rings (Polygon / mixed Point+Polygon).
                // Java writes an empty GEOMETRIES stream here for tessellated polygons; only do
                // so when explicitly forced (e.g. to preserve byte-for-byte Java compatibility).
                let typ = StreamType::Length(LengthType::Geometries);
                n += write_geo_u32_stream(&[], typ, "geometries", enc)?;

                let data = encode_root_length_stream(&vector_types, &part_offsets, LineString);
                let typ = StreamType::Length(LengthType::Parts);
                n += write_geo_u32_stream(&data, typ, "parts", enc)?;

                // part_offs is a dense N+1 array (one slot per geometry incl. Points);
                // ring_offs stores vertex offsets per slot.  The dense-aware helper skips
                // Point slots by index rather than a running counter.
                let has_line_string = vector_types
                    .iter()
                    .copied()
                    .any(GeometryType::is_linestring);
                let data = encode_ring_lengths_for_mixed(
                    &vector_types,
                    &part_offsets,
                    &ring_offsets,
                    has_line_string,
                );
                let typ = StreamType::Length(LengthType::Rings);
                n += write_geo_u32_stream(&data, typ, "parts_ring", enc)?;
            }
        }

        let typ = StreamType::Length(LengthType::Triangles);
        n += write_geo_u32_stream(&triangles, typ, "triangles", enc)?;
        let typ = StreamType::Offset(OffsetType::Index);
        n += write_geo_u32_stream(&index_buffer, typ, "triangles_indexes", enc)?;

        // Vertex streams — compute and write inline.
        match vertex_buffer_type {
            VertexBufferType::Vec2 => {
                let delta = encode_componentwise_delta_vec2s(&vertices);
                let typ = StreamType::Data(DictionaryType::Vertex);
                let logical = LogicalEncoding::ComponentwiseDelta;
                n += write_geo_precomputed_stream(&delta, typ, logical, "vertex", enc)?;
            }
            VertexBufferType::Morton => {
                // Reuse the MortonMeta computed during auto-selection to avoid a second vertex
                // range scan; compute it fresh only when the type was forced via override.
                let morton_meta = match auto_morton_meta {
                    Some(meta) => meta,
                    None => z_order_params(&vertices)?,
                };
                let (dict, offsets) = build_morton_dict(&vertices, morton_meta)?;
                let typ = StreamType::Offset(OffsetType::Vertex);
                n += write_geo_u32_stream(&offsets, typ, "vertex_offsets", enc)?;

                let data = morton_deltas(&dict);
                let typ = StreamType::Data(DictionaryType::Morton);
                let logical = LogicalEncoding::MortonDelta(morton_meta);
                n += write_geo_precomputed_stream(&data, typ, logical, "vertex", enc)?;
            }
        }

        // Patch the reserved stream-count byte.
        debug_assert!(n <= 127, "geometry stream count must fit in one byte");
        enc.data[stream_count_pos] = n;
        enc.increment_column_count();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_root_length_stream() {
        // Single Polygon geometry (no Multi)
        let types = vec![Polygon];
        let offsets = vec![0, 1]; // One polygon

        let lengths = encode_root_length_stream(&types, &offsets, Polygon);
        // Polygon == buffer_id, so no length encoded
        assert!(lengths.is_empty());

        // MultiPolygon needs length encoded
        let types = vec![GeometryType::MultiPolygon];
        let offsets = vec![0, 2]; // MultiPolygon with 2 polygons

        let lengths = encode_root_length_stream(&types, &offsets, Polygon);
        assert_eq!(lengths, vec![2]);
    }
}
