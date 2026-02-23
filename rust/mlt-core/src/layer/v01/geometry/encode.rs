use super::{DecodedGeometry, OwnedEncodedGeometry};
use crate::MltError;
use crate::utils::encode_componentwise_delta_vec2s;
use crate::v01::{
    DictionaryType, GeometryEncodingStrategy, GeometryType, LengthType, LogicalCodec, OffsetType,
    OwnedStream, PhysicalEncoding, PhysicalStreamType, StreamMeta,
};

/// Encode vertex buffer using componentwise delta encoding
fn encode_vertex_buffer(
    vertices: &[i32],
    physical: PhysicalEncoding,
) -> Result<OwnedStream, MltError> {
    // Componentwise delta encoding: delta X and Y separately
    let physical_u32 = encode_componentwise_delta_vec2s(vertices);
    let num_values = u32::try_from(physical_u32.len())?;
    let (data, physical_codec) = physical.encode_u32s(physical_u32);
    Ok(OwnedStream {
        meta: StreamMeta {
            physical_type: PhysicalStreamType::Data(DictionaryType::Vertex),
            num_values,
            logical_codec: LogicalCodec::ComponentwiseDelta,
            physical_codec,
        },
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
            ring_idx += num_geoms;
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

/// Main geometry encoding function
pub fn encode_geometry(
    decoded: &DecodedGeometry,
    config: GeometryEncodingStrategy,
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

    // Encode geometry types (meta stream)
    let meta = {
        let vector_types_u32: Vec<u32> = vector_types.iter().map(|t| *t as u32).collect();
        OwnedStream::encode_u32s(&vector_types_u32, config.meta_logical, config.meta_physical)?
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
                config.num_geometries_logical,
                config.num_geometries_physical,
                PhysicalStreamType::Length(LengthType::Geometries),
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
                        config.rings_logical,
                        config.rings_physical,
                        PhysicalStreamType::Length(LengthType::Parts),
                    )?);
                }

                let ring_lengths =
                    encode_level2_length_stream(vector_types, geom_offs, part_offs, ring_offs);
                if !ring_lengths.is_empty() {
                    items.push(OwnedStream::encode_u32s_of_type(
                        &ring_lengths,
                        config.rings2_logical,
                        config.rings2_physical,
                        PhysicalStreamType::Length(LengthType::Rings),
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
                        config.no_rings_logical,
                        config.no_rings_physical,
                        PhysicalStreamType::Length(LengthType::Parts),
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
                    config.parts_logical,
                    config.parts_physical,
                    PhysicalStreamType::Length(LengthType::Parts),
                )?);
            }

            // Ring lengths
            let ring_lengths =
                encode_level1_length_stream(vector_types, part_offs, ring_offs, has_linestrings);
            if !ring_lengths.is_empty() {
                items.push(OwnedStream::encode_u32s_of_type(
                    &ring_lengths,
                    config.parts_ring_logical,
                    config.parts_ring_physical,
                    PhysicalStreamType::Length(LengthType::Rings),
                )?);
            }
        } else {
            // Only parts (e.g., LineString)
            let lengths = encode_root_length_stream(vector_types, part_offs, GeometryType::Point);
            if !lengths.is_empty() {
                items.push(OwnedStream::encode_u32s_of_type(
                    &lengths,
                    config.only_parts_logical,
                    config.only_parts_physical,
                    PhysicalStreamType::Length(LengthType::Parts),
                )?);
            }
        }
    }

    // Encode triangles stream if present (for pre-tessellated polygons)
    if let Some(tris) = triangles {
        items.push(OwnedStream::encode_u32s_of_type(
            tris,
            config.triangles_logical,
            config.triangles_physical,
            PhysicalStreamType::Length(LengthType::Triangles),
        )?);
    }

    // Encode index buffer if present (for pre-tessellated polygons)
    if let Some(idx_buf) = index_buffer {
        items.push(OwnedStream::encode_u32s_of_type(
            idx_buf,
            config.triangles_indexes_logical,
            config.triangles_indexes_physical,
            PhysicalStreamType::Offset(OffsetType::Index),
        )?);
    }

    // Encode vertex offsets if present (dictionary encoding)
    if let Some(v_offs) = vertex_offsets {
        items.push(OwnedStream::encode_u32s_of_type(
            v_offs,
            config.vertex_offsets_logical,
            config.vertex_offsets_physical,
            PhysicalStreamType::Offset(OffsetType::Vertex),
        )?);
    }

    // Encode vertex buffer
    if let Some(verts) = vertices {
        items.push(encode_vertex_buffer(verts, config.vertex_physical)?);
    }

    Ok(OwnedEncodedGeometry { meta, items })
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
