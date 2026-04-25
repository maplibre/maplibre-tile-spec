use super::model::{IdWidth, StagedId};
use crate::MltResult;
use crate::decoder::{ColumnType, DictionaryType, StreamType};
use crate::encoder::Encoder;
use crate::encoder::model::StreamCtx;
use crate::encoder::stream::{DataProfile, IntEncoder, LogicalEncoder, do_write_u32, do_write_u64};

struct SequenceStats {
    is_sequential: bool,
    is_constant: bool,
    id_width: IdWidth,
}

/// Collect `is_sequential`, `is_constant`, and [`IdWidth`] from the dense values in a single pass.
///
/// Returns `None` for the empty or all-null case (no dense values) so callers can return early.
fn calc_sequence_stats(ids: &StagedId) -> Option<SequenceStats> {
    let values = ids.dense_values();
    let &first = values.first()?; // None → empty or all-null: skip

    let has_nulls = matches!(ids, StagedId::OptId { .. });
    let mut is_sequential = true;
    let mut is_constant = true;
    let mut max_value = first;
    let mut prev = first;

    for &v in &values[1..] {
        max_value = max_value.max(v);
        if v != prev.wrapping_add(1) {
            is_sequential = false;
        }
        if v != first {
            is_constant = false;
        }
        prev = v;
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

impl StagedId {
    /// Encode and write the ID column to `enc`.
    ///
    /// Returns `false` when the ID list is empty or every value is `None`
    /// (nothing is written in that case).
    #[hotpath::measure]
    pub fn write_to(self, enc: &mut Encoder) -> MltResult<bool> {
        let Some(stat) = calc_sequence_stats(&self) else {
            return Ok(false);
        };

        let id_width = enc.override_id_width(stat.id_width);

        // Nullability comes from the actual data or an explicit override_presence request.
        let has_nulls = matches!(self, Self::OptId { .. })
            || enc.override_presence(&StreamCtx::id(StreamType::Present));
        let use_64bit = matches!(id_width, IdWidth::Id64 | IdWidth::OptId64);
        let col_type = match (has_nulls, use_64bit) {
            (false, false) => ColumnType::Id,
            (false, true) => ColumnType::LongId,
            (true, false) => ColumnType::OptId,
            (true, true) => ColumnType::OptLongId,
        };
        enc.write_column_type(col_type)?;

        // Presence stream
        if has_nulls {
            let feature_count = self.feature_count();
            match &self {
                Self::OptId { presence, .. } => {
                    enc.write_presence_section(presence.iter().copied())?;
                }
                Self::Id(_) => {
                    // override_presence requested a stream for an all-present column
                    enc.write_presence_section(std::iter::repeat_n(true, feature_count))?;
                }
            }
        }

        // Fast-path for small or obviously structured sequences.
        let values = self.dense_values();
        let single_enc = if let Some(int_enc) =
            enc.override_int_enc(&StreamCtx::id(StreamType::Data(DictionaryType::None)))
        {
            Some(int_enc)
        } else if values.len() <= 2 {
            Some(IntEncoder::varint_with(LogicalEncoder::None))
        } else if stat.is_sequential && values.len() > 4 {
            Some(IntEncoder::varint_with(LogicalEncoder::DeltaRle))
        } else if stat.is_constant {
            Some(IntEncoder::varint_with(LogicalEncoder::Rle))
        } else {
            None
        };

        if let Some(single_enc) = single_enc {
            write_id_value_stream(&self, id_width, single_enc, enc)?;
        } else {
            let candidates = match id_width {
                IdWidth::Id32 | IdWidth::OptId32 => {
                    #[expect(
                        clippy::cast_possible_truncation,
                        reason = "width was deduced as ≤ u32::MAX so truncation is safe"
                    )]
                    let vals: Vec<u32> = values.iter().map(|&v| v as u32).collect();
                    DataProfile::prune_candidates::<i32>(&vals)
                }
                IdWidth::Id64 | IdWidth::OptId64 => DataProfile::prune_candidates::<i64>(values),
            };
            let mut alt = enc.try_alternatives();
            for &cand in &candidates {
                alt.with(|enc| write_id_value_stream(&self, id_width, cand, enc))?;
            }
        }

        enc.increment_column_count();
        Ok(true)
    }
}

/// Write just the ID value stream (without presence/header). Used by the explicit and
/// auto-candidate paths in `write_to`.
#[hotpath::measure]
pub(crate) fn write_id_value_stream(
    ids: &StagedId,
    id_width: IdWidth,
    int_enc: IntEncoder,
    enc: &mut Encoder,
) -> MltResult<()> {
    use IdWidth as CFG;
    if matches!(id_width, CFG::Id32 | CFG::OptId32) {
        #[expect(clippy::cast_possible_truncation, reason = "truncation was requested")]
        let vals: Vec<u32> = ids.dense_values().iter().map(|v| *v as u32).collect();
        do_write_u32(&vals, StreamType::Data(DictionaryType::None), int_enc, enc)
    } else {
        do_write_u64(
            ids.dense_values(),
            StreamType::Data(DictionaryType::None),
            int_enc,
            enc,
        )
    }
}
