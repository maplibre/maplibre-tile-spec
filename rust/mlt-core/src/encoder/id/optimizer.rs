#[cfg(feature = "__private")]
use super::encode::write_id_col_impl;
use super::encode::{write_id_to, write_id_value_stream};
use super::model::IdWidth;
use crate::MltResult;
use crate::codecs::bytes::encode_bools_to_bytes;
use crate::codecs::rle::encode_byte_rle;
use crate::decoder::{
    ColumnType, IdValues, IntEncoding, LogicalEncoder, LogicalEncoding, PhysicalEncoding, RleMeta,
    StreamMeta, StreamType,
};
#[cfg(feature = "__private")]
use crate::encoder::optimizer::ExplicitEncoder;
use crate::encoder::stream::{DataProfile, IntEncoder};
use crate::encoder::{EncodedStream, EncodedStreamData, Encoder, EncoderConfig};
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
    /// Encode using an [`ExplicitEncoder`] and write the ID column.
    ///
    /// Auto-detects the [`IdWidth`] from the data, then passes it through
    /// `cfg.override_id_width` so callers can pin the width if needed.
    ///
    /// Writes the column-type byte to [`enc.meta`](Encoder::meta) and the
    /// presence + value streams to [`enc.data`](Encoder::data).
    /// Returns `false` when the ID list is empty or every value is `None`
    /// (nothing is written in that case).
    ///
    /// For automatic encoding, use [`IdValues::write_to`].
    #[cfg(feature = "__private")]
    pub fn write_to_with(self, enc: &mut Encoder, cfg: &ExplicitEncoder) -> MltResult<bool> {
        let ids = &self.0;
        let Some(stat) = calc_sequence_stats(ids) else {
            return Ok(false);
        };
        let id_width = (cfg.override_id_width)(stat.id_width);
        let int_enc = (cfg.get_int_encoder)("id", "value", None);
        write_id_col_impl(&self, id_width, int_enc, false, enc)
    }

    /// Automatically select the best encoder, encode, and write the ID column.
    ///
    /// Writes the column-type byte to [`enc.meta`](Encoder::meta) and the
    /// presence + value streams to [`enc.data`](Encoder::data).
    /// Returns `false` when the ID list is empty or every value is `None`
    /// (nothing is written in that case).
    pub fn write_to(self, enc: &mut Encoder, _cfg: EncoderConfig) -> MltResult<bool> {
        let ids = &self.0;

        let Some(stat) = calc_sequence_stats(ids) else {
            return Ok(false);
        };

        // Fast-path for small or obviously structured sequences.
        if ids.len() <= 2 {
            let int_enc = IntEncoder::varint_with(LogicalEncoder::None);
            return write_id_to(&self, stat.id_width, int_enc, enc);
        }
        if stat.is_sequential && ids.len() > 4 {
            let int_enc = IntEncoder::varint_with(LogicalEncoder::DeltaRle);
            return write_id_to(&self, stat.id_width, int_enc, enc);
        }
        if stat.is_constant {
            let int_enc = IntEncoder::varint_with(LogicalEncoder::Rle);
            return write_id_to(&self, stat.id_width, int_enc, enc);
        }

        // General case: write header+presence once, then try all candidates via
        // start_alternative so only the shortest value-stream encoding is kept.
        let has_nulls = ids.iter().any(Option::is_none);
        let col_type = match (has_nulls, &stat.id_width) {
            (false, IdWidth::Id32 | IdWidth::OptId32) => ColumnType::Id,
            (false, IdWidth::Id64 | IdWidth::OptId64) => ColumnType::LongId,
            (true, IdWidth::Id32 | IdWidth::OptId32) => ColumnType::OptId,
            (true, IdWidth::Id64 | IdWidth::OptId64) => ColumnType::OptLongId,
        };
        col_type.write_to(&mut enc.meta)?;

        // Presence stream (fixed regardless of value encoding choice).
        if has_nulls {
            let present: Vec<bool> = ids.iter().map(Option::is_some).collect();
            let num_values = u32::try_from(present.len())?;
            let data = encode_byte_rle(&encode_bools_to_bytes(&present));
            let runs = num_values.div_ceil(8);
            let num_rle_values = u32::try_from(data.len())?;
            let presence = EncodedStream {
                meta: StreamMeta::new(
                    StreamType::Present,
                    IntEncoding::new(
                        LogicalEncoding::Rle(RleMeta {
                            runs,
                            num_rle_values,
                        }),
                        PhysicalEncoding::None,
                    ),
                    num_values,
                ),
                data: EncodedStreamData::Encoded(data),
            };
            enc.write_boolean_stream(&presence)?;
        }

        // Compete: try every candidate and keep the shortest value stream.
        let candidates = pruned_candidates(ids, stat.id_width);
        for &cand in &candidates {
            enc.start_alternative();
            write_id_value_stream(&self, stat.id_width, cand, enc)?;
        }
        enc.finish_alternatives();

        Ok(true)
    }
}
