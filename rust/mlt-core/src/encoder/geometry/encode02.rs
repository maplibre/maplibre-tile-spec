//! Geometry section writer for tag `0x02` (v2) layers.

use usize_cast::IntoUsize as _;

use super::streams::{
    dict_may_be_beneficial, encode_hilbert_vertex_streams02, encode_hilbert_xyz_vertex_streams02,
    encode_level1_length_stream, encode_level1_without_ring_buffer_length_stream,
    encode_level2_length_stream, encode_morton_vertex_streams02, encode_rans_vertex_stream02,
    encode_ring_lengths_for_mixed, encode_root_length_stream, encode_vec2_delta2_vertex_stream02,
    encode_vec2_vertex_stream02, encode_vec3_vertex_stream02, normalize_geometry_offsets,
    normalize_part_offsets_for_rings, seed_curve_caches, xyz_dict_may_be_beneficial,
};
use crate::codecs::tri::{self, TriTrees};
use crate::decoder::GeometryType::{LineString, Point, Polygon};
use crate::decoder::stream::header02::{Family, WordWidth};
use crate::decoder::{
    GeoLayout, GeometryType, GeometryValues, LengthType, Levels, OffsetType, StreamType, Topology,
    VertexStorage, decode_topology, run_offsets,
};
use crate::encoder::geometry::model::VertexBufferType;
use crate::encoder::model::StreamCtx;
use crate::encoder::{Codecs, Encoder};
use crate::tile::ZAxis;
use crate::{Decoder, MltError, MltResult};

/// Wrap a computed length stream: an empty stream is not written (and not
/// declared by the layout), matching the v1 writer's skip-empty behavior.
fn non_empty(data: Vec<u32>) -> Option<Vec<u32>> {
    if data.is_empty() { None } else { Some(data) }
}

/// The triangle streams of a tessellated or mesh layer, which precede its vertices.
struct Tessellation {
    triangles: Vec<u32>,
    data: TessData,
}

enum TessData {
    IndexBuffer(Vec<u32>),
    Prebuilt {
        trees: TriTrees,
        vertex_order: Vec<u32>,
    },
}

impl Tessellation {
    /// The triangle trees that store the index buffer, and any mesh vertex permutation.
    fn into_trees(self) -> (Vec<u32>, TriTrees, Vec<u32>) {
        match self.data {
            TessData::Prebuilt {
                trees,
                vertex_order,
            } => (self.triangles, trees, vertex_order),
            TessData::IndexBuffer(index_buffer) => {
                let offsets: Vec<u32> = std::iter::once(0)
                    .chain(self.triangles.iter().scan(0, |total, &n| {
                        *total += n;
                        Some(*total)
                    }))
                    .collect();
                let trees = tri::encode_trees(&index_buffer, &offsets);
                debug_assert!(
                    tri::decode_trees(
                        &trees.symbols,
                        &trees.cycles,
                        &mut Decoder::with_max_size(u32::MAX)
                    )
                    .is_ok_and(|back| tri::same_triangles(
                        &index_buffer,
                        &back,
                        &offsets
                    )),
                    "triangle trees keep each feature's triangles"
                );
                (self.triangles, trees, Vec::new())
            }
        }
    }
}

/// Whether a tessellated layer keeps the outline topology next to its triangles.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Outlines {
    /// Keep every topology stream, as a layer with anything but polygons must.
    Keep,
    /// Drop them when every feature is a polygon, so only the triangles are stored.
    DropForPolygons,
}

/// The streams of a v2 geometry section, before the layout byte that declares them is settled.
pub(crate) struct GeometrySection02 {
    types: Vec<GeometryType>,
    geo_lengths: Option<Vec<u32>>,
    topology: Topology,
    tessellation: Option<Tessellation>,
    vertices: Vec<i32>,
    /// The grid of each vertex's third word, or [`None`] when the vertices are `(x, y)` pairs.
    z_axis: Option<ZAxis>,
    meshes: bool,
}

