use hex::ToHex as _;
use mlt_core::{Decoder, Parser};

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
        let Ok(layers) = Parser::default().parse_layers(&self.bytes) else {
            return;
        };
        for layer in layers {
            let _ = layer.decode_all(&mut Decoder::default());
        }
    }
}
