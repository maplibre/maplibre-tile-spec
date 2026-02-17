#![no_main]

use borrowme::ToOwned as _;
use hex::ToHex as _;
use libfuzzer_sys::fuzz_target;
use mlt_nom::Layer;

fuzz_target!(|input: Input| {
    input.fuzz();
});

#[derive(arbitrary::Arbitrary)]
struct Input {
    bytes: Vec<u8>,
}
impl Input {
    fn fuzz(self) {
        let total_len = self.bytes.len();

        // Try to parse the layer
        let Ok((remaining, layer)) = Layer::parse(&self.bytes) else {
            return;
        };
        if remaining.len() != 0 {
            return; // not interesting to debug
        }
        let consumed_input_bytes_size = total_len - remaining.len();
        let consumed_input = &self.bytes[..consumed_input_bytes_size];

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
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Template for reproducing fuzzer-found issues
    ///
    /// lets say you got
    /// ```raw
    /// Failing input:
    ///
    ///     artifacts/layer/crash-c172bc72586b2b327cecd0847691a14cf06e46a3
    ///
    /// Reproduce with:
    ///
    ///     cargo fuzz run layer artifacts/layer/crash-c172bc72586b2b327cecd0847691a14cf06e46a3
    ///
    /// Minimize test case with:
    ///
    ///     cargo fuzz tmin layer artifacts/layer/crash-c172bc72586b2b327cecd0847691a14cf06e46a3
    /// ```
    ///
    /// After running the minimized test case: `artifacts/layer/minimized-from-af930eb07155be59d6a983ccc5938df89c114fa7`
    ///
    /// When the fuzzer finds a crash, replace the filename below with the artifact path:
    /// 1. Update the filename: `artifacts/layer/crash-c172bc72586b2b327cecd0847691a14cf06e46a3`
    /// 2. Run: `cargo test --all-targets`
    #[test]
    fn test_reproduction() {
        let bytes = include_bytes!("../artifacts/layer/minimized-from-af930eb07155be59d6a983ccc5938df89c114fa7");
        let input = Input {
            bytes: bytes.to_vec(),
        };
        input.fuzz();
    }
}
