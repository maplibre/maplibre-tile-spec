use super::model::IdWidth;
use crate::MltResult;
use crate::codecs::bytes::encode_bools_to_bytes;
use crate::codecs::rle::encode_byte_rle;
use crate::decoder::{
    ColumnType, IdValues, IntEncoding, LogicalEncoding, PhysicalEncoding, RleMeta, StreamMeta,
    StreamType,
};
use crate::encoder::stream::IntEncoder;
use crate::encoder::{EncodedStream, EncodedStreamData, Encoder};
use crate::utils::BinarySerializer as _;

/// Encode `ids` using an explicit `id_width` and `int_enc`, writing the column-type byte to
/// `enc.meta` and all streams to `enc.data`.
///
/// Returns `false` when the column was omitted (empty or all-null).
///
/// For automatic encoding use [`IdValues::write_to`].
pub fn write_id_to(
    ids: &IdValues,
    id_width: IdWidth,
    int_enc: IntEncoder,
    enc: &mut Encoder,
) -> MltResult<bool> {
    write_id_col_impl(ids, id_width, int_enc, false, enc)
}

/// Shared implementation used by both the explicit and auto paths.
pub(crate) fn write_id_col_impl(
    ids: &IdValues,
    id_width: IdWidth,
    int_enc: IntEncoder,
    forced_presence: bool,
    enc: &mut Encoder,
) -> MltResult<bool> {
    use IdWidth as CFG;

    let presence = if forced_presence || ids.0.iter().any(Option::is_none) {
        let present: Vec<bool> = ids.0.iter().map(Option::is_some).collect();
        let num_values = u32::try_from(present.len())?;
        let data = encode_byte_rle(&encode_bools_to_bytes(&present));

        let runs = num_values.div_ceil(8);
        let num_rle_values = u32::try_from(data.len())?;
        let meta = StreamMeta::new(
            StreamType::Present,
            IntEncoding::new(
                LogicalEncoding::Rle(RleMeta {
                    runs,
                    num_rle_values,
                }),
                PhysicalEncoding::None,
            ),
            num_values,
        );

        Some(EncodedStream {
            meta,
            data: EncodedStreamData::Encoded(data),
        })
    } else {
        None
    };

    let col_type = match (&presence, &id_width) {
        (None, CFG::Id32 | CFG::OptId32) => ColumnType::Id,
        (None, CFG::Id64 | CFG::OptId64) => ColumnType::LongId,
        (Some(_), CFG::Id32 | CFG::OptId32) => ColumnType::OptId,
        (Some(_), CFG::Id64 | CFG::OptId64) => ColumnType::OptLongId,
    };
    col_type.write_to(&mut enc.meta)?;

    enc.write_optional_stream(presence.as_ref())?;

    write_id_value_stream(ids, id_width, int_enc, enc)?;

    enc.push_layer_column();
    Ok(true)
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
