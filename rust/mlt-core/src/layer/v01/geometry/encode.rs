use std::collections::BTreeSet;

use super::{DecodedGeometry, OwnedEncodedGeometry, VertexBufferType};
use crate::MltError;
use crate::utils::encode_componentwise_delta_vec2s;
use crate::v01::LengthType::VarBinary;
use crate::v01::{
    DictionaryType, Encoder, GeometryType, LengthType, LogicalEncoding, MortonMeta, OffsetType,
    OwnedStream, PhysicalEncoder, StreamMeta, StreamType,
};

/// Encode vertex buffer using componentwise delta encoding
fn encode_vertex_buffer(
    vertices: &[i32],
    physical: PhysicalEncoder,
) -> Result<OwnedStream, MltError> {
    // Componentwise delta encoding: delta X and Y separately
    let physical_u32 = encode_componentwise_delta_vec2s(vertices);
    let num_values = u32::try_from(physical_u32.len())?;
    let (data, physical_encoding) = physical.encode_u32s(physical_u32)?;
    Ok(OwnedStream {
        meta: StreamMeta::new(
            StreamType::Data(DictionaryType::Vertex),
            LogicalEncoding::ComponentwiseDelta,
            physical_encoding,
            num_values,
        ),
        data,
    })
}

/// Encode a Morton vertex dictionary stream.
///
/// `codes` must be the sorted unique Morton codes for the dictionary.
/// They are delta-encoded before physical encoding.
fn encode_morton_vertex_buffer(
    codes: &[u32],
    meta: MortonMeta,
    physical: PhysicalEncoder,
) -> Result<OwnedStream, MltError> {
    let deltas: Vec<u32> = std::iter::once(codes[0])
        .chain(codes.windows(2).map(|w| w[1] - w[0]))
        .collect();
    let num_values = u32::try_from(deltas.len())?;
    let (data, physical_encoding) = physical.encode_u32s(deltas)?;
    Ok(OwnedStream {
        meta: StreamMeta::new(
            StreamType::Data(DictionaryType::Morton),
            LogicalEncoding::MortonDelta(meta),
            physical_encoding,
            num_values,
        ),
        data,
    })
}

/// Compute `ZOrderCurve` parameters from the vertex value range.
///
/// Returns `(num_bits, coordinate_shift)` matching Java's `SpaceFillingCurve`.
fn zorder_params(vertices: &[i32]) -> Result<(u32, u32), MltError> {
    let min_v = vertices.iter().copied().min().unwrap_or(0);
    let max_v = vertices.iter().copied().max().unwrap_or(0);
    let coordinate_shift: u32 = if min_v < 0 { min_v.unsigned_abs() } else { 0 };
    let tile_extent = i64::from(max_v) + i64::from(coordinate_shift);
    let num_bits = if tile_extent <= 0 {
        0u32
    } else {
        // ceil(log2(tile_extent + 1)), matching Java's Math.ceil(Math.log(...) / Math.log(2)).
        // Computed with integer arithmetic: for te >= 1, this equals `u64::BITS - te.leading_zeros()`.
        // Capped at 16: Morton codes are u32, so each axis may use at most 16 bits.
        let te = u64::try_from(tile_extent)?;
        (u64::BITS - te.leading_zeros()).min(16)
    };
    Ok((num_bits, coordinate_shift))
}

/// Encode a single `(x, y)` pair to its Z-order (Morton) code.
fn morton_encode(x: i32, y: i32, num_bits: u32, coordinate_shift: u32) -> Result<u32, MltError> {
    let sx = u32::try_from(i64::from(x) + i64::from(coordinate_shift))?;
    let sy = u32::try_from(i64::from(y) + i64::from(coordinate_shift))?;
    let mut code = 0u32;
    for i in 0..num_bits {
        // num_bits is capped at 16, so 2*i+1 <= 31 => no shift overflow possible.
        code |= ((sx >> i) & 1) << (2 * i);
        code |= ((sy >> i) & 1) << (2 * i + 1);
    }
    Ok(code)
}