/// Turn a layer's geometries into the v2 stream set.
pub(crate) fn encode_geometry02(
    geometry: GeometryValues,
    outlines: Outlines,
) -> MltResult<GeometrySection02> {
    let GeometryValues {
        vector_types,
        geometry_offsets,
        part_offsets,
        ring_offsets,
        index_buffer,
        triangle_offsets,
        vertices,
        z_axis,
        meshes,
    } = geometry;

    if meshes || vector_types.contains(&GeometryType::Mesh) {
        return encode_meshes02(
            vector_types,
            geometry_offsets,
            triangle_offsets,
            index_buffer,
            vertices,
            z_axis,
        );
    }

    let geom_offsets = geometry_offsets.unwrap_or_default();
    let part_offsets = part_offsets.unwrap_or_default();
    let ring_offsets = ring_offsets.unwrap_or_default();
    let vertices = vertices.unwrap_or_default();
    let triangles = triangle_offsets
        .unwrap_or_default()
        .windows(2)
        .map(|w| w[1] - w[0])
        .collect();
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

    let mut topology = match (part_lengths, ring_lengths) {
        (None, None) => Topology::Flat,
        (Some(parts), None) => Topology::Parts(parts),
        (Some(parts), Some(rings)) => Topology::PartsAndRings { parts, rings },
        (None, Some(_)) => return Err(MltError::RingLengthsWithoutPartLengths),
    };

    // An empty index buffer means no polygon tessellated into a triangle, so the layer
    // keeps a plain layout: dropping the streams is lossless.
    let tessellation = (!index_buffer.is_empty()).then_some(Tessellation {
        triangles,
        data: TessData::IndexBuffer(index_buffer),
    });
    // The tessellated layouts declare all three topology streams or none, so a layer
    // missing one fills the gap with an empty stream rather than dropping the others:
    // the decoder then rebuilds what the stream would have said.
    if tessellation.is_some() {
        let polygons_only = vector_types.iter().all(|t| t.is_polygon());
        if outlines == Outlines::DropForPolygons && polygons_only {
            topology = Topology::Flat;
            geo_lengths = None;
        } else {
            topology = topology.with_rings();
            geo_lengths.get_or_insert_with(Vec::new);
        }
    }

    Ok(GeometrySection02 {
        types: vector_types,
        geo_lengths,
        topology,
        tessellation,
        vertices,
        z_axis,
        meshes: false,
    })
}

/// The streams of a `Meshes` layer: each feature's vertex count and triangles, then its
/// `(x, y, z)` vertices.
///
/// The layout holds meshes only, and always writes the triangle streams, even when no
/// feature has a triangle.
fn encode_meshes02(
    vector_types: Vec<GeometryType>,
    geometry_offsets: Option<Vec<u32>>,
    triangle_offsets: Option<Vec<u32>>,
    index_buffer: Option<Vec<u32>>,
    vertices: Option<Vec<i32>>,
    z_axis: Option<ZAxis>,
) -> MltResult<GeometrySection02> {
    if vector_types.iter().any(|&t| t != GeometryType::Mesh) {
        return Err(MltError::MixedMeshLayer);
    }
    if z_axis.is_none() {
        return Err(MltError::MeshWithoutZ);
    }
    let geo_offsets = geometry_offsets.unwrap_or_default();
    let tri_offsets = triangle_offsets.unwrap_or_default();
    let index_buffer = index_buffer.unwrap_or_default();
    let mut vertices = vertices.unwrap_or_default();
    let geo_lengths = geo_offsets.windows(2).map(|w| w[1] - w[0]).collect();
    let triangles = tri_offsets.windows(2).map(|w| w[1] - w[0]).collect();

    let mut trees = tri::encode_trees(&index_buffer, &tri_offsets);
    debug_assert!(
        tri::decode_trees(
            &trees.symbols,
            &trees.cycles,
            &mut Decoder::with_max_size(u32::MAX)
        )
        .is_ok_and(|back| tri::same_triangles(&index_buffer, &back, &tri_offsets)),
        "triangle trees keep each feature's triangles"
    );
    let vertex_order = reorder_mesh_vertices(&geo_offsets, &mut trees.cycles, &mut vertices);

    Ok(GeometrySection02 {
        types: vector_types,
        geo_lengths: Some(geo_lengths),
        topology: Topology::Flat,
        tessellation: Some(Tessellation {
            triangles,
            data: TessData::Prebuilt {
                trees,
                vertex_order,
            },
        }),
        vertices,
        z_axis,
        meshes: true,
    })
}

