use strum::{EnumCount as _, IntoEnumIterator as _};

use crate::MltError;
use crate::v01::property::optimizer::{
    encode_properties, encode_properties_automatic, encode_properties_with_profile,
};
use crate::v01::sort::{reorder_features, spatial_sort_likely_to_help};
use crate::v01::tile::TileLayer01;
use crate::v01::{
    EncodedLayer01, GeometryEncoder, GeometryProfile, IdEncoder, IdProfile, PropertyEncoder,
    PropertyProfile, SortStrategy, StagedLayer01,
};

/// Feature-count threshold above which the spatial trial is subject to the
/// bounding-box pruning heuristic.
const SORT_TRIAL_THRESHOLD: usize = 512;

/// Candidate sort strategies evaluated during automatic competitive trialing.
const TRIAL_STRATEGIES: [Option<SortStrategy>; 3] = [
    None,
    Some(SortStrategy::SpatialMorton),
    Some(SortStrategy::Id),
];

impl StagedLayer01 {
    /// Encode this layer using the given encoder, producing a wire-ready [`EncodedLayer01`].
    pub fn encode(self, encoder: Tag01Encoder) -> Result<EncodedLayer01, MltError> {
        let mut source = TileLayer01::from(self);
        reorder_features(&mut source, encoder.sort_strategy);
        let staged = StagedLayer01::from(source);

        let id = match (encoder.id, staged.id) {
            (Some(id_enc), Some(id)) => id.encode(id_enc)?,
            _ => None,
        };
        let geometry = staged.geometry.encode(encoder.geometry)?;
        let properties = encode_properties(&staged.properties, encoder.properties)?;

        Ok(EncodedLayer01 {
            name: staged.name,
            extent: staged.extent,
            id,
            geometry,
            properties,
            #[cfg(fuzzing)]
            layer_order: staged.layer_order,
        })
    }

    /// Encode using profile-driven encoder settings. Returns the encoded layer and
    /// the encoder configuration that was chosen.
    pub fn encode_with_profile(
        self,
        profile: &Tag01Profile,
    ) -> Result<(EncodedLayer01, Tag01Encoder), MltError> {
        let sort_strategy = profile.sort_strategy();
        let mut source = TileLayer01::from(self);
        reorder_features(&mut source, sort_strategy);
        let staged = StagedLayer01::from(source);

        let (id_encoded, id_encoder) = match staged.id {
            Some(ref id) => id.encode_with_profile(&profile.id)?,
            None => (None, None),
        };
        let mut props = staged.properties;
        let (properties, prop_encoder) =
            encode_properties_with_profile(&mut props, &profile.properties)?;
        let (geometry, geom_encoder) = staged.geometry.encode_with_profile(&profile.geometry)?;

        let encoder = Tag01Encoder {
            sort_strategy,
            id: id_encoder,
            properties: prop_encoder,
            geometry: geom_encoder,
        };

        Ok((
            EncodedLayer01 {
                name: staged.name,
                extent: staged.extent,
                id: id_encoded,
                geometry,
                properties,
                #[cfg(fuzzing)]
                layer_order: staged.layer_order,
            },
            encoder,
        ))
    }

