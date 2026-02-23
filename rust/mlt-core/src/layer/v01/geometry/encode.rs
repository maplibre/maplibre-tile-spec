//! Geometry encoding implementation for converting decoded geometries to encoded format.

use integer_encoding::VarInt as _;
use zigzag::ZigZag as _;

use super::{DecodedGeometry, OwnedEncodedGeometry};
use crate::MltError;
use crate::utils::encode_componentwise_delta_vec2s;
use crate::v01::{
    DictionaryType, GeometryEncodingStrategy, GeometryType, LengthType, LogicalCodec,
    LogicalEncoding, OffsetType, OwnedStream, PhysicalEncoding, PhysicalStreamType, StreamMeta,
};

/// Encode a length stream (for geometries, parts, rings)
fn encode_length_stream(
    lengths: &[u32],
    length_type: LengthType,
    logical: LogicalEncoding,
    physical: PhysicalEncoding,
) -> Result<OwnedStream, MltError> {
    OwnedStream::encode_u32s_of_type(
        lengths,
        logical,
        physical,
        PhysicalStreamType::Length(length_type),
    )
}

/// Encode a u32 stream with automatic encoding selection (plain, delta, RLE, delta-RLE)
fn encode_u32_stream_auto(
    values: &[u32],
    physical_type: PhysicalStreamType,
) -> Result<OwnedStream, MltError> {
    let num_values = u32::try_from(values.len())?;
    // Try different encodings and select the smallest
    let (data, logical_codec) = LogicalEncoding::None.encode_u32s(values)?;
    let (data, physical_codec) = PhysicalEncoding::VarInt.encode_u32s(data);
    Ok(OwnedStream {
        meta: StreamMeta {
            physical_type,
            num_values,
            logical_codec,
            physical_codec,
        },
        data,
    })
}

/// Encode a u32 stream with specified encoding
fn encode_u32_stream(
    values: &[u32],
    logical: LogicalEncoding,
    physical: PhysicalEncoding,
    physical_type: PhysicalStreamType,
) -> Result<OwnedStream, MltError> {
    OwnedStream::encode_u32s_of_type(values, logical, physical, physical_type)
}

