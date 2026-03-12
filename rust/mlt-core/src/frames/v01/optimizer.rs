use crate::MltError;
use crate::optimizer::{AutomaticOptimisation, ManualOptimisation, ProfileOptimisation};
use crate::v01::sort::{
    ensure_decoded, geometry_feature_count, reorder_features, spatial_sort_likely_to_help,
};
use crate::v01::{
    GeometryEncoder, GeometryProfile, IdEncoder, IdProfile, OwnedLayer01, PropertyEncoder,
    PropertyProfile, SortStrategy, SpaceFillingCurve,
};

impl ManualOptimisation for OwnedLayer01 {
    type UsedEncoder = Tag01Encoder;

    fn manual_optimisation(&mut self, encoder: Self::UsedEncoder) -> Result<(), MltError> {
        reorder_features(self, encoder.sort_strategy)?;
        if let Some(id) = encoder.id {
            self.id.manual_optimisation(id)?;
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
        reorder_features(self, profile.preferred_sort_strategy)?;
        let id = self.id.profile_driven_optimisation(&profile.id)?;
        let properties = self
            .properties
            .profile_driven_optimisation(&profile.properties)?;
        let geometry = self
            .geometry
            .profile_driven_optimisation(&profile.geometry)?;

        Ok(Tag01Encoder {
            sort_strategy: profile.preferred_sort_strategy,
            id,
            properties,
            geometry,
        })
    }
}

// ─── Automatic competitive trialing ──────────────────────────────────────────

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
const TRIAL_STRATEGIES: [SortStrategy; 3] = [
    SortStrategy::None,
    SortStrategy::Spatial(SpaceFillingCurve::Morton),
    SortStrategy::Id,
];

impl AutomaticOptimisation for OwnedLayer01 {
    type UsedEncoder = Tag01Encoder;

    /// Automatically select the best sort strategy and stream-level encoders by
    /// competitive trialing.
    ///
    /// # Algorithm
    ///
    /// 1. Bring the layer into decoded form and read the feature count `N`.
    /// 2. Build a candidate set: `[None, Spatial(Morton), Id]`.
    ///    - When `N >= 512`, apply a bounding-box heuristic: if the vertex
    ///      spread covers more than 80% of the tile extent on both axes,
    ///      spatial sorting is unlikely to cluster features and is dropped from
    ///      the candidates.
    /// 3. For each candidate strategy:
    ///    - Clone the (decoded) layer.
    ///    - Apply `reorder_features` to the clone.
    ///    - Run automatic stream-level optimisation on id, properties, and
    ///      geometry.
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

        // Bring into decoded form so every trial starts from identical data.
        ensure_decoded(self)?;
        let n = geometry_feature_count(&self.geometry)?;

        // Build the candidate slice, optionally pruning spatial sort.
        let filtered: [SortStrategy; 2];
        let candidates: &[SortStrategy] =
            if n < SORT_TRIAL_THRESHOLD || spatial_sort_likely_to_help(self) {
                &TRIAL_STRATEGIES
            } else {
                // Bounding box is too spread out — skip the spatial trial.
                filtered = [SortStrategy::None, SortStrategy::Id];
                &filtered
            };

        // ── Competitive trial loop ────────────────────────────────────────────

        let mut best: Option<TrialResult> = None;

