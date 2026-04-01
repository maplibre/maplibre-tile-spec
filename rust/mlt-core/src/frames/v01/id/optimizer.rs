use crate::MltResult;
use crate::v01::{
    DataProfile, EncodedId, EncoderConfig, IdEncoder, IdValues, IdWidth, IntEncoder,
    LogicalEncoder, PhysicalEncoder,
};

struct SequenceStats {
    is_sequential: bool,
    is_constant: bool,
    id_width: IdWidth,
}

/// Collect `is_sequential`, `is_constant`, and [`IdWidth`] in a single pass.
///
/// Returns `Err(default_encoder)` for the empty or all-null case so callers
/// can return early.
fn calc_sequence_stats(ids: &[Option<u64>]) -> Option<SequenceStats> {
    let mut has_nulls = false;
    let mut is_sequential = true;
    let mut is_constant = true;

    let mut ids_iter = ids.iter();
    let first_non_null = loop {
        match ids_iter.next() {
            Some(Some(id)) => break *id,
            Some(None) => has_nulls = true,
            None => return None, // no ids or all are None
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

    let fits_u32 = u32::try_from(max_value).is_ok();
    let id_width = match (has_nulls, fits_u32) {
        (false, true) => IdWidth::Id32,
        (true, true) => IdWidth::OptId32,
        (false, false) => IdWidth::Id64,
        (true, false) => IdWidth::OptId64,
    };

    Some(SequenceStats {
        is_sequential,
        is_constant,
        id_width,
    })
}

/// Run [`DataProfile::prune_candidates`] and filter the result to VarInt-only.
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

impl IdValues {
    /// Returns `true` when the column carries no encodable data — either it is
    /// empty or every value is `None`.  Both cases produce no wire output.
    fn is_empty_or_all_null(&self) -> bool {
        self.0.is_empty() || self.0.iter().all(Option::is_none)
    }

    /// Encode this ID column using the given encoder, consuming `self`.
    /// Returns `None` when the ID list is empty or every value is `None`.
    pub fn encode(self, encoder: IdEncoder) -> MltResult<Option<EncodedId>> {
        if self.is_empty_or_all_null() {
            Ok(None)
        } else {
            Ok(Some(EncodedId::encode(&self, encoder)?))
        }
    }

    /// Automatically select the best encoder and encode, consuming `self`.
    /// Returns `(None, None)` when the ID list is empty or every value is `None`.
    pub fn encode_auto(self, _cfg: EncoderConfig) -> MltResult<Option<(EncodedId, IdEncoder)>> {
        let ids = &self.0;

        let Some(stat) = calc_sequence_stats(ids) else {
            return Ok(None);
        };

        let enc = if ids.len() <= 2 {
            IdEncoder::new(LogicalEncoder::None, stat.id_width)
        } else if stat.is_sequential && ids.len() > 4 {
            IdEncoder::new(LogicalEncoder::DeltaRle, stat.id_width)
        } else if stat.is_constant {
            IdEncoder::new(LogicalEncoder::Rle, stat.id_width)
        } else {
            let candidates = pruned_candidates(ids, stat.id_width);
            let logical = compete_with(ids, stat.id_width, &candidates);
            IdEncoder::new(logical, stat.id_width)
        };

        let encoded = EncodedId::encode(&self, enc)?;

        Ok(Some((encoded, enc)))
    }
}
