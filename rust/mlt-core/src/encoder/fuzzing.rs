use arbitrary::Error::IncorrectFormat;

use crate::encoder::EncodedGeometry;

impl arbitrary::Arbitrary<'_> for EncodedGeometry {
    fn arbitrary(u: &mut arbitrary::Unstructured<'_>) -> arbitrary::Result<Self> {
        let decoded = u.arbitrary()?;
        let enc = u.arbitrary()?;
        let geom = Self::encode(&decoded, enc).map_err(|_| IncorrectFormat)?;
        Ok(geom)
    }
}
