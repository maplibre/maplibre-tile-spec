use mlt_core::encoder::{Encoder, StagedLayer, StagedLayer01};
use mlt_core::{Decoder, Layer, Parser};

/// Fuzz input that starts from a staged layer and tests encode → decode roundtrip.
///
/// Generates valid [`StagedLayer01`] values directly and verifies that encoding
/// and re-parsing them yields a valid layer.
pub struct DecodedLayerInput {
    pub layer: StagedLayer01,
}

impl arbitrary::Arbitrary<'_> for DecodedLayerInput {
    fn arbitrary(u: &mut arbitrary::Unstructured<'_>) -> arbitrary::Result<Self> {
        let layer01: StagedLayer01 = u.arbitrary()?;
        Ok(Self { layer: layer01 })
    }
}

impl DecodedLayerInput {
    pub fn fuzz_roundtrip(self) {
        // Encode the staged layer to a buffer.
        let mut enc = Encoder::default();
        StagedLayer::Tag01(self.layer)
            .encode_into(&mut enc)
            .expect("encode_into cannot fail for a valid staged layer");
        let buffer = enc
            .into_layer_bytes()
            .expect("into_layer_bytes cannot fail");

        // Parse the written bytes back — must not fail.
        let mut parser = Parser::default();
        let Ok((remaining, parsed_back)) = Layer::from_bytes(&buffer, &mut parser) else {
            panic!("Written layer cannot be re-parsed");
        };
        assert!(
            remaining.is_empty(),
            "Re-parsing written layer left {} trailing bytes",
            remaining.len(),
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