/// Number each feature's vertices in the order `cycles` first visits them (followed by any
/// unreferenced vertices of the feature), rewriting `cycles` and `vertices` in place and
/// returning `new_to_old` so vertex-scoped m-values stay attached to their vertices.
fn reorder_mesh_vertices(
    geo_offsets: &[u32],
    cycles: &mut [u32],
    vertices: &mut Vec<i32>,
) -> Vec<u32> {
    let vertex_count = vertices.len() / 3;
    if vertex_count == 0
        || vertices.len() != vertex_count * 3
        || geo_offsets.last().copied().map(u32::into_usize) != Some(vertex_count)
    {
        return Vec::new();
    }
    let mut old_to_new = vec![u32::MAX; vertex_count];
    let mut new_to_old = Vec::with_capacity(vertex_count);
    let mut relabelled = Vec::with_capacity(cycles.len());
    let mut cursor = 0;
    for &[v_lo, v_hi] in geo_offsets.array_windows() {
        if v_hi < v_lo || v_hi.into_usize() > vertex_count {
            return Vec::new();
        }
        while let Some(&v) = cycles.get(cursor)
            && v < v_hi
        {
            if v < v_lo {
                return Vec::new();
            }
            let slot = &mut old_to_new[v.into_usize()];
            if *slot == u32::MAX {
                *slot = u32::try_from(new_to_old.len()).expect("<= vertex_count < 2^32");
                new_to_old.push(v);
            }
            relabelled.push(*slot);
            cursor += 1;
        }
        for v in v_lo..v_hi {
            let slot = &mut old_to_new[v.into_usize()];
            if *slot == u32::MAX {
                *slot = u32::try_from(new_to_old.len()).expect("<= vertex_count < 2^32");
                new_to_old.push(v);
            }
        }
    }
    if cursor != cycles.len() {
        return Vec::new();
    }
    cycles.copy_from_slice(&relabelled);
    if new_to_old
        .iter()
        .enumerate()
        .any(|(new, &old)| old.into_usize() != new)
    {
        let mut reordered = Vec::with_capacity(vertices.len());
        for &old in &new_to_old {
            let base = old.into_usize() * 3;
            reordered.extend_from_slice(&vertices[base..base + 3]);
        }
        *vertices = reordered;
    }
    new_to_old
}

/// What a written geometry section puts in the layer's two header bytes.
pub(crate) struct GeometryHeader02 {
    pub(crate) layout: GeoLayout,
    /// The type every feature has, when the section wrote no types stream.
    pub(crate) uniform_type: Option<GeometryType>,
    /// For each triangle in the order the trees decode, its index in the layer's triangle
    /// sequence as staged; empty when the layer has no triangles.
    pub(crate) triangle_order: Vec<u32>,
    /// For each mesh vertex as written, its index in the layer's vertex sequence as staged;
    /// empty when vertices were not reordered.
    pub(crate) vertex_order: Vec<u32>,
}

impl GeometrySection02 {
    /// The one type every feature has, when they share one.
    ///
    /// Such a types stream is a single run saying nothing the header byte's nibble
    /// cannot, so the section drops it and the nibble carries the type instead.
    fn uniform_type(&self) -> Option<GeometryType> {
        if self.meshes {
            return Some(GeometryType::Mesh);
        }
        let (first, rest) = self.types.split_first()?;
        rest.iter().all(|t| t == first).then_some(*first)
    }

