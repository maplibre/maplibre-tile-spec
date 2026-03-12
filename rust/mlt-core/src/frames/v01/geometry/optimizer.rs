use std::collections::HashMap;

use crate::decode::Decode as _;
use crate::optimizer::{AutomaticOptimisation, ManualOptimisation, ProfileOptimisation};
use crate::v01::encode::{encode_geometry, z_order_params};
use crate::v01::{
    DataProfile, DecodedGeometry, DictionaryType, GeometryEncoder, IntEncoder, LengthType,
    OffsetType, OwnedEncodedGeometry, OwnedGeometry, StreamType, VertexBufferType,
};
use crate::{FromDecoded as _, MltError};

/// If the ratio of unique vertices to total vertices is below this threshold,
/// Morton dictionary encoding is preferred over Vec2 componentwise-delta.
///
/// A lower ratio means more coordinate repetition, which is precisely the
/// scenario where the dictionary overhead pays off.
const MORTON_UNIQUENESS_THRESHOLD: f64 = 0.5;

/// A pre-computed set of per-stream [`IntEncoder`] candidates derived from a
/// representative sample of tiles.
///
/// Building a profile once from sample tiles avoids re-running
/// [`DataProfile::prune_candidates`] on every subsequent tile; the profile's
/// per-stream candidate lists are used directly in the competition step instead.
///
/// Profiles from multiple samples are combined with [`GeometryProfile::merge`],
/// which takes the union of each stream's candidate sets.
#[derive(Debug, Clone, PartialEq)]
pub struct GeometryProfile {
    /// Per-stream encoder candidates to use during competition.
    ///
    /// An absent entry causes the caller to fall back to [`IntEncoder::auto_u32`].
    stream_candidates: HashMap<StreamType, Vec<IntEncoder>>,
}

impl GeometryProfile {
    #[doc(hidden)]
    #[must_use]
    pub fn new(stream_candidates: HashMap<StreamType, Vec<IntEncoder>>) -> Self {
        Self { stream_candidates }
    }

    /// Build a profile from a sample of decoded geometry.
    pub fn from_sample(decoded: &DecodedGeometry) -> Result<Self, MltError> {
        let vertex_buffer_type = decoded
            .vertices
            .as_deref()
            .map_or(VertexBufferType::Vec2, select_vertex_strategy);

        let mut probe = GeometryEncoder::all(IntEncoder::varint());
        probe.vertex_buffer_type(vertex_buffer_type);

        let mut stream_candidates: HashMap<StreamType, Vec<IntEncoder>> = HashMap::new();
        encode_geometry(
            decoded,
            &probe,
            Some(&mut |st: StreamType, data: &[u32]| {
                let candidates = DataProfile::prune_candidates::<i32>(data);
                stream_candidates.insert(st, candidates);
            }),
        )?;

        Ok(Self { stream_candidates })
    }

    /// Merge two profiles by taking the union of per-stream candidate sets.
    ///
    /// Encoders already present for a stream in `self` are not duplicated.
    #[must_use]
    pub fn merge(mut self, other: &Self) -> Self {
        for (st, candidates) in &other.stream_candidates {
            let entry = self.stream_candidates.entry(*st).or_default();
            for &enc in candidates {
                if !entry.contains(&enc) {
                    entry.push(enc);
                }
            }
        }
        self
    }
}

/// Analyze `decoded` and encode it with automatically selected per-stream encoders.
///
/// # Design
///
/// 1. **Probe** - call `encode_geometry` with an all-varint encoder and an
///    `on_stream` callback that collects the raw `u32` payload for every stream.
/// 2. **Select** - run [`IntEncoder::auto_u32`] on each payload to pick the best
///    physical/logical combination per stream.
fn optimize(decoded: &DecodedGeometry) -> Result<GeometryEncoder, MltError> {
    let vertex_buffer_type = decoded
        .vertices
        .as_deref()
        .map_or(VertexBufferType::Vec2, select_vertex_strategy);

    // Pass 1: probe with an all-varint encoder and collect the raw u32
    // payload for each stream via the on_stream callback.
    let mut probe = GeometryEncoder::all(IntEncoder::varint());
    probe.vertex_buffer_type(vertex_buffer_type);

    let mut payloads: HashMap<StreamType, Vec<u32>> = HashMap::new();
    encode_geometry(
        decoded,
        &probe,
        Some(&mut |st: StreamType, data: &[u32]| {
            payloads.insert(st, data.to_vec());
        }),
    )?;

    let opt = |st: StreamType| -> IntEncoder {
        payloads
            .get(&st)
            .map_or_else(IntEncoder::varint, |data| IntEncoder::auto_u32(data))
    };

    Ok(build_encoder(decoded, vertex_buffer_type, opt))
}

