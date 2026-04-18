use std::fmt::Debug;

use crate::codecs::varint::parse_varint;
use crate::decoder::{
    DictionaryType, GeometryType, GeometryValues, IntEncoding, LengthType, OffsetType, RawGeometry,
    RawStream, StreamMeta, StreamType,
};
use crate::errors::AsMltError as _;
use crate::utils::{AsUsize as _, OptSeq, SetOptionOnce as _};
use crate::{Decode, Decoder, MltError, MltResult, Parser};

/// Advance `offset` by `count` and extend `buffer` with the consecutive values
/// `old_offset + 1, old_offset + 2, …, new_offset`.
fn push_consecutive_offsets(
    buffer: &mut Vec<u32>,
    offset: &mut u32,
    count: usize,
) -> MltResult<()> {
    if count > 0 {
        let count = u32::try_from(count).or_overflow()?;
        *offset = offset.checked_add(count).or_overflow()?;
        // Safety: offset+1 cannot overflow because offset+count didn't and count >= 1.
        buffer.extend((*offset - count + 1)..=*offset);
    }
    Ok(())
}

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
    let alloc_size = geometry_types.len().checked_add(1).or_overflow()?;
    let mut root_buffer_offsets = dec.alloc(alloc_size)?;

    root_buffer_offsets.push(0);
    let mut offset = 0_u32;
    let mut root_length_counter = 0_usize;
    for &geom_type in geometry_types {
        let increment = if geom_type > buffer_id {
            let val = *root_length_stream
                .get(root_length_counter)
                .ok_or(MltError::GeometryIndexOutOfBounds(root_length_counter))?;
            // Safety: counter bounded by `geometry_types.len()`, which fits in usize.
            root_length_counter += 1;
            val
        } else {
            1
        };
        offset = offset.checked_add(increment).or_overflow()?;
        root_buffer_offsets.push(offset);
    }

    dec.adjust_alloc(&root_buffer_offsets, alloc_size)?;
    Ok(root_buffer_offsets)
}

/// Case where no ring buffer exists so no `MultiPolygon` or `Polygon` geometry is part of the buffer
pub fn decode_level1_without_ring_buffer_length_stream(
    geometry_types: &[GeometryType],
    root_offset_buffer: &[u32],
    level1_length_buffer: &[u32],
    dec: &mut Decoder,
) -> MltResult<Vec<u32>> {
    // Safety: root_offset_buffer is produced by decode_root_length_stream, which always
    // pushes an initial 0, so it is never empty.
    let alloc_size = root_offset_buffer[root_offset_buffer.len() - 1]
        .as_usize()
        .checked_add(1)
        .or_overflow()?;
    let mut level1_buffer_offsets = dec.alloc(alloc_size)?;
    level1_buffer_offsets.push(0);
    let mut offset = 0_u32;
    let mut level1_length_counter = 0_usize;

    for (&geometry_type, w) in geometry_types.iter().zip(root_offset_buffer.windows(2)) {
        let num_geometries = w[1].checked_sub(w[0]).or_overflow()?.as_usize();

        if geometry_type.is_linestring() {
            // For MultiLineString and LineString a value in the level1LengthBuffer exists
            for _j in 0..num_geometries {
                let val = *level1_length_buffer
                    .get(level1_length_counter)
                    .ok_or(MltError::GeometryIndexOutOfBounds(level1_length_counter))?;
                offset = offset.checked_add(val).or_overflow()?;
                level1_buffer_offsets.push(offset);
                // Safety: counter bounded by slice lengths, which fit in usize.
                level1_length_counter += 1;
            }
        } else {
            // For MultiPoint and Point no value in level1LengthBuffer exists
            push_consecutive_offsets(&mut level1_buffer_offsets, &mut offset, num_geometries)?;
        }
    }

    dec.adjust_alloc(&level1_buffer_offsets, alloc_size)?;
    Ok(level1_buffer_offsets)
}

pub fn decode_level1_length_stream(
    geometry_types: &[GeometryType],
    root_offset_buffer: &[u32],
    level1_length_buffer: &[u32],
    is_line_string_present: bool,
    dec: &mut Decoder,
) -> MltResult<Vec<u32>> {
    // Safety: root_offset_buffer is produced by decode_root_length_stream, which always
    // pushes an initial 0, so it is never empty.
    let alloc_size = root_offset_buffer[root_offset_buffer.len() - 1]
        .as_usize()
        .checked_add(1)
        .or_overflow()?;
    let mut level1_buffer_offsets = dec.alloc(alloc_size)?;
    level1_buffer_offsets.push(0);
    let mut offset = 0_u32;
    let mut level1_length_buffer_counter = 0_usize;

    for (&geometry_type, w) in geometry_types.iter().zip(root_offset_buffer.windows(2)) {
        let num_geometries = w[1].checked_sub(w[0]).or_overflow()?.as_usize();

        if geometry_type.is_polygon() || (is_line_string_present && geometry_type.is_linestring()) {
            // For MultiPolygon, Polygon and in some cases for MultiLineString and LineString
            // a value in the level1LengthBuffer exists
            for _j in 0..num_geometries {
                let val = *level1_length_buffer
                    .get(level1_length_buffer_counter)
                    .ok_or(MltError::GeometryIndexOutOfBounds(
                        level1_length_buffer_counter,
                    ))?;
                offset = offset.checked_add(val).or_overflow()?;
                level1_buffer_offsets.push(offset);
                // Safety: counter bounded by slice lengths, which fit in usize.
                level1_length_buffer_counter += 1;
            }
        } else {
            // For MultiPoint and Point and in some cases for MultiLineString and LineString
            // no value in the level1LengthBuffer exists
            push_consecutive_offsets(&mut level1_buffer_offsets, &mut offset, num_geometries)?;
        }
    }

    dec.adjust_alloc(&level1_buffer_offsets, alloc_size)?;
    Ok(level1_buffer_offsets)
}

