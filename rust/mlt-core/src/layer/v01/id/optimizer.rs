use super::DecodedId;
use crate::optimizer::AutomaticOptimisation;
use crate::v01::{
    DataProfile, IdEncoder, IdWidth, IntEncoder, LogicalEncoder, OwnedEncodedId, OwnedId,
    PhysicalEncoder,
};
use crate::{FromDecoded as _, FromEncoded as _, MltError};

/// Analyses a [`DecodedId`] and returns an [`IdEncoder`] with near-optimal
/// encoding settings.
///
/// The returned encoder is guaranteed to be compatible with
/// `OwnedEncodedId::from_decoded`,
/// which unconditionally uses [`PhysicalEncoder::VarInt`].
///
/// # Pipeline
///
/// 1. If the input contains no data, immediately return a zero-cost default.
/// 2. **Single-pass deduction**
///    One iteration over `Vec<Option<u64>>` collects `has_nulls`, `max_value`,
///    `is_sequential`, and `is_constant` simultaneously.  `IdWidth` is derived
///    deterministically from `has_nulls` and `max_value` - it is not a choice
///    but a strict consequence of the data.
/// 3. **Select `LogicalEncoder`**
///    * **Fast-path:**
///      * `is_sequential` -> `DeltaRle`
///      * `is_constant` -> `Rle`.
///    * **Competition:**
///      Extract non-null values, prune candidates via `DataProfile::prune_candidates`,
///      retain only `VarInt` physical encoders (the only physical encoder used by ID streams),
///      then pick the winner with `DataProfile::min_size_encoding_*`.
/// 4. **Assemble and return `IdEncoder`.**
pub struct IdOptimizer;

impl IdOptimizer {
    /// Analyze and return a configured [`IdEncoder`].
    #[must_use]
    pub fn optimize(decoded: &DecodedId) -> IdEncoder {
        let Some(ids) = &decoded.0 else {
            return IdEncoder::new(LogicalEncoder::None, IdWidth::Id32);
        };
        let (is_sequential, is_constant, id_width) = match Self::single_pass_statistics(ids) {
            Ok(value) => value,
            Err(value) => return value,
        };

        // None is optimal; skip allocation and trial encoding.
        if ids.len() <= 2 {
            return IdEncoder::new(LogicalEncoder::None, id_width);
        }

        // Fast-path: all consecutive non-null values increment by exactly 1.
        // DeltaRle is optimal; skip allocation and trial encoding.
        if is_sequential && ids.len() > 4 {
            return IdEncoder::new(LogicalEncoder::DeltaRle, id_width);
        }

        // Fast-path: every non-null value is identical.
        // Rle is optimal; skip allocation and trial encoding.
        if is_constant {
            return IdEncoder::new(LogicalEncoder::Rle, id_width);
        }
        // Profile, prune, filter, and compete to find the best logical encoder.
        let logical = Self::compete(ids, id_width);
        IdEncoder::new(logical, id_width)
    }

    fn single_pass_statistics(ids: &[Option<u64>]) -> Result<(bool, bool, IdWidth), IdEncoder> {
        let mut has_nulls = false;
        let mut is_sequential = true;
        let mut is_constant = true;

        // Find first non-null
        let mut ids_iter = ids.iter();
        let first_non_null = loop {
            if let Some(opt_id) = ids_iter.next() {
                if let Some(id) = opt_id {
                    break *id;
                }
                has_nulls = true;
            } else {
                // Either empty or all values are null; return early with the trivial default.
                return Err(IdEncoder::new(LogicalEncoder::None, IdWidth::Id32));
            }
        };

        let mut max_value = first_non_null;
        let mut prev_non_null = first_non_null;

        for &id in ids_iter {
            match id {
                None => has_nulls = true,
                Some(v) => {
                    max_value = max_value.max(v);
                    // Sequential: each consecutive non-null must be exactly prev_non_null + 1
                    if v != prev_non_null.wrapping_add(1) {
                        is_sequential = false;
                    }
                    // Constant: every non-null must equal the first
                    if v != first_non_null {
                        is_constant = false;
                    }
                    prev_non_null = v;
                }
            }
        }

        let id_width = Self::deduce_width(has_nulls, max_value);
        Ok((is_sequential, is_constant, id_width))
    }

    /// Determine the narrowest correct `IdWidth` for the given data.
    ///
    /// Width and nullability are properties of the data, not choices to optimise.
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

    /// Run the profiling-competition pipeline to select the best [`LogicalEncoder`].
    ///
    /// Candidates are pruned by [`DataProfile::prune_candidates`] and then
    /// filtered to retain only those with `physical == VarInt`, because
    /// `OwnedEncodedId::from_decoded`
    /// always uses [`PhysicalEncoder::VarInt`] for ID streams.
    fn compete(ids: &[Option<u64>], id_width: IdWidth) -> LogicalEncoder {
        match id_width {
            IdWidth::Id32 | IdWidth::OptId32 => {
                #[expect(
                    clippy::cast_possible_truncation,
                    reason = "width was deduced as ≤ u32::MAX so truncation is safe"
                )]
                let vals: Vec<u32> = ids.iter().flatten().map(|&v| v as u32).collect();
                let candidates = DataProfile::prune_candidates::<i32>(&vals);
                let varint_candidates = Self::filter_varint(&candidates);
                DataProfile::compete_u32(&varint_candidates, &vals).logical
            }
            IdWidth::Id64 | IdWidth::OptId64 => {
                let vals: Vec<u64> = ids.iter().flatten().copied().collect();
                let candidates = DataProfile::prune_candidates::<i64>(&vals);
                let varint_candidates = Self::filter_varint(&candidates);
                DataProfile::compete_u64(&varint_candidates, &vals).logical
            }
        }
    }

    /// Retain only candidates whose physical encoder is [`PhysicalEncoder::VarInt`].
    ///
    /// Falls back to a single plain `VarInt` encoder if filtering would produce
    /// an empty list (defensive; should not occur in practice since u64 pruning
    /// never emits `FastPFOR`).
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
}

impl AutomaticOptimisation for OwnedId {
    type UsedEncoder = Option<IdEncoder>;

    fn automatic_encoding_optimisation(&mut self) -> Result<Self::UsedEncoder, MltError> {
        match self {
            OwnedId::Decoded(dec) => {
                let enc = IdOptimizer::optimize(dec);
                *self = OwnedId::Encoded(OwnedEncodedId::from_decoded(dec, enc)?);
                Ok(Some(enc))
            }
            OwnedId::Encoded(e) => {
                let dec = DecodedId::from_encoded(borrowme::borrow(e))?;
                *self = OwnedId::Decoded(dec);
                self.automatic_encoding_optimisation()
            }
            OwnedId::None => Ok(None),
        }
    }
}
