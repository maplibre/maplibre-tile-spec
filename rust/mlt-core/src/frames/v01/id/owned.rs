use super::{EncodedId, EncodedIdValue, ParsedId, RawId, RawIdValue};
use crate::v01::RawStream;

impl ParsedId {
    #[must_use]
    pub fn to_owned(&self) -> Self {
        self.clone()
    }
}

impl RawId<'_> {
    #[must_use]
    pub fn to_owned(&self) -> EncodedId {
        EncodedId {
            presence: self.presence.as_ref().map(RawStream::to_owned),
            value: self.value.to_owned(),
        }
    }
}

impl RawIdValue<'_> {
    #[must_use]
    pub fn to_owned(&self) -> EncodedIdValue {
        match self {
            Self::Id32(stream) => EncodedIdValue::Id32(stream.to_owned()),
            Self::Id64(stream) => EncodedIdValue::Id64(stream.to_owned()),
        }
    }
}
