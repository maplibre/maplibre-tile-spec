use std::collections::HashMap;
use std::mem;

use geo_types::Coord;
use probabilistic_collections::SipHasherBuilder;
use probabilistic_collections::hyperloglog::HyperLogLog;

use super::model::VertexBufferType;
use crate::MltResult;
use crate::codecs::hilbert::hilbert_sort_key;
use crate::codecs::zigzag::encode_componentwise_delta_vec2s;
use crate::decoder::GeometryType::{LineString, Point, Polygon};
use crate::decoder::{
    ColumnType, DictionaryType, GeometryType, GeometryValues, LengthType, LogicalEncoding, Morton,
    OffsetType, PhysicalEncoding, StreamMeta, StreamType,
};
use crate::encoder::model::{CurveParams, StreamCtx};
use crate::encoder::stream::write_u32_stream;
use crate::encoder::{
    Codecs, Encoder, PhysicalCodecs, PhysicalEncoder, PhysicalIntStreamKind, U32Physical,
    write_stream_payload,
};
use crate::utils::AsUsize as _;

/// Compute `ZOrderCurve` parameters from the vertex value range.
///
/// Returns `(bits, shift)` matching Java's `SpaceFillingCurve`.
/// Build a sorted unique Morton dictionary and per-vertex offset indices from a flat
/// `[x0, y0, x1, y1, …]` vertex slice.
///
/// Returns `(sorted_unique_codes, per_vertex_offsets)`.
#[hotpath::measure]
fn build_morton_dict(vertices: &[i32], meta: Morton) -> MltResult<(Vec<u32>, Vec<u32>)> {
    let codes: Vec<u32> = vertices
        .chunks_exact(2)
        .map(|c| meta.encode_morton(c[0], c[1]))
        .collect::<Result<_, _>>()?;

    let mut dict = codes.clone();
    dict.sort_unstable();
    dict.dedup();

    #[expect(
        clippy::cast_possible_truncation,
        reason = "dict.len() <= u32::MAX (deduped u32 codes)"
    )]
    let code_to_idx: HashMap<u32, u32> = dict
        .iter()
        .enumerate()
        .map(|(i, &c)| (c, i as u32))
        .collect();
    let offsets: Vec<u32> = codes.iter().map(|code| code_to_idx[code]).collect();

    Ok((dict, offsets))
}

/// Build a Hilbert-curve-sorted unique vertex dictionary into caller-provided
/// scratch.
///
/// On return, `dict_xy` holds the deduplicated `[x, y, …]` dictionary in
/// Hilbert order and `offsets[i]` is the slot of input vertex `i`; `indexed`
/// and `remap` are left as opaque scratch.
///
/// Dedup is keyed on the Hilbert curve index. Inside the `params.bits` grid
/// the index ↔ `(x, y)` mapping is bijective, so dedup-by-index is equivalent
/// to dedup-by-coordinate without the cost of hashing pairs.
#[hotpath::measure]
fn build_hilbert_dict(
    vertices: &[i32],
    params: CurveParams,
    offsets: &mut Vec<u32>,
    indexed: &mut Vec<u64>,
    dict_xy: &mut Vec<i32>,
    remap: &mut HashMap<u32, u32>,
) {
    offsets.clear();
    indexed.clear();
    dict_xy.clear();
    remap.clear();

    let coord_count = vertices.len() / 2;
    if coord_count == 0 {
        return;
    }
    offsets.reserve(coord_count);
    indexed.reserve(coord_count);
    dict_xy.reserve(coord_count * 2);
    remap.reserve(coord_count);

    for (i, c) in vertices.chunks_exact(2).enumerate() {
        let k = hilbert_sort_key(Coord { x: c[0], y: c[1] }, params);
        offsets.push(k);
        // Key in the high 32 bits so a single u64 sort orders by Hilbert
        // index while preserving the original position for tie-breaking.
        let packed = (u64::from(k) << 32) | (i as u64);
        indexed.push(packed);
    }
    indexed.sort_unstable();

    let mut last_key: Option<u32> = None;
    for &packed in &*indexed {
        let key = (packed >> 32) as u32;
        let src_idx = (packed & 0xFFFF_FFFF) as usize;
        if last_key != Some(key) {
            #[expect(
                clippy::cast_possible_truncation,
                reason = "dict.len() <= coord_count <= u32::MAX"
            )]
            let slot = (dict_xy.len() / 2) as u32;
            dict_xy.push(vertices[src_idx * 2]);
            dict_xy.push(vertices[src_idx * 2 + 1]);
            remap.insert(key, slot);
            last_key = Some(key);
        }
    }

    for k in offsets.iter_mut() {
        *k = remap[k];
    }
}

