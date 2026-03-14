use strum::{EnumCount as _, IntoEnumIterator as _};

use crate::MltError;
use crate::v01::sort::{reorder_features, spatial_sort_likely_to_help};
use crate::v01::tile::TileLayer01;
use crate::v01::{
    EncodeProperties as _, EncodedLayer01, GeometryEncoder, GeometryProfile, IdEncoder, IdProfile,
    PropertyEncoder, PropertyProfile, SortStrategy, StagedLayer01,
};

impl StagedLayer01 {
    /// Encode using a specific [`StagedLayer01Encoder`], consuming `self` and producing [`EncodedLayer01`].
    pub fn encode(self, encoder: StagedLayer01Encoder) -> Result<EncodedLayer01, MltError> {
        let geometry = self.geometry.encode(encoder.geometry)?;

        let id = match (encoder.id, self.id) {
            (Some(id_enc), Some(id)) => id.encode(id_enc)?,
            (None, Some(id)) => id.encode_auto()?.0,
            _ => None,
        };

        let properties = self.properties.encode(encoder.properties)?;

        Ok(EncodedLayer01 {
            name: self.name,
            extent: self.extent,
            id,
            geometry,
            properties,
            #[cfg(fuzzing)]
            layer_order: vec![],
        })
    }

    /// Profile-driven encode, consuming `self` and producing `(EncodedLayer01, StagedLayer01Encoder)`.
    ///
    /// Note: sort ordering is not applied here; call [`Tile01Encoder::encode`] before this method
    /// if feature ordering matters.
    pub fn encode_with_profile(
        self,
        profile: &Tag01Profile,
    ) -> Result<(EncodedLayer01, StagedLayer01Encoder), MltError> {
        let (geometry, geom_enc) = self.geometry.encode_with_profile(&profile.geometry)?;

        let id_enc;
        let id;
        if let Some(parsed_id) = self.id {
            let (enc_id, enc) = parsed_id.encode_with_profile(&profile.id)?;
            id = enc_id;
            id_enc = enc;
        } else {
            id = None;
            id_enc = None;
        }

        let (properties, props_enc) = self.properties.encode_with_profile(&profile.properties)?;

        let encoder = StagedLayer01Encoder {
            id: id_enc,
            properties: props_enc,
            geometry: geom_enc,
        };

        Ok((
            EncodedLayer01 {
                name: self.name,
                extent: self.extent,
                id,
                geometry,
                properties,
                #[cfg(fuzzing)]
                layer_order: vec![],
            },
            encoder,
        ))
    }

    /// Automatically select the best stream-level encoders by competitive trialing.
    ///
    /// This method does **not** attempt different sort strategies; call
    /// [`Tile01Encoder::encode_auto`] instead when sort optimisation is also desired.
    pub fn encode_auto(self) -> Result<(EncodedLayer01, StagedLayer01Encoder), MltError> {
        let (geom_enc_result, geom_enc) = self.geometry.encode_auto()?;
        let (id_enc_result, id_enc) = match self.id {
            Some(parsed_id) => parsed_id.encode_auto()?,
            None => (None, None),
        };
        let (encoded_properties, props_enc) = self.properties.encode_auto()?;

        let stream_encoder = StagedLayer01Encoder {
            id: id_enc,
            properties: props_enc,
            geometry: geom_enc,
        };

        let layer = EncodedLayer01 {
            name: self.name,
            extent: self.extent,
            id: id_enc_result,
            geometry: geom_enc_result,
            properties: encoded_properties,
            #[cfg(fuzzing)]
            layer_order: vec![],
        };

        Ok((layer, stream_encoder))
    }
}

/// Feature-count threshold above which the spatial trial is subject to the
/// bounding-box pruning heuristic.
const SORT_TRIAL_THRESHOLD: usize = 512;

/// Candidate sort strategies evaluated during automatic competitive trialing.
const TRIAL_STRATEGIES: [Option<SortStrategy>; 3] = [
    None,
    Some(SortStrategy::SpatialMorton),
    Some(SortStrategy::Id),
];

/// Stream-level encoder configuration for a v01 layer.
///
/// Produced by any of the three optimisation paths (manual, automatic, or profile-driven)
/// and consumed by [`StagedLayer01::encode`].  Sort ordering is handled separately by
/// [`Tile01Encoder`] before this stage.
#[derive(Debug, Clone)]
pub struct StagedLayer01Encoder {
    pub id: Option<IdEncoder>,
    pub properties: Vec<PropertyEncoder>,
    pub geometry: GeometryEncoder,
}

/// Entry-point encoder that converts a [`TileLayer01`] into a [`StagedLayer01`] and
/// optionally reorders features according to a [`SortStrategy`].
///
/// For automatic sort-strategy selection, use [`Tile01Encoder::encode_auto`].
#[derive(Debug, Clone, Default)]
pub struct Tile01Encoder {
    /// How to reorder features before columnar staging.  `None` preserves the
    /// original input order.
    pub sort_strategy: Option<SortStrategy>,
}

