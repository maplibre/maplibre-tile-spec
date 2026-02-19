use crate::MltError;
use crate::v01::{GeometryType, Stream};

pub fn decode_geometry_types(meta: Stream) -> Result<Vec<GeometryType>, MltError> {
    // TODO: simplify this, e.g. use u8 or even GeometryType directly rather than going via Vec<u32>
    let vector_types: Vec<u32> = meta.decode_bits_u32()?.decode_u32()?;
    let vector_types: Vec<GeometryType> = vector_types
        .into_iter()
        .map::<Result<GeometryType, MltError>, _>(|v| Ok(u8::try_from(v)?.try_into()?))
        .collect::<Result<_, _>>()?;
    Ok(vector_types)
}

/// Handle the parsing of the different topology length buffers separate not generic to reduce the
/// branching and improve the performance
pub fn decode_root_length_stream(
    geometry_types: &[GeometryType],
    root_length_stream: &[u32],
    buffer_id: GeometryType,
) -> Vec<u32> {
    let mut root_buffer_offsets = Vec::with_capacity(geometry_types.len() + 1);
    root_buffer_offsets.push(0);
    let mut previous_offset = 0_u32;
    let mut root_length_counter = 0_usize;
    for &geom_type in geometry_types {
        let offset = previous_offset
            + if geom_type > buffer_id {
                let val = root_length_stream[root_length_counter];
                root_length_counter += 1;
                val
            } else {
                1
            };
        root_buffer_offsets.push(offset);
        previous_offset = offset;
    }
    root_buffer_offsets
}

/// Case where no ring buffer exists so no `MultiPolygon` or `Polygon` geometry is part of the buffer
pub fn decode_level1_without_ring_buffer_length_stream(
    geometry_types: &[GeometryType],
    root_offset_buffer: &[u32],
    level1_length_buffer: &[u32],
) -> Vec<u32> {
    let final_size = root_offset_buffer[root_offset_buffer.len() - 1] as usize + 1;
    let mut level1_buffer_offsets = Vec::with_capacity(final_size);
    level1_buffer_offsets.push(0);
    let mut previous_offset = 0_u32;
    let mut level1_offset_buffer_counter = 1_usize;
    let mut level1_length_counter = 0_usize;

    for (i, &geometry_type) in geometry_types.iter().enumerate() {
        let num_geometries = (root_offset_buffer[i + 1] - root_offset_buffer[i]) as usize;

        if geometry_type == GeometryType::MultiLineString
            || geometry_type == GeometryType::LineString
        {
            // For MultiLineString and LineString a value in the level1LengthBuffer exists
            for _j in 0..num_geometries {
                previous_offset += level1_length_buffer[level1_length_counter];
                level1_length_counter += 1;
                level1_buffer_offsets.push(previous_offset);
                level1_offset_buffer_counter += 1;
            }
        } else {
            // For MultiPoint and Point no value in level1LengthBuffer exists
            for _j in 0..num_geometries {
                previous_offset += 1;
                level1_buffer_offsets.push(previous_offset);
                level1_offset_buffer_counter += 1;
            }
        }
    }

    level1_buffer_offsets
}

pub fn decode_level1_length_stream(
    geometry_types: &[GeometryType],
    root_offset_buffer: &[u32],
    level1_length_buffer: &[u32],
    is_line_string_present: bool,
) -> Vec<u32> {
    let final_size = root_offset_buffer[root_offset_buffer.len() - 1] as usize + 1;
    let mut level1_buffer_offsets = Vec::with_capacity(final_size);
    level1_buffer_offsets.push(0);
    let mut previous_offset = 0_u32;
    let mut level1_buffer_counter = 1_usize;
    let mut level1_length_buffer_counter = 0_usize;

    for (i, &geometry_type) in geometry_types.iter().enumerate() {
        let num_geometries = (root_offset_buffer[i + 1] - root_offset_buffer[i]) as usize;

        if geometry_type == GeometryType::MultiPolygon
            || geometry_type == GeometryType::Polygon
            || (is_line_string_present
                && (geometry_type == GeometryType::MultiLineString
                    || geometry_type == GeometryType::LineString))
        {
            // For MultiPolygon, Polygon and in some cases for MultiLineString and LineString
            // a value in the level1LengthBuffer exists
            for _j in 0..num_geometries {
                previous_offset += level1_length_buffer[level1_length_buffer_counter];
                level1_length_buffer_counter += 1;
                level1_buffer_offsets.push(previous_offset);
                level1_buffer_counter += 1;
            }
        } else {
            // For MultiPoint and Point and in some cases for MultiLineString and LineString
            // no value in the level1LengthBuffer exists
            for _j in 0..num_geometries {
                previous_offset += 1;
                level1_buffer_offsets.push(previous_offset);
                level1_buffer_counter += 1;
            }
        }
    }

    level1_buffer_offsets
}

pub fn decode_level2_length_stream(
    geometry_types: &[GeometryType],
    root_offset_buffer: &[u32],
    level1_offset_buffer: &[u32],
    level2_length_buffer: &[u32],
) -> Vec<u32> {
    let final_size = level1_offset_buffer[level1_offset_buffer.len() - 1] as usize + 1;
    let mut level2_buffer_offsets = Vec::with_capacity(final_size);
    level2_buffer_offsets.push(0);
    let mut previous_offset = 0_u32;
    let mut level1_offset_buffer_counter = 1_usize;
    let mut level2_offset_buffer_counter = 1_usize;
    let mut level2_length_buffer_counter = 0_usize;

    for (i, &geometry_type) in geometry_types.iter().enumerate() {
        let num_geometries = (root_offset_buffer[i + 1] - root_offset_buffer[i]) as usize;

        if geometry_type != GeometryType::Point && geometry_type != GeometryType::MultiPoint {
            // For MultiPolygon, MultiLineString, Polygon and LineString a value in level2LengthBuffer
            // exists
            for _j in 0..num_geometries {
                let num_parts = (level1_offset_buffer[level1_offset_buffer_counter]
                    - level1_offset_buffer[level1_offset_buffer_counter - 1])
                    as usize;
                level1_offset_buffer_counter += 1;
                for _k in 0..num_parts {
                    previous_offset += level2_length_buffer[level2_length_buffer_counter];
                    level2_length_buffer_counter += 1;
                    level2_buffer_offsets.push(previous_offset);
                    level2_offset_buffer_counter += 1;
                }
            }
        } else {
            // For MultiPoint and Point no value in level2LengthBuffer exists
            for _j in 0..num_geometries {
                previous_offset += 1;
                level2_buffer_offsets.push(previous_offset);
                level2_offset_buffer_counter += 1;
                level1_offset_buffer_counter += 1;
            }
        }
    }

    level2_buffer_offsets
}
