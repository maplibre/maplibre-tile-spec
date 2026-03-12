use crate::decode::impl_decodable;
use crate::encode::impl_encodable;
use crate::utils::apply_present;
use crate::v01::{DecodedId, EncodedId, EncodedIdValue, Id, OwnedEncodedId, OwnedId, Stream};
use crate::{Decode, DecodeInto as _, MltError};

impl_decodable!(Id<'a>, Option<EncodedId<'a>>, Option<DecodedId>);
impl_encodable!(OwnedId, Option<DecodedId>, Option<OwnedEncodedId>);

impl<'a> From<EncodedId<'a>> for Id<'a> {
    fn from(value: EncodedId<'a>) -> Self {
        Self::Encoded(Some(value))
    }
}

impl<'a> Id<'a> {
    #[must_use]
    pub fn new_encoded(presence: Option<Stream<'a>>, value: EncodedIdValue<'a>) -> Self {
        Self::Encoded(Some(EncodedId { presence, value }))
    }

    #[inline]
    pub fn decode(self) -> Result<DecodedId, MltError> {
        Ok(match self {
            Self::Encoded(v) => Option::<DecodedId>::decode(v)?.unwrap_or_default(),
            Self::Decoded(v) => v.unwrap_or_default(),
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

impl<'a> Decode<Option<EncodedId<'a>>> for Option<DecodedId> {
    fn decode(input: Option<EncodedId<'a>>) -> Result<Self, MltError> {
        input.map(DecodedId::try_from).transpose()
    }
}