    /// Write the geometry streams to `enc` and return what the header bytes must say.
    ///
    /// The layout is only known once the vertex streams are written, since the
    /// dictionary layouts are picked by writing them and keeping the shortest, so
    /// the caller patches the layout byte it reserved before this call.
    ///
    /// Expects `enc.count_context` to hold the layer's `feature_count`.
    pub(crate) fn write_to(
        self,
        enc: &mut Encoder,
        codecs: &mut Codecs,
    ) -> MltResult<GeometryHeader02> {
        // Types and length streams are integer streams, only the vertex streams have their own family.
        enc.family_context = Family::Int(WordWidth::W32);

        // Types stream: implicit count = feature_count (the current count context).
        // A layer whose features share a type writes the header byte's nibble instead.
        let uniform_type = self.uniform_type();
        if uniform_type.is_none() {
            let types: Vec<u32> = self.types.iter().map(|t| *t as u32).collect();
            let ctx = StreamCtx::geom(StreamType::Length(LengthType::VarBinary), "meta");
            codecs.write_int_stream(&types, &ctx, enc)?;
        }

        let (part_lengths, ring_lengths) = self.topology.streams();
        let lengths = [
            (
                self.geo_lengths.as_deref(),
                LengthType::Geometries,
                "geometries",
            ),
            (part_lengths, LengthType::Parts, "parts"),
            (ring_lengths, LengthType::Rings, "rings"),
        ];
        for (stream, length_type, name) in lengths {
            if let Some(data) = stream {
                let ctx = StreamCtx::geom(StreamType::Length(length_type), name);
                codecs.write_int_stream(data, &ctx, enc)?;
            }
        }

        let tessellated = self.tessellation.is_some();
        let mut triangle_order = Vec::new();
        let mut vertex_order = Vec::new();
        if let Some(tess) = self.tessellation {
            let (triangles, trees, v_order) = tess.into_trees();
            let ctx = StreamCtx::geom(StreamType::Length(LengthType::Triangles), "triangles");
            codecs.write_int_stream(&triangles, &ctx, enc)?;
            let ctx = StreamCtx::geom(StreamType::Offset(OffsetType::TreeSymbols), "tree_symbols");
            codecs.write_int_stream(&trees.symbols, &ctx, enc)?;
            let ctx = StreamCtx::geom(StreamType::Offset(OffsetType::TreeCycles), "tree_cycles");
            codecs.write_int_stream(&trees.cycles, &ctx, enc)?;
            triangle_order = trees.order;
            vertex_order = v_order;
        }

        let geo_lengths = self.geo_lengths;
        let topology = self.topology;
        let types = self.types;
        let vertices = match self.z_axis {
            Some(axis) => write_xyz_vertices(&self.vertices, axis, tessellated, enc, codecs)?,
            None => write_vertices(
                &self.vertices,
                &|| {
                    let (parts, rings) = topology.streams();
                    let lengths = Levels {
                        geometries: geo_lengths.clone(),
                        parts: parts.map(<[u32]>::to_vec),
                        rings: rings.map(<[u32]>::to_vec),
                    };
                    let levels =
                        decode_topology(&types, lengths, &mut Decoder::with_max_size(u32::MAX))?;
                    Ok(run_offsets(&levels).map(<[u32]>::to_vec))
                },
                tessellated,
                enc,
                codecs,
            )?,
        };
        let layout = if uniform_type == Some(GeometryType::Mesh) {
            GeoLayout::Meshes
        } else if tessellated {
            GeoLayout::tessellated(geo_lengths.is_some())
        } else {
            GeoLayout::from_topology(&topology, geo_lengths.is_some(), vertices)
        };
        Ok(GeometryHeader02 {
            layout,
            uniform_type,
            triangle_order,
            vertex_order,
        })
    }
}

/// Write the vertex streams and report the storage they used.
///
/// Tessellated layers keep their vertices plain: no layout code pairs a triangle
/// buffer with a vertex dictionary, so a forced dictionary falls back to [`VertexBufferType::Vec2`].
fn write_vertices(
    vertices: &[i32],
    runs: &dyn Fn() -> MltResult<Option<Vec<u32>>>,
    tessellated: bool,
    enc: &mut Encoder,
    codecs: &mut Codecs,
) -> MltResult<VertexStorage> {
    seed_curve_caches(enc, vertices);
    enc.family_context = Family::Vertex;

    match (enc.override_vertex_buffer_type(), tessellated) {
        (Some(VertexBufferType::Rans), _) => {
            encode_rans_vertex_stream02(vertices, runs()?.as_deref(), enc)?;
            return Ok(VertexStorage::Plain);
        }
        (Some(_), true) | (Some(VertexBufferType::Vec2), false) => {
            encode_vec2_vertex_stream02(vertices, enc, codecs)?;
            return Ok(VertexStorage::Plain);
        }
        (None, true) => {
            write_plain_vertices(vertices, runs, enc, codecs)?;
            return Ok(VertexStorage::Plain);
        }
        (Some(VertexBufferType::Morton), false) => {
            encode_morton_vertex_streams02(vertices, enc, codecs)?;
            return Ok(VertexStorage::Dict);
        }
        (Some(VertexBufferType::Hilbert), false) => {
            encode_hilbert_vertex_streams02(vertices, enc, codecs)?;
            return Ok(VertexStorage::Dict);
        }
        (None, false) => {}
    }

    if !dict_may_be_beneficial(vertices, enc) {
        write_plain_vertices(vertices, runs, enc, codecs)?;
        return Ok(VertexStorage::Plain);
    }

    // Morton fits (the gate above ensures it), so race all three layouts and keep
    // the shortest, as the v1 writer does.
    keep_shortest(
        enc,
        codecs,
        &[
            (VertexStorage::Plain, &|enc, codecs| {
                write_plain_vertices(vertices, runs, enc, codecs)
            }),
            (VertexStorage::Dict, &|enc, codecs| {
                encode_hilbert_vertex_streams02(vertices, enc, codecs)
            }),
            (VertexStorage::Dict, &|enc, codecs| {
                encode_morton_vertex_streams02(vertices, enc, codecs)
            }),
        ],
    )
}

