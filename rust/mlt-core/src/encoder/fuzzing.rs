use arbitrary::Error::IncorrectFormat;
use arbitrary::{Arbitrary, Result, Unstructured};

use crate::decoder::IdValues;
use crate::encoder::{
    EncodedGeometry, EncodedId, EncodedLayer01, EncodedProperty, IdEncoder, ScalarEncoder,
    StagedProperty, StagedSharedDict, StagedStrings,
};

impl Arbitrary<'_> for EncodedGeometry {
    fn arbitrary(u: &mut Unstructured<'_>) -> Result<Self> {
        let decoded = u.arbitrary()?;
        let enc = u.arbitrary()?;
        let geom = Self::encode(&decoded, enc).map_err(|_| IncorrectFormat)?;
        Ok(geom)
    }
}

impl Arbitrary<'_> for EncodedLayer01 {
    fn arbitrary(u: &mut Unstructured<'_>) -> Result<Self> {
        let name: String = u.arbitrary()?;
        let extent: u32 = u.arbitrary()?;
        let id: Option<EncodedId> = if u.arbitrary()? {
            Some(u.arbitrary()?)
        } else {
            None
        };
        let geometry = u.arbitrary()?;
        let properties: Vec<EncodedProperty> = u.arbitrary()?;

        #[cfg(fuzzing)]
        let layer_order = {
            use crate::decoder::fuzzing::LayerOrdering;
            // Build a valid layer_order and Fisher-Yates shuffle it.
            let mut layer_order: Vec<LayerOrdering> = Vec::new();
            if id.is_some() {
                layer_order.push(LayerOrdering::Id);
            }
            layer_order.push(LayerOrdering::Geometry);
            for _ in &properties {
                layer_order.push(LayerOrdering::Property);
            }
            let n = layer_order.len();
            for i in (1..n).rev() {
                let j: usize = u.int_in_range(0..=i)?;
                layer_order.swap(i, j);
            }
            layer_order
        };

        Ok(Self {
            name,
            extent,
            id,
            geometry,
            properties,
            #[cfg(fuzzing)]
            layer_order,
        })
    }
}

impl<'a> Arbitrary<'a> for StagedSharedDict {
    fn arbitrary(u: &mut Unstructured<'a>) -> Result<Self> {
        let items_raw: Vec<(String, Vec<Option<String>>)> = u.arbitrary()?;
        if items_raw.is_empty() {
            return Ok(Self {
                prefix: u.arbitrary()?,
                data: String::new(),
                items: Vec::new(),
            });
        }
        let prefix: String = u.arbitrary()?;
        Self::new(prefix, items_raw).map_err(|_| IncorrectFormat)
    }
}

impl Arbitrary<'_> for EncodedProperty {
    fn arbitrary(u: &mut Unstructured<'_>) -> Result<Self> {
        let decoded: StagedProperty = u.arbitrary()?;
        let encoder: ScalarEncoder = u.arbitrary()?;
        let prop: Option<Self> = Self::encode(&decoded, encoder).map_err(|_| IncorrectFormat)?;
        prop.ok_or(IncorrectFormat)
    }
}

impl Arbitrary<'_> for StagedProperty {
    fn arbitrary(u: &mut Unstructured<'_>) -> Result<Self> {
        let values: Vec<Option<u32>> = u.arbitrary()?;
        Ok(Self::u32("prop", values))
    }
}

impl Arbitrary<'_> for StagedStrings {
    fn arbitrary(u: &mut Unstructured<'_>) -> Result<Self> {
        Ok(Self::from_optional(
            u.arbitrary::<String>()?,
            u.arbitrary::<Vec<Option<String>>>()?,
        ))
    }
}

impl Arbitrary<'_> for EncodedId {
    fn arbitrary(u: &mut Unstructured<'_>) -> Result<Self> {
        let parsed: IdValues = u.arbitrary()?;
        let encoder: IdEncoder = u.arbitrary()?;
        let owned_id = Self::encode(&parsed, encoder).map_err(|_| IncorrectFormat)?;
        Ok(owned_id)
    }
}
