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

/// Convert geometry offsets to length stream for encoding.
/// This is the inverse of `decode_root_length_stream`.
///
/// The offset array can be either:
/// - Sparse: entries only for geometries that need them (types > `buffer_id`), N+1 entries for N matching geoms
/// - Dense (normalized): N+1 entries for N geometry types, indexed by geometry position
///
/// If dense `(len == geometry_types.len() + 1)`, use geometry index directly.
/// If sparse, use sequential indexing for matching geometry types.
fn encode_root_length_stream(
    geometry_types: &[GeometryType],
    geometry_offsets: &[u32],
    buffer_id: GeometryType,
) -> Vec<u32> {
    let mut lengths = Vec::new();

    if geometry_offsets.len() == geometry_types.len() + 1 {
        // Dense array: use geometry index directly
        for (i, &geom_type) in geometry_types.iter().enumerate() {
            if geom_type > buffer_id {
                let start = geometry_offsets[i];
                let end = geometry_offsets[i + 1];
                lengths.push(end - start);
            }
        }
    } else {
        // Sparse array: sequential indexing for matching types
        let mut offset_idx = 0;
        for &geom_type in geometry_types {
            if geom_type > buffer_id {
                let start = geometry_offsets[offset_idx];
                let end = geometry_offsets[offset_idx + 1];
                lengths.push(end - start);
                offset_idx += 1;
            }
        }
    }

    lengths
}

