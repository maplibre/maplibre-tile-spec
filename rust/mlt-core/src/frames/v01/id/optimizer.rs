use crate::optimizer::{AutomaticOptimisation, ManualOptimisation, ProfileOptimisation};
use crate::v01::{
    DataProfile, DecodedId, IdEncoder, IdWidth, IntEncoder, LogicalEncoder, OwnedEncodedId,
    OwnedId, PhysicalEncoder,
};
use crate::{FromDecoded as _, MltError};

/// A pre-computed set of [`IntEncoder`] candidates derived from a representative
/// sample of tiles.
///
/// Building a profile once from sample tiles avoids re-running
/// [`DataProfile::prune_candidates`] on every subsequent tile; the profile's
/// candidate list is used directly in the competition step instead.
///
/// [`IdWidth`] is not stored because it is always re-derived from the actual
/// data of the tile being encoded.
///
/// Profiles from multiple samples are combined with [`IdProfile::merge`], which
/// takes the union of both candidate sets.
#[derive(Debug, Clone, PartialEq)]
pub struct IdProfile {
    /// Encoder candidates to use during competition.
    ///
    /// An empty list causes the caller to fall back to automatic optimization.
    candidates: Vec<IntEncoder>,
}

impl IdProfile {
    #[doc(hidden)]
    #[must_use]
    pub fn new(candidates: Vec<IntEncoder>) -> Self {
        Self { candidates }
    }

    /// Build a profile from a sample of decoded IDs.
    #[must_use]
    pub fn from_sample(decoded: &DecodedId) -> Self {
        let ids = &decoded.0;
        let Ok((_, _, id_width)) = single_pass_statistics(ids) else {
            return Self {
                candidates: vec![IntEncoder::varint()],
            };
        };
        Self {
            candidates: pruned_candidates(ids, id_width),
        }
    }

    /// Merge two profiles by taking the union of their candidate sets.
    ///
    /// Encoders already present in `self` are not duplicated.
    #[must_use]
    pub fn merge(mut self, other: &Self) -> Self {
        for &enc in &other.candidates {
            if !self.candidates.contains(&enc) {
                self.candidates.push(enc);
            }
        }
        self
    }
}

/// Analyze `decoded` and return a near-optimal [`IdEncoder`].
///
/// Fast paths (short sequences, sequential, constant) are checked first.
/// Otherwise, the full pruning + competition pipeline runs.
fn optimize(decoded: &DecodedId) -> IdEncoder {
    let ids = &decoded.0;
    let (is_sequential, is_constant, id_width) = match single_pass_statistics(ids) {
        Ok(stats) => stats,
        Err(default_enc) => return default_enc,
    };

    if ids.len() <= 2 {
        return IdEncoder::new(LogicalEncoder::None, id_width);
    }

    if is_sequential && ids.len() > 4 {
        return IdEncoder::new(LogicalEncoder::DeltaRle, id_width);
    }

    if is_constant {
        return IdEncoder::new(LogicalEncoder::Rle, id_width);
    }

    let candidates = pruned_candidates(ids, id_width);
    let logical = compete_with(ids, id_width, &candidates);
    IdEncoder::new(logical, id_width)
}

/// Apply a profile to `decoded`, re-deriving [`IdWidth`] from the tile's data.
///
/// The same fast paths as [`optimize`] are applied first. For the general case,
/// competition is run over the profile's pre-computed candidate list rather
/// than re-running the full pruning analysis.
fn apply_profile(decoded: &DecodedId, profile: &IdProfile) -> IdEncoder {
    let ids = &decoded.0;
    let (is_sequential, is_constant, id_width) = match single_pass_statistics(ids) {
        Ok(stats) => stats,
        Err(default_enc) => return default_enc,
    };

    if ids.len() <= 2 {
        return IdEncoder::new(LogicalEncoder::None, id_width);
    }

    if is_sequential && ids.len() > 4 {
        return IdEncoder::new(LogicalEncoder::DeltaRle, id_width);
    }

    if is_constant {
        return IdEncoder::new(LogicalEncoder::Rle, id_width);
    }

    let logical = compete_with(ids, id_width, &profile.candidates);
    IdEncoder::new(logical, id_width)
}

/// Collect `is_sequential`, `is_constant`, and [`IdWidth`] in a single pass.
///
/// Returns `Err(default_encoder)` for the empty or all-null case so callers
/// can return early.
fn single_pass_statistics(ids: &[Option<u64>]) -> Result<(bool, bool, IdWidth), IdEncoder> {
    let mut has_nulls = false;
    let mut is_sequential = true;
    let mut is_constant = true;

    let mut ids_iter = ids.iter();
    let first_non_null = loop {
        match ids_iter.next() {
            Some(Some(id)) => break *id,
            Some(None) => has_nulls = true,
            None => return Err(IdEncoder::new(LogicalEncoder::None, IdWidth::Id32)),
        }
    };

    let mut max_value = first_non_null;
    let mut prev_non_null = first_non_null;

    for &id in ids_iter {
        match id {
            None => has_nulls = true,
            Some(v) => {
                max_value = max_value.max(v);
                if v != prev_non_null.wrapping_add(1) {
                    is_sequential = false;
                }
                if v != first_non_null {
                    is_constant = false;
                }
                prev_non_null = v;
            }
        }
    }

    Ok((
        is_sequential,
        is_constant,
        deduce_width(has_nulls, max_value),
    ))
}

