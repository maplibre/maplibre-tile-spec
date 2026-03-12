use crate::decode::impl_decodable;
use crate::encode::impl_encodable;
use crate::utils::apply_present;
use crate::v01::{DecodedId, EncodedId, EncodedIdValue, Id, OwnedEncodedId, OwnedId, Stream};
use crate::{Decode, DecodeInto as _, MltError};

impl_decodable!(Id<'a>, EncodedId<'a>, DecodedId);
impl_encodable!(OwnedId, DecodedId, OwnedEncodedId);

impl<'a> Id<'a> {
    #[must_use]
    pub fn new_encoded(presence: Option<Stream<'a>>, value: EncodedIdValue<'a>) -> Self {
        Self::Encoded(EncodedId { presence, value })
    }

    #[inline]
    pub fn decode(self) -> Result<DecodedId, MltError> {
        Ok(match self {
            Self::Encoded(v) => v.decode_into()?,
            Self::Decoded(v) => v,
        })
    }
}

impl DecodedId {
    #[must_use]
    pub fn values(&self) -> &[Option<u64>] {
        &self.0
    }
}

impl TryFrom<EncodedId<'_>> for DecodedId {
    type Error = MltError;

    fn try_from(EncodedId { presence, value }: EncodedId<'_>) -> Result<Self, MltError> {
        // Decode the ID values first
        let ids_u64: Vec<u64> = match value {
            EncodedIdValue::Id32(stream) => {
                // Decode 32-bit IDs as u32, then convert to u64
                let ids: Vec<u32> = stream.decode_into()?;
                ids.into_iter().map(u64::from).collect()
            }
            EncodedIdValue::Id64(stream) => {
                // Decode 64-bit IDs directly as u64
                stream.decode_into()?
            }
        };
        Ok(DecodedId(apply_present(presence, ids_u64)?))
    }
}

impl<'a> Decode<EncodedId<'a>> for DecodedId {
    fn decode(input: EncodedId<'a>) -> Result<Self, MltError> {
        DecodedId::try_from(input)
    }
}
