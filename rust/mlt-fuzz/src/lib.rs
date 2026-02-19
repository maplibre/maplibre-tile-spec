use borrowme::ToOwned as _;
use hex::ToHex as _;
use mlt_core::Layer;

#[derive(arbitrary::Arbitrary)]
pub struct LayerInput {
    pub bytes: Vec<u8>,
}
impl LayerInput {
    pub fn fuzz_roundtrip(self) {
        let total_len = self.bytes.len();

        // Try to parse the layer
        let Ok((remaining, layer)) = Layer::parse(&self.bytes) else {
            return;
        };
        if layer.as_layer01().is_none() {
            return; // FIXME: not interesting to debug, but has roundtrippability issues
        }
        if !remaining.is_empty() {
            return; // not interesting to debug
        }
        let consumed_input_bytes_size = total_len - remaining.len();
        let consumed_input = &self.bytes[..consumed_input_bytes_size];

        let owned_layer = layer.to_owned();

        // Write the layer to a buffer
        let mut buffer = Vec::<u8>::with_capacity(consumed_input_bytes_size);
        let Ok(_) = owned_layer.write_to(&mut buffer) else {
            return; // FIXME: implement full layer writes
        };
        let buffer_bytes_size = buffer.len();

        // Compare without printing to avoid printing lots of data
        if consumed_input != buffer.as_slice() {
            let consumed_input_hex = consumed_input.encode_hex::<String>();
            let buffer_hex = buffer.encode_hex::<String>();
            let out = Layer::parse(&buffer);
            let msg = format!(
                "Buffer [{buffer_hex}; {buffer_bytes_size}] does not match consumed input [{consumed_input_hex}; {consumed_input_bytes_size}].",
            );
            // sometimes we can be more helpful and print the diff in terms of debug output
            if let Ok((_, out)) = out {
                let written_owned = out.to_owned();
                pretty_assertions::assert_eq!(
                    format!("{written_owned:#?}"),
                    format!("{owned_layer:#?}"),
                    "{msg}"
                );
            }
            panic!(
                "{msg}\nDecoded debugging output (written to disk prints to the same output, but has a different byte sequence written to disk!):\n{owned_layer:#?}"
            );
        }
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
