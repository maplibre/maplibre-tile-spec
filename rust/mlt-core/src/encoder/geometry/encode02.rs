//! Geometry section writer for tag `0x02` (v2) layers.

use super::model::VertexBufferType;
use super::streams::{
    dict_may_be_beneficial, encode_hilbert_vertex_streams02, encode_level1_length_stream,
    encode_level1_without_ring_buffer_length_stream, encode_level2_length_stream,
    encode_morton_vertex_streams02, encode_ring_lengths_for_mixed, encode_root_length_stream,
    encode_vec2_vertex_stream, normalize_geometry_offsets, normalize_part_offsets_for_rings,
    seed_curve_caches,
};
use crate::MltResult;
use crate::decoder::GeometryType::{LineString, Point, Polygon};
use crate::decoder::stream::header02::Family;
use crate::decoder::{
    GeoLayout, GeometryType, GeometryValues, LengthType, OffsetType, StreamType, VertexStorage,
};
use crate::encoder::model::StreamCtx;
use crate::encoder::{Codecs, Encoder};

/// Wrap a computed length stream: an empty stream is not written (and not
/// declared by the layout), matching the v1 writer's skip-empty behavior.
fn non_empty(data: Vec<u32>) -> Option<Vec<u32>> {
    if data.is_empty() { None } else { Some(data) }
}

/// The triangle streams of a tessellated layer, which precede its vertices.
struct Tessellation {
    triangles: Vec<u32>,
    index_buffer: Vec<u32>,
}

/// The streams of a v2 geometry section, before the layout byte that declares them is settled.
pub(crate) struct GeometrySection02 {
    types: Vec<u32>,
    geo_lengths: Option<Vec<u32>>,
    part_lengths: Option<Vec<u32>>,
    ring_lengths: Option<Vec<u32>>,
    tessellation: Option<Tessellation>,
    vertices: Vec<i32>,
}

/// Turn a layer's geometries into the v2 stream set.
pub(crate) fn encode_geometry02(geometry: GeometryValues) -> MltResult<GeometrySection02> {
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
    let triangles = triangles.unwrap_or_default();
    let index_buffer = index_buffer.unwrap_or_default();

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

    // An empty triangle stream means no polygon was tessellated, so the layer keeps
    // a plain layout. `index_buffer` can be empty next to a non-empty `triangles`
    // (a polygon that earcut turned into no triangle at all), and is written anyway
    // because the tessellated layouts declare both streams.
    let tessellation = (!triangles.is_empty()).then_some(Tessellation {
        triangles,
        index_buffer,
    });
    // The tessellated layouts declare all three topology streams or none, so a layer
    // without Multi* geometries fills the gap with an empty one rather than dropping
    // it: the decoder then rebuilds one geometry per feature, which is what the
    // stream would have said.
    if tessellation.is_some() && geo_lengths.is_none() && part_lengths.is_some() {
        geo_lengths = Some(Vec::new());
    }

    let section = GeometrySection02 {
        types: vector_types.iter().map(|t| *t as u32).collect(),
        geo_lengths,
        part_lengths,
        ring_lengths,
        tessellation,
        vertices,
    };
    // Reject a topology that has no layout before any of it is written.
    section.layout(section.fallback_storage())?;
    Ok(section)
}

impl GeometrySection02 {
    /// The layout declaring these streams, once the vertex storage is known.
    fn layout(&self, vertices: VertexStorage) -> MltResult<GeoLayout> {
        GeoLayout::from_streams(
            self.geo_lengths.is_some(),
            self.part_lengths.is_some(),
            self.ring_lengths.is_some(),
            vertices,
        )
    }

    /// How the vertices are stored unless a dictionary layout wins the race.
    fn fallback_storage(&self) -> VertexStorage {
        if self.tessellation.is_some() {
            VertexStorage::Tessellated
        } else {
            VertexStorage::Plain
        }
    }

