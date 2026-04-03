use super::model::IdWidth;
use crate::MltResult;
use crate::codecs::bytes::encode_bools_to_bytes;
use crate::codecs::rle::encode_byte_rle;
use crate::decoder::{
    ColumnType, IdValues, IntEncoding, LogicalEncoder, LogicalEncoding, PhysicalEncoding, RleMeta,
    StreamMeta, StreamType,
};
use crate::encoder::stream::IntEncoder;
use crate::encoder::{EncodedStream, EncodedStreamData, Encoder};
use crate::utils::BinarySerializer as _;

/// How to encode IDs
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(all(not(test), feature = "arbitrary"), derive(arbitrary::Arbitrary))]
pub struct IdEncoder {
    pub int_encoder: IntEncoder,
    pub id_width: IdWidth,
    #[cfg(feature = "__private")]
    pub(super) forced_presence: bool,
}

impl IdEncoder {
    #[must_use]
    pub fn new(logical: LogicalEncoder, id_width: IdWidth) -> Self {
        Self {
            int_encoder: IntEncoder::varint_with(logical),
            id_width,
            #[cfg(feature = "__private")]
            forced_presence: false,
        }
    }

    #[must_use]
    pub fn with_int_encoder(int_encoder: IntEncoder, id_width: IdWidth) -> Self {
        Self {
            int_encoder,
            id_width,
            #[cfg(feature = "__private")]
            forced_presence: false,
        }
    }
}

impl Default for IdEncoder {
    fn default() -> Self {
        Self::new(LogicalEncoder::None, IdWidth::Id32)
    }
}

/// Encode `ids` using `encoder` and write the column type to `enc.meta` and all streams
/// to `enc.data`. Returns `false` if the column was omitted (empty or all-null).
pub(super) fn write_id_to(
    ids: &IdValues,
    encoder: IdEncoder,
    enc: &mut Encoder,
) -> MltResult<bool> {
    use IdWidth as CFG;

    #[cfg(feature = "__private")]
    let forced_presence = encoder.forced_presence;
    #[cfg(not(feature = "__private"))]
    let forced_presence = false;

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

    let col_type = match (&presence, &encoder.id_width) {
        (None, CFG::Id32 | CFG::OptId32) => ColumnType::Id,
        (None, CFG::Id64 | CFG::OptId64) => ColumnType::LongId,
        (Some(_), CFG::Id32 | CFG::OptId32) => ColumnType::OptId,
        (Some(_), CFG::Id64 | CFG::OptId64) => ColumnType::OptLongId,
    };
    col_type.write_to(&mut enc.meta)?;

    // Write presence + value streams to enc.data.
    enc.write_optional_stream(presence.as_ref())?;

    if matches!(encoder.id_width, CFG::Id32 | CFG::OptId32) {
        #[expect(clippy::cast_possible_truncation, reason = "truncation was requested")]
        let vals: Vec<u32> = ids
            .0
            .iter()
            .filter_map(|&id| id)
            .map(|v| v as u32)
            .collect();
        let stream = EncodedStream::encode_u32s(&vals, encoder.int_encoder)?;
        enc.write_stream(&stream)?;
    } else {
        let vals: Vec<u64> = ids.0.iter().filter_map(|&id| id).collect();
        let stream = EncodedStream::encode_u64s(&vals, encoder.int_encoder)?;
        enc.write_stream(&stream)?;
    }

    Ok(true)
}
