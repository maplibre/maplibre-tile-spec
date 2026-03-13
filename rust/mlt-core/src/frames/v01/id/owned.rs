use super::{DecodedId, EncodedId, EncodedIdValue, OwnedEncodedId, OwnedEncodedIdValue};
use crate::v01::Stream;

impl DecodedId {
    #[must_use]
    pub fn to_owned(&self) -> Self {
        self.clone()
    }
}

impl EncodedId<'_> {
    #[must_use]
    pub fn to_owned(&self) -> OwnedEncodedId {
        OwnedEncodedId {
            presence: self.presence.as_ref().map(Stream::to_owned),
            value: self.value.to_owned(),
        }
    }
}

impl EncodedIdValue<'_> {
    #[must_use]
    pub fn to_owned(&self) -> OwnedEncodedIdValue {
        match self {
            Self::Id32(stream) => OwnedEncodedIdValue::Id32(stream.to_owned()),
            Self::Id64(stream) => OwnedEncodedIdValue::Id64(stream.to_owned()),
        }
    }
}
