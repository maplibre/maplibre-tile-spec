use arbitrary::Error::IncorrectFormat;
use arbitrary::{Arbitrary, Result, Unstructured};

use crate::encoder::{StagedLayer01, StagedProperty, StagedSharedDict, StagedStrings};

impl Arbitrary<'_> for StagedLayer01 {
    fn arbitrary(u: &mut Unstructured<'_>) -> Result<Self> {
        // Bound name length to prevent OOM from unbounded string generation
        let name_len = u.int_in_range(0..=32u8)? as usize;
        let name: String = (0..name_len)
            .map(|_| u.arbitrary::<char>())
            .collect::<Result<_>>()?;
        let extent: u32 = u.arbitrary()?;
        let id = if u.arbitrary::<bool>()? {
            let count = u.int_in_range(0..=32u8)? as usize;
            let ids: Vec<Option<u64>> = (0..count)
                .map(|_| -> Result<_> {
                    if u.arbitrary::<bool>()? {
                        Ok(Some(u.arbitrary::<u64>()?))
                    } else {
                        Ok(None)
                    }
                })
                .collect::<Result<_>>()?;
            Some(crate::decoder::IdValues(ids))
        } else {
            None
        };
        let geometry = u.arbitrary()?;
        // Bound property count to prevent OOM from unbounded vector generation
        let prop_count = u.int_in_range(0..=4u8)? as usize;
        let properties: Vec<StagedProperty> = (0..prop_count)
            .map(|_| u.arbitrary())
            .collect::<Result<_>>()?;

        Ok(Self {
            name,
            extent,
            id,
            geometry,
            properties,
        })
    }
}

impl<'a> Arbitrary<'a> for StagedSharedDict {
    fn arbitrary(u: &mut Unstructured<'a>) -> Result<Self> {
        // Bound item count and string sizes to prevent OOM
        let item_count = u.int_in_range(0..=8u8)? as usize;
        let items_raw: Vec<(String, Vec<Option<String>>)> = (0..item_count)
            .map(|_| -> Result<_> {
                let name = bounded_string(u, 32)?;
                let val_count = u.int_in_range(0..=16u8)? as usize;
                let values: Vec<Option<String>> = (0..val_count)
                    .map(|_| -> Result<_> {
                        if u.arbitrary()? {
                            Ok(Some(bounded_string(u, 64)?))
                        } else {
                            Ok(None)
                        }
                    })
                    .collect::<Result<_>>()?;
                Ok((name, values))
            })
            .collect::<Result<_>>()?;
        if items_raw.is_empty() {
            return Ok(Self {
                prefix: bounded_string(u, 32)?,
                data: String::new(),
                items: Vec::new(),
            });
        }
        let prefix = bounded_string(u, 32)?;
        Self::new(prefix, items_raw).map_err(|_| IncorrectFormat)
    }
}

impl Arbitrary<'_> for StagedProperty {
    fn arbitrary(u: &mut Unstructured<'_>) -> Result<Self> {
        // Bound value count to prevent OOM from unbounded vector generation
        let count = u.int_in_range(0..=64u8)? as usize;
        let values: Vec<Option<u32>> = (0..count).map(|_| u.arbitrary()).collect::<Result<_>>()?;
        Ok(Self::u32("prop", values))
    }
}

impl Arbitrary<'_> for StagedStrings {
    fn arbitrary(u: &mut Unstructured<'_>) -> Result<Self> {
        let name = bounded_string(u, 32)?;
        // Bound string count and individual string lengths to prevent OOM
        let count = u.int_in_range(0..=16u8)? as usize;
        let values: Vec<Option<String>> = (0..count)
            .map(|_| -> Result<_> {
                if u.arbitrary()? {
                    Ok(Some(bounded_string(u, 64)?))
                } else {
                    Ok(None)
                }
            })
            .collect::<Result<_>>()?;
        Ok(Self::from_optional(name, values))
    }
}

/// Generate a string with bounded length to prevent OOM from unbounded string generation.
pub fn bounded_string(u: &mut Unstructured<'_>, max_len: u8) -> Result<String> {
    let len = u.int_in_range(0..=max_len)? as usize;
    (0..len)
        .map(|_| u.arbitrary::<char>())
        .collect::<Result<_>>()
}
