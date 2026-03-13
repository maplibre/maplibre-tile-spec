use strum::{EnumCount as _, IntoEnumIterator as _};

use crate::MltError;
use crate::optimizer::{AutomaticOptimisation, ManualOptimisation, ProfileOptimisation};
use crate::v01::sort::{reorder_features, spatial_sort_likely_to_help};
use crate::v01::source::SourceLayer01;
use crate::v01::{
    GeometryEncoder, GeometryProfile, IdEncoder, IdProfile, OwnedLayer01, PropertyEncoder,
    PropertyProfile, SortStrategy,
};

impl ManualOptimisation for OwnedLayer01 {
    type UsedEncoder = Tag01Encoder;

    fn manual_optimisation(&mut self, encoder: Self::UsedEncoder) -> Result<(), MltError> {
        let mut source = SourceLayer01::try_from(std::mem::replace(self, dummy_layer()))?;
        reorder_features(&mut source, encoder.sort_strategy);
        *self = OwnedLayer01::from(source);

        if let (Some(id_enc), Some(id)) = (encoder.id, &mut self.id) {
            id.manual_optimisation(id_enc)?;
        }
        self.properties.manual_optimisation(encoder.properties)?;
        self.geometry.manual_optimisation(encoder.geometry)?;
        Ok(())
    }
}

impl ProfileOptimisation for OwnedLayer01 {
    type UsedEncoder = Tag01Encoder;
    type Profile = Tag01Profile;

    fn profile_driven_optimisation(
        &mut self,
        profile: &Self::Profile,
    ) -> Result<Self::UsedEncoder, MltError> {
        let sort_strategy = profile.sort_strategy();

        let mut source = SourceLayer01::try_from(std::mem::replace(self, dummy_layer()))?;
        reorder_features(&mut source, sort_strategy);
        *self = OwnedLayer01::from(source);

        let id = match &mut self.id {
            Some(id) => id.profile_driven_optimisation(&profile.id)?,
            None => None,
        };
        let properties = self
            .properties
            .profile_driven_optimisation(&profile.properties)?;
        let geometry = self
            .geometry
            .profile_driven_optimisation(&profile.geometry)?;

        Ok(Tag01Encoder {
            sort_strategy,
            id,
            properties,
            geometry,
        })
    }
}

/// Feature-count threshold above which the spatial trial is subject to the
/// bounding-box pruning heuristic.
///
/// Below this count every candidate is always trialed unconditionally — the
/// cost is negligible for small layers and edge-case gains are worth capturing.
const SORT_TRIAL_THRESHOLD: usize = 512;

/// Candidate sort strategies evaluated during automatic competitive trialing.
///
/// Morton is preferred over Hilbert in the automatic path because it is
/// cheaper to compute; Hilbert can be selected explicitly via manual or
/// profile-driven optimisation.
const TRIAL_STRATEGIES: [Option<SortStrategy>; 3] = [
    None,
    Some(SortStrategy::SpatialMorton),
    Some(SortStrategy::Id),
];

impl AutomaticOptimisation for OwnedLayer01 {
    type UsedEncoder = Tag01Encoder;

    /// Automatically select the best sort strategy and stream-level encoders by
    /// competitive trialing.
    ///
    /// # Algorithm
    ///
    /// 1. Convert to [`SourceLayer01`] (decoded, row-oriented).
    /// 2. Build a candidate set: `[None, Some(Spatial(Morton)), Some(Id)]`.
    ///    - When `N >= 512`, apply a bounding-box heuristic: if the vertex
    ///      spread covers more than 80% of the tile extent on both axes,
    ///      spatial sorting is unlikely to cluster features and is dropped from
    ///      the candidates.
    /// 3. For each candidate strategy:
    ///    - Clone the source layer.
    ///    - Apply `reorder_features` to the clone.
    ///    - Convert the clone back to `OwnedLayer01` and run automatic
    ///      stream-level optimisation on id, properties, and geometry.
    ///    - Serialise the fully-encoded clone to a scratch buffer and record
    ///      its byte count.
    /// 4. Keep the trial that produced the smallest byte count and replace
    ///    `self` with the winning encoded layer.
    fn automatic_encoding_optimisation(&mut self) -> Result<Self::UsedEncoder, MltError> {
        struct TrialResult {
            layer: OwnedLayer01,
            encoder: Tag01Encoder,
            byte_count: usize,
        }

        // Convert to source form once; every trial clones this.
        let source = SourceLayer01::try_from(std::mem::replace(self, dummy_layer()))?;
        let n = source.features.len();

        // Build the candidate slice, optionally pruning spatial sort.
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
            let mut trial = OwnedLayer01::from(trial_source);

            let id = match &mut trial.id {
                Some(id) => id.automatic_encoding_optimisation()?,
                None => None,
            };
            let properties = trial.properties.automatic_encoding_optimisation()?;
            let geometry = trial.geometry.automatic_encoding_optimisation()?;

            let encoder = Tag01Encoder {
                sort_strategy: strategy,
                id,
                properties,
                geometry,
            };

            // Serialise to a scratch buffer to measure the total byte cost of
            // this sort strategy (geometry + IDs + properties combined).
            let mut buf: Vec<u8> = Vec::new();
            trial.write_to(&mut buf)?;
            let byte_count = buf.len();

            if best.as_ref().is_none_or(|b| byte_count < b.byte_count) {
                best = Some(TrialResult {
                    layer: trial,
                    encoder,
                    byte_count,
                });
            }
        }

        // `candidates` always contains at least `None`, so `best` is always `Some`.
        let winner = best.unwrap();
        *self = winner.layer;
        Ok(winner.encoder)
    }
}

/// Produce a cheap placeholder `OwnedLayer01` used with `std::mem::replace`
/// to take ownership of `self` without cloning.
fn dummy_layer() -> OwnedLayer01 {
    use crate::v01::{ParsedGeometry, StagedGeometry};
    OwnedLayer01 {
        name: String::new(),
        extent: 0,
        id: None,
        geometry: StagedGeometry::Decoded(ParsedGeometry::default()),
        properties: Vec::new(),
        #[cfg(fuzzing)]
        layer_order: vec![],
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
