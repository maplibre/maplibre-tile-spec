use std::collections::HashMap;

use super::encode::{encode_geometry, z_order_params};
use super::{DecodedGeometry, GeometryEncoder, OwnedEncodedGeometry, VertexBufferType};
use crate::MltError;
use crate::v01::{DictionaryType, IntEncoder, LengthType, OffsetType, StreamType};

/// If the ratio of unique vertices to total vertices is below this threshold,
/// Morton dictionary encoding is preferred over Vec2 componentwise-delta.
///
/// A lower ratio means more coordinate repetition, which is precisely the
/// scenario where the dictionary overhead pays off.
const MORTON_UNIQUENESS_THRESHOLD: f64 = 0.5;

/// Automated optimizer for geometry encoding.
///
/// `GeometryOptimizer` analyses a [`DecodedGeometry`] and picks the best
/// combination of encoding strategies for every stream it contains.  It
/// replaces the need to configure a [`GeometryEncoder`] by hand.
///
/// # Design
///
/// 1. **Probe** - call `encode_geometry` with an all-varint encoder and an
///    `on_stream` callback that collects the raw `u32` payload for every stream.
/// 2. **Select** - run [`IntEncoder::auto_u32`] on each payload to pick the best
///    physical/logical combination per stream.
/// 3. **Encode** - call `encode_geometry` again with the optimized encoder.
pub struct GeometryOptimizer;

impl GeometryOptimizer {
    /// Analyze `decoded` and encode it with automatically selected per-stream
    /// encoders.
    pub fn optimize_and_encode(
        decoded: &DecodedGeometry,
    ) -> Result<OwnedEncodedGeometry, MltError> {
        let vertex_buffer_type = decoded
            .vertices
            .as_deref()
            .map_or(VertexBufferType::Vec2, Self::select_vertex_strategy);

        // Pass 1: probe with an all-varint encoder and collect the raw u32
        // payload for each stream via the on_stream callback.
        let mut probe = GeometryEncoder::all(IntEncoder::varint());
        probe.vertex_buffer_type(vertex_buffer_type);

        let mut profiles: HashMap<StreamType, Vec<u32>> = HashMap::new();
        encode_geometry(
            decoded,
            &probe,
            Some(&mut |st: StreamType, data: &[u32]| {
                profiles.insert(st, data.to_vec());
            }),
        )?;

        // Build an optimized encoder from the profiled payloads.
        let opt = |st: StreamType| -> IntEncoder {
            profiles
                .get(&st)
                .map_or_else(IntEncoder::varint, |data| IntEncoder::auto_u32(data))
        };

        // Parts and Rings map to different GeometryEncoder fields depending on
        // which topology branch fired.  Since only one branch can fire per call,
        // each StreamType appears at most once in `profiles`.
        let has_geom_offs = decoded.geometry_offsets.is_some();
        let has_ring_offs = decoded.ring_offsets.is_some();

        let parts_enc = opt(StreamType::Length(LengthType::Parts));
        let rings_enc = opt(StreamType::Length(LengthType::Rings));

        // The vertex data StreamType depends on the chosen layout strategy.
        let vertex_st = match vertex_buffer_type {
            VertexBufferType::Vec2 => StreamType::Data(DictionaryType::Vertex),
            VertexBufferType::Morton => StreamType::Data(DictionaryType::Morton),
        };

        let mut encoder = GeometryEncoder::all(IntEncoder::varint());
        encoder
            .meta(opt(StreamType::Length(LengthType::VarBinary)))
            .geometries(opt(StreamType::Length(LengthType::Geometries)))
            .triangles(opt(StreamType::Length(LengthType::Triangles)))
            .triangles_indexes(opt(StreamType::Offset(OffsetType::Index)))
            .vertex(opt(vertex_st))
            .vertex_offsets(opt(StreamType::Offset(OffsetType::Vertex)))
            .vertex_buffer_type(vertex_buffer_type);

        match (has_geom_offs, has_ring_offs) {
            (true, true) => {
                encoder.rings(parts_enc).rings2(rings_enc);
            }
            (true, false) => {
                encoder.no_rings(parts_enc);
            }
            (false, true) => {
                encoder.parts(parts_enc).parts_ring(rings_enc);
            }
            (false, false) => {
                encoder.only_parts(parts_enc);
            }
        }

        // Pass 2: encode with the optimized encoder.
        encode_geometry(decoded, &encoder, None)
    }

    /// Choose between Vec2 componentwise-delta and Morton dictionary encoding.
    ///
    /// Morton is only selected when:
    /// - The coordinate range fits within 16 bits per axis (required by the spec), and
    /// - The uniqueness ratio is below [`MORTON_UNIQUENESS_THRESHOLD`], meaning
    ///   enough vertices are repeated that the dictionary overhead is worthwhile.
    fn select_vertex_strategy(vertices: &[i32]) -> VertexBufferType {
        let total = vertices.len() / 2;
        if total == 0 {
            return VertexBufferType::Vec2;
        }

        // Morton requires coordinates in a bounded range.
        if z_order_params(vertices).is_err() {
            return VertexBufferType::Vec2;
        }

        // Count unique (x, y) pairs via a hash set.
        let unique_count = vertices
            .chunks_exact(2)
            .map(|c| (c[0], c[1]))
            .collect::<std::collections::HashSet<_>>()
            .len();

        #[expect(clippy::cast_precision_loss)]
        let uniqueness_ratio = unique_count as f64 / total as f64;

        if uniqueness_ratio < MORTON_UNIQUENESS_THRESHOLD {
            VertexBufferType::Morton
        } else {
            VertexBufferType::Vec2
        }
    }
}