/// Push consecutive offset-differences from `offsets` onto `lengths`.
///
/// Expects a slice of `n + 1` elements and produces `n` lengths,
/// one per consecutive pair: `offsets[i + 1] - offsets[i]`.
#[inline]
fn extend_offsets(lengths: &mut Vec<u32>, offsets: &[u32]) -> usize {
    lengths.extend(offsets.windows(2).map(|w| w[1] - w[0]));
    offsets.len() - 1
}

/// Convert geometry offsets to length stream for encoding.
/// This is the inverse of `decode_root_length_stream`.
///
/// The offset array can be either:
/// - Sparse: entries only for geometries that need them (types > `buffer_id`), N+1 entries for N matching geoms
/// - Dense (normalized): N+1 entries for N geometry types, indexed by geometry position
///
/// If dense `(len == geom_types.len() + 1)`, use geometry index directly.
/// If sparse, use sequential indexing for matching geometry types.
fn encode_root_length_stream(
    geom_types: &[GeometryType],
    geom_offsets: &[u32],
    buffer_id: GeometryType,
) -> Vec<u32> {
    if geom_offsets.len() == geom_types.len() + 1 {
        // Dense: zip by position, then filter out non-contributing types.
        geom_types
            .iter()
            .zip(geom_offsets.windows(2))
            .filter(|&(&t, _)| t > buffer_id)
            .map(|(_, w)| w[1] - w[0])
            .collect()
    } else {
        // Sparse: filter types first, then zip with consecutive offset pairs.
        geom_types
            .iter()
            .filter(|&&t| t > buffer_id)
            .zip(geom_offsets.windows(2))
            .map(|(_, w)| w[1] - w[0])
            .collect()
    }
}

/// Convert part offsets to length stream for level 1 encoding.
fn encode_level1_length_stream(
    geom_types: &[GeometryType],
    geom_offsets: &[u32],
    part_offsets: &[u32],
    is_line_string_present: bool,
) -> Vec<u32> {
    let mut lengths = Vec::new();
    let mut part_idx = 0;

    for (i, &geom_type) in geom_types.iter().enumerate() {
        if geom_type.is_polygon() || (is_line_string_present && geom_type.is_linestring()) {
            let n = (geom_offsets[i + 1] - geom_offsets[i]).as_usize();
            part_idx += extend_offsets(&mut lengths, &part_offsets[part_idx..=part_idx + n]);
        }
        // Note: Point/MultiPoint don't have entries in the sparse part_offsets used
        // at this call site, so part_idx must not advance for non-length types here.
    }

    lengths
}

