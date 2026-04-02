use super::model::{EncodedId, EncodedIdValue, IdWidth};
use crate::MltResult;
use crate::codecs::bytes::encode_bools_to_bytes;
use crate::codecs::rle::encode_byte_rle;
use crate::encoder::stream::IntEncoder;
use crate::encoder::{EncodedStream, EncodedStreamData};
use crate::v01::{
    IdValues, IntEncoding, LogicalEncoder, LogicalEncoding, PhysicalEncoding, RleMeta, StreamMeta,
    StreamType,
};

/// How to encode IDs
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(all(not(test), feature = "arbitrary"), derive(arbitrary::Arbitrary))]
pub struct IdEncoder {
    pub int_encoder: IntEncoder,
    pub id_width: IdWidth,
    #[cfg(feature = "__private")]
    forced_presence: bool,
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

impl EncodedId {
    pub(crate) fn encode(value: &IdValues, encoder: IdEncoder) -> MltResult<Self> {
        use IdWidth as CFG;

        #[cfg(feature = "__private")]
        let forced_presence = encoder.forced_presence;
        #[cfg(not(feature = "__private"))]
        let forced_presence = false;

        let presence = if forced_presence || value.0.iter().any(Option::is_none) {
            let present: Vec<bool> = value.0.iter().map(Option::is_some).collect();
            let num_values = u32::try_from(present.len())?;
            let data = encode_byte_rle(&encode_bools_to_bytes(&present));

            // Presence streams always use byte-RLE encoding.
            // The RleMeta values are computed by readers from the stream itself
            // (runs = num_values.div_ceil(8), num_rle_values = byte_length).
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

        let value_stream = if matches!(encoder.id_width, CFG::Id32 | CFG::OptId32) {
            #[expect(clippy::cast_possible_truncation, reason = "truncation was requested")]
            let vals: Vec<u32> = value
                .0
                .iter()
                .filter_map(|&id| id)
                .map(|v| v as u32)
                .collect();
            EncodedIdValue::Id32(EncodedStream::encode_u32s(&vals, encoder.int_encoder)?)
        } else {
            let vals: Vec<u64> = value.0.iter().filter_map(|&id| id).collect();
            EncodedIdValue::Id64(EncodedStream::encode_u64s(&vals, encoder.int_encoder)?)
        };

        Ok(Self {
            presence,
            value: value_stream,
        })
    }
}
