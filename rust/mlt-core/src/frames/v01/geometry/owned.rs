use super::{EncodedGeometry, RawGeometry};
use crate::v01::RawStream;

impl RawGeometry<'_> {
    #[must_use]
    pub fn to_owned(&self) -> EncodedGeometry {
        EncodedGeometry {
            meta: self.meta.to_owned(),
            items: self.items.iter().map(RawStream::to_owned).collect(),
        }
    }
}