/// Compute ring vertex-count lengths for the no-geometry-offsets + has-ring-offsets case.
///
/// In this branch `part_offsets` is a **dense** N+1 array (one slot per geometry,
/// including Points) and `ring_offsets` holds the vertex offsets for every slot.
/// Using the geometry index directly as the ring-slot index avoids the
/// running-counter misalignment that `encode_level1_length_stream` would produce
/// when non-length types (Points) occupy slots that a sparse counter skips.
fn encode_ring_lengths_for_mixed(
    geom_types: &[GeometryType],
    part_offsets: &[u32],
    ring_offsets: &[u32],
    has_line_string: bool,
) -> Vec<u32> {
    let mut lengths = Vec::new();
    for (i, &geom_type) in geom_types.iter().enumerate() {
        if geom_type.is_polygon() || (has_line_string && geom_type.is_linestring()) {
            let s = part_offsets[i].as_usize();
            let e = part_offsets[i + 1].as_usize();
            extend_offsets(&mut lengths, &ring_offsets[s..=e]);
        }
    }
    lengths
}

/// Convert ring offsets to length stream for level 2 encoding.
/// This is the inverse of `decode_level2_length_stream`.
///
/// The `geom_offsets` array is expected to be an N+1 element array for N geometries.
/// The `part_offsets` array tracks ring counts cumulatively.
fn encode_level2_length_stream(
    geom_types: &[GeometryType],
    geom_offsets: &[u32],
    part_offsets: &[u32],
    ring_offsets: &[u32],
) -> Vec<u32> {
    let mut lengths = Vec::new();
    let mut part_idx = 0;
    let mut ring_idx = 0;

    for (i, &geom_type) in geom_types.iter().enumerate() {
        let count = (geom_offsets[i + 1] - geom_offsets[i]).as_usize();

        // Only Polygon and MultiPolygon have ring data in level 2
        // LineStrings with Polygon present add their vertex counts directly to ring_offsets,
        // but they don't have parts (ring count per linestring is always 1 implicitly)
        if geom_type.is_polygon() {
            // Polygon/MultiPolygon: iterate through sub-polygons, each has parts (ring counts)
            for _ in 0..count {
                let n = (part_offsets[part_idx + 1] - part_offsets[part_idx]).as_usize();
                ring_idx += extend_offsets(&mut lengths, &ring_offsets[ring_idx..=ring_idx + n]);
                part_idx += 1;
            }
        } else if geom_type.is_linestring() {
            // LineStrings contribute to ring_offsets directly (vertex counts)
            ring_idx += extend_offsets(&mut lengths, &ring_offsets[ring_idx..=ring_idx + count]);
        }
        // Note: Point/MultiPoint don't contribute to ring_offsets
    }

    lengths
}

/// Convert part offsets without ring buffer to length stream.
///
/// This path is reached only when `ring_offsets` is absent, which means no Polygon/MultiPolygon
/// types are present (they always create `ring_offsets`).  Only LineString/MultiLineString
/// contribute vertex-count lengths here; Point/MultiPoint use an implicit count of 1 in the
/// decoder and produce no entry in this stream.
fn encode_level1_without_ring_buffer_length_stream(
    geom_types: &[GeometryType],
    geom_offsets: &[u32],
    part_offsets: &[u32],
) -> Vec<u32> {
    let mut lengths = Vec::new();
    let mut part_idx = 0;

    for (i, &geom_type) in geom_types.iter().enumerate() {
        if geom_type.is_linestring() {
            let n = (geom_offsets[i + 1] - geom_offsets[i]).as_usize();
            part_idx += extend_offsets(&mut lengths, &part_offsets[part_idx..=part_idx + n]);
        }
        // Point/MultiPoint don't contribute to part_offsets; part_idx must not advance.
    }

    lengths
}

/// Normalize `geom_offsets` for mixed geometry types.
fn normalize_geometry_offsets(vector_types: &[GeometryType], geom_offsets: &[u32]) -> Vec<u32> {
    let mut normalized = Vec::with_capacity(vector_types.len() + 1);
    let mut offset = 0_u32;
    let mut sparse_idx = 0_usize; // Index into sparse geom_offsets

    for &geom_type in vector_types {
        normalized.push(offset);

        if geom_type.is_multi() {
            // Multi* types get their count from the sparse array
            if sparse_idx + 1 < geom_offsets.len() {
                let start = geom_offsets[sparse_idx];
                let end = geom_offsets[sparse_idx + 1];
                offset += end - start;
                sparse_idx += 1;
            }
        } else {
            // Non-Multi types have implicit count of 1
            offset += 1;
        }
    }

    normalized.push(offset);
    normalized
}