    /// Automatically select the best sort strategy and stream-level encoders by
    /// competitive trialing. Returns the encoded layer and the chosen encoder.
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
    ///    - Encode with automatic stream-level optimisation.
    ///    - Serialise to a scratch buffer and record byte count.
    /// 4. Return the trial with the smallest byte count.
    pub fn encode_automatic(self) -> Result<(EncodedLayer01, Tag01Encoder), MltError> {
        struct TrialResult {
            layer: EncodedLayer01,
            encoder: Tag01Encoder,
            byte_count: usize,
        }

        let source = TileLayer01::from(self);
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
            let staged = StagedLayer01::from(trial_source);

            let (id_encoded, id_encoder) = match staged.id {
                Some(ref id) => id.encode_automatic()?,
                None => (None, None),
            };
            let mut props = staged.properties;
            let (properties, prop_encoder) = encode_properties_automatic(&mut props)?;
            let (geometry, geom_encoder) = staged.geometry.encode_automatic()?;

            let encoder = Tag01Encoder {
                sort_strategy: strategy,
                id: id_encoder,
                properties: prop_encoder,
                geometry: geom_encoder,
            };

            let trial_layer = EncodedLayer01 {
                name: staged.name,
                extent: staged.extent,
                id: id_encoded,
                geometry,
                properties,
                #[cfg(fuzzing)]
                layer_order: staged.layer_order,
            };

            let mut buf: Vec<u8> = Vec::new();
            trial_layer.write_to(&mut buf)?;
            let byte_count = buf.len();

            if best.as_ref().is_none_or(|b| byte_count < b.byte_count) {
                best = Some(TrialResult {
                    layer: trial_layer,
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
///
/// `None` is always index 0; concrete variants follow in `SortStrategy::iter()`
/// declaration order starting at 1.
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
///
/// The active sort strategy is derived on demand from `strategy_votes` via
/// [`Tag01Profile::sort_strategy`].
///
/// ## Profile merging
///
/// When profiles are accumulated from multiple sample tiles and combined with
/// [`Tag01Profile::merge`], conflicting sort strategies are resolved by
/// **majority vote**: each profile carries a `strategy_votes` tally
/// (one vote per strategy variant) that is summed across all merged profiles.
/// The strategy with the most accumulated votes wins; ties are broken in
/// favour of the simpler strategy (`None` > `Spatial(Morton)` >
/// `Spatial(Hilbert)` > `Id`).
#[derive(Debug, Clone)]
pub struct Tag01Profile {
    /// Per-strategy vote counts used by [`Tag01Profile::merge`] to resolve
    /// conflicts across profiles built from multiple sample tiles.
    ///
    /// Index 0 is always `Option::None` (no sort); indices 1..=COUNT map to
    /// `SortStrategy::iter()` in declaration order.
    strategy_votes: [u32; SortStrategy::COUNT + 1],
    pub id: IdProfile,
    pub properties: PropertyProfile,
    pub geometry: GeometryProfile,
}

impl Tag01Profile {
    /// Construct a profile that votes once for `sort_strategy`.
    ///
    /// This is the standard constructor.  It initialises the `strategy_votes`
    /// tally with a single vote for the given strategy so that
    /// [`Tag01Profile::merge`] can resolve conflicts across profiles built
    /// from multiple sample tiles.
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

    /// Derive the winning sort strategy from accumulated votes.
    ///
    /// Returns the [`SortStrategy`] that received the most votes, or `None`
    /// if no-sort is winning.  Ties break in favour of the lower-index
    /// variant (declaration order in [`SortStrategy`], with `None` first).
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

    /// Override the sort strategy, replacing all accumulated votes with a
    /// single vote for `strategy`.
    ///
    /// This is useful when constructing a profile via a builder chain and you
    /// want to force a particular ordering regardless of any prior votes.
    pub fn set_sort_strategy(&mut self, strategy: Option<SortStrategy>) {
        self.strategy_votes = [0u32; SortStrategy::COUNT + 1];
        self.strategy_votes[sort_strategy_index(strategy)] = 1;
    }

    /// Merge two profiles into one.
    ///
    /// ## Sort strategy resolution
    ///
    /// The `strategy_votes` tallies are summed element-wise.  The strategy
    /// with the highest total vote count is returned by `sort_strategy`.
    /// Ties are broken by strategy index (lowest index wins), giving a
    /// conservative preference for `None` (no sorting) when no clear winner
    /// emerges.
    ///
    /// ## Sub-profile merging
    ///
    /// `id`, `properties`, and `geometry` sub-profiles are each merged using
    /// their own `merge` implementations, which take the union of the
    /// respective candidate encoder sets.
    #[must_use]
    pub fn merge(self, other: &Self) -> Self {
        // Sum vote tallies.
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
