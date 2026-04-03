use super::model::IdWidth;
use crate::MltResult;
use crate::codecs::bytes::encode_bools_to_bytes;
use crate::codecs::rle::encode_byte_rle;
use crate::decoder::{
    ColumnType, IdValues, IntEncoding, LogicalEncoder, LogicalEncoding, PhysicalEncoding, RleMeta,
    StreamMeta, StreamType,
};
use crate::encoder::stream::{DataProfile, IntEncoder};
use crate::encoder::{EncodedStream, EncodedStreamData, Encoder};
use crate::utils::BinarySerializer as _;

struct SequenceStats {
    is_sequential: bool,
    is_constant: bool,
    id_width: IdWidth,
}

/// Collect `is_sequential`, `is_constant`, and [`IdWidth`] in a single pass.
///
/// Returns `None` for the empty or all-null case so callers can return early.
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

/// Run [`DataProfile::prune_candidates`] for the given ID width.
fn pruned_candidates(ids: &[Option<u64>], id_width: IdWidth) -> Vec<IntEncoder> {
    match id_width {
        IdWidth::Id32 | IdWidth::OptId32 => {
            #[expect(
                clippy::cast_possible_truncation,
                reason = "width was deduced as ≤ u32::MAX so truncation is safe"
            )]
            let vals: Vec<u32> = ids.iter().flatten().map(|&v| v as u32).collect();
            DataProfile::prune_candidates::<i32>(&vals)
        }
        IdWidth::Id64 | IdWidth::OptId64 => {
            let vals: Vec<u64> = ids.iter().flatten().copied().collect();
            DataProfile::prune_candidates::<i64>(&vals)
        }
    }
}

impl IdValues {
    /// Encode and write the ID column to `enc`.
    ///
    /// If [`Encoder::get_int_encoder`](Encoder::get_int_encoder) returns
    /// [`Some`] for `"id"` / `"value"`, uses that encoder and
    /// [`Encoder::override_id_width_for_id`](Encoder::override_id_width).
    /// Otherwise, selects among candidate encodings automatically.
    ///
    /// Writes column-type byte to [`enc.meta`](Encoder::meta) and the
    /// presence + value streams to [`enc.data`](Encoder::data).
    /// Returns `false` when the ID list is empty or every value is `None`
    /// (nothing is written in that case).
    pub fn write_to(self, enc: &mut Encoder) -> MltResult<bool> {
        let ids = &self.0;

        let Some(stat) = calc_sequence_stats(ids) else {
            return Ok(false);
        };

        let id_width = enc.override_id_width(stat.id_width);
        let col_type: ColumnType = id_width.into();
        col_type.write_to(&mut enc.meta)?;

        // Presence stream (fixed regardless of value encoding choice).
        if matches!(id_width, IdWidth::OptId32 | IdWidth::OptId64)
            || enc.override_presence("id", "", None)
        {
            let present: Vec<bool> = ids.iter().map(Option::is_some).collect();
            let num_values = u32::try_from(present.len())?;
            let data = encode_byte_rle(&encode_bools_to_bytes(&present));
            let runs = num_values.div_ceil(8);
            let num_rle_values = u32::try_from(data.len())?;
            let int_enc = IntEncoding::new(
                LogicalEncoding::Rle(RleMeta {
                    runs,
                    num_rle_values,
                }),
                PhysicalEncoding::None,
            );
            let presence = EncodedStream {
                meta: StreamMeta::new(StreamType::Present, int_enc, num_values),
                data: EncodedStreamData::Encoded(data),
            };
            enc.write_boolean_stream(&presence)?;
        }

        // Fast-path for small or obviously structured sequences.
        let single_enc = if let Some(int_enc) = enc.get_int_encoder("id", "", None) {
            Some(int_enc)
        } else if ids.len() <= 2 {
            Some(IntEncoder::varint_with(LogicalEncoder::None))
        } else if stat.is_sequential && ids.len() > 4 {
            Some(IntEncoder::varint_with(LogicalEncoder::DeltaRle))
        } else if stat.is_constant {
            Some(IntEncoder::varint_with(LogicalEncoder::Rle))
        } else {
            None
        };

        if let Some(single_enc) = single_enc {
            write_id_value_stream(&self, id_width, single_enc, enc)?;
        } else {
            // Compete: try every candidate and keep the shortest value stream.
            let candidates = pruned_candidates(ids, id_width);
            for &cand in &candidates {
                enc.start_alternative();
                write_id_value_stream(&self, id_width, cand, enc)?;
            }
            enc.finish_alternatives();
        }

        enc.push_layer_column();
        Ok(true)
    }
}

/// Write just the ID value stream (without presence/header). Used by the auto path's
/// `start_alternative` loop.
pub(crate) fn write_id_value_stream(
    ids: &IdValues,
    id_width: IdWidth,
    int_enc: IntEncoder,
    enc: &mut Encoder,
) -> MltResult<()> {
    use IdWidth as CFG;
    if matches!(id_width, CFG::Id32 | CFG::OptId32) {
        #[expect(clippy::cast_possible_truncation, reason = "truncation was requested")]
        let vals: Vec<u32> = ids.0.iter().flatten().map(|v| *v as u32).collect();
        enc.write_stream(&EncodedStream::encode_u32s(&vals, int_enc)?)?;
    } else {
        let vals: Vec<u64> = ids.0.iter().flatten().copied().collect();
        enc.write_stream(&EncodedStream::encode_u64s(&vals, int_enc)?)?;
    }
    Ok(())
}
