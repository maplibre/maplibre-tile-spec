use crate::lazy_state::Decode;
use crate::utils::{AsUsize as _, SetOptionOnce as _};
use crate::v01::{
    DictionaryType, GeometryType, GeometryValues, LengthType, OffsetType, RawGeometry, RawStream,
    StreamType,
};
use crate::{Decoder, MltError, MltResult};

pub fn decode_geometry_types(
    meta: RawStream<'_>,
    dec: &mut Decoder,
) -> MltResult<Vec<GeometryType>> {
    // TODO: simplify this, e.g. use u8 or even GeometryType directly rather than going via Vec<u32>
    let vector_types: Vec<u32> = meta.decode_u32s(dec)?;
    let vector_types: Vec<GeometryType> = vector_types
        .into_iter()
        .map::<MltResult<GeometryType>, _>(|v| Ok(u8::try_from(v)?.try_into()?))
        .collect::<Result<_, _>>()?;
    Ok(vector_types)
}

/// Handle the parsing of the different topology length buffers separate not generic to reduce the
/// branching and improve the performance
pub fn decode_root_length_stream(
    geometry_types: &[GeometryType],
    root_length_stream: &[u32],
    buffer_id: GeometryType,
    dec: &mut Decoder,
) -> MltResult<Vec<u32>> {
    let alloc_size = geometry_types.len() + 1;
    let mut root_buffer_offsets = dec.alloc(alloc_size)?;

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

    dec.adjust_alloc(&root_buffer_offsets, alloc_size);
    Ok(root_buffer_offsets)
}

/// Case where no ring buffer exists so no `MultiPolygon` or `Polygon` geometry is part of the buffer
pub fn decode_level1_without_ring_buffer_length_stream(
    geometry_types: &[GeometryType],
    root_offset_buffer: &[u32],
    level1_length_buffer: &[u32],
    dec: &mut Decoder,
) -> MltResult<Vec<u32>> {
    let alloc_size = root_offset_buffer[root_offset_buffer.len() - 1].as_usize() + 1;
    let mut level1_buffer_offsets = dec.alloc(alloc_size)?;
    level1_buffer_offsets.push(0);
    let mut previous_offset = 0_u32;
    let mut level1_length_counter = 0_usize;

    for (i, &geometry_type) in geometry_types.iter().enumerate() {
        let num_geometries = (root_offset_buffer[i + 1] - root_offset_buffer[i]).as_usize();

        if geometry_type.is_linestring() {
            // For MultiLineString and LineString a value in the level1LengthBuffer exists
            for _j in 0..num_geometries {
                previous_offset += level1_length_buffer[level1_length_counter];
                level1_length_counter += 1;
                level1_buffer_offsets.push(previous_offset);
            }
        } else {
            // For MultiPoint and Point no value in level1LengthBuffer exists
            for _j in 0..num_geometries {
                previous_offset += 1;
                level1_buffer_offsets.push(previous_offset);
            }
        }
    }

    dec.adjust_alloc(&level1_buffer_offsets, alloc_size);
    Ok(level1_buffer_offsets)
}

pub fn decode_level1_length_stream(
    geometry_types: &[GeometryType],
    root_offset_buffer: &[u32],
    level1_length_buffer: &[u32],
    is_line_string_present: bool,
    dec: &mut Decoder,
) -> MltResult<Vec<u32>> {
    let alloc_size = root_offset_buffer[root_offset_buffer.len() - 1].as_usize() + 1;
    let mut level1_buffer_offsets = dec.alloc(alloc_size)?;
    level1_buffer_offsets.push(0);
    let mut previous_offset = 0_u32;
    let mut level1_length_buffer_counter = 0_usize;

    for (i, &geometry_type) in geometry_types.iter().enumerate() {
        let num_geometries = (root_offset_buffer[i + 1] - root_offset_buffer[i]).as_usize();

        if geometry_type.is_polygon() || (is_line_string_present && geometry_type.is_linestring()) {
            // For MultiPolygon, Polygon and in some cases for MultiLineString and LineString
            // a value in the level1LengthBuffer exists
            for _j in 0..num_geometries {
                previous_offset += level1_length_buffer[level1_length_buffer_counter];
                level1_length_buffer_counter += 1;
                level1_buffer_offsets.push(previous_offset);
            }
        } else {
            // For MultiPoint and Point and in some cases for MultiLineString and LineString
            // no value in the level1LengthBuffer exists
            for _j in 0..num_geometries {
                previous_offset += 1;
                level1_buffer_offsets.push(previous_offset);
            }
        }
    }

    dec.adjust_alloc(&level1_buffer_offsets, alloc_size);
    Ok(level1_buffer_offsets)
}