/// A way of writing a layer's vertex streams, and the storage it declares.
type VertexCandidate<'a> = (
    VertexStorage,
    &'a dyn Fn(&mut Encoder, &mut Codecs) -> MltResult<()>,
);

/// Write every candidate, keep the one that stores the fewest bytes, and report its storage.
fn keep_shortest(
    enc: &mut Encoder,
    codecs: &mut Codecs,
    candidates: &[VertexCandidate<'_>],
) -> MltResult<VertexStorage> {
    let mut winner = VertexStorage::Plain;
    let mut winner_size = usize::MAX;
    let mut alt = enc.try_alternatives();
    for &(storage, write) in candidates {
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
        })?;
    }
    drop(alt);
    Ok(winner)
}

/// Write `(x, y, z)` vertex streams and report the storage they used.
///
/// A Morton code spans only `x` and `y`, so the plain triples race only the Hilbert dictionary.
fn write_xyz_vertices(
    vertices: &[i32],
    axis: ZAxis,
    tessellated: bool,
    enc: &mut Encoder,
    codecs: &mut Codecs,
) -> MltResult<VertexStorage> {
    enc.family_context = Family::Vertex;

    if tessellated {
        encode_vec3_vertex_stream02(vertices, axis, enc, codecs)?;
        return Ok(VertexStorage::Plain);
    }

    if enc.hilbert_cache.is_none() || enc.morton_cache.is_none() {
        let xy: Vec<i32> = vertices
            .as_chunks::<3>()
            .0
            .iter()
            .flat_map(|&[x, y, _]| [x, y])
            .collect();
        seed_curve_caches(enc, &xy);
    }

    if let Some(forced) = enc.override_vertex_buffer_type() {
        return match forced {
            VertexBufferType::Vec2 => {
                encode_vec3_vertex_stream02(vertices, axis, enc, codecs)?;
                Ok(VertexStorage::Plain)
            }
            VertexBufferType::Hilbert => {
                encode_hilbert_xyz_vertex_streams02(vertices, axis, enc, codecs)?;
                Ok(VertexStorage::Dict)
            }
            VertexBufferType::Morton => Err(MltError::NotImplemented(
                "Morton vertices with z coordinates, since a Morton code spans only x and y",
            )),
            VertexBufferType::Rans => Err(MltError::NotImplemented(
                "rANS vertices with z coordinates, since its tables are defined over pairs",
            )),
        };
    }

    if !xyz_dict_may_be_beneficial(vertices, enc) {
        encode_vec3_vertex_stream02(vertices, axis, enc, codecs)?;
        return Ok(VertexStorage::Plain);
    }

    keep_shortest(
        enc,
        codecs,
        &[
            (VertexStorage::Plain, &|enc, codecs| {
                encode_vec3_vertex_stream02(vertices, axis, enc, codecs)
            }),
            (VertexStorage::Dict, &|enc, codecs| {
                encode_hilbert_xyz_vertex_streams02(vertices, axis, enc, codecs)
            }),
        ],
    )
}

/// The plain vertex stream the encoder picks itself: componentwise delta, raced against second-order deltas and rANS when the config allows them.
fn write_plain_vertices(
    vertices: &[i32],
    runs: &dyn Fn() -> MltResult<Option<Vec<u32>>>,
    enc: &mut Encoder,
    codecs: &mut Codecs,
) -> MltResult<()> {
    let allow_delta2 = enc.config().allow_delta2();
    let allow_rans = enc.config().allow_rans_vertices();
    if vertices.is_empty() || !(allow_delta2 || allow_rans) {
        return encode_vec2_vertex_stream02(vertices, enc, codecs);
    }
    let offsets = if allow_rans { runs()? } else { None };
    let mut alt = enc.try_alternatives();
    alt.with(|enc| encode_vec2_vertex_stream02(vertices, enc, codecs))?;
    if allow_delta2 {
        alt.with(|enc| encode_vec2_delta2_vertex_stream02(vertices, enc, codecs))?;
    }
    if allow_rans {
        alt.with(|enc| encode_rans_vertex_stream02(vertices, offsets.as_deref(), enc))?;
    }
    Ok(())
}
