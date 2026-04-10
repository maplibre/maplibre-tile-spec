use arbitrary::Error::IncorrectFormat;
use arbitrary::{Arbitrary, Result, Unstructured};

use crate::encoder::model::StagedLayer01;
use crate::encoder::{StagedProperty, StagedSharedDict, StagedStrings};

impl Arbitrary<'_> for StagedLayer01 {
    fn arbitrary(u: &mut Unstructured<'_>) -> Result<Self> {
        // Bound name length to prevent OOM from unbounded string generation
        let name_len = u.int_in_range(0..=32u8)? as usize;
        let name: String = (0..name_len)
            .map(|_| u.arbitrary::<char>())
            .collect::<Result<_>>()?;
        let extent: u32 = u.arbitrary()?;
        // Generate geometry first -- its feature count drives ID and property columns.
        let geometry: crate::decoder::GeometryValues = u.arbitrary()?;
        let fc = geometry.vector_types().len();
        let id = if u.arbitrary::<bool>()? {
            let ids: Vec<Option<u64>> = (0..fc)
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
        // Bound property count to prevent OOM from unbounded vector generation.
        // Each column must have exactly `fc` values to match the feature count.
        let prop_count = u.int_in_range(0..=4u8)? as usize;
        let properties: Vec<StagedProperty> = (0..prop_count)
            .map(|_| {
                let values: Vec<Option<u32>> =
                    (0..fc).map(|_| u.arbitrary()).collect::<Result<_>>()?;
                Ok(StagedProperty::opt_u32("prop", values))
            })
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
            .map(|_| Ok((bounded_string(u, 32)?, generate_strings(u)?)))
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
        Ok(Self::opt_u32("prop", values))
    }
}

impl Arbitrary<'_> for StagedStrings {
    fn arbitrary(u: &mut Unstructured<'_>) -> Result<Self> {
        Ok(Self::from_optional(
            bounded_string(u, 32)?,
            generate_strings(u)?,
        ))
    }
}

/// Generate a string with bounded length to prevent OOM from unbounded string generation.
pub fn bounded_string(u: &mut Unstructured<'_>, max_len: u8) -> Result<String> {
    let len = u.int_in_range(0..=max_len)? as usize;
    (0..len)
        .map(|_| u.arbitrary::<char>())
        .collect::<Result<_>>()
}

fn generate_strings(u: &mut Unstructured) -> Result<Vec<Option<String>>> {
    // Bound string count and individual string lengths to prevent OOM
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
    Ok(values)
}