/// Normalize `part_offsets` for ring-based indexing (Polygon mixed with `Point`/`LineString`).
///
/// Called only when `geom_offsets` is absent (no Multi\* types) and `ring_offsets` is
/// present.  In this context `part_offsets` is a compact polygon-only array; this function
/// expands it to a dense per-geometry array so that `encode_ring_lengths_for_mixed` can index
/// directly by geometry position.
///
/// Each slot in the output holds the first index into `ring_offsets` for that geometry:
/// - `Point`: no contribution — slot range is empty (`ring_idx` unchanged).
/// - `LineString`: contributes 1 slot (vertex count) — slot range is 1.
/// - `Polygon`: contributes `ring_count` slots — slot range equals its ring count.
fn normalize_part_offsets_for_rings(
    vector_types: &[GeometryType],
    part_offsets: &[u32],
    ring_offsets: &[u32],
) -> Vec<u32> {
    let mut normalized = Vec::with_capacity(vector_types.len() + 1);
    let mut ring_idx = 0_u32;
    let mut part_idx = 0_usize;

    for &geom_type in vector_types {
        normalized.push(ring_idx);

        if geom_type == Point {
            // Point has no vertex-count slot in ring_offsets.
        } else if geom_type.is_linestring() {
            // Each LineString occupies exactly one slot in ring_offsets.
            ring_idx += 1;
        } else if geom_type.is_polygon() && part_idx + 1 < part_offsets.len() {
            // Polygon occupies ring_count slots (one vertex-count per ring).
            let ring_count = part_offsets[part_idx + 1] - part_offsets[part_idx];
            ring_idx += ring_count;
            part_idx += 1;
        }
        // No Multi* types can appear here (they always produce geom_offsets).
    }

    // ring_idx must equal ring_offsets.len() - 1 for well-formed data.
    debug_assert_eq!(
        ring_idx as usize,
        ring_offsets.len().saturating_sub(1),
        "ring index mismatch after normalization"
    );
    normalized.push(ring_idx);
    normalized
}

/// Whether to race dictionary-based vertex layouts (Hilbert, Morton) against
/// the plain Vec2 layout for this geometry column.
///
/// Profiling showed unconditional racing is ~2× slower overall: most layers
/// have high vertex uniqueness, where the dict layouts cannot win and the
/// extra sort + `HashMap` build is wasted. Gate on Morton fitting in 16 bits
/// per axis (required by the spec) and on a `HyperLogLog`-estimated
/// uniqueness ratio below the threshold.
#[hotpath::measure]
fn dict_may_be_beneficial(vertices: &[i32], enc: &Encoder) -> bool {
    const MAXIMUM_UNIQUENESS_THRESHOLD_FOR_DICT: f64 = 0.66;

    let coord_count = vertices.len() / 2;
    if coord_count == 0 || enc.morton_cache.is_none() {
        return false;
    }

    let mut hll = HyperLogLog::<Coord<i32>>::with_hasher(0.03, SipHasherBuilder::from_seed(0, 0));
    for c in vertices.chunks_exact(2) {
        hll.insert(&Coord::<i32> { x: c[0], y: c[1] });
    }
    #[expect(clippy::cast_precision_loss)]
    let estimated_unique = hll.len().clamp(0.0, coord_count as f64);
    #[expect(clippy::cast_precision_loss)]
    let uniqueness_ratio = estimated_unique / coord_count as f64;
    uniqueness_ratio < MAXIMUM_UNIQUENESS_THRESHOLD_FOR_DICT
}

