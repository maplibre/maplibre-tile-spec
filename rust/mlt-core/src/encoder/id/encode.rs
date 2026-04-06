use super::model::IdWidth;
use crate::MltResult;
use crate::codecs::bytes::encode_bools_to_bytes;
use crate::codecs::rle::encode_byte_rle;
use crate::decoder::{
    ColumnType, DictionaryType, IdValues, IntEncoding, LogicalEncoding, PhysicalEncoding, RleMeta,
    StreamMeta, StreamType,
};
use crate::encoder::model::StreamCtx;
use crate::encoder::stream::{DataProfile, IntEncoder, LogicalEncoder, do_write_u32, do_write_u64};
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

impl IdValues {
    /// Encode and write the ID column to `enc`.
    ///
    /// If `Encoder::get_int_encoder` returns
    /// [`Some`] for `"id"` / `"value"`, uses that encoder and
    /// `Encoder::override_id_width`.
    /// Otherwise, selects among candidate encodings automatically.
    ///
    /// Writes column-type byte to [`enc.meta`](Encoder::meta) and the
    /// presence + value streams to [`enc.data`](Encoder::data).
    /// Returns `false` when the ID list is empty or every value is `None`
    /// (nothing is written in that case).
    #[cfg_attr(feature = "__hotpath", hotpath::measure)]
    pub fn write_to(self, enc: &mut Encoder) -> MltResult<bool> {
        let ids = &self.0;

        let Some(stat) = calc_sequence_stats(ids) else {
            return Ok(false);
        };

        let id_width = enc.override_id_width(stat.id_width);

        // Nullability comes from the actual data (stat.id_width pre-override) or an explicit
        // override_presence request.  `override_id_width` only affects the bit width (32 vs 64);
        // it must not suppress a presence stream when real nulls exist in the data.
        let has_nulls = matches!(stat.id_width, IdWidth::OptId32 | IdWidth::OptId64)
            || enc.override_presence(&StreamCtx::id(StreamType::Present));
        let use_64bit = matches!(id_width, IdWidth::Id64 | IdWidth::OptId64);
        let col_type = match (has_nulls, use_64bit) {
            (false, false) => ColumnType::Id,
            (false, true) => ColumnType::LongId,
            (true, false) => ColumnType::OptId,
            (true, true) => ColumnType::OptLongId,
        };
        col_type.write_to(&mut enc.meta)?;

        // Presence stream (fixed regardless of value encoding choice).
        if has_nulls {
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
        let single_enc = if let Some(int_enc) =
            enc.override_int_enc(&StreamCtx::id(StreamType::Data(DictionaryType::None)))
        {
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
            let candidates = match id_width {
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
#[cfg_attr(feature = "__hotpath", hotpath::measure)]
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
        do_write_u32(&vals, StreamType::Data(DictionaryType::None), int_enc, enc)
    } else {
        let vals: Vec<u64> = ids.0.iter().flatten().copied().collect();
        do_write_u64(&vals, StreamType::Data(DictionaryType::None), int_enc, enc)
    }
}
