//! Geometry section writer for tag `0x02` (v2) layers.
//!
//! The v2 geometry section is `[geometry_layout u8]` followed by the streams
//! the layout declares, in fixed order: Types, `GeoLengths?`, `PartLengths?`,
//! `RingLengths?`, Vertices. This replaces v1's `stream_count` varint and the
//! per-stream `stream_type` bytes.
//!
//! The topology encoding (offsets → length streams) intentionally mirrors
//! [`GeometryValues::write_to`] branch for branch, so a v2 layer decodes into
//! exactly the same geometry as its v1 counterpart. A computed length stream
//! that comes out empty is not written — the layout simply does not declare
//! it, matching v1's skip-empty behavior.
//!
//! Vertex data is always written as a plain Vec2 stream for now.
// TODO(v2): dictionary (plain / Morton / Hilbert) vertex layouts and
//           tessellation layouts.

use super::encode::{
    encode_level1_length_stream, encode_level1_without_ring_buffer_length_stream,
    encode_level2_length_stream, encode_ring_lengths_for_mixed, encode_root_length_stream,
    encode_vec2_vertex_stream, normalize_geometry_offsets, normalize_part_offsets_for_rings,
};
use crate::decoder::GeometryType::{LineString, Point, Polygon};
use crate::decoder::{GeoLayout, GeometryType, GeometryValues, LengthType, StreamType};
use crate::encoder::model::StreamCtx;
use crate::encoder::{Codecs, Encoder};
use crate::{MltError, MltResult};

/// Wrap a computed length stream: an empty stream is not written (and not
/// declared by the layout), matching the v1 writer's skip-empty behavior.
fn non_empty(data: Vec<u32>) -> Option<Vec<u32>> {
    if data.is_empty() { None } else { Some(data) }
}

/// Write the v2 geometry section to `enc`.
///
/// Expects `enc.count_context` to hold the layer's `feature_count`.
pub(crate) fn write_geometry02(
    geometry: GeometryValues,
    enc: &mut Encoder,
    codecs: &mut Codecs,
) -> MltResult<()> {
    let GeometryValues {
        vector_types,
        geometry_offsets,
        part_offsets,
        ring_offsets,
        index_buffer,
        triangles,
        vertices,
    } = geometry;

    let geom_offsets = geometry_offsets.unwrap_or_default();
    let part_offsets = part_offsets.unwrap_or_default();
    let ring_offsets = ring_offsets.unwrap_or_default();
    let vertices = vertices.unwrap_or_default();

    if !triangles.unwrap_or_default().is_empty() || !index_buffer.unwrap_or_default().is_empty() {
        return Err(MltError::NotImplemented("v2 tessellated geometry layouts"));
    }

    // Same part-offset normalization as the v1 writer.
    let part_offsets = if geom_offsets.is_empty()
        && !ring_offsets.is_empty()
        && !part_offsets.is_empty()
        && part_offsets.len() != vector_types.len() + 1
    {
        normalize_part_offsets_for_rings(&vector_types, &part_offsets, &ring_offsets)
    } else {
        part_offsets
    };

    // Compute the length streams with the same branch structure as the v1
    // writer (`GeometryValues::write_to`); see there for the reasoning behind
    // each case.
    let mut geo_lengths: Option<Vec<u32>> = None;
    let mut part_lengths: Option<Vec<u32>> = None;
    let mut ring_lengths: Option<Vec<u32>> = None;

    if !geom_offsets.is_empty() {
        let geom_offsets = if geom_offsets.len() == vector_types.len() + 1 {
            geom_offsets
        } else {
            normalize_geometry_offsets(&vector_types, &geom_offsets)
        };
        geo_lengths = non_empty(encode_root_length_stream(
            &vector_types,
            &geom_offsets,
            Polygon,
        ));

        if !part_offsets.is_empty() {
            if ring_offsets.is_empty() {
                part_lengths = non_empty(encode_level1_without_ring_buffer_length_stream(
                    &vector_types,
                    &geom_offsets,
                    &part_offsets,
                ));
            } else {
                part_lengths = non_empty(encode_level1_length_stream(
                    &vector_types,
                    &geom_offsets,
                    &part_offsets,
                    false,
                ));
                ring_lengths = non_empty(encode_level2_length_stream(
                    &vector_types,
                    &geom_offsets,
                    &part_offsets,
                    &ring_offsets,
                ));
            }
        }
    } else if !part_offsets.is_empty() {
        if ring_offsets.is_empty() {
            part_lengths = non_empty(encode_root_length_stream(
                &vector_types,
                &part_offsets,
                Point,
            ));
        } else {
            part_lengths = non_empty(encode_root_length_stream(
                &vector_types,
                &part_offsets,
                LineString,
            ));
            let has_line_string = vector_types
                .iter()
                .copied()
                .any(GeometryType::is_linestring);
            ring_lengths = non_empty(encode_ring_lengths_for_mixed(
                &vector_types,
                &part_offsets,
                &ring_offsets,
                has_line_string,
            ));
        }
    }

    let layout = GeoLayout::from_streams(
        geo_lengths.is_some(),
        part_lengths.is_some(),
        ring_lengths.is_some(),
    )?;
    enc.data_mut().push(layout as u8);

    // Types stream: implicit count = feature_count (the current count context).
    let types: Vec<u32> = vector_types.iter().map(|t| *t as u32).collect();
    let ctx = StreamCtx::geom(StreamType::Length(LengthType::VarBinary), "meta");
    codecs.write_int_stream(&types, &ctx, enc)?;

    let lengths = [
        (geo_lengths, LengthType::Geometries, "geometries"),
        (part_lengths, LengthType::Parts, "parts"),
        (ring_lengths, LengthType::Rings, "rings"),
    ];
    for (stream, length_type, name) in lengths {
        if let Some(data) = stream {
            let ctx = StreamCtx::geom(StreamType::Length(length_type), name);
            codecs.write_int_stream(&data, &ctx, enc)?;
        }
    }

    encode_vec2_vertex_stream(&vertices, enc, codecs)?;
    Ok(())
}
