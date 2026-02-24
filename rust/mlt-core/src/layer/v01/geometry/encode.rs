use super::{DecodedGeometry, OwnedEncodedGeometry};
use crate::MltError;
use crate::utils::encode_componentwise_delta_vec2s;
use crate::v01::LengthType::VarBinary;
use crate::v01::{
    DictionaryType, Encoder, GeometryType, LengthType, LogicalEncoding, OffsetType, OwnedStream,
    PhysicalEncoder, StreamMeta, StreamType,
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

/// Convert geometry offsets to length stream for encoding
/// This is the inverse of `decode_root_length_stream`
fn encode_root_length_stream(
    geometry_types: &[GeometryType],
    geometry_offsets: &[u32],
    buffer_id: GeometryType,
) -> Vec<u32> {
    let mut lengths = Vec::new();

    for (i, &geom_type) in geometry_types.iter().enumerate() {
        // Only encode lengths for geometry types > buffer_id
        // (e.g., for Polygon buffer_id, encode MultiPolygon and MultiLineString lengths)
        if geom_type > buffer_id {
            let start = geometry_offsets[i];
            let end = geometry_offsets[i + 1];
            lengths.push(end - start);
        }
    }

    lengths
}

/// Convert part offsets to length stream for level 1 encoding
/// This is the inverse of `decode_level1_length_stream`
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
        } else {
            // Skip entries that don't have lengths in the stream
            part_idx += num_geoms;
        }
    }

    lengths
}

/// Convert ring offsets to length stream for level 2 encoding
/// This is the inverse of `decode_level2_length_stream`
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

        if geom_type != GeometryType::Point && geom_type != GeometryType::MultiPoint {
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
        } else {
            // Point/MultiPoint don't have ring lengths
            // But we still need to advance part_idx appropriately because
            // decode_level2_length_stream advances its level1_offset_buffer_counter
            part_idx += num_geoms;
        }
    }

    lengths
}

/// Convert part offsets without ring buffer to length stream
/// This is the inverse of `decode_level1_without_ring_buffer_length_stream`
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
        } else {
            // Point/MultiPoint don't have lengths in this stream
            part_idx += num_geoms;
        }
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

