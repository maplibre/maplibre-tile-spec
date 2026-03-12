use crate::utils::apply_present;
use crate::v01::{DecodedId, EncodedId, EncodedIdValue, Id, OwnedEncodedId, OwnedId, Stream};
use crate::{Decode, DecodeInto as _, MltError};

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

    #[must_use]
    pub fn to_owned(&self) -> OwnedId {
        match self {
            Self::Encoded(encoded) => OwnedId::Encoded(encoded.to_owned()),
            Self::Decoded(decoded) => OwnedId::Decoded(decoded.to_owned()),
        }
    }
}

impl TryFrom<OwnedId> for DecodedId {
    type Error = MltError;

    fn try_from(owned: OwnedId) -> Result<Self, MltError> {
        match owned {
            OwnedId::Encoded(encoded) => DecodedId::try_from(encoded),
            OwnedId::Decoded(decoded) => Ok(decoded),
        }
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

impl TryFrom<OwnedEncodedId> for DecodedId {
    type Error = MltError;

    fn try_from(encoded: OwnedEncodedId) -> Result<Self, MltError> {
        use crate::v01::{OwnedEncodedIdValue, OwnedStream};
        let presence = encoded.presence.as_ref().map(OwnedStream::as_borrowed);
        let value = match &encoded.value {
            OwnedEncodedIdValue::Id32(s) => EncodedIdValue::Id32(s.as_borrowed()),
            OwnedEncodedIdValue::Id64(s) => EncodedIdValue::Id64(s.as_borrowed()),
        };
        DecodedId::try_from(EncodedId { presence, value })
    }
}
