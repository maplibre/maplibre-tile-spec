use crate::utils::apply_present;
use crate::v01::{
    EncodedId, EncodedIdValue, EncodedStream, Id, IdValues, RawId, RawIdValue, RawStream,
};
use crate::{Decode, DecodeInto as _, MltError};

impl<'a> Id<'a> {
    #[must_use]
    pub fn new_encoded(presence: Option<RawStream<'a>>, value: RawIdValue<'a>) -> Self {
        Self::Encoded(RawId { presence, value })
    }

    #[inline]
    pub fn decode(self) -> Result<IdValues, MltError> {
        Ok(match self {
            Self::Encoded(v) => v.decode_into()?,
            Self::Decoded(v) => v,
        })
    }
}

impl IdValues {
    #[must_use]
    pub fn values(&self) -> &[Option<u64>] {
        &self.0
    }
}

impl TryFrom<RawId<'_>> for IdValues {
    type Error = MltError;

    fn try_from(RawId { presence, value }: RawId<'_>) -> Result<Self, MltError> {
        // Decode the ID values first
        let ids_u64: Vec<u64> = match value {
            RawIdValue::Id32(stream) => {
                // Decode 32-bit IDs as u32, then convert to u64
                let ids: Vec<u32> = stream.decode_into()?;
                ids.into_iter().map(u64::from).collect()
            }
            RawIdValue::Id64(stream) => {
                // Decode 64-bit IDs directly as u64
                stream.decode_into()?
            }
        };
        Ok(IdValues(apply_present(presence, ids_u64)?))
    }
}

impl<'a> Decode<RawId<'a>> for IdValues {
    fn decode(input: RawId<'a>) -> Result<Self, MltError> {
        IdValues::try_from(input)
    }
}

impl TryFrom<EncodedId> for IdValues {
    type Error = MltError;

    fn try_from(encoded: EncodedId) -> Result<Self, MltError> {
        let presence = encoded.presence.as_ref().map(EncodedStream::as_borrowed);
        let value = match &encoded.value {
            EncodedIdValue::Id32(s) => RawIdValue::Id32(s.as_borrowed()),
            EncodedIdValue::Id64(s) => RawIdValue::Id64(s.as_borrowed()),
        };
        IdValues::try_from(RawId { presence, value })
    }
}
