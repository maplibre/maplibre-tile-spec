//! Geometry encoding implementation for converting decoded geometries to encoded format.

use integer_encoding::VarInt as _;
use zigzag::ZigZag as _;

use super::{DecodedGeometry, OwnedEncodedGeometry};
use crate::MltError;
use crate::v01::geometry::GeometryType;
use crate::v01::{
    DictionaryType, LengthType, LogicalCodec, OwnedEncodedData, OwnedStream, OwnedStreamData,
    PhysicalCodec, PhysicalStreamType, RleMeta, StreamMeta,
};

/// Configuration for geometry encoding
#[derive(Debug, Clone, Copy)]
pub struct GeometryEncodingStrategy {
    /// Physical encoding technique for topology streams
    pub physical_codec: PhysicalCodec,
}

impl Default for GeometryEncodingStrategy {
    fn default() -> Self {
        Self {
            physical_codec: PhysicalCodec::VarInt,
        }
    }
}

impl GeometryEncodingStrategy {
    /// Create a new strategy with `VarInt` physical encoding
    #[must_use]
    pub fn varint() -> Self {
        Self {
            physical_codec: PhysicalCodec::VarInt,
        }
    }
}

/// Encode geometry types stream using RLE if beneficial
fn encode_geometry_types(types: &[GeometryType]) -> Result<OwnedStream, MltError> {
    if types.is_empty() {
        Ok(empty_stream(DictionaryType::None, LogicalCodec::None))
    } else {
        let values: Vec<u32> = types.iter().map(|t| *t as u32).collect();
        encode_u32_stream_auto(&values, PhysicalStreamType::Data(DictionaryType::None))
    }
}

fn empty_stream(dict_type: DictionaryType, logical: LogicalCodec) -> OwnedStream {
    OwnedStream {
        meta: StreamMeta {
            physical_type: PhysicalStreamType::Data(dict_type),
            num_values: 0,
            logical_codec: logical,
            physical_codec: PhysicalCodec::None,
        },
        data: OwnedStreamData::Encoded(OwnedEncodedData { data: Vec::new() }),
    }
}

/// Encode a length stream (for geometries, parts, rings)
fn encode_length_stream(
    lengths: &[u32],
    length_type: LengthType,
    physical_codec: PhysicalCodec,
) -> Result<OwnedStream, MltError> {
    if lengths.is_empty() {
        return Ok(OwnedStream {
            meta: StreamMeta {
                physical_type: PhysicalStreamType::Length(length_type),
                num_values: 0,
                logical_codec: LogicalCodec::None,
                physical_codec: PhysicalCodec::None,
            },
            data: OwnedStreamData::Encoded(OwnedEncodedData { data: Vec::new() }),
        });
    }

    encode_u32_stream(
        lengths,
        PhysicalStreamType::Length(length_type),
        LogicalCodec::None,
        physical_codec,
    )
}

