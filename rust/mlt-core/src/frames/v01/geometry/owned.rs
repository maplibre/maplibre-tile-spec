use super::{EncodedGeometry, ParsedGeometry, RawGeometry};
use crate::v01::RawStream;

impl ParsedGeometry {
    #[must_use]
    pub fn to_owned(&self) -> Self {
        self.clone()
    }
}

impl RawGeometry<'_> {
    #[must_use]
    pub fn to_owned(&self) -> EncodedGeometry {
        EncodedGeometry {
            meta: self.meta.to_owned(),
            items: self.items.iter().map(RawStream::to_owned).collect(),
        }
    }
}
