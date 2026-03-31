use arbitrary::Error::IncorrectFormat;
use arbitrary::Unstructured;

use crate::v01::{
    EncodedProperty, ScalarEncoder, StagedProperty, StagedSharedDict, StagedStrings,
    build_staged_shared_dict,
};

impl<'a> arbitrary::Arbitrary<'a> for StagedSharedDict {
    fn arbitrary(u: &mut Unstructured<'a>) -> arbitrary::Result<Self> {
        let items_raw: Vec<(String, Vec<Option<String>>)> = u.arbitrary()?;
        if items_raw.is_empty() {
            return Ok(Self {
                prefix: u.arbitrary()?,
                data: String::new(),
                items: Vec::new(),
            });
        }
        let prefix: String = u.arbitrary()?;
        let staged_items: Vec<(String, StagedStrings)> = items_raw
            .into_iter()
            .map(|(suffix, vals)| (suffix, StagedStrings::from(vals)))
            .collect();
        build_staged_shared_dict(prefix, staged_items).map_err(|_| IncorrectFormat)
    }
}

impl arbitrary::Arbitrary<'_> for EncodedProperty {
    fn arbitrary(u: &mut Unstructured<'_>) -> arbitrary::Result<Self> {
        let decoded: StagedProperty = u.arbitrary()?;
        let encoder: ScalarEncoder = u.arbitrary()?;
        let prop: Option<Self> = Self::encode(&decoded, encoder).map_err(|_| IncorrectFormat)?;
        prop.ok_or(IncorrectFormat)
    }
}

impl arbitrary::Arbitrary<'_> for StagedProperty {
    fn arbitrary(u: &mut Unstructured<'_>) -> arbitrary::Result<Self> {
        let values: Vec<Option<u32>> = u.arbitrary()?;
        Ok(Self::u32("prop", values))
    }
}

impl arbitrary::Arbitrary<'_> for StagedStrings {
    fn arbitrary(u: &mut Unstructured<'_>) -> arbitrary::Result<Self> {
        Ok(Self::from(u.arbitrary::<Vec<Option<String>>>()?))
    }
}