pub fn decode_level2_length_stream(
    geometry_types: &[GeometryType],
    root_offset_buffer: &[u32],
    level1_offset_buffer: &[u32],
    level2_length_buffer: &[u32],
    dec: &mut Decoder,
) -> MltResult<Vec<u32>> {
    let alloc_size = level1_offset_buffer[level1_offset_buffer.len() - 1].as_usize() + 1;
    let mut level2_buffer_offsets = dec.alloc(alloc_size)?;
    level2_buffer_offsets.push(0);
    let mut previous_offset = 0_u32;
    let mut level1_offset_buffer_counter = 1_usize;
    let mut level2_length_buffer_counter = 0_usize;

    for (i, &geometry_type) in geometry_types.iter().enumerate() {
        let num_geometries = (root_offset_buffer[i + 1] - root_offset_buffer[i]).as_usize();

        if geometry_type != GeometryType::Point && geometry_type != GeometryType::MultiPoint {
            // For MultiPolygon, MultiLineString, Polygon and LineString a value in level2LengthBuffer
            // exists
            for _j in 0..num_geometries {
                let num_parts = (level1_offset_buffer[level1_offset_buffer_counter]
                    - level1_offset_buffer[level1_offset_buffer_counter - 1])
                    .as_usize();
                level1_offset_buffer_counter += 1;
                for _k in 0..num_parts {
                    previous_offset += level2_length_buffer[level2_length_buffer_counter];
                    level2_length_buffer_counter += 1;
                    level2_buffer_offsets.push(previous_offset);
                }
            }
        } else {
            // For MultiPoint and Point no value in level2LengthBuffer exists
            for _j in 0..num_geometries {
                previous_offset += 1;
                level2_buffer_offsets.push(previous_offset);
                level1_offset_buffer_counter += 1;
            }
        }
    }

    dec.adjust_alloc(&level2_buffer_offsets, alloc_size);
    Ok(level2_buffer_offsets)
}

impl Decode<GeometryValues> for RawGeometry<'_> {
    fn decode(self, decoder: &mut Decoder) -> MltResult<GeometryValues> {
        RawGeometry::decode(self, decoder)
    }
}

