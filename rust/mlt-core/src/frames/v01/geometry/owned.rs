use super::{DecodedGeometry, EncodedGeometry, OwnedEncodedGeometry};
use crate::v01::Stream;

impl DecodedGeometry {
    #[must_use]
    pub fn to_owned(&self) -> Self {
        self.clone()
    }
}

impl EncodedGeometry<'_> {
    #[must_use]
    pub fn to_owned(&self) -> OwnedEncodedGeometry {
        OwnedEncodedGeometry {
            meta: self.meta.to_owned(),
            items: self.items.iter().map(Stream::to_owned).collect(),
        }
    }
}
