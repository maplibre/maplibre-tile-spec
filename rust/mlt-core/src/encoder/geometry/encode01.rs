//! Geometry section writer for tag `0x01` (v1) layers.

use super::model::VertexBufferType;
use super::streams::{
    dict_may_be_beneficial, encode_hilbert_vertex_streams, encode_level1_length_stream,
    encode_level1_without_ring_buffer_length_stream, encode_level2_length_stream,
    encode_morton_vertex_streams, encode_ring_lengths_for_mixed, encode_root_length_stream,
    encode_vec2_vertex_stream, normalize_geometry_offsets, normalize_part_offsets_for_rings,
    seed_curve_caches, write_geo_u32_stream,
};
use crate::MltResult;
use crate::decoder::GeometryType::{LineString, Point, Polygon};
use crate::decoder::{
    ColumnType, GeometryType, GeometryValues, LengthType, OffsetType, StreamType,
};
use crate::encoder::model::StreamCtx;
use crate::encoder::{Codecs, Encoder};

impl GeometryValues {
    /// Write the geometry column to `enc`.
    #[hotpath::measure]
    pub fn write_to(self, enc: &mut Encoder, codecs: &mut Codecs) -> MltResult<()> {
        let Self {
            vector_types,
            geometry_offsets,
            part_offsets,
            ring_offsets,
            index_buffer,
            triangles,
            vertices,
        } = self;

        // Flatten every Option<Vec> -> Vec  (empty == not present).
        // triangles: None means no tessellation; Some([]) can't occur in practice (each
        // push_geom appends a count), so empty == absent is safe here too.
        // vertices: None means no coordinate data (e.g. empty layer).
        let geom_offsets = geometry_offsets.unwrap_or_default();
        let part_offsets = part_offsets.unwrap_or_default();
        let ring_offsets = ring_offsets.unwrap_or_default();
        let index_buffer = index_buffer.unwrap_or_default();
        let triangles = triangles.unwrap_or_default();
        let vertices = vertices.unwrap_or_default();

        seed_curve_caches(enc, &vertices);

        let meta: Vec<u32> = vector_types.iter().map(|t| *t as u32).collect();

        let part_offsets = if geom_offsets.is_empty()
            && !ring_offsets.is_empty()
            && !part_offsets.is_empty()
            && part_offsets.len() != vector_types.len() + 1
        {
            // Normalize part_offsets when there are no geometry offsets but ring offsets exist.
            normalize_part_offsets_for_rings(&vector_types, &part_offsets, &ring_offsets)
        } else {
            part_offsets
        };

        // Write column type to meta; reserve exactly 1 byte for stream count
        // (geometry never exceeds ~8 streams, always fits in a single varint byte).
        enc.write_column_type(ColumnType::Geometry)?;
        let stream_count_pos = enc.data().len();
        enc.data_mut().push(0); // placeholder - patched below
        let mut n: u8 = 0;

        // Meta stream - always written, even for a zero-feature layer.
        let ctx = StreamCtx::geom(StreamType::Length(LengthType::VarBinary), "meta");
        codecs.write_int_stream(&meta, &ctx, enc)?;
        n += 1;

        // Topology: compute each length stream and write it immediately.
        if !geom_offsets.is_empty() {
            let geom_offsets = if geom_offsets.len() == vector_types.len() + 1 {
                geom_offsets
            } else {
                normalize_geometry_offsets(&vector_types, &geom_offsets)
            };
            let data = encode_root_length_stream(&vector_types, &geom_offsets, Polygon);
            let ctx = StreamCtx::geom(StreamType::Length(LengthType::Geometries), "geometries");
            n += write_geo_u32_stream(&data, ctx, enc, codecs)?;

            // part_offsets is intentionally kept sparse here (polygon-only cumulative
            // ring counts). encode_level1/2_length_stream navigate it with a running
            // part_idx counter that advances only for Polygon/LineString types, which
            // matches the sparse layout. Densifying via normalize_part_offsets_for_rings
            // would insert Point slots and corrupt the counter arithmetic.
            if !part_offsets.is_empty() {
                if ring_offsets.is_empty() {
                    // geom -> parts only (no rings).
                    let data = encode_level1_without_ring_buffer_length_stream(
                        &vector_types,
                        &geom_offsets,
                        &part_offsets,
                    );
                    let ctx = StreamCtx::geom(StreamType::Length(LengthType::Parts), "no_rings");
                    n += write_geo_u32_stream(&data, ctx, enc, codecs)?;
                } else {
                    // Full topology: geom -> parts -> rings.
                    // LineStrings contribute to rings here, not to parts.
                    let data = encode_level1_length_stream(
                        &vector_types,
                        &geom_offsets,
                        &part_offsets,
                        false,
                    );
                    let ctx = StreamCtx::geom(StreamType::Length(LengthType::Parts), "rings");
                    n += write_geo_u32_stream(&data, ctx, enc, codecs)?;

                    let data = encode_level2_length_stream(
                        &vector_types,
                        &geom_offsets,
                        &part_offsets,
                        &ring_offsets,
                    );
                    let ctx = StreamCtx::geom(StreamType::Length(LengthType::Rings), "rings2");
                    n += write_geo_u32_stream(&data, ctx, enc, codecs)?;
                }
            }
        } else if !part_offsets.is_empty() {
            if ring_offsets.is_empty() {
                let data = encode_root_length_stream(&vector_types, &part_offsets, Point);
                let ctx = StreamCtx::geom(StreamType::Length(LengthType::Parts), "no_rings");
                n += write_geo_u32_stream(&data, ctx, enc, codecs)?;
            } else {
                // No Multi* types; parts -> rings (Polygon / mixed Point+Polygon).
                // Java writes an empty GEOMETRIES stream here for tessellated polygons; only do
                // so when explicitly forced (e.g. to preserve byte-for-byte Java compatibility).
                let ctx = StreamCtx::geom(StreamType::Length(LengthType::Geometries), "geometries");
                n += write_geo_u32_stream(&[], ctx, enc, codecs)?;

                let data = encode_root_length_stream(&vector_types, &part_offsets, LineString);
                let ctx = StreamCtx::geom(StreamType::Length(LengthType::Parts), "parts");
                n += write_geo_u32_stream(&data, ctx, enc, codecs)?;

                // part_offs is a dense N+1 array (one slot per geometry incl. Points);
                // ring_offs stores vertex offsets per slot.  The dense-aware helper skips
                // Point slots by index rather than a running counter.
                let has_line_string = vector_types
                    .iter()
                    .copied()
                    .any(GeometryType::is_linestring);
                let data = encode_ring_lengths_for_mixed(
                    &vector_types,
                    &part_offsets,
                    &ring_offsets,
                    has_line_string,
                );
                let ctx = StreamCtx::geom(StreamType::Length(LengthType::Rings), "parts_ring");
                n += write_geo_u32_stream(&data, ctx, enc, codecs)?;
            }
        }

        let ctx = StreamCtx::geom(StreamType::Length(LengthType::Triangles), "triangles");
        n += write_geo_u32_stream(&triangles, ctx, enc, codecs)?;
        let ctx = StreamCtx::geom(StreamType::Offset(OffsetType::Index), "triangles_indexes");
        n += write_geo_u32_stream(&index_buffer, ctx, enc, codecs)?;

        if let Some(forced) = enc.override_vertex_buffer_type() {
            n += match forced {
                VertexBufferType::Vec2 => encode_vec2_vertex_stream(&vertices, enc, codecs)?,
                VertexBufferType::Morton => encode_morton_vertex_streams(&vertices, enc, codecs)?,
                VertexBufferType::Hilbert => encode_hilbert_vertex_streams(&vertices, enc, codecs)?,
            };
        } else if dict_may_be_beneficial(&vertices, enc) {
            // Morton fits (the gate above ensures it), so race all three.
            let mut winner_size: usize = usize::MAX;
            let mut winner_stream_cnt: u8 = 0;
            let mut alt = enc.try_alternatives();
            alt.with(|e| {
                let ds = e.data().len();
                let ms = e.meta().len();
                winner_stream_cnt = encode_vec2_vertex_stream(&vertices, e, codecs)?;
                winner_size = (e.data().len() - ds) + (e.meta().len() - ms);
                Ok(())
            })?;
            alt.with(|e| {
                let ds = e.data().len();
                let ms = e.meta().len();
                let cnt = encode_hilbert_vertex_streams(&vertices, e, codecs)?;
                let size = (e.data().len() - ds) + (e.meta().len() - ms);
                if size < winner_size {
                    winner_stream_cnt = cnt;
                    winner_size = size;
                }
                Ok(())
            })?;
            alt.with(|e| {
                let ds = e.data().len();
                let ms = e.meta().len();
                let cnt = encode_morton_vertex_streams(&vertices, e, codecs)?;
                let size = (e.data().len() - ds) + (e.meta().len() - ms);
                if size < winner_size {
                    winner_stream_cnt = cnt;
                }
                Ok(())
            })?;
            drop(alt);
            n += winner_stream_cnt;
        } else {
            n += encode_vec2_vertex_stream(&vertices, enc, codecs)?;
        }

        // Patch the reserved stream-count byte.
        debug_assert!(n <= 127, "geometry stream count must fit in one byte");
        enc.data_mut()[stream_count_pos] = n;
        Ok(())
    }
}