impl RawGeometry<'_> {
    /// Decode into [`GeometryValues`], charging `dec` before each `Vec<T>`
    /// allocation.  All streams carry `num_values` in their metadata so every
    /// charge is pre-hoc.
    pub fn decode(self, dec: &mut Decoder) -> MltResult<GeometryValues> {
        let RawGeometry { meta, items } = self;
        let vector_types = decode_geometry_types(meta, dec)?;
        let mut geometry_offsets: Option<Vec<u32>> = None;
        let mut part_offsets: Option<Vec<u32>> = None;
        let mut ring_offsets: Option<Vec<u32>> = None;
        let mut vertex_offsets: Option<Vec<u32>> = None;
        let mut index_buffer: Option<Vec<u32>> = None;
        let mut triangles: Option<Vec<u32>> = None;
        let mut vertices: Option<Vec<i32>> = None;

        for stream in items {
            match stream.meta.stream_type {
                StreamType::Present => {}
                StreamType::Data(v) => match v {
                    DictionaryType::Vertex | DictionaryType::Morton => {
                        vertices.set_once(stream.decode_i32s(dec)?)?;
                    }
                    _ => Err(MltError::UnexpectedStreamType(stream.meta.stream_type))?,
                },
                StreamType::Offset(v) => {
                    let target = match v {
                        OffsetType::Vertex => &mut vertex_offsets,
                        OffsetType::Index => &mut index_buffer,
                        _ => Err(MltError::UnexpectedStreamType(stream.meta.stream_type))?,
                    };
                    target.set_once(stream.decode_u32s(dec)?)?;
                }
                StreamType::Length(v) => {
                    let target = match v {
                        LengthType::Geometries => &mut geometry_offsets,
                        LengthType::Parts => &mut part_offsets,
                        LengthType::Rings => &mut ring_offsets,
                        LengthType::Triangles => &mut triangles,
                        _ => Err(MltError::UnexpectedStreamType(stream.meta.stream_type))?,
                    };
                    target.set_once(stream.decode_u32s(dec)?)?;
                }
            }
        }

        if index_buffer.is_some() && part_offsets.is_none() {
            // Case when the indices of a Polygon outline are not encoded in the data so no
            // topology data are present in the tile
            //
            // return FlatGpuVector::new(vector_types, triangles, index_buffer, vertices);
            return Err(MltError::NotImplemented(
                "index_buffer.is_some() && part_offsets.is_none() case",
            ));
        }

        // Use decode_root_length_stream if geometry_offsets is present
        if let Some(offsets) = geometry_offsets.take() {
            geometry_offsets = Some(decode_root_length_stream(
                &vector_types,
                &offsets,
                GeometryType::Polygon,
                dec,
            )?);
            if let Some(part_offsets_copy) = part_offsets.take() {
                if let Some(ring_offsets_copy) = ring_offsets.take() {
                    part_offsets = Some(decode_level1_length_stream(
                        &vector_types,
                        geometry_offsets.as_ref().unwrap(),
                        &part_offsets_copy,
                        false, // isLineStringPresent
                        dec,
                    )?);
                    ring_offsets = Some(decode_level2_length_stream(
                        &vector_types,
                        geometry_offsets.as_ref().unwrap(),
                        part_offsets.as_ref().unwrap(),
                        &ring_offsets_copy,
                        dec,
                    )?);
                } else {
                    part_offsets = Some(decode_level1_without_ring_buffer_length_stream(
                        &vector_types,
                        geometry_offsets.as_ref().unwrap(),
                        &part_offsets_copy,
                        dec,
                    )?);
                }
            }
        } else if let Some(offsets) = part_offsets.take() {
            if let Some(ring_offsets_copy) = ring_offsets.take() {
                let is_line_string_present = vector_types.iter().any(|t| t.is_linestring());
                part_offsets = Some(decode_root_length_stream(
                    &vector_types,
                    &offsets,
                    GeometryType::LineString,
                    dec,
                )?);
                ring_offsets = Some(decode_level1_length_stream(
                    &vector_types,
                    part_offsets.as_ref().unwrap(),
                    &ring_offsets_copy,
                    is_line_string_present,
                    dec,
                )?);
            } else {
                part_offsets = Some(decode_root_length_stream(
                    &vector_types,
                    &offsets,
                    GeometryType::Point,
                    dec,
                )?);
            }
        }

        // Case when the indices of a Polygon outline are encoded in the tile
        // This is handled by including index_buffer in the GeometryValues

        // Expand vertex dictionary:
        // If a vertex offset stream was present,
        // - `vertices` holds only the unique dictionary entries and
        // - `vertex_offsets` holds per-vertex indices into it.
        //
        // Expand them into a single flat (x, y) sequence so that `GeometryValues` always
        // represents fully decoded data, regardless of the encoding that was used.
        if let Some(offsets) = vertex_offsets.take()
            && let Some(dict) = vertices.as_deref()
        {
            dec.consume_items::<[i32; 2]>(offsets.len())?;
            vertices = Some(
                offsets
                    .iter()
                    .flat_map(|&i| {
                        let i = i.as_usize();
                        [dict[i * 2], dict[i * 2 + 1]]
                    })
                    .collect(),
            );
        }

        Ok(GeometryValues {
            vector_types,
            geometry_offsets,
            part_offsets,
            ring_offsets,
            index_buffer,
            triangles,
            vertices,
        })
    }
}
