use hex::ToHex as _;
use mlt_core::{Decoder, Layer, Parser};

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
        let mut parser = Parser::default();
        let Ok((remaining, mut layer)) = Layer::from_bytes(&self.bytes, &mut parser) else {
            return;
        };
        if !remaining.is_empty() {
            return;
        }
        let _ = layer.decode_all(&mut Decoder::default());
    }
}
