use strum::{EnumCount as _, IntoEnumIterator as _};

use crate::MltError;
use crate::v01::sort::{reorder_features, spatial_sort_likely_to_help};
use crate::v01::tile::TileLayer01;
use crate::v01::{
    EncodeProperties as _, EncodedLayer01, GeometryEncoder, GeometryProfile, IdEncoder, IdProfile,
    PropertyEncoder, PropertyProfile, SortStrategy, StagedLayer01,
};

impl StagedLayer01 {
    /// Encode using a specific [`Tag01Encoder`], consuming `self` and producing [`EncodedLayer01`].
    ///
    /// If `encoder.sort_strategy` is `Some`, features are reordered before encoding.
    pub fn encode(self, encoder: Tag01Encoder) -> Result<EncodedLayer01, MltError> {
        // Apply sort strategy if requested (Stage 5 will move this into Tile01Encoder).
        let this = if let Some(strategy) = encoder.sort_strategy {
            let mut tile = TileLayer01::try_from(self)?;
            reorder_features(&mut tile, Some(strategy));
            StagedLayer01::from(tile)
        } else {
            self
        };

        let geometry = this.geometry.encode(encoder.geometry)?;

        let id = match (encoder.id, this.id) {
            (Some(id_enc), Some(id)) => id.encode(id_enc)?,
            (None, Some(id)) => id.encode_auto()?.0,
            _ => None,
        };

        let properties = this.properties.encode(encoder.properties)?;

        Ok(EncodedLayer01 {
            name: this.name,
            extent: this.extent,
            id,
            geometry,
            properties,
        })
    }

    /// Profile-driven encode, consuming `self` and producing `(EncodedLayer01, Tag01Encoder)`.
    pub fn encode_with_profile(
        self,
        profile: &Tag01Profile,
    ) -> Result<(EncodedLayer01, Tag01Encoder), MltError> {
        let sort_strategy = profile.sort_strategy();

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

        let encoder = Tag01Encoder {
            sort_strategy,
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
            },
            encoder,
        ))
    }

    /// Automatically select the best sort strategy and stream-level encoders by
    /// competitive trialing.
    ///
    /// # Algorithm
    ///
    /// 1. Convert to [`TileLayer01`] (decoded, row-oriented).
    /// 2. Build a candidate set: `[None, Some(Spatial(Morton)), Some(Id)]`.
    ///    - When `N >= 512`, apply a bounding-box heuristic: if the vertex
    ///      spread covers more than 80% of the tile extent on both axes,
    ///      spatial sorting is unlikely to cluster features and is dropped from
    ///      the candidates.
    /// 3. For each candidate strategy:
    ///    - Clone the source layer.
    ///    - Apply `reorder_features` to the clone.
    ///    - Convert the clone back to `StagedLayer01` and run automatic
    ///      stream-level optimisation on id, geometry, and properties.
    ///    - Serialise the fully-encoded clone to a scratch buffer and record
    ///      its byte count.
    /// 4. Return the trial that produced the smallest byte count.
    pub fn encode_auto(self) -> Result<(EncodedLayer01, Tag01Encoder), MltError> {
        struct TrialResult {
            layer: EncodedLayer01,
            encoder: Tag01Encoder,
            byte_count: usize,
        }

        // Convert to source form once; every trial clones this.
        let source = TileLayer01::try_from(self)?;
        let n = source.features.len();

        let filtered: [Option<SortStrategy>; 2];
        let candidates: &[Option<SortStrategy>] =
            if n < SORT_TRIAL_THRESHOLD || spatial_sort_likely_to_help(&source) {
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

            let (geom_enc_result, geom_enc) = trial_staged.geometry.clone().encode_auto()?;
            let (id_enc_result, id_enc) = match trial_staged.id.clone() {
                Some(parsed_id) => parsed_id.encode_auto()?,
                None => (None, None),
            };

            let (encoded_properties, props_enc) = trial_staged.properties.clone().encode_auto()?;

            let encoder = Tag01Encoder {
                sort_strategy: strategy,
                id: id_enc,
                properties: props_enc,
                geometry: geom_enc,
            };

            let trial_encoded = EncodedLayer01 {
                name: trial_staged.name,
                extent: trial_staged.extent,
                id: id_enc_result,
                geometry: geom_enc_result,
                properties: encoded_properties,
            };

            let mut buf: Vec<u8> = Vec::new();
            trial_encoded.write_to(&mut buf)?;
            let byte_count = buf.len();

            if best.as_ref().is_none_or(|b| byte_count < b.byte_count) {
                best = Some(TrialResult {
                    layer: trial_encoded,
                    encoder,
                    byte_count,
                });
            }
        }

        // `candidates` always contains at least `None`, so `best` is always `Some`.
        let winner = best.unwrap();
        Ok((winner.layer, winner.encoder))
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

/// Fully-specified encoder configuration for a v01 layer, produced by any of
/// the three optimization paths (manual, automatic, or profile-driven).
///
/// The `sort_strategy` field controls whether features are reordered before
/// stream-level encoding takes place.  It defaults to [`None`] so that
/// existing callers that construct `Tag01Encoder` directly are unaffected.
#[derive(Debug, Clone)]
pub struct Tag01Encoder {
    /// How to reorder features before encoding.  `None` preserves the
    /// original input order.
    pub sort_strategy: Option<SortStrategy>,
    pub id: Option<IdEncoder>,
    pub properties: Vec<PropertyEncoder>,
    pub geometry: GeometryEncoder,
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
