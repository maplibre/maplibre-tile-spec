use mlt_core::v01::{EncodedProperty, StagedGeometry, StagedId, StagedLayer01, StagedProperty};
use mlt_core::{Layer, StagedLayer};

use crate::LayerInput;

/// Fuzz input that starts from an already-decoded layer and tests encode → decode roundtrip.
///
/// Unlike [`LayerInput`] (which starts from raw bytes), this drives the fuzzer to generate
/// valid [`StagedLayer`] values directly and verifies that writing and re-parsing them yields
/// an identical layer.
///
/// All geometry, ID, and property columns are always generated in their `Encoded` form if present
pub struct DecodedLayerInput {
    pub layer: StagedLayer,
}

impl arbitrary::Arbitrary<'_> for DecodedLayerInput {
    fn arbitrary(u: &mut arbitrary::Unstructured<'_>) -> arbitrary::Result<Self> {
        let name: String = u.arbitrary()?;
        let extent: u32 = u.arbitrary()?;

        let id: Option<StagedId> = if u.arbitrary()? {
            Some(StagedId::Encoded(u.arbitrary()?))
        } else {
            None
        };
        let geometry = StagedGeometry::Encoded(u.arbitrary()?);
        let properties: Vec<StagedProperty> = u
            .arbitrary::<Vec<EncodedProperty>>()?
            .into_iter()
            .map(StagedProperty::Encoded)
            .collect();

        // In fuzzing mode StagedLayer01 carries an explicit layer_order field that drives the
        // column serialisation order.  Build a valid ordering (1 Geometry, N Property, 0-or-1 Id)
        // and then Fisher-Yates shuffle it using the fuzzer's unstructured data.
        #[cfg(fuzzing)]
        let layer_order = {
            use mlt_core::v01::LayerOrdering;

            let mut order: Vec<LayerOrdering> = Vec::new();
            if id.is_some() {
                order.push(LayerOrdering::Id);
            }
            order.push(LayerOrdering::Geometry);
            for _ in &properties {
                order.push(LayerOrdering::Property);
            }

            let n = order.len();
            for i in (1..n).rev() {
                let j: usize = u.int_in_range(0..=i)?;
                order.swap(i, j);
            }
            order
        };

        let layer01 = StagedLayer01 {
            name,
            extent,
            id,
            geometry,
            properties,
            #[cfg(fuzzing)]
            layer_order,
        };

        Ok(Self {
            layer: StagedLayer::Tag01(layer01),
        })
    }
}

impl DecodedLayerInput {
    pub fn fuzz_roundtrip(self) {
        // Write the arbitrary layer to a buffer.
        // Every column is in its Encoded form, so write_to must not fail — treat any
        // failure as a bug in the serialiser rather than a known gap.
        let mut buffer = Vec::<u8>::new();
        self.layer
            .write_to(&mut buffer)
            .expect("write_to cannot fail for a fully-Encoded layer");

        // Parse the written bytes back
        let Ok((remaining, parsed_back)) = Layer::parse(&buffer) else {
            panic!(
                "Written layer cannot be re-parsed\nOriginal layer:\n{:#?}",
                self.layer
            );
        };
        assert!(
            remaining.is_empty(),
            "Re-parsing written layer left {} trailing bytes\nOriginal layer:\n{:#?}",
            remaining.len(),
            self.layer
        );

        let owned_parsed_back = parsed_back.to_owned();

        if self.layer != owned_parsed_back {
            LayerInput::try_panic_if_debug_is_different(&self.layer, &owned_parsed_back);
            LayerInput::minimize_unequal_but_debug_equal(&self.layer, &owned_parsed_back);
        }
    }
}

impl std::fmt::Debug for DecodedLayerInput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "DecodedLayerInput {{\n\tlayer: {:#?}\n}}", self.layer)
    }
}
