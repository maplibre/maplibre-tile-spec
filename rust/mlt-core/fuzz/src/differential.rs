use std::path::PathBuf;
use std::process::Command;
use std::sync::OnceLock;

use hex::ToHex as _;
use mlt_core::encoder::{Codecs, Encoder, EncoderConfig, StagedLayer};
use mlt_core::geojson::FeatureCollection;
use mlt_core::{Decoder, Parser};

/// An arbitrary tile and encoder config, encoded by Rust and decoded by both
/// decoders. The Rust decoder and the C++ `mlt-cpp-json` tool must agree on the
/// [`FeatureCollection`] JSON.
#[derive(arbitrary::Arbitrary)]
pub struct DifferentialInput {
    pub layer: StagedLayer,
    pub config: EncoderConfig,
}

impl DifferentialInput {
    pub fn fuzz(self) {
        // Encode the arbitrary layer to MLT bytes with the fuzzed encoder
        // config. These bytes are the shared input fed to both decoders.
        let mut codecs = Codecs::default();
        let buffer = self
            .layer
            .encode_into(Encoder::new(self.config), &mut codecs)
            .expect("encode should not fail")
            .into_layer_bytes()
            .expect("into_layer_bytes should not fail");

        let rust_json = rust_decode(&buffer);

        // A C++ decode failure (unsupported technique, thrown exception) is not
        // a mismatch. Skip these inputs and only flag genuine disagreements
        // between the two decoders' output.
        let Some(cpp_json) = cpp_decode(&buffer) else {
            return;
        };

        let rust_value: serde_json::Value =
            serde_json::from_str(&rust_json).expect("rust JSON should parse");
        let cpp_value: serde_json::Value =
            serde_json::from_str(&cpp_json).expect("C++ JSON should parse");

        assert!(
            json_eq(&rust_value, &cpp_value),
            "Rust and C++ decoders disagree\n\
             rust: {rust_json}\n\
             cpp:  {cpp_json}\n\
             bytes: {}",
            buffer.encode_hex::<String>()
        );
    }
}

/// Decode MLT bytes with the Rust decoder to `FeatureCollection` JSON.
/// The format matches the output of `mlt-cpp-json`.
fn rust_decode(buffer: &[u8]) -> String {
    let layers = Parser::default()
        .parse_layers(buffer)
        .expect("layer must re-parse");
    let parsed = Decoder::default()
        .decode_all(layers)
        .expect("decode should not fail");
    let fc = FeatureCollection::from_layers(parsed).expect("FeatureCollection should build");
    serde_json::to_string(&fc).expect("FeatureCollection should serialize")
}

/// Decode MLT bytes with the C++ `mlt-cpp-json` tool.
/// Returns `None` when the tool exits non-zero, which covers decode errors and
/// thrown exceptions.
fn cpp_decode(buffer: &[u8]) -> Option<String> {
    let path = temp_tile_path();
    std::fs::write(path, buffer).expect("write temp tile");

    let output = Command::new(cpp_json_bin())
        .arg(path)
        .output()
        .expect("failed to run mlt-cpp-json");

    if !output.status.success() {
        return None;
    }
    Some(String::from_utf8(output.stdout).expect("C++ JSON should be valid UTF-8"))
}

/// Path to the `mlt-cpp-json` binary, from `$MLT_CPP_JSON_BIN`.
fn cpp_json_bin() -> &'static str {
    static BIN: OnceLock<String> = OnceLock::new();
    BIN.get_or_init(|| {
        std::env::var("MLT_CPP_JSON_BIN").unwrap_or_else(|_| {
            panic!(
                "set MLT_CPP_JSON_BIN to the path of the `mlt-cpp-json` binary \
                 (build it via the cpp CMake project)"
            )
        })
    })
}

/// A per-process temp file the C++ tool reads from.
/// `mlt-cpp-json` only accepts a file path.
/// Each input is written here and overwrites the previous one.
fn temp_tile_path() -> &'static PathBuf {
    static PATH: OnceLock<PathBuf> = OnceLock::new();
    PATH.get_or_init(|| std::env::temp_dir().join(format!("mlt-diff-{}.mlt", std::process::id())))
}

/// Compares two JSON values structurally.
/// Numbers are compared by value, so `0` and `0.0` count as equal.
/// This stops the two JSON libraries' integer-vs-float formatting from
/// reading as a difference.
///
/// Geometry coordinates are compared at `f32` precision because the C++ decoder
/// stores coordinates as 32-bit `float` by design. Comparing them at `f64`
/// would flag every coordinate above 2^24 as a difference and mask all other
/// divergences. Properties and extent are still compared exactly.
fn json_eq(a: &serde_json::Value, b: &serde_json::Value) -> bool {
    json_eq_inner(a, b, false)
}

/// `coord` is true inside a geometry's `coordinates`, enabling `f32` tolerance.
#[allow(
    clippy::cast_possible_truncation,
    reason = "intentional f64->f32 narrowing to match the C++ float coordinates"
)]
#[allow(
    clippy::float_cmp,
    reason = "exact equality is intended; the f32 cast and NaN checks are the explicit tolerances"
)]
fn json_eq_inner(a: &serde_json::Value, b: &serde_json::Value, coord: bool) -> bool {
    use serde_json::Value::{Array, Bool, Null, Number, Object, String};
    match (a, b) {
        (Null, Null) => true,
        (Bool(x), Bool(y)) => x == y,
        (String(x), String(y)) => x == y,
        (Number(x), Number(y)) => match (x.as_f64(), y.as_f64()) {
            (Some(x), Some(y)) => {
                x == y || (x.is_nan() && y.is_nan()) || (coord && x as f32 == y as f32)
            }
            _ => x == y,
        },
        (Array(x), Array(y)) => {
            x.len() == y.len() && x.iter().zip(y).all(|(x, y)| json_eq_inner(x, y, coord))
        }
        (Object(x), Object(y)) => {
            x.len() == y.len()
                && x.iter().all(|(k, xv)| {
                    y.get(k)
                        .is_some_and(|yv| json_eq_inner(xv, yv, coord || k == "coordinates"))
                })
        }
        _ => false,
    }
}

impl std::fmt::Debug for DifferentialInput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "DifferentialInput {{\n\tconfig: {:#?}\n\tlayer: {:#?}\n}}",
            self.config, self.layer
        )
    }
}
