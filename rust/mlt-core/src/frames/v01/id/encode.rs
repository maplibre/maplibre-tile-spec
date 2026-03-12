use crate::MltError;
use crate::encode::FromDecoded;
use crate::utils::{encode_bools_to_bytes, encode_byte_rle};
use crate::v01::{
    DecodedId, IdWidth, IntEncoder, IntEncoding, LogicalEncoder, LogicalEncoding, OwnedEncodedId,
    OwnedEncodedIdValue, OwnedStream, OwnedStreamData, PhysicalEncoder, PhysicalEncoding, RleMeta,
    StreamMeta, StreamType,
};

/// How to encode IDs
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(all(not(test), feature = "arbitrary"), derive(arbitrary::Arbitrary))]
pub struct IdEncoder {
    pub logical: LogicalEncoder,
    pub id_width: IdWidth,
}

impl IdEncoder {
    #[must_use]
    pub fn new(logical: LogicalEncoder, id_width: IdWidth) -> Self {
        Self { logical, id_width }
    }
}

#[cfg(all(not(test), feature = "arbitrary"))]
impl arbitrary::Arbitrary<'_> for OwnedEncodedId {
    fn arbitrary(u: &mut arbitrary::Unstructured<'_>) -> arbitrary::Result<Self> {
        let parsed: DecodedId = u.arbitrary()?;
        let encoder: IdEncoder = u.arbitrary()?;
        let owned_id =
            Self::from_decoded(&parsed, encoder).map_err(|_| arbitrary::Error::IncorrectFormat)?;
        Ok(owned_id)
    }
}

impl FromDecoded<'_> for OwnedEncodedId {
    type Input = DecodedId;
    type Encoder = IdEncoder;

    fn from_decoded(decoded: &Self::Input, encoder: IdEncoder) -> Result<Self, MltError> {
        use IdWidth as CFG;

        let presence = if matches!(encoder.id_width, CFG::OptId32 | CFG::OptId64) {
            let present: Vec<bool> = decoded.0.iter().map(Option::is_some).collect();
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

            Some(OwnedStream {
                meta,
                data: OwnedStreamData::Encoded(data),
            })
        } else {
            None
        };

        let value = if matches!(encoder.id_width, CFG::Id32 | CFG::OptId32) {
            #[expect(clippy::cast_possible_truncation, reason = "truncation was requested")]
            let vals: Vec<u32> = decoded
                .0
                .iter()
                .filter_map(|&id| id)
                .map(|v| v as u32)
                .collect();
            OwnedEncodedIdValue::Id32(OwnedStream::encode_u32s(
                &vals,
                IntEncoder::new(encoder.logical, PhysicalEncoder::VarInt),
            )?)
        } else {
            let vals: Vec<u64> = decoded.0.iter().filter_map(|&id| id).collect();
            OwnedEncodedIdValue::Id64(OwnedStream::encode_u64s(
                &vals,
                IntEncoder::new(encoder.logical, PhysicalEncoder::VarInt),
            )?)
        };

        Ok(Self { presence, value })
    }
}
