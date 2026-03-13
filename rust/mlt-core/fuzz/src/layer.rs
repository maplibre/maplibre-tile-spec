use hex::ToHex as _;
use mlt_core::{EncodedLayer, Layer};

#[derive(arbitrary::Arbitrary)]
pub struct LayerInput {
    pub bytes: Vec<u8>,
}
impl std::fmt::Debug for LayerInput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.bytes.encode_hex::<String>())
    }
}
impl LayerInput {
    pub fn fuzz_roundtrip(self) {
        let total_len = self.bytes.len();

        // Try to parse the layer
        let Ok((remaining, layer)) = Layer::parse(&self.bytes) else {
            return;
        };
        if layer.as_layer01().is_none() {
            return; // FIXME: not interesting to debug, but has roundtrip-ability issues
        }
        if !remaining.is_empty() {
            return; // not interesting to debug
        }
        let consumed_input_bytes_size = total_len - remaining.len();
        let consumed_input = &self.bytes[..consumed_input_bytes_size];

        let owned_layer = layer.to_owned().unwrap();

        // Encode to wire-ready form.
        let encoded_layer: EncodedLayer = match owned_layer.encode_auto() {
            Ok((el, _enc)) => el,
            Err(_) => return,
        };

        // Write the encoded layer to a buffer.
        let mut buffer = Vec::<u8>::with_capacity(consumed_input_bytes_size);
        let Ok(()) = encoded_layer.write_to(&mut buffer) else {
            return;
        };

        // Re-parse and fully decode the written bytes — must not panic.
        let Ok((remaining, mut parsed_back)) = Layer::parse(&buffer) else {
            panic!(
                "Written layer cannot be re-parsed\nOriginal input:\n{}",
                consumed_input.encode_hex::<String>()
            );
        };
        assert!(remaining.is_empty(), "Re-parsing left trailing bytes");
        let _ = parsed_back.decode_all();
    }

}