/// Encode vertex buffer using componentwise delta encoding
fn encode_vertex_buffer(
    vertices: &[i32],
    physical: PhysicalEncoding,
) -> Result<OwnedStream, MltError> {
    let num_values = u32::try_from(vertices.len())?;
    // Componentwise delta encoding: delta X and Y separately
    let physical_u32 = encode_componentwise_delta_vec2s(vertices);
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

/// Calculate RLE runs and delta-RLE runs for encoding selection
fn calculate_runs(values: &[u32]) -> (usize, usize) {
    if values.is_empty() {
        return (0, 0);
    }

    let mut runs = 1;
    let mut delta_runs = 1;
    let mut prev_value = values[0];
    let mut prev_delta = 0i64;

    for &value in values.iter().skip(1) {
        if value != prev_value {
            runs += 1;
        }
        let delta = i64::from(value) - i64::from(prev_value);
        if delta != prev_delta {
            delta_runs += 1;
        }
        prev_value = value;
        prev_delta = delta;
    }

    (runs, delta_runs)
}

/// Encode values as varints
fn encode_varints_u32(values: &[u32]) -> Vec<u8> {
    let mut result = Vec::with_capacity(values.len() * 2);
    for &v in values {
        result.extend_from_slice(&v.encode_var_vec());
    }
    result
}

/// Encode values with delta + zigzag + varint
fn encode_delta_zigzag_varints(values: &[u32]) -> Vec<u8> {
    let mut result = Vec::with_capacity(values.len() * 2);
    let mut prev = 0i64;
    for &v in values {
        let value = i64::from(v);
        let delta = value - prev;
        #[expect(clippy::cast_possible_truncation, reason = "delta fits in i32")]
        let delta_i32 = delta as i32;
        let zigzag = i32::encode(delta_i32);
        result.extend_from_slice(&zigzag.encode_var_vec());
        prev = value;
    }
    result
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

        let needs_length = matches!(
            geom_type,
            GeometryType::MultiPolygon | GeometryType::Polygon
        ) || (is_line_string_present
            && matches!(
                geom_type,
                GeometryType::MultiLineString | GeometryType::LineString
            ));

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
            for _ in 0..num_geoms {
                ring_idx += 1;
                part_idx += 1;
            }
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

        if matches!(
            geom_type,
            GeometryType::MultiLineString | GeometryType::LineString
        ) {
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
        OwnedStream::encode_u32s(
            &vector_types_u32,
            LogicalEncoding::None,
            PhysicalEncoding::None,
        )?
    };

    let mut items = Vec::new();
    let has_linestrings = vector_types.iter().any(GeometryType::is_linestring);

    // Encode topology streams based on geometry structure
    if let Some(geom_offs) = geometry_offsets {
        // Encode geometry lengths (NumGeometries)
        let lengths = encode_root_length_stream(vector_types, geom_offs, GeometryType::Polygon);
        if !lengths.is_empty() {
            items.push(encode_length_stream(
                &lengths,
                LengthType::Geometries,
                LogicalEncoding::None,
                PhysicalEncoding::None,
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
                    items.push(encode_length_stream(
                        &part_lengths,
                        LengthType::Parts,
                        LogicalEncoding::None,
                        PhysicalEncoding::None,
                    )?);
                }

                let ring_lengths =
                    encode_level2_length_stream(vector_types, geom_offs, part_offs, ring_offs);
                if !ring_lengths.is_empty() {
                    items.push(encode_length_stream(
                        &ring_lengths,
                        LengthType::Rings,
                        LogicalEncoding::None,
                        PhysicalEncoding::None,
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
                    items.push(encode_length_stream(
                        &part_lengths,
                        LengthType::Parts,
                        LogicalEncoding::None,
                        PhysicalEncoding::None,
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
                items.push(encode_length_stream(
                    &part_lengths,
                    LengthType::Parts,
                    LogicalEncoding::None,
                    PhysicalEncoding::None,
                )?);
            }

            // Ring lengths
            let ring_lengths =
                encode_level1_length_stream(vector_types, part_offs, ring_offs, has_linestrings);
            if !ring_lengths.is_empty() {
                items.push(encode_length_stream(
                    &ring_lengths,
                    LengthType::Rings,
                    LogicalEncoding::None,
                    PhysicalEncoding::None,
                )?);
            }
        } else {
            // Only parts (e.g., LineString)
            let lengths = encode_root_length_stream(vector_types, part_offs, GeometryType::Point);
            if !lengths.is_empty() {
                items.push(encode_length_stream(
                    &lengths,
                    LengthType::Parts,
                    LogicalEncoding::None,
                    PhysicalEncoding::None,
                )?);
            }
        }
    }

    // Encode triangles stream if present (for pre-tessellated polygons)
    if let Some(tris) = triangles {
        items.push(encode_length_stream(
            tris,
            LengthType::Triangles,
            LogicalEncoding::None,
            PhysicalEncoding::None,
        )?);
    }

    // Encode index buffer if present (for pre-tessellated polygons)
    if let Some(idx_buf) = index_buffer {
        items.push(encode_u32_stream(
            idx_buf,
            LogicalEncoding::None,
            PhysicalEncoding::None,
            PhysicalStreamType::Offset(OffsetType::Index),
        )?);
    }

    // Encode vertex offsets if present (dictionary encoding)
    if let Some(v_offs) = vertex_offsets {
        items.push(encode_u32_stream(
            v_offs,
            LogicalEncoding::None,
            PhysicalEncoding::None,
            PhysicalStreamType::Offset(OffsetType::Vertex),
        )?);
    }

    // Encode vertex buffer
    if let Some(verts) = vertices {
        items.push(encode_vertex_buffer(verts, PhysicalEncoding::VarInt)?);
    }

    Ok(OwnedEncodedGeometry { meta, items })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_geometry() {
        let decoded = DecodedGeometry::default();
        let config = GeometryEncodingStrategy::default();
        let result = encode_geometry(&decoded, config);
        assert!(result.is_ok());
    }

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