/// Pre-populated by [`StagedLayer::encode_into`](crate::encoder::StagedLayer::encode_into);
/// callers must have gated on [`dict_may_be_beneficial`] which rejects layers
/// whose extent does not fit Morton.
fn get_morton(enc: &Encoder) -> Morton {
    enc.morton_cache.expect(
        "morton_cache populated by StagedLayer::encode_into; gated by dict_may_be_beneficial",
    )
}

/// Pre-populated by [`StagedLayer::encode_into`](crate::encoder::StagedLayer::encode_into).
fn get_hilbert_params(enc: &Encoder) -> CurveParams {
    enc.hilbert_cache
        .expect("hilbert_cache populated by StagedLayer::encode_into")
}

/// Encode the plain Vec2 vertex layout: componentwise-delta over the raw
/// `[x0, y0, x1, y1, …]` slice.
fn encode_vec2_vertex_stream(
    vertices: &[i32],
    enc: &mut Encoder,
    codecs: &mut Codecs,
) -> MltResult<u8> {
    let delta = encode_componentwise_delta_vec2s(vertices, &mut codecs.logical.u32_tmp);
    let ctx = StreamCtx::geom(StreamType::Data(DictionaryType::Vertex), "vertex");
    let logical = LogicalEncoding::ComponentwiseDelta;
    write_geo_precomputed_stream(delta, ctx, logical, enc, &mut codecs.physical)
}

/// Encode a Morton-keyed vertex dictionary: per-vertex offsets stream
/// followed by a delta-encoded Morton-code dictionary.
fn encode_morton_vertex_streams(
    vertices: &[i32],
    enc: &mut Encoder,
    codecs: &mut Codecs,
) -> MltResult<u8> {
    let morton = get_morton(enc);
    let (dict, offsets) = build_morton_dict(vertices, morton)?;
    let mut n: u8 = 0;

    let ctx = StreamCtx::geom(StreamType::Offset(OffsetType::Vertex), "vertex_offsets");
    n += write_geo_u32_stream(&offsets, ctx, enc, codecs)?;

    let delta = encode_morton_deltas(&dict, &mut codecs.logical.u32_tmp);
    let ctx = StreamCtx::geom(StreamType::Data(DictionaryType::Morton), "vertex");
    let logical = LogicalEncoding::MortonDelta(morton);
    n += write_geo_precomputed_stream(delta, ctx, logical, enc, &mut codecs.physical)?;
    Ok(n)
}

/// Encode a Hilbert-keyed vertex dictionary: per-vertex offsets stream
/// followed by a componentwise-delta-encoded `[x, y, …]` dictionary in
/// Hilbert order.
fn encode_hilbert_vertex_streams(
    vertices: &[i32],
    enc: &mut Encoder,
    codecs: &mut Codecs,
) -> MltResult<u8> {
    let params = get_hilbert_params(enc);
    let mut n: u8 = 0;

    // Take scratch ownership locally: `write_geo_*_stream` needs `&mut Codecs`,
    // which would otherwise conflict with our `&[..]` views into these slots.
    let mut offsets = mem::take(&mut codecs.logical.hilbert_offsets);
    let mut indexed = mem::take(&mut codecs.logical.hilbert_indexed);
    let mut dict_xy = mem::take(&mut codecs.logical.hilbert_dict_xy);
    let mut remap = mem::take(&mut codecs.logical.hilbert_remap);

    build_hilbert_dict(
        vertices,
        params,
        &mut offsets,
        &mut indexed,
        &mut dict_xy,
        &mut remap,
    );
    // Done with these — restore so the physical-encoding race below can use
    // them via the codec.
    codecs.logical.hilbert_indexed = indexed;
    codecs.logical.hilbert_remap = remap;

    let ctx = StreamCtx::geom(StreamType::Offset(OffsetType::Vertex), "vertex_offsets");
    n += write_geo_u32_stream(&offsets, ctx, enc, codecs)?;

    // Reuse `offsets` as the delta output rather than allocating another Vec;
    // also keeps `codecs.logical.u32_values` free for the inner race.
    encode_componentwise_delta_vec2s(&dict_xy, &mut offsets);
    let ctx = StreamCtx::geom(StreamType::Data(DictionaryType::Vertex), "vertex");
    let logical = LogicalEncoding::ComponentwiseDelta;
    n += write_geo_precomputed_stream(&offsets, ctx, logical, enc, &mut codecs.physical)?;

    codecs.logical.hilbert_offsets = offsets;
    codecs.logical.hilbert_dict_xy = dict_xy;
    Ok(n)
}

