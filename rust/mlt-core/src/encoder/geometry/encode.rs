use std::collections::BTreeSet;

use integer_encoding::VarIntWriter as _;

use super::model::VertexBufferType;
use crate::MltResult;
use crate::codecs::morton::{encode_morton, morton_deltas, z_order_params};
use crate::codecs::zigzag::encode_componentwise_delta_vec2s;
use crate::decoder::{
    ColumnType, DictionaryType, GeometryType, GeometryValues, IntEncoding, LengthType,
    LogicalEncoding, MortonMeta, OffsetType, StreamMeta, StreamType,
};
#[cfg(feature = "__private")]
use crate::encoder::optimizer::ExplicitEncoder;
use crate::encoder::stream::{DataProfile, PhysicalEncoder};
use crate::encoder::{EncodedStream, Encoder};
use crate::errors::AsMltError as _;
use crate::utils::{AsUsize as _, BinarySerializer as _, checked_sum2};

/// Encode pre-computed componentwise-delta vertex values with a given physical encoder.
///
/// Shared by both the auto path (which loops over candidates) and the explicit `__private` path.
fn encode_vertex_delta_stream(
    delta: &[u32],
    physical: PhysicalEncoder,
) -> MltResult<EncodedStream> {
    let num_values = u32::try_from(delta.len())?;
    let (data, physical_encoding) = physical.encode_u32s(delta.to_vec())?;
    Ok(EncodedStream {
        meta: StreamMeta::new(
            StreamType::Data(DictionaryType::Vertex),
            IntEncoding::new(LogicalEncoding::ComponentwiseDelta, physical_encoding),
            num_values,
        ),
        data,
    })
}

/// Encode raw vertex data: applies componentwise delta, then calls [`encode_vertex_delta_stream`].
#[cfg(feature = "__private")]
fn encode_vertex_buffer(vertices: &[i32], physical: PhysicalEncoder) -> MltResult<EncodedStream> {
    let delta = encode_componentwise_delta_vec2s(vertices);
    encode_vertex_delta_stream(&delta, physical)
}

/// Encode pre-computed Morton delta values with a given physical encoder.
///
/// Shared by both the auto path and the explicit `__private` path.
fn encode_morton_delta_stream(
    deltas: Vec<u32>,
    meta: MortonMeta,
    physical: PhysicalEncoder,
) -> MltResult<EncodedStream> {
    let num_values = u32::try_from(deltas.len())?;
    let (data, physical_encoding) = physical.encode_u32s(deltas)?;
    Ok(EncodedStream {
        meta: StreamMeta::new(
            StreamType::Data(DictionaryType::Morton),
            IntEncoding::new(LogicalEncoding::MortonDelta(meta), physical_encoding),
            num_values,
        ),
        data,
    })
}