        for &strategy in candidates {
            let mut trial = self.clone();
            reorder_features(&mut trial, strategy)?;

            let id = trial.id.automatic_encoding_optimisation()?;
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

        // `candidates` always contains at least `SortStrategy::None`, so the
        // loop runs at least once and `best` is always `Some` here.
        let winner = best.unwrap();
        *self = winner.layer;
        Ok(winner.encoder)
    }
}

// ─── Layer encoder ────────────────────────────────────────────────────────────

/// Fully-specified encoder configuration for a v01 layer, produced by any of
/// the three optimization paths (manual, automatic, or profile-driven).
///
/// The `sort_strategy` field controls whether features are reordered before
/// stream-level encoding takes place.  It defaults to [`SortStrategy::None`]
/// so that existing callers that construct `Tag01Encoder` directly are
/// unaffected.
#[derive(Debug, Clone)]
pub struct Tag01Encoder {
    /// How to reorder features before encoding.  Defaults to
    /// [`SortStrategy::None`] (preserve original order).
    pub sort_strategy: SortStrategy,
    pub id: Option<IdEncoder>,
    pub properties: Vec<PropertyEncoder>,
    pub geometry: GeometryEncoder,
}

// ─── Layer profile ────────────────────────────────────────────────────────────

/// All concrete [`SortStrategy`] variants in their canonical order.
///
/// The position of each variant in this array is its *vote index* — the
/// index used inside [`Tag01Profile::strategy_votes`] and returned by
/// [`strategy_index`].  The ordering (None first) doubles as the tie-breaking
/// rule in [`Tag01Profile::merge`]: ties favour the simpler strategy.
const STRATEGY_VARIANTS: [SortStrategy; 4] = [
    SortStrategy::None,
    SortStrategy::Spatial(SpaceFillingCurve::Morton),
    SortStrategy::Spatial(SpaceFillingCurve::Hilbert),
    SortStrategy::Id,
];

/// Map a [`SortStrategy`] to its position in [`STRATEGY_VARIANTS`].
fn strategy_index(s: SortStrategy) -> usize {
    match s {
        SortStrategy::None => 0,
        SortStrategy::Spatial(SpaceFillingCurve::Morton) => 1,
        SortStrategy::Spatial(SpaceFillingCurve::Hilbert) => 2,
        SortStrategy::Id => 3,
    }
}

/// Profile for a v01 layer, built by running automatic optimisation over a
/// representative sample of tiles and capturing the chosen encoders.
///
/// `preferred_sort_strategy` records the sorting policy that was used (or
/// should be used) so that profile-driven encoding can reproduce the same
/// feature ordering on subsequent tiles.
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
    /// The sort strategy to apply before encoding each tile under this profile.
    pub preferred_sort_strategy: SortStrategy,

    /// Per-strategy vote counts used by [`Tag01Profile::merge`] to resolve
    /// conflicts across profiles built from multiple sample tiles.
    ///
    /// Index mapping mirrors [`STRATEGY_VARIANTS`]:
    /// `[None, Spatial(Morton), Spatial(Hilbert), Id]`.
    strategy_votes: [u32; 4],

    pub id: IdProfile,
    pub properties: PropertyProfile,
    pub geometry: GeometryProfile,
}

impl Tag01Profile {
    /// Construct a profile that votes once for `preferred_sort_strategy`.
    ///
    /// This is the standard constructor.  It initialises the `strategy_votes`
    /// tally with a single vote for the given strategy so that
    /// [`Tag01Profile::merge`] can resolve conflicts across profiles built
    /// from multiple sample tiles.
    #[must_use]
    pub fn new(
        preferred_sort_strategy: SortStrategy,
        id: IdProfile,
        properties: PropertyProfile,
        geometry: GeometryProfile,
    ) -> Self {
        let mut strategy_votes = [0u32; 4];
        strategy_votes[strategy_index(preferred_sort_strategy)] = 1;
        Self {
            preferred_sort_strategy,
            strategy_votes,
            id,
            properties,
            geometry,
        }
    }

    /// Merge two profiles into one.
    ///
    /// ## Sort strategy resolution
    ///
    /// The `strategy_votes` tallies are summed element-wise.  The strategy
    /// with the highest total vote count becomes the new
    /// `preferred_sort_strategy`.  Ties are broken by strategy index (lowest
    /// index wins), giving a conservative preference for `SortStrategy::None`
    /// when no clear winner emerges.
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

        // Pick the first (lowest-index) strategy with the highest vote count.
        // Ties therefore break in favour of the simpler strategy.
        let winner_idx =
            votes.iter().enumerate().fold(
                0usize,
                |best, (i, &v)| if v > votes[best] { i } else { best },
            );

        let preferred_sort_strategy = STRATEGY_VARIANTS[winner_idx];

        Self {
            preferred_sort_strategy,
            strategy_votes: votes,
            id: self.id.merge(&other.id),
            properties: self.properties.merge(&other.properties),
            geometry: self.geometry.merge(&other.geometry),
        }
    }
}