pub fn decode_level2_length_stream(
    geometry_types: &[GeometryType],
    root_offset_buffer: &[u32],
    level1_offset_buffer: &[u32],
    level2_length_buffer: &[u32],
    dec: &mut Decoder,
) -> MltResult<Vec<u32>> {
    // Safety: level1_offset_buffer is produced by decode_level1_*_length_stream, which
    // always pushes an initial 0, so it is never empty.
    let last = level1_offset_buffer[level1_offset_buffer.len() - 1];
    let alloc_size = last.as_usize().checked_add(1).or_overflow()?;
    let mut level2_buffer_offsets = dec.alloc(alloc_size)?;
    level2_buffer_offsets.push(0);
    let mut previous_offset = 0_u32;
    let mut level1_tail = level1_offset_buffer;
    let mut level2_pos = 0_usize;

    for (&geometry_type, w) in geometry_types.iter().zip(root_offset_buffer.windows(2)) {
        let num_geometries = w[1].checked_sub(w[0]).or_overflow()?.as_usize();

        if geometry_type != GeometryType::Point && geometry_type != GeometryType::MultiPoint {
            // For MultiPolygon, MultiLineString, Polygon and LineString a value in level2LengthBuffer
            // exists
            for _j in 0..num_geometries {
                let [base, next, ..] = *level1_tail else {
                    return Err(MltError::IntegerOverflow);
                };
                let num_parts = next.checked_sub(base).or_overflow()?.as_usize();
                level1_tail = &level1_tail[1..];
                for _k in 0..num_parts {
                    let val = *level2_length_buffer
                        .get(level2_pos)
                        .ok_or(MltError::GeometryIndexOutOfBounds(level2_pos))?;
                    previous_offset = previous_offset.checked_add(val).or_overflow()?;
                    // Safety: counter bounded by slice lengths, which fit in usize.
                    level2_pos += 1;
                    level2_buffer_offsets.push(previous_offset);
                }
            }
        } else {
            // For MultiPoint and Point no value in level2LengthBuffer exists
            push_consecutive_offsets(
                &mut level2_buffer_offsets,
                &mut previous_offset,
                num_geometries,
            )?;
            if num_geometries > level1_tail.len() {
                return Err(MltError::IntegerOverflow);
            }
            level1_tail = &level1_tail[num_geometries..];
        }
    }

    dec.adjust_alloc(&level2_buffer_offsets, alloc_size)?;
    Ok(level2_buffer_offsets)
}

impl<'a> RawGeometry<'a> {
    /// Parse encoded geometry from bytes (expects varint stream count + streams).
    /// Reserves decoded memory against the parser's budget.
    pub fn from_bytes(input: &'a [u8], parser: &mut Parser) -> crate::MltRefResult<'a, Self> {
        let (input, stream_count) = parse_varint::<u32>(input)?;
        let stream_count = stream_count.as_usize();
        if stream_count == 0 {
            return Ok((
                input,
                Self {
                    meta: RawStream::new(
                        StreamMeta::new(
                            StreamType::Data(DictionaryType::None),
                            IntEncoding::none(),
                            0,
                        ),
                        &[],
                    ),
                    items: Vec::new(),
                },
            ));
        }

        let (input, meta) = RawStream::from_bytes(input, parser)?;
        // Safety: stream_count is validated != 0
        let (input, items) = RawStream::parse_multiple(input, stream_count - 1, parser)?;

        Ok((input, Self { meta, items }))
    }
}

impl Decode<GeometryValues> for RawGeometry<'_> {
    /// Decode into [`GeometryValues`], charging `dec` before each `Vec<T>`
    /// allocation.  All streams carry `num_values` in their metadata so every
    /// charge is pre-hoc.
    fn decode(self, dec: &mut Decoder) -> MltResult<GeometryValues> {
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
            // SAFETY:
            // Check before multiplying: i < dict_vertex_count guarantees
            // i * 2 + 1 < dict.len() with no risk of overflow, because
            // Rust limits Vec::len() to isize::MAX, so
            // dict_vertex_count <= isize::MAX / 2, meaning
            // i * 2 + 1 <= isize::MAX < usize::MAX.
            let dict_vertex_count = dict.len() / 2;
            vertices = Some(offsets.iter().try_fold(
                Vec::with_capacity(offsets.len() * 2),
                |mut acc, &idx| -> MltResult<_> {
                    let i = idx.as_usize();
                    if i >= dict_vertex_count {
                        return Err(MltError::DictIndexOutOfBounds(idx, dict_vertex_count));
                    }
                    acc.push(dict[i * 2]);
                    acc.push(dict[i * 2 + 1]);
                    Ok(acc)
                },
            )?);
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

impl Debug for GeometryValues {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self {
            vector_types,
            geometry_offsets,
            part_offsets,
            ring_offsets,
            index_buffer,
            triangles,
            vertices,
        } = self;
        f.debug_struct("GeometryValues")
            .field("vector_types", &OptSeq(Some(vector_types)))
            .field("geometry_offsets", &OptSeq(geometry_offsets.as_deref()))
            .field("part_offsets", &OptSeq(part_offsets.as_deref()))
            .field("ring_offsets", &OptSeq(ring_offsets.as_deref()))
            .field("index_buffer", &OptSeq(index_buffer.as_deref()))
            .field("triangles", &OptSeq(triangles.as_deref()))
            .field("vertices", &OptSeq(vertices.as_deref()))
            .finish()
    }
}
