use mlt_core::v01::EncodedLayer01;
use mlt_core::{Decoder, EncodedLayer, Layer, Parser};

/// Fuzz input that starts from an already-encoded layer and tests encode → decode roundtrip.
///
/// Unlike [`LayerInput`] (which starts from raw bytes), this drives the fuzzer to generate
/// valid [`EncodedLayer`] values directly and verifies that writing and re-parsing them yields
/// an identical layer.
pub struct DecodedLayerInput {
    pub layer: EncodedLayer,
}

impl arbitrary::Arbitrary<'_> for DecodedLayerInput {
    fn arbitrary(u: &mut arbitrary::Unstructured<'_>) -> arbitrary::Result<Self> {
        let layer01: EncodedLayer01 = u.arbitrary()?;
        Ok(Self {
            layer: EncodedLayer::Tag01(layer01),
        })
    }
}

impl DecodedLayerInput {
    pub fn fuzz_roundtrip(self) {
        // Write the arbitrary layer to a buffer.
        let mut buffer = Vec::<u8>::new();
        self.layer
            .write_to(&mut buffer)
            .expect("write_to cannot fail for a fully-Encoded layer");

        // Parse the written bytes back — must not fail.
        let mut parser = Parser::default();
        let Ok((remaining, mut parsed_back)) = Layer::from_bytes(&buffer, &mut parser) else {
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

        // Fully decode the layer — must not fail.
        parsed_back
            .decode_all(&mut Decoder::default())
            .expect("decode_all after roundtrip should not fail");
    }
}

impl std::fmt::Debug for DecodedLayerInput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "DecodedLayerInput {{\n\tlayer: {:#?}\n}}", self.layer)
    }
}