/// Normalize `part_offsets` for ring-based indexing (Polygon mixed with `Point`/`LineString`)
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

        if geom_type == GeometryType::Point || geom_type == GeometryType::LineString {
            // Point/LineString don't contribute to ring_offsets (0 rings)
        } else if geom_type == GeometryType::Polygon {
            // Polygon consumes rings tracked in part_offsets
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
    config: &dyn GeometryEncoder,
) -> Result<OwnedEncodedGeometry, MltError> {
    let DecodedGeometry {
        vector_types,
        geometry_offsets,
        part_offsets,
        ring_offsets,
        vertex_offsets,
        index_buffer,
        triangles,
        vertices,
    } = decoded;

    // Normalize part_offsets if needed for mixed geometry types
    let normalized_parts = if geometry_offsets.is_none() {
        if let Some(part_offs) = part_offsets {
            if let Some(ring_offs) = ring_offsets {
                // Mixed with rings (e.g., Point + Polygon) - normalize for ring-based indexing
                Some(normalize_part_offsets_for_rings(
                    vector_types,
                    part_offs,
                    ring_offs,
                ))
            } else if let Some(verts) = vertices {
                // Mixed without rings (e.g., Point + LineString) - normalize for vertex-based indexing
                Some(normalize_part_offsets_for_vertices(
                    vector_types,
                    part_offs,
                    verts,
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

    // Encode geometry types (meta stream)
    let meta = {
        let vector_types_u32: Vec<u32> = vector_types.iter().map(|t| *t as u32).collect();
        OwnedStream::encode_u32s_of_type(
            &vector_types_u32,
            config.meta(),
            StreamType::Length(VarBinary),
        )?
    };

    let mut items = Vec::new();
    let has_linestrings = vector_types
        .iter()
        .copied()
        .any(GeometryType::is_linestring);

    // Encode topology streams based on geometry structure
    if let Some(geom_offs) = geometry_offsets {
        // Encode geometry lengths (NumGeometries)
        let lengths = encode_root_length_stream(vector_types, geom_offs, GeometryType::Polygon);
        if !lengths.is_empty() {
            items.push(OwnedStream::encode_u32s_of_type(
                &lengths,
                config.num_geometries(),
                StreamType::Length(LengthType::Geometries),
            )?);
        }

        if let Some(part_offs) = part_offsets {
            if let Some(ring_offs) = ring_offsets {
                // Full topology: geom -> parts -> rings
                let part_lengths = encode_level1_length_stream(
                    vector_types,
                    geom_offs,
                    part_offs,
                    has_linestrings,
                );
                if !part_lengths.is_empty() {
                    items.push(OwnedStream::encode_u32s_of_type(
                        &part_lengths,
                        config.rings(),
                        StreamType::Length(LengthType::Parts),
                    )?);
                }

                let ring_lengths =
                    encode_level2_length_stream(vector_types, geom_offs, part_offs, ring_offs);
                if !ring_lengths.is_empty() {
                    items.push(OwnedStream::encode_u32s_of_type(
                        &ring_lengths,
                        config.rings2(),
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
                        config.no_rings(),
                        StreamType::Length(LengthType::Parts),
                    )?);
                }
            }
        }
    } else if let Some(part_offs) = part_offsets {
        // No geometry offsets, encode from parts
        if let Some(ring_offs) = ring_offsets {
            // parts -> rings (e.g., Polygon without Multi)
            let part_lengths =
                encode_root_length_stream(vector_types, part_offs, GeometryType::LineString);
            if !part_lengths.is_empty() {
                items.push(OwnedStream::encode_u32s_of_type(
                    &part_lengths,
                    config.parts(),
                    StreamType::Length(LengthType::Parts),
                )?);
            }

            // Ring lengths
            let ring_lengths =
                encode_level1_length_stream(vector_types, part_offs, ring_offs, has_linestrings);
            if !ring_lengths.is_empty() {
                items.push(OwnedStream::encode_u32s_of_type(
                    &ring_lengths,
                    config.parts_ring(),
                    StreamType::Length(LengthType::Rings),
                )?);
            }
        } else {
            // Only parts (e.g., LineString)
            let lengths = encode_root_length_stream(vector_types, part_offs, GeometryType::Point);
            if !lengths.is_empty() {
                items.push(OwnedStream::encode_u32s_of_type(
                    &lengths,
                    config.only_parts(),
                    StreamType::Length(LengthType::Parts),
                )?);
            }
        }
    }

    // Encode triangles stream if present (for pre-tessellated polygons)
    if let Some(tris) = triangles {
        items.push(OwnedStream::encode_u32s_of_type(
            tris,
            config.triangles(),
            StreamType::Length(LengthType::Triangles),
        )?);
    }

    // Encode index buffer if present (for pre-tessellated polygons)
    if let Some(idx_buf) = index_buffer {
        items.push(OwnedStream::encode_u32s_of_type(
            idx_buf,
            config.triangles_indexes(),
            StreamType::Offset(OffsetType::Index),
        )?);
    }

    // Encode vertex offsets if present (dictionary encoding)
    if let Some(v_offs) = vertex_offsets {
        items.push(OwnedStream::encode_u32s_of_type(
            v_offs,
            config.vertex_offsets(),
            StreamType::Offset(OffsetType::Vertex),
        )?);
    }

    // Encode vertex buffer
    if let Some(verts) = vertices {
        items.push(encode_vertex_buffer(verts, config.vertex().physical)?);
    }

    Ok(OwnedEncodedGeometry { meta, items })
}

pub trait GeometryEncoder {
    /// Encoding settings for the geometry types (meta) stream.
    fn meta(&self) -> Encoder {
        unreachable!()
    }

    /// Encoding for the geometry length stream.
    fn num_geometries(&self) -> Encoder {
        unreachable!()
    }

    /// Encoding for parts length stream when rings are present.
    fn rings(&self) -> Encoder {
        unreachable!()
    }
    /// Encoding for ring vertex-count stream.
    fn rings2(&self) -> Encoder {
        unreachable!()
    }
    /// Encoding for parts length stream when rings are not present.
    fn no_rings(&self) -> Encoder {
        unreachable!()
    }

    /// Encoding for parts length stream (with rings) when `geometry_offsets` absent.
    fn parts(&self) -> Encoder {
        unreachable!()
    }
    /// Encoding for ring lengths when `geometry_offsets` absent.
    fn parts_ring(&self) -> Encoder {
        unreachable!()
    }

    /// Encoding for parts-only stream (e.g. `LineString`, no rings).
    fn only_parts(&self) -> Encoder {
        unreachable!()
    }

    /// Encoding for triangles count stream (pre-tessellated polygons).
    fn triangles(&self) -> Encoder {
        unreachable!()
    }
    /// Encoding for triangle index buffer (pre-tessellated polygons).
    fn triangles_indexes(&self) -> Encoder {
        unreachable!()
    }

    /// Encoding for the vertex data stream (logical is always `ComponentwiseDelta`; only physical varies).
    fn vertex(&self) -> Encoder {
        unreachable!()
    }
    /// Encoding for vertex offsets (dictionary encoding).
    fn vertex_offsets(&self) -> Encoder {
        unreachable!()
    }
}

/// How to encode Geometry
#[derive(Debug, Clone, Copy)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub struct GeometryEncoderAll {
    /// Encoding settings for the geometry types (meta) stream.
    pub meta: Encoder,

    /// Encoding for the geometry length stream.
    pub num_geometries: Encoder,

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

    /// Encoding for the vertex data stream (logical is always `ComponentwiseDelta`; only physical varies).
    pub vertex: Encoder,
    /// Encoding for vertex offsets (dictionary encoding).
    pub vertex_offsets: Encoder,
}

impl GeometryEncoder for GeometryEncoderAll {
    fn meta(&self) -> Encoder {
        self.meta
    }
    fn num_geometries(&self) -> Encoder {
        self.num_geometries
    }
    fn rings(&self) -> Encoder {
        self.rings
    }
    fn rings2(&self) -> Encoder {
        self.rings2
    }
    fn no_rings(&self) -> Encoder {
        self.no_rings
    }
    fn parts(&self) -> Encoder {
        self.parts
    }
    fn parts_ring(&self) -> Encoder {
        self.parts_ring
    }
    fn only_parts(&self) -> Encoder {
        self.only_parts
    }
    fn triangles(&self) -> Encoder {
        self.triangles
    }
    fn triangles_indexes(&self) -> Encoder {
        self.triangles_indexes
    }
    fn vertex(&self) -> Encoder {
        self.vertex
    }
    fn vertex_offsets(&self) -> Encoder {
        self.vertex_offsets
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
