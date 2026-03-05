/// Fuzz input that starts from an already-decoded layer and tests encode → decode roundtrip.
///
/// Unlike [`LayerInput`] (which starts from raw bytes), this drives the fuzzer to generate
/// valid [`OwnedLayer`] values directly and verifies that writing and re-parsing them yields
/// an identical layer.
#[derive(arbitrary::Arbitrary)]
pub struct DecodedLayerInput {
    pub layer: OwnedLayer,
}

impl DecodedLayerInput {
    pub fn fuzz_roundtrip(self) {
        // Write the arbitrary layer to a buffer
        let mut buffer = Vec::<u8>::new();
        let Ok(()) = self.layer.write_to(&mut buffer) else {
            return; // FIXME: implement full layer writes
        };

        // Parse the written bytes back
        let Ok((remaining, parsed_back)) = Layer::parse(&buffer) else {
            panic!(
                "Written layer cannot be re-parsed\nOriginal layer:\n{:#?}",
                self.layer
            );
        };
        if !remaining.is_empty() {
            panic!(
                "Re-parsing written layer left {} trailing bytes\nOriginal layer:\n{:#?}",
                remaining.len(),
                self.layer
            );
        }

        let owned_parsed_back = parsed_back.to_owned();

        if self.layer != owned_parsed_back {
            LayerInput::try_panic_if_debug_is_different(&self.layer, &owned_parsed_back);
            LayerInput::minimize_inequal_but_debug_equal(&self.layer, &owned_parsed_back);
        }
    }
}

impl std::fmt::Debug for DecodedLayerInput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "DecodedLayerInput {{\n\tlayer: {:#?}\n}}", self.layer)
    }
}

impl std::fmt::Debug for LayerInput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Input {{\n\tbytes: [0x{}; {}]\n}}\n",
            self.bytes.encode_hex::<String>(),
            self.bytes.len()
        )?;
        write!(f, "As a layer: {:#?}", Layer::parse(&self.bytes))
    }
}