/// Encode a u32 stream with automatic encoding selection (plain, delta, RLE, delta-RLE)
fn encode_u32_stream_auto(
    values: &[u32],
    physical_type: PhysicalStreamType,
) -> Result<OwnedStream, MltError> {
    if values.is_empty() {
        return Ok(OwnedStream {
            meta: StreamMeta {
                physical_type,
                num_values: 0,
                logical_codec: LogicalCodec::None,
                physical_codec: PhysicalCodec::None,
            },
            data: OwnedStreamData::Encoded(OwnedEncodedData { data: Vec::new() }),
        });
    }

    // Calculate statistics for encoding selection
    let (runs, delta_runs) = calculate_runs(values);

    // Try different encodings and select the smallest
    let plain_encoded = encode_varints_u32(values);
    let delta_encoded = encode_delta_zigzag_varints(values);

    let mut best_data = plain_encoded;
    let mut best_logical = LogicalCodec::None;
    let mut rle_runs = 0;
    let mut rle_num_values = 0;

    if delta_encoded.len() < best_data.len() {
        best_data = delta_encoded;
        best_logical = LogicalCodec::Delta;
    }

    // Try RLE if compression ratio is good enough (at least 2:1)
    if values.len() / runs >= 2 {
        let (rle_data, r, n) = encode_rle_varints(values);
        if rle_data.len() < best_data.len() {
            best_data = rle_data;
            best_logical = LogicalCodec::Rle(RleMeta {
                runs: r,
                num_rle_values: n,
            });
            rle_runs = r;
            rle_num_values = n;
        }
    }

    // Try Delta-RLE if compression ratio is good enough
    if values.len() / delta_runs >= 2 {
        let (delta_rle_data, r, n) = encode_delta_rle_varints(values);
        if delta_rle_data.len() < best_data.len() {
            best_data = delta_rle_data;
            best_logical = LogicalCodec::DeltaRle(RleMeta {
                runs: r,
                num_rle_values: n,
            });
            rle_runs = r;
            rle_num_values = n;
        }
    }

    // Check for const stream (single RLE run) - force RLE encoding
    if runs == 1 && values.len() > 1 {
        let (rle_data, r, n) = encode_rle_varints(values);
        best_data = rle_data;
        best_logical = LogicalCodec::Rle(RleMeta {
            runs: r,
            num_rle_values: n,
        });
        rle_runs = r;
        rle_num_values = n;
    }

    let num_values = match best_logical {
        LogicalCodec::Rle(_) | LogicalCodec::DeltaRle(_) => rle_runs + rle_num_values,
        _ => u32::try_from(values.len())?,
    };

    Ok(OwnedStream {
        meta: StreamMeta {
            physical_type,
            num_values,
            logical_codec: best_logical,
            physical_codec: PhysicalCodec::VarInt,
        },
        data: OwnedStreamData::VarInt(crate::v01::OwnedDataVarInt { data: best_data }),
    })
}

/// Encode a u32 stream with specified encoding
fn encode_u32_stream(
    values: &[u32],
    physical_type: PhysicalStreamType,
    logical_codec: LogicalCodec,
    physical_codec: PhysicalCodec,
) -> Result<OwnedStream, MltError> {
    if values.is_empty() {
        return Ok(OwnedStream {
            meta: StreamMeta {
                physical_type,
                num_values: 0,
                logical_codec: LogicalCodec::None,
                physical_codec: PhysicalCodec::None,
            },
            data: OwnedStreamData::Encoded(OwnedEncodedData { data: Vec::new() }),
        });
    }

    let data = match logical_codec {
        LogicalCodec::None => encode_varints_u32(values),
        LogicalCodec::Delta => encode_delta_zigzag_varints(values),
        _ => return Err(MltError::NotImplemented("unsupported logical decoder")),
    };

    let num_values = u32::try_from(values.len())?;

    let stream_data = match physical_codec {
        PhysicalCodec::VarInt => OwnedStreamData::VarInt(crate::v01::OwnedDataVarInt { data }),
        PhysicalCodec::None => OwnedStreamData::Encoded(OwnedEncodedData { data }),
        _ => return Err(MltError::NotImplemented("unsupported physical decoder")),
    };

    Ok(OwnedStream {
        meta: StreamMeta {
            physical_type,
            num_values,
            logical_codec,
            physical_codec,
        },
        data: stream_data,
    })
}