/// Encode a Morton vertex dictionary: computes deltas, then calls [`encode_morton_delta_stream`].
#[cfg(feature = "__private")]
fn encode_morton_vertex_buffer(
    codes: &[u32],
    meta: MortonMeta,
    physical: PhysicalEncoder,
) -> MltResult<EncodedStream> {
    encode_morton_delta_stream(morton_deltas(codes), meta, physical)
}

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

        if geom_type == GeometryType::Point {
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

/// Auto-encode a stream with `start_alternative`/`finish_alternatives`, trying all
/// pruned `IntEncoder` candidates and keeping the shortest encoding.
fn write_stream_auto(data: &[u32], stream_type: StreamType, enc: &mut Encoder) -> MltResult<()> {
    let candidates = DataProfile::prune_candidates::<i32>(data);
    for &cand in &candidates {
        enc.start_alternative();
        enc.write_stream(&EncodedStream::encode_u32s_of_type(
            data,
            cand,
            stream_type,
        )?)?;
    }
    enc.finish_alternatives();
    Ok(())
}

/// Internal representation of pre-computed geometry topology payloads.
///
/// Each element of `topology` is `(stream_type, raw_u32_payload)` in the
/// order they must be written.  `vertex_buffer_type` is determined by
/// `select_vertex_strategy` for the auto path.
pub struct GeometryPayloads {
    pub meta: Vec<u32>,
    pub topology: Vec<(StreamType, Vec<u32>)>,
    pub vertex_buffer_type: VertexBufferType,
    // Vertex raw payloads
    pub vertex_vec2_delta: Option<Vec<u32>>,
    pub morton_offsets: Option<Vec<u32>>,
    pub morton_dict: Option<(Vec<u32>, MortonMeta)>, // (dict_u32s, meta)
}

/// Compute all topology payloads (raw u32 arrays) for a `GeometryValues` without
/// applying any integer codec.  Both the auto path and the `__private` explicit path
/// use this to share topology-computation logic.
pub fn compute_geometry_payloads(
    decoded: &GeometryValues,
    vertex_buffer_type: VertexBufferType,
) -> MltResult<GeometryPayloads> {
    let GeometryValues {
        vector_types,
        geometry_offsets,
        part_offsets,
        ring_offsets,
        index_buffer,
        triangles,
        vertices,
    } = decoded;

    // Normalize offsets (same logic as encode_geometry).
    let normalized_parts = if geometry_offsets.is_none() && ring_offsets.is_some() {
        if let (Some(part_offs), Some(ring_offs)) = (part_offsets, ring_offsets) {
            Some(normalize_part_offsets_for_rings(
                vector_types,
                part_offs,
                ring_offs,
            ))
        } else {
            part_offsets.clone()
        }
    } else {
        part_offsets.clone()
    };
    let part_offsets = &normalized_parts;

    // Normalize geometry_offsets for mixed geometry types
    let normalized_geom_offs = geometry_offsets
        .as_ref()
        .map(|g| normalize_geometry_offsets(vector_types, g));
    let geometry_offsets = &normalized_geom_offs;

    let meta: Vec<u32> = vector_types.iter().map(|t| *t as u32).collect();

    let has_linestrings = vector_types
        .iter()
        .copied()
        .any(GeometryType::is_linestring);
    let has_tessellation = triangles.is_some();

    let mut topology: Vec<(StreamType, Vec<u32>)> = Vec::new();

    if let Some(geom_offs) = geometry_offsets {
        let lengths = encode_root_length_stream(vector_types, geom_offs, GeometryType::Polygon);
        if !lengths.is_empty() || has_tessellation {
            topology.push((StreamType::Length(LengthType::Geometries), lengths));
        }

        if let Some(part_offs) = part_offsets {
            if let Some(ring_offs) = ring_offsets {
                // Full topology: geom -> parts -> rings
                // When rings are present (Polygon in layer), LineStrings contribute to rings, not parts.
                // So is_line_string_present should be false for the parts stream.
                let part_lengths =
                    encode_level1_length_stream(vector_types, geom_offs, part_offs, false);
                if !part_lengths.is_empty() {
                    topology.push((StreamType::Length(LengthType::Parts), part_lengths));
                }
                let ring_lengths =
                    encode_level2_length_stream(vector_types, geom_offs, part_offs, ring_offs);
                if !ring_lengths.is_empty() {
                    topology.push((StreamType::Length(LengthType::Rings), ring_lengths));
                }
            } else {
                // Only geom -> parts (no rings)
                let part_lengths = encode_level1_without_ring_buffer_length_stream(
                    vector_types,
                    geom_offs,
                    part_offs,
                );
                if !part_lengths.is_empty() {
                    topology.push((StreamType::Length(LengthType::Parts), part_lengths));
                }
            }
        }
    } else if let Some(part_offs) = part_offsets {
        // No geometry offsets (no Multi* types), encode from parts
        if let Some(ring_offs) = ring_offsets {
            // parts -> rings (e.g., Polygon without Multi, or mixed Point + Polygon)

            // For tessellated polygons with outlines, Java includes an empty geometries stream
            if has_tessellation {
                topology.push((StreamType::Length(LengthType::Geometries), vec![]));
            }
            let part_lengths =
                encode_root_length_stream(vector_types, part_offs, GeometryType::LineString);
            if !part_lengths.is_empty() {
                topology.push((StreamType::Length(LengthType::Parts), part_lengths));
            }
            // Ring lengths: part_offs is a dense N+1 array (one slot per geometry,
            // including Points).  ring_offs stores vertex offsets for each slot.
            // Use the dense-aware helper so Point slots are correctly skipped by
            // index rather than by a running counter that ignores non-length types.
            let ring_lengths =
                encode_ring_lengths_for_mixed(vector_types, part_offs, ring_offs, has_linestrings);
            if !ring_lengths.is_empty() {
                topology.push((StreamType::Length(LengthType::Rings), ring_lengths));
            }
        } else {
            let lengths = encode_root_length_stream(vector_types, part_offs, GeometryType::Point);
            if !lengths.is_empty() {
                topology.push((StreamType::Length(LengthType::Parts), lengths));
            }
        }
    }

    // Encode triangles stream if present (for pre-tessellated polygons)
    if let Some(tris) = triangles {
        topology.push((StreamType::Length(LengthType::Triangles), tris.clone()));
    }
    if let Some(idx_buf) = index_buffer {
        topology.push((StreamType::Offset(OffsetType::Index), idx_buf.clone()));
    }

    // Vertex payloads
    let (vertex_vec2_delta, morton_offsets, morton_dict) = match (vertices, vertex_buffer_type) {
        (Some(verts), VertexBufferType::Vec2) => {
            let delta = encode_componentwise_delta_vec2s(verts);
            (Some(delta), None, None)
        }
        (Some(verts), VertexBufferType::Morton) => {
            let morton_meta = z_order_params(verts)?;
            let (dict, offsets) = build_morton_dict(verts, morton_meta)?;
            (None, Some(offsets), Some((dict, morton_meta)))
        }
        (None, _) => (None, None, None),
    };

    Ok(GeometryPayloads {
        meta,
        topology,
        vertex_buffer_type,
        vertex_vec2_delta,
        morton_offsets,
        morton_dict,
    })
}

/// Automatically encode geometry using `start_alternative`/`finish_alternatives` per stream
/// to select the shortest encoding for each stream independently.
pub fn write_geometry_auto(payloads: &GeometryPayloads, enc: &mut Encoder) -> MltResult<()> {
    let vertex_stream_count = match payloads.vertex_buffer_type {
        _ if payloads.vertex_vec2_delta.is_none() && payloads.morton_offsets.is_none() => 0,
        VertexBufferType::Vec2 => 1,
        VertexBufferType::Morton => 2,
    };
    let stream_count = checked_sum2(
        u32::try_from(1 + payloads.topology.len() + vertex_stream_count)?,
        0,
    )?;

    ColumnType::Geometry.write_to(&mut enc.meta)?;
    enc.write_varint(stream_count)?;

    // Meta stream (geometry types).
    write_stream_auto(
        &payloads.meta,
        StreamType::Length(LengthType::VarBinary),
        enc,
    )?;

    // Topology streams.
    for (stream_type, data) in &payloads.topology {
        write_stream_auto(data, *stream_type, enc)?;
    }

    // Vertex stream(s).
    if let Some(delta) = &payloads.vertex_vec2_delta {
        let candidates = DataProfile::prune_candidates::<i32>(delta);
        for &cand in &candidates {
            enc.start_alternative();
            enc.write_stream(&encode_vertex_delta_stream(delta, cand.physical)?)?;
        }
        enc.finish_alternatives();
    } else if let (Some(offsets), Some((dict, morton_meta))) =
        (&payloads.morton_offsets, &payloads.morton_dict)
    {
        // Morton vertex offsets stream.
        write_stream_auto(offsets, StreamType::Offset(OffsetType::Vertex), enc)?;

        // Morton dict (delta-encoded) stream.
        let dict_deltas = morton_deltas(dict);
        let candidates = DataProfile::prune_candidates::<i32>(&dict_deltas);
        for &cand in &candidates {
            enc.start_alternative();
            enc.write_stream(&encode_morton_delta_stream(
                dict_deltas.clone(),
                *morton_meta,
                cand.physical,
            )?)?;
        }
        enc.finish_alternatives();
    }

    Ok(())
}

/// Encode geometry with explicit per-stream encoders (synthetics / `__private` path).
///
/// Stream names passed to [`ExplicitEncoder::get_int_encoder`] with kind `"geo"` are:
/// `"meta"`, `"geometries"`, `"rings"`, `"rings2"`, `"no_rings"`, `"parts"`,
/// `"parts_ring"`, `"triangles"`, `"triangles_indexes"`,
/// `"vertex"`, `"vertex_offsets"`.
///
/// Writes the `Geometry` column-type byte to [`enc.meta`](Encoder::meta) and the
/// stream count + all geometry streams to [`enc.data`](Encoder::data).
#[cfg(feature = "__private")]
pub(crate) fn encode_geometry(
    decoded: &GeometryValues,
    cfg: &ExplicitEncoder,
    enc: &mut Encoder,
) -> MltResult<()> {
    let get_enc = |name: &str| (cfg.get_int_encoder)("geo", name, None);
    let payloads = compute_geometry_payloads(decoded, cfg.vertex_buffer_type)?;

    // Determine which topology branch fired (for stream-name lookup).
    let has_geom_offs = decoded.geometry_offsets.is_some();
    let has_ring_offs = decoded.ring_offsets.is_some();
    let is_part_with_rings = has_geom_offs && has_ring_offs;
    let is_ring_level2 = !has_geom_offs && has_ring_offs;

    let vertex_stream_count = match cfg.vertex_buffer_type {
        _ if decoded.vertices.is_none() => 0,
        VertexBufferType::Vec2 => 1,
        VertexBufferType::Morton => 2,
    };
    let stream_count = checked_sum2(
        u32::try_from(1 + payloads.topology.len() + vertex_stream_count)?,
        0,
    )?;

    ColumnType::Geometry.write_to(&mut enc.meta)?;
    enc.write_varint(stream_count)?;

    // Meta stream.
    enc.write_stream(&EncodedStream::encode_u32s_of_type(
        &payloads.meta,
        get_enc("meta"),
        StreamType::Length(LengthType::VarBinary),
    )?)?;

    // Topology streams: map StreamType → stream name → IntEncoder.
    for (stream_type, data) in &payloads.topology {
        let name = match stream_type {
            StreamType::Length(LengthType::Geometries) => "geometries",
            StreamType::Length(LengthType::Parts) => {
                if is_part_with_rings {
                    "rings"
                } else if is_ring_level2 {
                    "parts"
                } else {
                    "no_rings"
                }
            }
            StreamType::Length(LengthType::Rings) => {
                if is_part_with_rings {
                    "rings2"
                } else {
                    "parts_ring"
                }
            }
            StreamType::Length(LengthType::Triangles) => "triangles",
            StreamType::Offset(OffsetType::Index) => "triangles_indexes",
            _ => "meta",
        };
        enc.write_stream(&EncodedStream::encode_u32s_of_type(
            data,
            get_enc(name),
            *stream_type,
        )?)?;
    }

    // Vertex streams.
    if payloads.vertex_vec2_delta.is_some() {
        enc.write_stream(&encode_vertex_buffer(
            decoded.vertices.as_deref().unwrap_or(&[]),
            get_enc("vertex").physical,
        )?)?;
    } else if let (Some(offsets), Some((dict, morton_meta))) =
        (&payloads.morton_offsets, &payloads.morton_dict)
    {
        enc.write_stream(&EncodedStream::encode_u32s_of_type(
            offsets,
            get_enc("vertex_offsets"),
            StreamType::Offset(OffsetType::Vertex),
        )?)?;
        enc.write_stream(&encode_morton_vertex_buffer(
            dict,
            *morton_meta,
            get_enc("vertex").physical,
        )?)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_root_length_stream() {
        // Single Polygon geometry (no Multi)
        let types = vec![GeometryType::Polygon];
        let offsets = vec![0, 1]; // One polygon

        let lengths = encode_root_length_stream(&types, &offsets, GeometryType::Polygon);
        // Polygon == buffer_id, so no length encoded
        assert!(lengths.is_empty());

        // MultiPolygon needs length encoded
        let types = vec![GeometryType::MultiPolygon];
        let offsets = vec![0, 2]; // MultiPolygon with 2 polygons

        let lengths = encode_root_length_stream(&types, &offsets, GeometryType::Polygon);
        assert_eq!(lengths, vec![2]);
    }
}
