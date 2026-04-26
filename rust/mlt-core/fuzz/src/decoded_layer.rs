use mlt_core::encoder::{Encoder, SortStrategy, StagedLayer, analyze_layer};
use mlt_core::{Decoder, Layer, Parser, TileLayer};

/// Fuzz input that starts from a staged layer and tests encode → decode roundtrip.
///
/// Generates valid [`StagedLayer`] values directly and verifies that the
/// canonical roundtrip (`Tile -> Staged -> bytes -> Tile`) is idempotent.
pub struct DecodedLayerInput {
    pub layer: StagedLayer,
}

impl arbitrary::Arbitrary<'_> for DecodedLayerInput {
    fn arbitrary(u: &mut arbitrary::Unstructured<'_>) -> arbitrary::Result<Self> {
        let layer: StagedLayer = u.arbitrary()?;
        Ok(Self { layer })
    }
}

impl DecodedLayerInput {
    pub fn fuzz_roundtrip(self) {
        // Normalize: encode the fuzzed StagedLayer and decode to TileLayer.
        // This drops all-null columns, etc. — expected encoder behavior.
        let tile1 = encode_decode(self.layer);

        // Canonical roundtrip per CONTRIBUTING.md:
        // Tile → Staged → bytes → Tile
        let analysis = analyze_layer(&tile1, false);
        let tile2 = encode_decode(StagedLayer::from_tile(
            tile1,
            SortStrategy::Unsorted,
            &analysis,
            false,
        ));

        // Same roundtrip again — must be a fixpoint.
        let analysis = analyze_layer(&tile2, false);
        let tile3 = encode_decode(StagedLayer::from_tile(
            tile2.clone(),
            SortStrategy::Unsorted,
            &analysis,
            false,
        ));

        assert_eq!(tile2, tile3, "canonical roundtrip is not idempotent");
    }
}

/// Encode a [`StagedLayer`] to bytes, then parse and decode back to a
/// row-oriented [`TileLayer`].
fn encode_decode(staged: StagedLayer) -> TileLayer {
    let buffer = staged
        .encode_into(Encoder::default())
        .expect("encode should not fail")
        .into_layer_bytes()
        .expect("into_layer_bytes should not fail");

    let mut layers = Parser::default()
        .parse_layers(&buffer)
        .expect("layer must re-parse");
    assert_eq!(layers.len(), 1, "expected exactly one layer");
    let Layer::Tag01(lazy) = layers.remove(0) else {
        panic!("expected Tag01 layer");
    };
    lazy.into_tile(&mut Decoder::default())
        .expect("into_tile should not fail")
}

impl std::fmt::Debug for DecodedLayerInput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "DecodedLayerInput {{\n\tlayer: {:#?}\n}}", self.layer)
    }
}