/// Convert part offsets to length stream for level 1 encoding.
fn encode_level1_length_stream(
    geometry_types: &[GeometryType],
    geometry_offsets: &[u32],
    part_offsets: &[u32],
    is_line_string_present: bool,
) -> Vec<u32> {
    let mut lengths = Vec::new();
    let mut part_idx = 0;

    for (i, &geom_type) in geometry_types.iter().enumerate() {
        let num_geoms = geometry_offsets[i + 1] - geometry_offsets[i];

        let needs_length =
            geom_type.is_polygon() || (is_line_string_present && geom_type.is_linestring());

        if needs_length {
            for _ in 0..num_geoms {
                let start = part_offsets[part_idx];
                let end = part_offsets[part_idx + 1];
                lengths.push(end - start);
                part_idx += 1;
            }
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
    geometry_types: &[GeometryType],
    part_offsets: &[u32],
    ring_offsets: &[u32],
    is_line_string_present: bool,
) -> Vec<u32> {
    let mut lengths = Vec::new();
    for (i, &geom_type) in geometry_types.iter().enumerate() {
        let needs_length =
            geom_type.is_polygon() || (is_line_string_present && geom_type.is_linestring());
        if needs_length {
            let slot_start = part_offsets[i].as_usize();
            let slot_end = part_offsets[i + 1].as_usize();
            for slot in slot_start..slot_end {
                lengths.push(ring_offsets[slot + 1] - ring_offsets[slot]);
            }
        }
    }
    lengths
}

/// Convert ring offsets to length stream for level 2 encoding.
/// This is the inverse of `decode_level2_length_stream`.
///
/// The `geometry_offsets` array is expected to be an N+1 element array for N geometries.
/// The `part_offsets` array tracks ring counts cumulatively.
fn encode_level2_length_stream(
    geometry_types: &[GeometryType],
    geometry_offsets: &[u32],
    part_offsets: &[u32],
    ring_offsets: &[u32],
) -> Vec<u32> {
    let mut lengths = Vec::new();
    let mut part_idx = 0;
    let mut ring_idx = 0;

    for (i, &geom_type) in geometry_types.iter().enumerate() {
        let num_geoms = geometry_offsets[i + 1] - geometry_offsets[i];

        // Only Polygon and MultiPolygon have ring data in level 2
        // LineStrings with Polygon present add their vertex counts directly to ring_offsets,
        // but they don't have parts (ring count per linestring is always 1 implicitly)
        if geom_type.is_polygon() {
            // Polygon/MultiPolygon: iterate through sub-polygons, each has parts (ring counts)
            for _ in 0..num_geoms {
                let num_parts = part_offsets[part_idx + 1] - part_offsets[part_idx];
                part_idx += 1;
                for _ in 0..num_parts {
                    let start = ring_offsets[ring_idx];
                    let end = ring_offsets[ring_idx + 1];
                    lengths.push(end - start);
                    ring_idx += 1;
                }
            }
        } else if geom_type.is_linestring() {
            // LineStrings contribute to ring_offsets directly (vertex counts)
            // Each linestring is implicitly 1 "ring" in terms of vertex counts
            for _ in 0..num_geoms {
                let start = ring_offsets[ring_idx];
                let end = ring_offsets[ring_idx + 1];
                lengths.push(end - start);
                ring_idx += 1;
            }
        }
        // Note: Point/MultiPoint don't contribute to ring_offsets
    }

    lengths
}

/// Convert part offsets without ring buffer to length stream.
fn encode_level1_without_ring_buffer_length_stream(
    geometry_types: &[GeometryType],
    geometry_offsets: &[u32],
    part_offsets: &[u32],
) -> Vec<u32> {
    let mut lengths = Vec::new();
    let mut part_idx = 0;

    for (i, &geom_type) in geometry_types.iter().enumerate() {
        let num_geoms = (geometry_offsets[i + 1] - geometry_offsets[i]).as_usize();

        if geom_type.is_linestring() || geom_type.is_polygon() {
            for _ in 0..num_geoms {
                let start = part_offsets[part_idx];
                let end = part_offsets[part_idx + 1];
                lengths.push(end - start);
                part_idx += 1;
            }
        }
        // Note: Point/MultiPoint don't contribute to part_offsets, so don't advance part_idx
    }

    lengths
}

/// Normalize `geometry_offsets` for mixed geometry types.
fn normalize_geometry_offsets(vector_types: &[GeometryType], geometry_offsets: &[u32]) -> Vec<u32> {
    // Check if already normalized (has N+1 entries for N geometry types)
    if geometry_offsets.len() == vector_types.len() + 1 {
        return geometry_offsets.to_vec();
    }

    let mut normalized = Vec::with_capacity(vector_types.len() + 1);
    let mut current_offset = 0_u32;
    let mut sparse_idx = 0_usize; // Index into sparse geometry_offsets

    for &geom_type in vector_types {
        normalized.push(current_offset);

        if geom_type.is_multi() {
            // Multi* types get their count from the sparse array
            if sparse_idx + 1 < geometry_offsets.len() {
                let start = geometry_offsets[sparse_idx];
                let end = geometry_offsets[sparse_idx + 1];
                current_offset += end - start;
                sparse_idx += 1;
            }
        } else {
            // Non-Multi types have implicit count of 1
            current_offset += 1;
        }
    }

    normalized.push(current_offset);
    normalized
}

/// Normalize `part_offsets` for ring-based indexing (Polygon mixed with `Point`/`LineString`).
fn normalize_part_offsets_for_rings(
    vector_types: &[GeometryType],
    part_offsets: &[u32],
    ring_offsets: &[u32],
) -> Vec<u32> {
    let num_rings = if ring_offsets.is_empty() {
        0
    } else {
        u32::try_from(ring_offsets.len() - 1).expect("ring count overflow")
    };

    // Check if already normalized (has N+1 entries for N geometry types)
    if part_offsets.len() == vector_types.len() + 1 {
        return part_offsets.to_vec();
    }

    // Build normalized offset array for ring-based indexing
    let mut normalized = Vec::with_capacity(vector_types.len() + 1);
    let mut ring_idx = 0_u32;
    let mut part_idx = 0_usize;

    for &geom_type in vector_types {
        normalized.push(ring_idx);

        if geom_type == Point {
            // Point doesn't contribute to ring_offsets
        } else if geom_type.is_linestring() {
            // LineString contributes 1 entry to ring_offsets (its vertex count)
            ring_idx += 1;
        } else if geom_type.is_polygon() && part_idx + 1 < part_offsets.len() {
            // Polygon contributes ring_count entries (vertex count for each ring)
            let ring_count = part_offsets[part_idx + 1] - part_offsets[part_idx];
            ring_idx += ring_count;
            part_idx += 1;
        }
        // MultiPolygon is handled through geometry_offsets
    }

    normalized.push(ring_idx.min(num_rings));
    normalized
}

/// Choose between Vec2 componentwise-delta and Morton dictionary encoding.
///
/// Morton is only selected when:
/// - The coordinate range fits within 16 bits per axis (required by the spec), and
/// - The uniqueness ratio is below the threshold, meaning enough vertices are
///   repeated that the dictionary overhead is worthwhile.
pub fn select_vertex_strategy(vertices: &[i32]) -> VertexBufferType {
    const MORTON_UNIQUENESS_THRESHOLD: f64 = 0.5;

    let total = vertices.len() / 2;
    if total == 0 {
        return VertexBufferType::Vec2;
    }

    if z_order_params(vertices).is_err() {
        return VertexBufferType::Vec2;
    }

    let unique_count = vertices
        .chunks_exact(2)
        .map(|c| (c[0], c[1]))
        .collect::<std::collections::HashSet<_>>()
        .len();

    #[expect(clippy::cast_precision_loss)]
    let uniqueness_ratio = unique_count as f64 / total as f64;

    if uniqueness_ratio < MORTON_UNIQUENESS_THRESHOLD {
        VertexBufferType::Morton
    } else {
        VertexBufferType::Vec2
    }
}

/// Write a geometry `u32` stream: [`Encoder::get_int_encoder`] when explicit mode is active,
/// otherwise try all pruned candidates and keep the shortest.
///
/// Returns `1` if the stream was written, `0` if it was skipped.  Empty streams are skipped
/// unless [`Encoder::force_stream`] returns `true` for `geo_stream_name`.
fn write_geo_u32_stream(
    data: &[u32],
    stream_type: StreamType,
    geo_stream_name: &'static str,
    enc: &mut Encoder,
) -> MltResult<u8> {
    Ok(if data.is_empty() && !enc.force_stream(geo_stream_name) {
        0
    } else {
        write_u32_stream(data, stream_type, "geo", geo_stream_name, "", enc)?;
        1
    })
}

/// Like [`write_geo_u32_stream`] but for pre-logically-encoded data: delegates to
/// [`write_precomputed_u32`] instead of [`write_u32_stream`].
///
/// Returns `1` if the stream was written, `0` if skipped (empty + no force).
fn write_geo_precomputed_stream(
    data: &[u32],
    stream_type: StreamType,
    logical: LogicalEncoding,
    geo_stream_name: &'static str,
    enc: &mut Encoder,
) -> MltResult<u8> {
    Ok(if data.is_empty() && !enc.force_stream(geo_stream_name) {
        0
    } else {
        write_precomputed_u32(data, stream_type, logical, "geo", geo_stream_name, enc)?;
        1
    })
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
        let geometry_offsets = geometry_offsets.unwrap_or_default();
        let part_offsets = part_offsets.unwrap_or_default();
        let ring_offsets = ring_offsets.unwrap_or_default();
        let index_buffer = index_buffer.unwrap_or_default();
        let triangles = triangles.unwrap_or_default();
        let vertices = vertices.unwrap_or_default();

        let vertex_buffer_type = enc
            .override_vertex_buffer_type()
            .unwrap_or_else(|| select_vertex_strategy(&vertices));

        let meta: Vec<u32> = vector_types.iter().map(|t| *t as u32).collect();

        // Normalize part_offsets when there are no geometry offsets but ring offsets exist.
        let normalized_parts = if geometry_offsets.is_empty()
            && !ring_offsets.is_empty()
            && !part_offsets.is_empty()
        {
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
        write_u32_stream(&meta, typ, "geo", "meta", "", enc)?;
        n += 1;

        // Topology: compute each length stream and write it immediately.
        if !geometry_offsets.is_empty() {
            let geom_offs = normalize_geometry_offsets(&vector_types, &geometry_offsets);
            let lengths = encode_root_length_stream(&vector_types, &geom_offs, Polygon);
            let typ = StreamType::Length(LengthType::Geometries);
            n += write_geo_u32_stream(&lengths, typ, "geometries", enc)?;

            if !normalized_parts.is_empty() {
                if ring_offsets.is_empty() {
                    // geom → parts only (no rings).
                    let pl = encode_level1_without_ring_buffer_length_stream(
                        &vector_types,
                        &geom_offs,
                        &normalized_parts,
                    );
                    let typ = StreamType::Length(LengthType::Parts);
                    n += write_geo_u32_stream(&pl, typ, "no_rings", enc)?;
                } else {
                    // Full topology: geom → parts → rings.
                    // LineStrings contribute to rings here, not to parts.
                    let pl = encode_level1_length_stream(
                        &vector_types,
                        &geom_offs,
                        &normalized_parts,
                        false,
                    );
                    let typ = StreamType::Length(LengthType::Parts);
                    n += write_geo_u32_stream(&pl, typ, "rings", enc)?;

                    let rl = encode_level2_length_stream(
                        &vector_types,
                        &geom_offs,
                        &normalized_parts,
                        &ring_offsets,
                    );
                    let typ = StreamType::Length(LengthType::Rings);
                    n += write_geo_u32_stream(&rl, typ, "rings2", enc)?;
                }
            }
        } else if !normalized_parts.is_empty() {
            if ring_offsets.is_empty() {
                let lengths = encode_root_length_stream(&vector_types, &normalized_parts, Point);
                let typ = StreamType::Length(LengthType::Parts);
                n += write_geo_u32_stream(&lengths, typ, "no_rings", enc)?;
            } else {
                // No Multi* types; parts → rings (Polygon / mixed Point+Polygon).
                // Java writes an empty GEOMETRIES stream here for tessellated polygons; only do
                // so when explicitly forced (e.g. to preserve byte-for-byte Java compatibility).
                let typ = StreamType::Length(LengthType::Geometries);
                n += write_geo_u32_stream(&[], typ, "geometries", enc)?;

                let pl = encode_root_length_stream(&vector_types, &normalized_parts, LineString);
                let typ = StreamType::Length(LengthType::Parts);
                n += write_geo_u32_stream(&pl, typ, "parts", enc)?;

                // part_offs is a dense N+1 array (one slot per geometry incl. Points);
                // ring_offs stores vertex offsets per slot.  The dense-aware helper skips
                // Point slots by index rather than a running counter.
                let rl = encode_ring_lengths_for_mixed(
                    &vector_types,
                    &normalized_parts,
                    &ring_offsets,
                    vector_types
                        .iter()
                        .copied()
                        .any(GeometryType::is_linestring),
                );
                let typ = StreamType::Length(LengthType::Rings);
                n += write_geo_u32_stream(&rl, typ, "parts_ring", enc)?;
            }
        }

        let typ = StreamType::Length(LengthType::Triangles);
        n += write_geo_u32_stream(&triangles, typ, "triangles", enc)?;
        let typ = StreamType::Offset(OffsetType::Index);
        n += write_geo_u32_stream(&index_buffer, typ, "triangles_indexes", enc)?;

        // Vertex streams — compute and write inline.
        // write_geo_precomputed_stream skips writing when the input is empty (no vertices).
        match vertex_buffer_type {
            VertexBufferType::Vec2 => {
                let delta = encode_componentwise_delta_vec2s(&vertices);
                let typ = StreamType::Data(DictionaryType::Vertex);
                let logical = LogicalEncoding::ComponentwiseDelta;
                n += write_geo_precomputed_stream(&delta, typ, logical, "vertex", enc)?;
            }
            VertexBufferType::Morton => {
                let morton_meta = z_order_params(&vertices)?;
                let (dict, offsets) = build_morton_dict(&vertices, morton_meta)?;
                let typ = StreamType::Offset(OffsetType::Vertex);
                n += write_geo_u32_stream(&offsets, typ, "vertex_offsets", enc)?;
                let deltas = morton_deltas(&dict);
                let typ = StreamType::Data(DictionaryType::Morton);
                let logical = LogicalEncoding::MortonDelta(morton_meta);
                n += write_geo_precomputed_stream(&deltas, typ, logical, "vertex", enc)?;
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
