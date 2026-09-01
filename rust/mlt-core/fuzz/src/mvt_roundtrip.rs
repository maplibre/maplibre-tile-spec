use mlt_core::TileLayer;
use mlt_core::encoder::{EncoderConfig, StagedLayer};
use mlt_core::mvt::{mvt_to_tile_layers, tile_layers_to_mvt};
use mlt_core::test_helpers::assert_mvt_equivalent_layers;

use crate::roundtrip::encode_decode;

/// Fuzz input exercising `TileLayer -> MVT -> TileLayer`.
///
/// MVT's wire types are narrower than MLT's (all narrow integer widths
/// collapse to `sint64`/`uint64`, etc.), so the first round-trip is
/// normalizing; subsequent round-trips must be fixpoints.
#[derive(arbitrary::Arbitrary)]
pub struct MvtRoundtripInput {
    pub layer: StagedLayer,
    pub config: EncoderConfig,
}

impl MvtRoundtripInput {
    pub fn fuzz_roundtrip(self) {
        let Some(canonical) = encode_decode(self.layer, self.config) else {
            return; // the wire version cannot represent this layer
        };
        let normalized = mvt_roundtrip(canonical);
        let again = mvt_roundtrip(normalized.clone());
        assert_mvt_equivalent_layers(&normalized, &again);
    }
}

fn mvt_roundtrip(layer: TileLayer) -> TileLayer {
    let bytes = tile_layers_to_mvt(vec![layer]).expect("MVT encode should not fail");
    let mut layers = mvt_to_tile_layers(bytes).expect("MVT decode should not fail");
    assert_eq!(layers.len(), 1, "expected exactly one decoded MVT layer");
    layers.remove(0)
}

impl std::fmt::Debug for MvtRoundtripInput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "MvtRoundtripInput {{\n\tconfig: {:#?}\n\tlayer: {:#?}\n}}",
            self.config, self.layer
        )
    }
}
