#![no_main]

use borrowme::ToOwned as _;
use hex::ToHex as _;
use libfuzzer_sys::fuzz_target;
use mlt_nom::Layer;

fuzz_target!(|input: Input| {
    let total_len = input.bytes.len();

    // Try to parse the layer
    let Ok((remaining, layer)) = Layer::parse(&input.bytes) else {
        return;
    };
    if remaining.len() != 0 {
        return; // not interesting to debug
    }
    let consumed_input_bytes_size = total_len - remaining.len();
    let consumed_input = &input.bytes[..consumed_input_bytes_size];

    let owned_layer = layer.to_owned();

    // Write the layer to a buffer
    let mut buffer = Vec::<u8>::with_capacity(consumed_input_bytes_size);
    owned_layer
        .write_to(&mut buffer)
        .expect("Failed to write layer which was parsed");
    let buffer_bytes_size = buffer.len();

    // Compare without printing to avoid printing lots of data
    if consumed_input != buffer.as_slice() {
        let consumed_input_hex = consumed_input.encode_hex::<String>();
        let buffer_hex = buffer.encode_hex::<String>();
        panic!(
            "Buffer [{buffer_hex}; {buffer_bytes_size}] does not match consumed input [{consumed_input_hex}; {consumed_input_bytes_size}]"
        );
    }
});

#[derive(arbitrary::Arbitrary)]
struct Input {
    bytes: Vec<u8>,
}

impl std::fmt::Debug for Input {
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