/// Build a sorted unique Morton dictionary and per-vertex offset indices from a flat
/// `[x0, y0, x1, y1, …]` vertex slice.
///
/// Returns `(sorted_unique_codes, per_vertex_offsets)`.
fn build_morton_dict(
    vertices: &[i32],
    num_bits: u32,
    coordinate_shift: u32,
) -> Result<(Vec<u32>, Vec<u32>), MltError> {
    let codes: Vec<u32> = vertices
        .chunks_exact(2)
        .map(|c| morton_encode(c[0], c[1], num_bits, coordinate_shift))
        .collect::<Result<_, _>>()?;

    let dict: Vec<u32> = codes
        .iter()
        .copied()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();

    let offsets: Vec<u32> = codes
        .iter()
        .map(|&code| {
            u32::try_from(dict.partition_point(|&c| c < code))
                .map_err(|_| MltError::IntegerOverflow)
        })
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
/// This is the inverse of `decode_level1_length_stream`.
///
/// The `geometry_offsets` array is expected to be an N+1 element array for N geometries.
/// For mixed types, it should be normalized first.
fn encode_level1_length_stream(
    geometry_types: &[GeometryType],
    geometry_offsets: &[u32],
    part_offsets: &[u32],
    is_line_string_present: bool,
) -> Vec<u32> {
    let mut lengths = Vec::new();
    let mut part_idx = 0;

    for (i, &geom_type) in geometry_types.iter().enumerate() {
        let num_geoms = (geometry_offsets[i + 1] - geometry_offsets[i]) as usize;

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
        // Note: Point/MultiPoint don't contribute to part_offsets, so don't advance part_idx
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
        let num_geoms = (geometry_offsets[i + 1] - geometry_offsets[i]) as usize;

        // Only Polygon and MultiPolygon have ring data in level 2
        // LineStrings with Polygon present add their vertex counts directly to ring_offsets
        // but they don't have parts (ring count per linestring is always 1 implicitly)
        if geom_type.is_polygon() {
            // Polygon/MultiPolygon: iterate through sub-polygons, each has parts (ring counts)
            for _ in 0..num_geoms {
                let num_parts = (part_offsets[part_idx + 1] - part_offsets[part_idx]) as usize;
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
/// This is the inverse of `decode_level1_without_ring_buffer_length_stream`.
///
/// The `geometry_offsets` array is expected to be an N+1 element array for N geometries.
fn encode_level1_without_ring_buffer_length_stream(
    geometry_types: &[GeometryType],
    geometry_offsets: &[u32],
    part_offsets: &[u32],
) -> Vec<u32> {
    let mut lengths = Vec::new();
    let mut part_idx = 0;

    for (i, &geom_type) in geometry_types.iter().enumerate() {
        let num_geoms = (geometry_offsets[i + 1] - geometry_offsets[i]) as usize;

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

/// Normalize offset arrays to be indexed by geometry type.
/// When building `DecodedGeometry` via `push_*` methods, offset arrays may not include
/// entries for geometry types that don't need them (e.g., Points don't contribute to `part_offsets`).
/// This function expands the offset arrays to have one entry per geometry type plus a trailing entry.
///
/// For `part_offsets` without rings (Point + `LineString`):
///   - `part_offsets` tracks vertex ranges
///   - Points contribute 1 vertex each (implicit)
///   - `LineString` values contribute their vertex count from the sparse array
///
/// For `part_offsets` with rings (Point + Polygon, `LineString` + Polygon):
///   - `part_offsets` tracks ring indices
///   - Points and `LineString` values contribute 0 rings each (implicit)
///   - Polygons contribute their ring count from the sparse array
fn normalize_part_offsets_for_vertices(
    vector_types: &[GeometryType],
    part_offsets: &[u32],
    vertices: &[i32],
) -> Vec<u32> {
    let num_vertices = u32::try_from(vertices.len() / 2).expect("vertex count overflow");

    // Check if already normalized (has N+1 entries for N geometry types)
    if part_offsets.len() == vector_types.len() + 1 {
        return part_offsets.to_vec();
    }

    // Build normalized offset array for vertex-based indexing
    let mut normalized = Vec::with_capacity(vector_types.len() + 1);
    let mut vertex_idx = 0_u32;
    let mut part_idx = 0_usize;

    for &geom_type in vector_types {
        normalized.push(vertex_idx);

        if geom_type == GeometryType::Point {
            // Point always consumes exactly 1 vertex
            vertex_idx += 1;
        } else if geom_type == GeometryType::LineString {
            // LineString consumes vertices tracked in part_offsets
            if part_idx + 1 < part_offsets.len() {
                let len = part_offsets[part_idx + 1] - part_offsets[part_idx];
                vertex_idx += len;
                part_idx += 1;
            }
        }
        // Polygon/MultiPolygon are handled through ring_offsets
    }

    normalized.push(vertex_idx.min(num_vertices));
    normalized
}

/// Normalize `geometry_offsets` for mixed geometry types.
/// When only Multi* geometries contribute to `geometry_offsets`, this expands it to have
/// one entry per geometry plus a trailing entry.
///
/// The sparse `geometry_offsets` is a cumulative count array for Multi* types only:
///   `[0, count_of_first_multi, count_of_first_multi + count_of_second_multi, ...]`
///
/// Non-Multi types are not represented in `geometry_offsets` but each has an implicit
/// count of 1. The normalized output will have N+1 entries for N geometry types.
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

/// Create a synthetic `geometry_offsets` array where each geometry has exactly 1 sub-geometry.
/// This is used when there are no Multi* types, but we still need a dense offset array
/// for encoding functions that expect one.
fn create_unit_geometry_offsets(vector_types: &[GeometryType]) -> Vec<u32> {
    let size = u32::try_from(vector_types.len() + 1).expect("geometry count overflow");
    let mut offsets = Vec::with_capacity(size as usize);
    for i in 0..size {
        offsets.push(i);
    }
    offsets
}

/// Normalize `part_offsets` for ring-based indexing (Polygon mixed with `Point`/`LineString`).
/// Returns an offset array where `offset[i+1]` - `offset[i]` = number of ring length entries
/// that geometry i contributes to the rings stream.
///
/// When a Polygon is present:
/// - Point: 0 entries (doesn't contribute to rings)
/// - `LineString`: 1 entry (its vertex count goes to rings)
/// - Polygon: `ring_count` entries (each ring's vertex count)
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
        } else if geom_type.is_polygon() {
            // Polygon contributes ring_count entries (vertex count for each ring)
            if part_idx + 1 < part_offsets.len() {
                let ring_count = part_offsets[part_idx + 1] - part_offsets[part_idx];
                ring_idx += ring_count;
                part_idx += 1;
            }
        }
        // MultiPolygon is handled through geometry_offsets
    }

    normalized.push(ring_idx.min(num_rings));
    normalized
}

/// Main geometry encoding function
pub fn encode_geometry(
    decoded: &DecodedGeometry,
    config: &GeometryEncoder,
) -> Result<OwnedEncodedGeometry, MltError> {
    let DecodedGeometry {
        vector_types,
        geometry_offsets,
        part_offsets,
        ring_offsets,
        index_buffer,
        triangles,
        vertices,
    } = decoded;

    // Normalize part_offsets if needed for mixed geometry types with rings
    // When we have rings, we need to expand part_offsets to have N+1 entries
    // When we don't have rings, part_offsets is already in the correct sparse format
    let normalized_parts = if geometry_offsets.is_none() && ring_offsets.is_some() {
        if let Some(part_offs) = part_offsets {
            if let Some(ring_offs) = ring_offsets {
                // Mixed with rings (e.g., Point + Polygon) - normalize for ring-based indexing
                Some(normalize_part_offsets_for_rings(
                    vector_types,
                    part_offs,
                    ring_offs,
                ))
            } else {
                part_offsets.clone()
            }
        } else {
            None
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

    // Encode geometry types (meta stream)
    let meta = {
        let vector_types_u32: Vec<u32> = vector_types.iter().map(|t| *t as u32).collect();
        OwnedStream::encode_u32s_of_type(
            &vector_types_u32,
            config.meta,
            StreamType::Length(VarBinary),
        )?
    };

    let mut items = Vec::new();
    let has_linestrings = vector_types
        .iter()
        .copied()
        .any(GeometryType::is_linestring);
    let has_tessellation = triangles.is_some();

    // Encode topology streams based on geometry structure
    if let Some(geom_offs) = geometry_offsets {
        // Encode geometry lengths (NumGeometries)
        let lengths = encode_root_length_stream(vector_types, geom_offs, GeometryType::Polygon);
        if !lengths.is_empty() || has_tessellation {
            // For tessellated polygons with outlines, Java always includes this stream
            // even when empty
            items.push(OwnedStream::encode_u32s_of_type(
                &lengths,
                config.geometries,
                StreamType::Length(LengthType::Geometries),
            )?);
        }

        if let Some(part_offs) = part_offsets {
            if let Some(ring_offs) = ring_offsets {
                // Full topology: geom -> parts -> rings
                // When rings are present (Polygon in layer), LineStrings contribute to rings, not parts.
                // So is_line_string_present should be false for the parts stream.
                let part_lengths = encode_level1_length_stream(
                    vector_types,
                    geom_offs,
                    part_offs,
                    false, // LineStrings contribute to rings, not parts
                );
                if !part_lengths.is_empty() {
                    items.push(OwnedStream::encode_u32s_of_type(
                        &part_lengths,
                        config.rings,
                        StreamType::Length(LengthType::Parts),
                    )?);
                }

                let ring_lengths =
                    encode_level2_length_stream(vector_types, geom_offs, part_offs, ring_offs);
                if !ring_lengths.is_empty() {
                    items.push(OwnedStream::encode_u32s_of_type(
                        &ring_lengths,
                        config.rings2,
                        StreamType::Length(LengthType::Rings),
                    )?);
                }
            } else {
                // Only geom -> parts (no rings)
                let part_lengths = encode_level1_without_ring_buffer_length_stream(
                    vector_types,
                    geom_offs,
                    part_offs,
                );
                if !part_lengths.is_empty() {
                    items.push(OwnedStream::encode_u32s_of_type(
                        &part_lengths,
                        config.no_rings,
                        StreamType::Length(LengthType::Parts),
                    )?);
                }
            }
        }
    } else if let Some(part_offs) = part_offsets {
        // No geometry offsets (no Multi* types), encode from parts
        if let Some(ring_offs) = ring_offsets {
            // parts -> rings (e.g., Polygon without Multi, or mixed Point + Polygon)

            // For tessellated polygons with outlines, Java includes an empty geometries stream
            if has_tessellation {
                items.push(OwnedStream::encode_u32s_of_type(
                    &[],
                    config.geometries,
                    StreamType::Length(LengthType::Geometries),
                )?);
            }

            let part_lengths =
                encode_root_length_stream(vector_types, part_offs, GeometryType::LineString);
            if !part_lengths.is_empty() {
                items.push(OwnedStream::encode_u32s_of_type(
                    &part_lengths,
                    config.parts,
                    StreamType::Length(LengthType::Parts),
                )?);
            }

            // Ring lengths: part_offs is normalized to have N+1 entries where
            // part_offs[i+1] - part_offs[i] = number of rings for geometry i.
            // For Point/LineString this is 0, for Polygon it's the actual ring count.
            let ring_lengths =
                encode_level1_length_stream(vector_types, part_offs, ring_offs, has_linestrings);
            if !ring_lengths.is_empty() {
                items.push(OwnedStream::encode_u32s_of_type(
                    &ring_lengths,
                    config.parts_ring,
                    StreamType::Length(LengthType::Rings),
                )?);
            }
        } else {
            // Only parts (e.g., LineString)
            let lengths = encode_root_length_stream(vector_types, part_offs, GeometryType::Point);
            if !lengths.is_empty() {
                items.push(OwnedStream::encode_u32s_of_type(
                    &lengths,
                    config.only_parts,
                    StreamType::Length(LengthType::Parts),
                )?);
            }
        }
    }

    // Encode triangles stream if present (for pre-tessellated polygons)
    if let Some(tris) = triangles {
        items.push(OwnedStream::encode_u32s_of_type(
            tris,
            config.triangles,
            StreamType::Length(LengthType::Triangles),
        )?);
    }

    // Encode index buffer if present (for pre-tessellated polygons)
    if let Some(idx_buf) = index_buffer {
        items.push(OwnedStream::encode_u32s_of_type(
            idx_buf,
            config.triangles_indexes,
            StreamType::Offset(OffsetType::Index),
        )?);
    }

    // Encode vertex buffer (and dictionary offsets when Morton encoding is active)
    if let Some(verts) = vertices {
        match config.vertex_buffer_type {
            VertexBufferType::Vec2 => {
                items.push(encode_vertex_buffer(verts, config.vertex.physical)?);
            }
            VertexBufferType::Morton => {
                let (num_bits, coordinate_shift) = zorder_params(verts)?;
                let (dict, offsets) = build_morton_dict(verts, num_bits, coordinate_shift)?;
                let morton_meta = MortonMeta {
                    num_bits,
                    coordinate_shift,
                };

                items.push(OwnedStream::encode_u32s_of_type(
                    &offsets,
                    config.vertex_offsets,
                    StreamType::Offset(OffsetType::Vertex),
                )?);
                items.push(encode_morton_vertex_buffer(
                    &dict,
                    morton_meta,
                    config.vertex.physical,
                )?);
            }
        }
    }

    Ok(OwnedEncodedGeometry { meta, items })
}

/// How to encode Geometry
#[derive(Debug, Clone, Copy)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
#[cfg_attr(all(not(test), feature = "arbitrary"), derive(arbitrary::Arbitrary))]
pub struct GeometryEncoder {
    /// Encoding settings for the geometry types (meta) stream.
    pub meta: Encoder,

    /// Encoding for the geometry length stream.
    pub geometries: Encoder,

    /// Encoding for parts length stream when rings are present.
    pub rings: Encoder,
    /// Encoding for ring vertex-count stream.
    pub rings2: Encoder,
    /// Encoding for parts length stream when rings are not present.
    pub no_rings: Encoder,

    /// Encoding for parts length stream (with rings) when `geometry_offsets` absent.
    pub parts: Encoder,
    /// Encoding for ring lengths when `geometry_offsets` absent.
    pub parts_ring: Encoder,

    /// Encoding for parts-only stream (e.g. `LineString`, no rings).
    pub only_parts: Encoder,

    /// Encoding for triangles count stream (pre-tessellated polygons).
    pub triangles: Encoder,
    /// Encoding for triangle index buffer (pre-tessellated polygons).
    pub triangles_indexes: Encoder,

    /// Encoding for the vertex data stream (logical encoding is always `ComponentwiseDelta` if `vertex_buffer_type==Vec2`).
    pub vertex: Encoder,
    /// Encoding for vertex offsets (used when `vertex_buffer_type` is not `Vec2`).
    pub vertex_offsets: Encoder,

    /// How the vertex buffer should be encoded.
    // vertex_buffer_type is pinned to Vec2 in the Arbitrary impl below.
    // Morton encoding requires coordinates in a bounded range and is tested via dedicated tests only.
    #[cfg_attr(test, proptest(value = "VertexBufferType::Vec2"))]
    #[cfg_attr(all(not(test), feature = "arbitrary"), arbitrary(value = VertexBufferType::Vec2))]
    pub vertex_buffer_type: VertexBufferType,
}

impl GeometryEncoder {
    /// Use the provided encoder for all streams.
    #[must_use]
    pub fn all(encoder: Encoder) -> Self {
        Self {
            meta: encoder,
            geometries: encoder,
            rings: encoder,
            rings2: encoder,
            no_rings: encoder,
            parts: encoder,
            parts_ring: encoder,
            only_parts: encoder,
            triangles: encoder,
            triangles_indexes: encoder,
            vertex: encoder,
            vertex_offsets: encoder,
            vertex_buffer_type: VertexBufferType::Vec2,
        }
    }

    /// Set encoding for the geometry types (meta) stream.
    pub fn meta(&mut self, e: Encoder) -> &mut Self {
        self.meta = e;
        self
    }

    /// Set encoding for the geometry length stream.
    pub fn geometries(&mut self, e: Encoder) -> &mut Self {
        self.geometries = e;
        self
    }

    /// Set encoding for parts length stream when rings are present.
    pub fn rings(&mut self, e: Encoder) -> &mut Self {
        self.rings = e;
        self
    }

    /// Set encoding for ring vertex-count stream.
    pub fn rings2(&mut self, e: Encoder) -> &mut Self {
        self.rings2 = e;
        self
    }

    /// Set encoding for parts length stream when rings are not present.
    pub fn no_rings(&mut self, e: Encoder) -> &mut Self {
        self.no_rings = e;
        self
    }

    /// Set encoding for parts length stream (with rings) when `geometry_offsets` absent.
    pub fn parts(&mut self, e: Encoder) -> &mut Self {
        self.parts = e;
        self
    }

    /// Set encoding for ring lengths when `geometry_offsets` absent.
    pub fn parts_ring(&mut self, e: Encoder) -> &mut Self {
        self.parts_ring = e;
        self
    }

    /// Set encoding for parts-only stream (e.g. `LineString`, no rings).
    pub fn only_parts(&mut self, e: Encoder) -> &mut Self {
        self.only_parts = e;
        self
    }

    /// Set encoding for triangles count stream (pre-tessellated polygons).
    pub fn triangles(&mut self, e: Encoder) -> &mut Self {
        self.triangles = e;
        self
    }

    /// Set encoding for triangle index buffer (pre-tessellated polygons).
    pub fn triangles_indexes(&mut self, e: Encoder) -> &mut Self {
        self.triangles_indexes = e;
        self
    }

    /// Set encoding for the vertex data stream.
    pub fn vertex(&mut self, e: Encoder) -> &mut Self {
        self.vertex = e;
        self
    }

    /// Set encoding for vertex offsets (dictionary encoding).
    pub fn vertex_offsets(&mut self, e: Encoder) -> &mut Self {
        self.vertex_offsets = e;
        self
    }

    /// Set the vertex buffer encoding type.
    pub fn vertex_buffer_type(&mut self, t: VertexBufferType) -> &mut Self {
        self.vertex_buffer_type = t;
        self
    }
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