/// Write a geometry `u32` stream: [`Encoder::override_int_enc`] when explicit mode is active,
/// otherwise try all pruned candidates and keep the shortest.
///
/// Returns `1` if the stream was written, `0` if it was skipped.  Empty streams are skipped
/// unless [`Encoder::force_stream`] returns `true` for this stream's [`StreamCtx`].
fn write_geo_u32_stream(
    data: &[u32],
    ctx: StreamCtx,
    enc: &mut Encoder,
    codecs: &mut Codecs,
) -> MltResult<u8> {
    Ok(if data.is_empty() && !enc.force_stream(&ctx) {
        0
    } else {
        write_u32_stream(data, &ctx, enc, codecs)?;
        1
    })
}

/// Like [`write_geo_u32_stream`] but for pre-logically-encoded data: competes
/// only the physical encoders instead of applying a logical transform.
///
/// Returns `1` if the stream was written, `0` if skipped (empty + no force).
fn write_geo_precomputed_stream(
    data: &[u32],
    ctx: StreamCtx,
    logical: LogicalEncoding,
    enc: &mut Encoder,
    physical: &mut PhysicalCodecs,
) -> MltResult<u8> {
    Ok(if data.is_empty() && !enc.force_stream(&ctx) {
        0
    } else {
        if let Some(int_enc) = enc.override_int_enc(&ctx) {
            let stream_type = ctx.stream_type;
            let (phys, vals) = U32Physical::encode_physical(physical, data, int_enc.physical)?;
            let meta = StreamMeta::new2(stream_type, logical, phys, data.len())?;
            write_stream_payload(&mut enc.data, meta, false, vals)?;
        } else if data.is_empty() {
            let meta = StreamMeta::new2(ctx.stream_type, logical, PhysicalEncoding::None, 0)?;
            write_stream_payload(&mut enc.data, meta, false, &[])?;
        } else {
            let mut alt = enc.try_alternatives();
            for physical_enc in [PhysicalEncoder::FastPFOR, PhysicalEncoder::VarInt] {
                alt.with(|enc| {
                    let (phys, vals) = U32Physical::encode_physical(physical, data, physical_enc)?;
                    let meta = StreamMeta::new2(ctx.stream_type, logical, phys, data.len())?;
                    write_stream_payload(&mut enc.data, meta, false, vals)
                })?;
            }
        }
        1
    })
}

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

        // Flatten every Option<Vec> → Vec  (empty == not present).
        // triangles: None means no tessellation; Some([]) can't occur in practice (each
        // push_geom appends a count), so empty == absent is safe here too.
        // vertices: None means no coordinate data (e.g. empty layer).
        let geom_offsets = geometry_offsets.unwrap_or_default();
        let part_offsets = part_offsets.unwrap_or_default();
        let ring_offsets = ring_offsets.unwrap_or_default();
        let index_buffer = index_buffer.unwrap_or_default();
        let triangles = triangles.unwrap_or_default();
        let vertices = vertices.unwrap_or_default();

        // Direct callers (tests, custom drivers) skip `StagedLayer::encode_into`
        // and arrive with empty caches; populate from `vertices` so the
        // dictionary builders can rely on them unconditionally.
        if enc.hilbert_cache.is_none() {
            enc.hilbert_cache = Some(CurveParams::from_vertices(&vertices));
        }
        if enc.morton_cache.is_none() {
            let p = enc.hilbert_cache.expect("populated above");
            enc.morton_cache = Morton::new(p.bits, p.shift).ok();
        }

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
        let stream_count_pos = enc.data.len();
        enc.data.push(0); // placeholder — patched below
        let mut n: u8 = 0;

        // Meta stream — always written, even for a zero-feature layer.
        let ctx = StreamCtx::geom(StreamType::Length(LengthType::VarBinary), "meta");
        write_u32_stream(&meta, &ctx, enc, codecs)?;
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
                    // geom → parts only (no rings).
                    let data = encode_level1_without_ring_buffer_length_stream(
                        &vector_types,
                        &geom_offsets,
                        &part_offsets,
                    );
                    let ctx = StreamCtx::geom(StreamType::Length(LengthType::Parts), "no_rings");
                    n += write_geo_u32_stream(&data, ctx, enc, codecs)?;
                } else {
                    // Full topology: geom → parts → rings.
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
                // No Multi* types; parts → rings (Polygon / mixed Point+Polygon).
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
                let ds = e.data.len();
                let ms = e.meta.len();
                winner_stream_cnt = encode_vec2_vertex_stream(&vertices, e, codecs)?;
                winner_size = (e.data.len() - ds) + (e.meta.len() - ms);
                Ok(())
            })?;
            alt.with(|e| {
                let ds = e.data.len();
                let ms = e.meta.len();
                let cnt = encode_hilbert_vertex_streams(&vertices, e, codecs)?;
                let size = (e.data.len() - ds) + (e.meta.len() - ms);
                if size < winner_size {
                    winner_stream_cnt = cnt;
                    winner_size = size;
                }
                Ok(())
            })?;
            alt.with(|e| {
                let ds = e.data.len();
                let ms = e.meta.len();
                let cnt = encode_morton_vertex_streams(&vertices, e, codecs)?;
                let size = (e.data.len() - ds) + (e.meta.len() - ms);
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
        enc.data[stream_count_pos] = n;
        enc.increment_column_count();
        Ok(())
    }
}