impl Tile01Encoder {
    /// Reorder features in `data` according to the configured sort strategy
    /// (no-op when `sort_strategy` is `None`), then convert to [`StagedLayer01`].
    pub fn encode(&self, data: &mut TileLayer01) -> StagedLayer01 {
        reorder_features(data, self.sort_strategy);
        StagedLayer01::from(data.clone())
    }

    /// Automatically select the best sort strategy and stream-level encoders by
    /// competitive trialing.
    ///
    /// # Algorithm
    ///
    /// 1. Build a candidate set: `[None, Some(Spatial(Morton)), Some(Id)]`.
    ///    - When `N >= 512`, apply a bounding-box heuristic: if the vertex
    ///      spread covers more than 80% of the tile extent on both axes,
    ///      spatial sorting is unlikely to cluster features and is dropped from
    ///      the candidates.
    /// 2. For each candidate strategy:
    ///    - Clone the source layer.
    ///    - Apply `reorder_features` to the clone.
    ///    - Convert the clone to `StagedLayer01` and run automatic
    ///      stream-level optimisation on id, geometry, and properties.
    ///    - Serialise the fully-encoded clone to a scratch buffer and record
    ///      its byte count.
    /// 3. Return the trial that produced the smallest byte count.
    pub fn encode_auto(
        source: &TileLayer01,
    ) -> Result<(EncodedLayer01, StagedLayer01Encoder), MltError> {
        struct TrialResult {
            layer: EncodedLayer01,
            stream_enc: StagedLayer01Encoder,
            byte_count: usize,
        }

        let n = source.features.len();

        let filtered: [Option<SortStrategy>; 2];
        let candidates: &[Option<SortStrategy>] =
            if n < SORT_TRIAL_THRESHOLD || spatial_sort_likely_to_help(source) {
                &TRIAL_STRATEGIES
            } else {
                filtered = [None, Some(SortStrategy::Id)];
                &filtered
            };

        let mut best: Option<TrialResult> = None;

        for &strategy in candidates {
            let mut trial_source = source.clone();
            reorder_features(&mut trial_source, strategy);
            let trial_staged = StagedLayer01::from(trial_source);

            let (trial_layer, trial_stream_enc) = trial_staged.encode_auto()?;

            let mut buf: Vec<u8> = Vec::new();
            trial_layer.write_to(&mut buf)?;
            let byte_count = buf.len();

            if best.as_ref().is_none_or(|b| byte_count < b.byte_count) {
                best = Some(TrialResult {
                    layer: trial_layer,
                    stream_enc: trial_stream_enc,
                    byte_count,
                });
            }
        }

        // `candidates` always contains at least `None`, so `best` is always `Some`.
        let winner = best.unwrap();
        Ok((winner.layer, winner.stream_enc))
    }
}

/// Map `Option<SortStrategy>` to a vote-array index.
fn sort_strategy_index(s: Option<SortStrategy>) -> usize {
    match s {
        None => 0,
        Some(s) => {
            1 + SortStrategy::iter()
                .position(|v| v == s)
                .expect("variant must be present in iter()")
        }
    }
}

/// Profile for a v01 layer, built by running automatic optimisation over a
/// representative sample of tiles and capturing the chosen encoders.
#[derive(Debug, Clone)]
pub struct Tag01Profile {
    strategy_votes: [u32; SortStrategy::COUNT + 1],
    pub id: IdProfile,
    pub properties: PropertyProfile,
    pub geometry: GeometryProfile,
}

impl Tag01Profile {
    #[must_use]
    pub fn new(
        sort_strategy: Option<SortStrategy>,
        id: IdProfile,
        properties: PropertyProfile,
        geometry: GeometryProfile,
    ) -> Self {
        let mut strategy_votes = [0u32; SortStrategy::COUNT + 1];
        strategy_votes[sort_strategy_index(sort_strategy)] = 1;
        Self {
            strategy_votes,
            id,
            properties,
            geometry,
        }
    }

    #[must_use]
    pub fn sort_strategy(&self) -> Option<SortStrategy> {
        let winner_idx = self
            .strategy_votes
            .iter()
            .enumerate()
            .fold(0usize, |best, (i, &v)| {
                if v > self.strategy_votes[best] {
                    i
                } else {
                    best
                }
            });
        if winner_idx == 0 {
            None
        } else {
            SortStrategy::iter().nth(winner_idx - 1)
        }
    }

    pub fn set_sort_strategy(&mut self, strategy: Option<SortStrategy>) {
        self.strategy_votes = [0u32; SortStrategy::COUNT + 1];
        self.strategy_votes[sort_strategy_index(strategy)] = 1;
    }

    #[must_use]
    pub fn merge(self, other: &Self) -> Self {
        let mut votes = self.strategy_votes;
        for (v, &o) in votes.iter_mut().zip(other.strategy_votes.iter()) {
            *v = v.saturating_add(o);
        }
        Self {
            strategy_votes: votes,
            id: self.id.merge(&other.id),
            properties: self.properties.merge(&other.properties),
            geometry: self.geometry.merge(&other.geometry),
        }
    }
}