/// Apply a profile to `decoded`, re-deriving the vertex buffer strategy from
/// the tile's actual data.
///
/// The same probe pass as [`optimize`] is performed, but competition is run
/// over the profile's pre-computed per-stream candidate lists rather than
/// re-running [`DataProfile::prune_candidates`] from scratch.
fn apply_profile(
    decoded: &DecodedGeometry,
    profile: &GeometryProfile,
) -> Result<GeometryEncoder, MltError> {
    let vertex_buffer_type = decoded
        .vertices
        .as_deref()
        .map_or(VertexBufferType::Vec2, select_vertex_strategy);

    let mut probe = GeometryEncoder::all(IntEncoder::varint());
    probe.vertex_buffer_type(vertex_buffer_type);

    let mut payloads: HashMap<StreamType, Vec<u32>> = HashMap::new();
    encode_geometry(
        decoded,
        &probe,
        Some(&mut |st: StreamType, data: &[u32]| {
            payloads.insert(st, data.to_vec());
        }),
    )?;

    let opt = |st: StreamType| -> IntEncoder {
        let data = payloads.get(&st);
        let candidates = profile.stream_candidates.get(&st).map(Vec::as_slice);
        match (data, candidates) {
            (Some(data), Some(candidates)) if !candidates.is_empty() => {
                DataProfile::compete_u32(candidates, data)
            }
            (Some(data), _) => IntEncoder::auto_u32(data),
            _ => IntEncoder::varint(),
        }
    };

    Ok(build_encoder(decoded, vertex_buffer_type, opt))
}

/// Assemble a [`GeometryEncoder`] from a stream-type-to-encoder mapping function.
///
/// Shared by [`optimize`] and [`apply_profile`]; the only difference between
/// the two is how `opt` resolves the best [`IntEncoder`] for each stream.
fn build_encoder(
    decoded: &DecodedGeometry,
    vertex_buffer_type: VertexBufferType,
    mut opt: impl FnMut(StreamType) -> IntEncoder,
) -> GeometryEncoder {
    // Parts and Rings map to different GeometryEncoder fields depending on
    // which topology branch fired.  Since only one branch can fire per call,
    // each StreamType appears at most once in `payloads`.
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
    encoder
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

impl ManualOptimisation for OwnedGeometry {
    type UsedEncoder = GeometryEncoder;

    fn manual_optimisation(&mut self, encoder: Self::UsedEncoder) -> Result<(), MltError> {
        let dec = self.decode()?;
        *self = OwnedGeometry::Encoded(OwnedEncodedGeometry::from_decoded(&dec, encoder)?);
        Ok(())
    }
}

impl ProfileOptimisation for OwnedGeometry {
    type UsedEncoder = GeometryEncoder;
    type Profile = GeometryProfile;

    fn profile_driven_optimisation(
        &mut self,
        profile: &Self::Profile,
    ) -> Result<Self::UsedEncoder, MltError> {
        match self {
            OwnedGeometry::Decoded(dec) => {
                let enc = apply_profile(dec, profile)?;
                *self = OwnedGeometry::Encoded(OwnedEncodedGeometry::from_decoded(dec, enc)?);
                Ok(enc)
            }
            OwnedGeometry::Encoded(e) => {
                let dec = DecodedGeometry::decode(e.as_borrowed())?;
                *self = OwnedGeometry::Decoded(dec);
                self.profile_driven_optimisation(profile)
            }
        }
    }
}

impl AutomaticOptimisation for OwnedGeometry {
    type UsedEncoder = GeometryEncoder;

    fn automatic_encoding_optimisation(&mut self) -> Result<Self::UsedEncoder, MltError> {
        match self {
            OwnedGeometry::Decoded(dec) => {
                let enc = optimize(dec)?;
                *self = OwnedGeometry::Encoded(OwnedEncodedGeometry::from_decoded(dec, enc)?);
                Ok(enc)
            }
            OwnedGeometry::Encoded(e) => {
                let dec = DecodedGeometry::decode(e.as_borrowed())?;
                *self = OwnedGeometry::Decoded(dec);
                self.automatic_encoding_optimisation()
            }
        }
    }
}
