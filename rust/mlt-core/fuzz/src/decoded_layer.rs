use mlt_core::encoder::SortStrategy::Unsorted;
use mlt_core::encoder::{EncoderConfig, StagedLayer, stage_tile};

use crate::roundtrip::encode_decode;

/// Fuzz input that starts from a staged layer and tests encode -> decode roundtrip.
///
/// Generates valid [`StagedLayer`] values directly and verifies that the
/// canonical roundtrip (`Tile -> Staged -> bytes -> Tile`) is idempotent.
/// The config carries the wire version, so both v1 and v2 are exercised.
#[derive(arbitrary::Arbitrary)]
pub struct DecodedLayerInput {
    pub layer: StagedLayer,
    pub config: EncoderConfig,
}

impl DecodedLayerInput {
    pub fn fuzz_roundtrip(self) {
        let cfg = self.config;
        // Normalize: encode the fuzzed StagedLayer and decode to TileLayer.
        // This drops all-null columns, etc. - expected encoder behavior.
        let Some(tile1) = encode_decode(self.layer, cfg) else {
            return; // the wire version cannot represent this layer
        };
        let tile2 = restage_roundtrip(tile1, cfg);

        // Same roundtrip again - must be a fixpoint.
        let tile3 = restage_roundtrip(tile2.clone(), cfg);
        assert_eq!(tile2, tile3, "canonical roundtrip is not idempotent");
    }
}

/// Re-stage an already-decoded layer and run it through the same wire version again.
fn restage_roundtrip(tile: mlt_core::TileLayer, cfg: EncoderConfig) -> mlt_core::TileLayer {
    let staged = stage_tile(tile, Unsorted, cfg.allow_shared_dict(), cfg.tessellate());
    encode_decode(staged, cfg).expect("re-encoding a layer this version already produced")
}

impl std::fmt::Debug for DecodedLayerInput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "DecodedLayerInput {{\n\tconfig: {:#?}\n\tlayer: {:#?}\n}}",
            self.config, self.layer
        )
    }
}
