use mlt_core::encoder::{Codecs, Encoder, StagedLayer};
use mlt_core::mvt::{mvt_to_tile_layers, tile_layers_to_mvt};
use mlt_core::test_helpers::assert_mvt_equivalent_layers;
use mlt_core::{Decoder, Layer, Parser, TileLayer};

/// Fuzz input exercising `TileLayer → MVT → TileLayer`.
///
/// MVT's wire types are narrower than MLT's (all narrow integer widths
/// collapse to `sint64`/`uint64`, etc.), so the first round-trip is
/// normalizing; subsequent round-trips must be fixpoints.
pub struct MvtRoundtripInput {
    pub layer: StagedLayer,
}

impl arbitrary::Arbitrary<'_> for MvtRoundtripInput {
    fn arbitrary(u: &mut arbitrary::Unstructured<'_>) -> arbitrary::Result<Self> {
        Ok(Self {
            layer: u.arbitrary()?,
        })
    }
}

impl MvtRoundtripInput {
    pub fn fuzz_roundtrip(self) {
        let canonical = mlt_encode_decode(self.layer);
        let normalized = mvt_roundtrip(canonical);
        let again = mvt_roundtrip(normalized.clone());
        assert_mvt_equivalent_layers(&normalized, &again);
    }
}

/// Encode a [`StagedLayer`] to MLT bytes, then parse + decode back into a
/// row-oriented [`TileLayer`]. Mirrors `decoded_layer::encode_decode`.
fn mlt_encode_decode(staged: StagedLayer) -> TileLayer {
    let mut codecs = Codecs::default();
    let buffer = staged
        .encode_into(Encoder::default(), &mut codecs)
        .expect("encode should not fail")
        .into_layer_bytes()
        .expect("into_layer_bytes should not fail");

    let mut layers = Parser::default()
        .parse_layers(&buffer)
        .expect("layer must re-parse");
    assert_eq!(layers.len(), 1, "expected exactly one MLT layer");
    let Layer::Tag01(lazy) = layers.remove(0) else {
        panic!("expected Tag01 layer");
    };
    lazy.into_tile(&mut Decoder::default())
        .expect("into_tile should not fail")
}

fn mvt_roundtrip(layer: TileLayer) -> TileLayer {
    let bytes = tile_layers_to_mvt(vec![layer]).expect("MVT encode should not fail");
    let mut layers = mvt_to_tile_layers(bytes).expect("MVT decode should not fail");
    assert_eq!(layers.len(), 1, "expected exactly one decoded MVT layer");
    layers.remove(0)
}

impl std::fmt::Debug for MvtRoundtripInput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "MvtRoundtripInput {{\n\tlayer: {:#?}\n}}", self.layer)
    }
}