fn encode_morton_deltas<'a>(codes: &[u32], buffer: &'a mut Vec<u32>) -> &'a mut Vec<u32> {
    buffer.clear();
    if let Some(&first) = codes.first() {
        buffer.reserve(codes.len());
        buffer.extend(std::iter::once(first).chain(codes.windows(2).map(|w| w[1] - w[0])));
    }
    buffer
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_morton_dict() {
        let meta = Morton { bits: 4, shift: 0 };
        // vertices: [x0,y0, x1,y1, x2,y2, x3,y3] — repeat (1,2) to test dedup
        let vertices = [1, 2, 3, 4, 1, 2, 0, 0];
        let (dict, offsets) = build_morton_dict(&vertices, meta).unwrap();

        assert!(
            dict.windows(2).all(|w| w[0] < w[1]),
            "dict not sorted/unique"
        );
        assert_eq!(offsets.len(), 4, "offsets length == number of vertex pairs");
        assert_eq!(offsets[0], offsets[2], "duplicate (1,2) should share index");
        assert!(offsets.iter().all(|&o| (o as usize) < dict.len()));
    }

    #[test]
    fn test_encode_root_length_stream() {
        // Single Polygon geometry (no Multi)
        let types = vec![Polygon];
        let offsets = vec![0, 1]; // One polygon

        let lengths = encode_root_length_stream(&types, &offsets, Polygon);
        // Polygon == buffer_id, so no length encoded
        assert!(lengths.is_empty());

        // MultiPolygon needs length encoded
        let types = vec![GeometryType::MultiPolygon];
        let offsets = vec![0, 2]; // MultiPolygon with 2 polygons

        let lengths = encode_root_length_stream(&types, &offsets, Polygon);
        assert_eq!(lengths, vec![2]);
    }
}