/// Determine the narrowest correct [`IdWidth`] for the given data.
#[inline]
fn deduce_width(has_nulls: bool, max_value: u64) -> IdWidth {
    let fits_u32 = u32::try_from(max_value).is_ok();
    match (has_nulls, fits_u32) {
        (false, true) => IdWidth::Id32,
        (true, true) => IdWidth::OptId32,
        (false, false) => IdWidth::Id64,
        (true, false) => IdWidth::OptId64,
    }
}

/// Run [`DataProfile::prune_candidates`] and filter the result to VarInt-only.
///
/// This is the analysis half of automatic optimization; the competition half
/// is [`compete_with`]. Splitting them lets [`IdProfile`] cache the result.
fn pruned_candidates(ids: &[Option<u64>], id_width: IdWidth) -> Vec<IntEncoder> {
    match id_width {
        IdWidth::Id32 | IdWidth::OptId32 => {
            #[expect(
                clippy::cast_possible_truncation,
                reason = "width was deduced as ≤ u32::MAX so truncation is safe"
            )]
            let vals: Vec<u32> = ids.iter().flatten().map(|&v| v as u32).collect();
            filter_varint(&DataProfile::prune_candidates::<i32>(&vals))
        }
        IdWidth::Id64 | IdWidth::OptId64 => {
            let vals: Vec<u64> = ids.iter().flatten().copied().collect();
            filter_varint(&DataProfile::prune_candidates::<i64>(&vals))
        }
    }
}

/// Run trial encodings over `candidates` and return the [`LogicalEncoder`] that
/// produces the smallest output for `ids`.
fn compete_with(
    ids: &[Option<u64>],
    id_width: IdWidth,
    candidates: &[IntEncoder],
) -> LogicalEncoder {
    let candidates = if candidates.is_empty() {
        &[IntEncoder::varint()][..]
    } else {
        candidates
    };

    match id_width {
        IdWidth::Id32 | IdWidth::OptId32 => {
            #[expect(
                clippy::cast_possible_truncation,
                reason = "width was deduced as ≤ u32::MAX so truncation is safe"
            )]
            let vals: Vec<u32> = ids.iter().flatten().map(|&v| v as u32).collect();
            DataProfile::compete_u32(candidates, &vals).logical
        }
        IdWidth::Id64 | IdWidth::OptId64 => {
            let vals: Vec<u64> = ids.iter().flatten().copied().collect();
            DataProfile::compete_u64(candidates, &vals).logical
        }
    }
}

/// Retain only candidates whose physical encoder is [`PhysicalEncoder::VarInt`],
/// falling back to a single plain `VarInt` if the result would be empty.
fn filter_varint(candidates: &[IntEncoder]) -> Vec<IntEncoder> {
    let filtered: Vec<IntEncoder> = candidates
        .iter()
        .copied()
        .filter(|enc| enc.physical == PhysicalEncoder::VarInt)
        .collect();
    if filtered.is_empty() {
        vec![IntEncoder::varint()]
    } else {
        filtered
    }
}

impl ManualOptimisation for OwnedId {
    type UsedEncoder = IdEncoder;

    fn manual_optimisation(&mut self, encoder: Self::UsedEncoder) -> Result<(), MltError> {
        let dec = self.decode()?;
        if !dec.0.is_empty() {
            *self = OwnedId::Encoded(OwnedEncodedId::from_decoded(&dec, encoder)?);
        }
        Ok(())
    }
}

impl ProfileOptimisation for OwnedId {
    type UsedEncoder = Option<IdEncoder>;
    type Profile = IdProfile;

    fn profile_driven_optimisation(
        &mut self,
        profile: &Self::Profile,
    ) -> Result<Self::UsedEncoder, MltError> {
        match self {
            OwnedId::Decoded(dec) => {
                let enc = apply_profile(dec, profile);
                *self = OwnedId::Encoded(OwnedEncodedId::from_decoded(dec, enc)?);
                Ok(Some(enc))
            }
            OwnedId::Encoded(e) => {
                let dec = DecodedId::try_from(e.as_borrowed())?;
                *self = OwnedId::Decoded(dec);
                self.profile_driven_optimisation(profile)
            }
        }
    }
}

impl AutomaticOptimisation for OwnedId {
    type UsedEncoder = Option<IdEncoder>;

    fn automatic_encoding_optimisation(&mut self) -> Result<Self::UsedEncoder, MltError> {
        match self {
            OwnedId::Decoded(dec) => {
                let enc = optimize(dec);
                *self = OwnedId::Encoded(OwnedEncodedId::from_decoded(dec, enc)?);
                Ok(Some(enc))
            }
            OwnedId::Encoded(e) => {
                let dec = DecodedId::try_from(e.as_borrowed())?;
                *self = OwnedId::Decoded(dec);
                self.automatic_encoding_optimisation()
            }
        }
    }
}
