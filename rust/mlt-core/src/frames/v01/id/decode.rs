use super::model::{DecodedId, EncodedId, EncodedIdValue, Id, OwnedEncodedId, OwnedId};
use crate::MltError;
use crate::decode::{FromEncoded, impl_decodable};
use crate::encode::impl_encodable;
use crate::utils::apply_present;
use crate::v01::Stream;

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
            Self::Encoded(v) => Option::<DecodedId>::from_encoded(v)?.unwrap_or_default(),
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

impl<'a> FromEncoded<'a> for DecodedId {
    type Input = EncodedId<'a>;

    fn from_encoded(EncodedId { presence, value }: EncodedId<'_>) -> Result<Self, MltError> {
        let ids_u64: Vec<u64> = match value {
            EncodedIdValue::Id32(stream) => {
                let ids: Vec<u32> = stream.decode_bits_u32()?.decode_u32()?;
                ids.into_iter().map(u64::from).collect()
            }
            EncodedIdValue::Id64(stream) => stream.decode_u64()?,
        };

        let ids_optional = apply_present(presence, ids_u64)?;

        Ok(DecodedId(ids_optional))
    }
}

impl<'a> FromEncoded<'a> for Option<DecodedId> {
    type Input = Option<EncodedId<'a>>;

    fn from_encoded(input: Option<EncodedId<'_>>) -> Result<Self, MltError> {
        input.map(DecodedId::from_encoded).transpose()
    }
}
