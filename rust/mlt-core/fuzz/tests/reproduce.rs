use mlt_fuzz::LayerInput;

/// Template for reproducing fuzzer-found issues
///
/// When the fuzzer finds a crash, replace the filename below with the artifact path:
/// 1. Update the filename: `../artifacts/layer/crash-abc123`
/// 2. Run: `cargo test --manifest-path fuzz/Cargo.toml`
#[test]
fn fuzz_roundtrip() {
    let bytes = include_bytes!("../artifacts/layer/crash-abc123");
    let input = LayerInput {
        bytes: bytes.to_vec(),
    };
    input.fuzz_roundtrip();
}
