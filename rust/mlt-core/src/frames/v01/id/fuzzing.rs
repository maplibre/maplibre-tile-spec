use crate::v01::{EncodedId, IdEncoder, IdValues};

impl arbitrary::Arbitrary<'_> for EncodedId {
    fn arbitrary(u: &mut arbitrary::Unstructured<'_>) -> arbitrary::Result<Self> {
        let parsed: IdValues = u.arbitrary()?;
        let encoder: IdEncoder = u.arbitrary()?;
        let owned_id =
            Self::encode(&parsed, encoder).map_err(|_| arbitrary::Error::IncorrectFormat)?;
        Ok(owned_id)
    }
}