/// Encode vertex buffer using componentwise delta encoding
fn encode_vertex_buffer(vertices: &[i32]) -> Result<OwnedStream, MltError> {
    if vertices.is_empty() {
        return Ok(empty_stream(
            DictionaryType::Vertex,
            LogicalCodec::ComponentwiseDelta,
        ));
    }

    // Componentwise delta encoding: delta X and Y separately
    let encoded = encode_componentwise_delta_zigzag_varints(vertices);
    let num_values = u32::try_from(vertices.len())?;

    Ok(OwnedStream {
        meta: StreamMeta {
            physical_type: PhysicalStreamType::Data(DictionaryType::Vertex),
            num_values,
            logical_codec: LogicalCodec::ComponentwiseDelta,
            physical_codec: PhysicalCodec::VarInt,
        },
        data: OwnedStreamData::VarInt(crate::v01::OwnedDataVarInt { data: encoded }),
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

/// Encode values with RLE + varint
/// Returns (encoded data, `num_runs`, `num_rle_values`)
fn encode_rle_varints(values: &[u32]) -> (Vec<u8>, u32, u32) {
    if values.is_empty() {
        return (Vec::new(), 0, 0);
    }

    let mut runs = Vec::new();
    let mut vals = Vec::new();

    let mut current_val = values[0];
    let mut current_run = 1u32;

    for &val in &values[1..] {
        if val == current_val {
            current_run = current_run.saturating_add(1);
        } else {
            runs.push(current_run);
            vals.push(current_val);
            current_val = val;
            current_run = 1;
        }
    }
    runs.push(current_run);
    vals.push(current_val);

    let num_runs = u32::try_from(runs.len()).unwrap_or(u32::MAX);
    let num_rle_values = u32::try_from(vals.len()).unwrap_or(u32::MAX);

    // Encode runs followed by values
    let mut result = Vec::with_capacity((runs.len() + vals.len()) * 2);
    for &r in &runs {
        result.extend_from_slice(&r.encode_var_vec());
    }
    for &v in &vals {
        result.extend_from_slice(&v.encode_var_vec());
    }

    (result, num_runs, num_rle_values)
}

/// Encode values with delta + RLE + varint
/// Returns (encoded data, `num_runs`, `num_delta_rle_values`)
fn encode_delta_rle_varints(values: &[u32]) -> (Vec<u8>, u32, u32) {
    if values.is_empty() {
        return (Vec::new(), 0, 0);
    }

    // First compute deltas
    let mut deltas = Vec::with_capacity(values.len());
    let mut prev = 0i64;
    for &v in values {
        let value = i64::from(v);
        let delta = value - prev;
        deltas.push(delta);
        prev = value;
    }

    // RLE encode the deltas
    let mut runs = Vec::new();
    let mut vals = Vec::new();

    let mut current_val = deltas[0];
    let mut current_run = 1u32;

    for &delta in &deltas[1..] {
        if delta == current_val {
            current_run = current_run.saturating_add(1);
        } else {
            runs.push(current_run);
            vals.push(current_val);
            current_val = delta;
            current_run = 1;
        }
    }
    runs.push(current_run);
    vals.push(current_val);

    let num_runs = u32::try_from(runs.len()).unwrap_or(u32::MAX);
    let num_rle_values = u32::try_from(vals.len()).unwrap_or(u32::MAX);

    // Encode runs followed by zigzag-encoded values
    let mut result = Vec::with_capacity((runs.len() + vals.len()) * 2);
    for &r in &runs {
        result.extend_from_slice(&r.encode_var_vec());
    }
    for &v in &vals {
        #[expect(clippy::cast_possible_truncation, reason = "delta fits in i32")]
        let delta_i32 = v as i32;
        let zigzag = i32::encode(delta_i32);
        result.extend_from_slice(&zigzag.encode_var_vec());
    }

    (result, num_runs, num_rle_values)
}

/// Encode vertex buffer with componentwise delta + zigzag + varint
/// Input is interleaved [x0, y0, x1, y1, ...] coordinates
fn encode_componentwise_delta_zigzag_varints(vertices: &[i32]) -> Vec<u8> {
    let mut result = Vec::with_capacity(vertices.len() * 2);
    let mut prev_x = 0i32;
    let mut prev_y = 0i32;

    for pair in vertices.chunks_exact(2) {
        let x = pair[0];
        let y = pair[1];

        // Delta encoding per component
        let delta_x = x.wrapping_sub(prev_x);
        let delta_y = y.wrapping_sub(prev_y);

        // Zigzag encoding
        let zigzag_x = i32::encode(delta_x);
        let zigzag_y = i32::encode(delta_y);

        // Varint encoding
        result.extend_from_slice(&zigzag_x.encode_var_vec());
        result.extend_from_slice(&zigzag_y.encode_var_vec());

        prev_x = x;
        prev_y = y;
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

/// Determine if any polygon types are present
fn contains_polygon(types: &[GeometryType]) -> bool {
    types
        .iter()
        .any(|t| matches!(t, GeometryType::Polygon | GeometryType::MultiPolygon))
}

/// Determine if any linestring types are present (excluding points)
fn contains_linestring(types: &[GeometryType]) -> bool {
    types
        .iter()
        .any(|t| matches!(t, GeometryType::LineString | GeometryType::MultiLineString))
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

    if vector_types.is_empty() {
        return Ok(OwnedEncodedGeometry::default());
    }

    // Encode geometry types (meta stream)
    let meta = encode_geometry_types(vector_types)?;

    let mut items = Vec::new();

    let has_linestrings = contains_linestring(vector_types);

    // Encode topology streams based on geometry structure
    if let Some(geom_offs) = geometry_offsets {
        // Encode geometry lengths (NumGeometries)
        let lengths = encode_root_length_stream(vector_types, geom_offs, GeometryType::Polygon);
        if !lengths.is_empty() {
            items.push(encode_length_stream(
                &lengths,
                LengthType::Geometries,
                config.physical_codec,
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
                        config.physical_codec,
                    )?);
                }

                let ring_lengths =
                    encode_level2_length_stream(vector_types, geom_offs, part_offs, ring_offs);
                if !ring_lengths.is_empty() {
                    items.push(encode_length_stream(
                        &ring_lengths,
                        LengthType::Rings,
                        config.physical_codec,
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
                        config.physical_codec,
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
                    config.physical_codec,
                )?);
            }

            // Ring lengths
            let ring_lengths =
                encode_level1_length_stream(vector_types, part_offs, ring_offs, has_linestrings);
            if !ring_lengths.is_empty() {
                items.push(encode_length_stream(
                    &ring_lengths,
                    LengthType::Rings,
                    config.physical_codec,
                )?);
            }
        } else {
            // Only parts (e.g., LineString)
            let lengths = encode_root_length_stream(vector_types, part_offs, GeometryType::Point);
            if !lengths.is_empty() {
                items.push(encode_length_stream(
                    &lengths,
                    LengthType::Parts,
                    config.physical_codec,
                )?);
            }
        }
    }

    // Encode triangles stream if present (for pre-tessellated polygons)
    if let Some(tris) = triangles {
        items.push(encode_length_stream(
            tris,
            LengthType::Triangles,
            config.physical_codec,
        )?);
    }

    // Encode index buffer if present (for pre-tessellated polygons)
    if let Some(idx_buf) = index_buffer {
        items.push(encode_u32_stream(
            idx_buf,
            PhysicalStreamType::Offset(crate::v01::OffsetType::Index),
            LogicalCodec::None,
            config.physical_codec,
        )?);
    }

    // Encode vertex offsets if present (dictionary encoding)
    if let Some(v_offs) = vertex_offsets {
        items.push(encode_u32_stream(
            v_offs,
            PhysicalStreamType::Offset(crate::v01::OffsetType::Vertex),
            LogicalCodec::None,
            config.physical_codec,
        )?);
    }

    // Encode vertex buffer
    if let Some(verts) = vertices {
        items.push(encode_vertex_buffer(verts)?);
    }

    Ok(OwnedEncodedGeometry { meta, items })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_varints_u32() {
        let values = vec![1, 2, 127, 128, 16383, 16384];
        let encoded = encode_varints_u32(&values);
        assert!(!encoded.is_empty());
    }

    #[test]
    fn test_encode_delta_zigzag() {
        let values = vec![0, 1, 2, 3, 4];
        let encoded = encode_delta_zigzag_varints(&values);
        // Delta: [0, 1, 1, 1, 1] -> zigzag: [0, 2, 2, 2, 2]
        assert!(!encoded.is_empty());
    }

    #[test]
    fn test_componentwise_delta() {
        let vertices = vec![0, 0, 10, 20, 15, 25];
        let encoded = encode_componentwise_delta_zigzag_varints(&vertices);
        // Pairs: (0,0), (10,20), (15,25)
        // Delta X: 0, 10, 5
        // Delta Y: 0, 20, 5
        assert!(!encoded.is_empty());
    }

    #[test]
    fn test_calculate_runs() {
        let values = vec![1, 1, 1, 2, 2, 3];
        let (runs, delta_runs) = calculate_runs(&values);
        assert_eq!(runs, 3); // 1, 2, 3 (3 distinct values)
        // Deltas: [1, 0, 0, 1, 0, 1] -> delta_runs count changes in delta
        // Position 0: delta=1 (first)
        // Position 1: delta=0 (change from 1)
        // Position 2: delta=0 (same)
        // Position 3: delta=1 (change from 0)
        // Position 4: delta=0 (change from 1)
        // Position 5: delta=1 (change from 0)
        // Changes at: 1, 3, 4, 5 -> 4 transitions + 1 initial = 5? Let me trace...
        // Actually: prev_delta starts at 0, so first delta (values[1]-values[0]=0) != prev_delta=0? No.
        // Actually the value[0] = 1, prev=0, delta[0] = 1.
        // Then value[1] = 1, prev=1, delta[1] = 0, != prev_delta(=0 initially)? No, prev_delta starts at 0 after first iteration
        // Let me trace again:
        // i=1: value=1, delta=1-1=0, prev_delta=1 (set in i=0 loop didn't run)
        // Wait, the loop starts at skip(1), so first iteration is i=1
        // i=0 is handled outside: prev_value=values[0]=1, prev_delta=0
        // i=1: value=1, delta=0, delta!=prev_delta(0)? no. prev_delta=0, prev_value=1
        // i=2: value=1, delta=0, delta!=prev_delta(0)? no.
        // i=3: value=2, delta=1, delta!=prev_delta(0)? yes! delta_runs=2
        // i=4: value=2, delta=0, delta!=prev_delta(1)? yes! delta_runs=3
        // i=5: value=3, delta=1, delta!=prev_delta(0)? yes! delta_runs=4
        assert_eq!(delta_runs, 4);
    }

    #[test]
    fn test_calculate_runs_empty() {
        let (runs, delta_runs) = calculate_runs(&[]);
        assert_eq!(runs, 0);
        assert_eq!(delta_runs, 0);
    }

    #[test]
    fn test_rle_encoding() {
        let values = vec![1, 1, 1, 2, 2, 3];
        let (encoded, num_runs, num_vals) = encode_rle_varints(&values);
        assert_eq!(num_runs, 3);
        assert_eq!(num_vals, 3);
        assert!(!encoded.is_empty());
    }

    #[test]
    fn test_rle_encoding_empty() {
        let (encoded, num_runs, num_vals) = encode_rle_varints(&[]);
        assert_eq!(num_runs, 0);
        assert_eq!(num_vals, 0);
        assert!(encoded.is_empty());
    }

    #[test]
    fn test_delta_rle_encoding_empty() {
        let (encoded, num_runs, num_vals) = encode_delta_rle_varints(&[]);
        assert_eq!(num_runs, 0);
        assert_eq!(num_vals, 0);
        assert!(encoded.is_empty());
    }

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

    #[test]
    fn test_varint_constructor() {
        let config = GeometryEncodingStrategy::varint();
        assert!(matches!(config.physical_codec, PhysicalCodec::VarInt));
    }

    #[test]
    fn test_encode_empty_geometry_types() {
        let result = encode_geometry_types(&[]);
        assert!(result.is_ok());
        let stream = result.unwrap();
        assert_eq!(stream.meta.num_values, 0);
    }

    #[test]
    fn test_encode_empty_length_stream() {
        let result = encode_length_stream(&[], LengthType::Geometries, PhysicalCodec::VarInt);
        assert!(result.is_ok());
        let stream = result.unwrap();
        assert_eq!(stream.meta.num_values, 0);
    }

    #[test]
    fn test_encode_empty_u32_stream_auto() {
        let result = encode_u32_stream_auto(&[], PhysicalStreamType::Data(DictionaryType::None));
        assert!(result.is_ok());
        let stream = result.unwrap();
        assert_eq!(stream.meta.num_values, 0);
    }

    #[test]
    fn test_encode_empty_u32_stream() {
        let result = encode_u32_stream(
            &[],
            PhysicalStreamType::Data(DictionaryType::None),
            LogicalCodec::None,
            PhysicalCodec::VarInt,
        );
        assert!(result.is_ok());
        let stream = result.unwrap();
        assert_eq!(stream.meta.num_values, 0);
    }

    #[test]
    fn test_encode_empty_vertex_buffer() {
        let result = encode_vertex_buffer(&[]);
        assert!(result.is_ok());
        let stream = result.unwrap();
        assert_eq!(stream.meta.num_values, 0);
    }

    #[test]
    fn test_delta_encoding_wins() {
        // Create a sequence where delta encoding is more efficient:
        // Large values with constant increment of 1 - plain varint needs many bytes for
        // each large value, but delta only needs 1 byte per delta (zigzag 1 = 2)
        // Using values large enough that varint encoding is multiple bytes
        let values: Vec<u32> = (100_000..100_020).collect();
        let result =
            encode_u32_stream_auto(&values, PhysicalStreamType::Data(DictionaryType::None));
        assert!(result.is_ok());
        let stream = result.unwrap();
        // With 20 values starting at 100000, delta encoding should win over plain
        // since each value uses 3 bytes in plain varint but deltas use 1 byte
        assert!(
            matches!(
                stream.meta.logical_codec,
                LogicalCodec::Delta | LogicalCodec::DeltaRle(_)
            ),
            "Expected Delta or DeltaRle encoding, got {:?}",
            stream.meta.logical_codec
        );
    }

    #[test]
    fn test_delta_rle_encoding_wins() {
        // Create a sequence where delta-RLE is most efficient:
        // Monotonically increasing with constant step - deltas are all the same
        // e.g., [0, 10, 20, 30, 40, 50, 60, 70, 80, 90] -> deltas = [0, 10, 10, 10, 10, 10, 10, 10, 10, 10]
        // This compresses well with delta-RLE since the delta values are constant
        let values: Vec<u32> = (0..20).map(|i| i * 100).collect();
        let result =
            encode_u32_stream_auto(&values, PhysicalStreamType::Data(DictionaryType::None));
        assert!(result.is_ok());
        let stream = result.unwrap();
        // With constant deltas and enough values, delta-RLE should win
        assert!(
            matches!(stream.meta.logical_codec, LogicalCodec::DeltaRle(_)),
            "Expected DeltaRle, got {:?}",
            stream.meta.logical_codec
        );
    }

    #[test]
    fn test_encode_u32_stream_with_delta_codec() {
        let values = vec![100, 101, 102, 103];
        let result = encode_u32_stream(
            &values,
            PhysicalStreamType::Data(DictionaryType::None),
            LogicalCodec::Delta,
            PhysicalCodec::VarInt,
        );
        assert!(result.is_ok());
        let stream = result.unwrap();
        assert!(matches!(stream.meta.logical_codec, LogicalCodec::Delta));
    }

    #[test]
    fn test_encode_u32_stream_with_none_physical_codec() {
        let values = vec![1, 2, 3];
        let result = encode_u32_stream(
            &values,
            PhysicalStreamType::Data(DictionaryType::None),
            LogicalCodec::None,
            PhysicalCodec::None,
        );
        assert!(result.is_ok());
        let stream = result.unwrap();
        assert!(matches!(stream.meta.physical_codec, PhysicalCodec::None));
    }
}