    /// Write the geometry streams to `enc` and return the [`GeoLayout`] declaring them.
    ///
    /// The layout is only known once the vertex streams are written, since the
    /// dictionary layouts are picked by writing them and keeping the shortest, so
    /// the caller patches the layout byte it reserved before this call.
    ///
    /// Expects `enc.count_context` to hold the layer's `feature_count`.
    pub(crate) fn write_to(self, enc: &mut Encoder, codecs: &mut Codecs) -> MltResult<GeoLayout> {
        // Types and length streams are integer streams, only the vertex streams have their own family.
        enc.family_context = Family::Int;

        // Types stream: implicit count = feature_count (the current count context).
        let ctx = StreamCtx::geom(StreamType::Length(LengthType::VarBinary), "meta");
        codecs.write_int_stream(&self.types, &ctx, enc)?;

        let lengths = [
            (&self.geo_lengths, LengthType::Geometries, "geometries"),
            (&self.part_lengths, LengthType::Parts, "parts"),
            (&self.ring_lengths, LengthType::Rings, "rings"),
        ];
        for (stream, length_type, name) in lengths {
            if let Some(data) = stream {
                let ctx = StreamCtx::geom(StreamType::Length(length_type), name);
                codecs.write_int_stream(data, &ctx, enc)?;
            }
        }

        if let Some(tess) = &self.tessellation {
            let ctx = StreamCtx::geom(StreamType::Length(LengthType::Triangles), "triangles");
            codecs.write_int_stream(&tess.triangles, &ctx, enc)?;
            let ctx = StreamCtx::geom(StreamType::Offset(OffsetType::Index), "triangles_indexes");
            codecs.write_int_stream(&tess.index_buffer, &ctx, enc)?;
        }

        let vertices = write_vertices(&self.vertices, self.fallback_storage(), enc, codecs)?;
        self.layout(vertices)
    }
}

/// Write the vertex streams and report the storage they used.
///
/// Tessellated layers keep their vertices plain: no layout code pairs a triangle
/// buffer with a vertex dictionary.
fn write_vertices(
    vertices: &[i32],
    fallback: VertexStorage,
    enc: &mut Encoder,
    codecs: &mut Codecs,
) -> MltResult<VertexStorage> {
    seed_curve_caches(enc, vertices);
    enc.family_context = Family::Vertex;

    if fallback == VertexStorage::Tessellated {
        encode_vec2_vertex_stream(vertices, enc, codecs)?;
        return Ok(VertexStorage::Tessellated);
    }

    if let Some(forced) = enc.override_vertex_buffer_type() {
        return Ok(match forced {
            VertexBufferType::Vec2 => {
                encode_vec2_vertex_stream(vertices, enc, codecs)?;
                VertexStorage::Plain
            }
            VertexBufferType::Morton => {
                encode_morton_vertex_streams02(vertices, enc, codecs)?;
                VertexStorage::Dict
            }
            VertexBufferType::Hilbert => {
                encode_hilbert_vertex_streams02(vertices, enc, codecs)?;
                VertexStorage::Dict
            }
        });
    }

    if !dict_may_be_beneficial(vertices, enc) {
        encode_vec2_vertex_stream(vertices, enc, codecs)?;
        return Ok(VertexStorage::Plain);
    }

    // Morton fits (the gate above ensures it), so race all three layouts and keep
    // the shortest, as the v1 writer does.
    let mut winner = VertexStorage::Plain;
    let mut winner_size = usize::MAX;
    let mut alt = enc.try_alternatives();
    let mut candidate = |storage: VertexStorage,
                         write: &dyn Fn(&mut Encoder, &mut Codecs) -> MltResult<()>|
     -> MltResult<()> {
        alt.with(|enc| {
            let (data, meta) = (enc.data().len(), enc.meta().len());
            enc.family_context = Family::Vertex;
            write(enc, codecs)?;
            let size = (enc.data().len() - data) + (enc.meta().len() - meta);
            if size < winner_size {
                winner = storage;
                winner_size = size;
            }
            Ok(())
        })
    };
    candidate(VertexStorage::Plain, &|enc, codecs| {
        encode_vec2_vertex_stream(vertices, enc, codecs).map(|_| ())
    })?;
    candidate(VertexStorage::Dict, &|enc, codecs| {
        encode_hilbert_vertex_streams02(vertices, enc, codecs)
    })?;
    candidate(VertexStorage::Dict, &|enc, codecs| {
        encode_morton_vertex_streams02(vertices, enc, codecs)
    })?;
    drop(alt);
    Ok(winner)
}
