use mlt_core::encoder::{Encoder, SortStrategy, StagedLayer, StagedLayer01};
use mlt_core::{Decoder, Layer, Parser, TileLayer01};

/// Fuzz input that starts from a staged layer and tests encode → decode roundtrip.
///
/// Generates valid [`StagedLayer01`] values directly and verifies that the
/// canonical roundtrip (`Tile -> Staged -> bytes -> Tile`) is idempotent.
pub struct DecodedLayerInput {
    pub layer: StagedLayer01,
}

impl arbitrary::Arbitrary<'_> for DecodedLayerInput {
    fn arbitrary(u: &mut arbitrary::Unstructured<'_>) -> arbitrary::Result<Self> {
        let layer01: StagedLayer01 = u.arbitrary()?;
        Ok(Self { layer: layer01 })
    }
}

impl DecodedLayerInput {
    pub fn fuzz_roundtrip(self) {
        // Normalize: encode the fuzzed StagedLayer01 and decode to TileLayer01.
        // This drops all-null columns, etc. — expected encoder behavior.
        let tile1 = encode_decode(self.layer);

        // Canonical roundtrip per CONTRIBUTING.md:
        // Tile → Staged → bytes → Tile
        let tile2 =
            encode_decode(StagedLayer01::from_tile(tile1, SortStrategy::Unsorted, &[], false));

        // Same roundtrip again — must be a fixpoint.
        let tile3 = encode_decode(StagedLayer01::from_tile(
            tile2.clone(),
            SortStrategy::Unsorted,
            &[],
            false,
        ));

        assert_eq!(tile2, tile3, "canonical roundtrip is not idempotent");
    }
}

/// Encode a [`StagedLayer01`] to bytes, then parse and decode back to a
/// row-oriented [`TileLayer01`].
fn encode_decode(staged: StagedLayer01) -> TileLayer01 {
    let buffer = StagedLayer::Tag01(staged)
        .encode_into(Encoder::default())
        .expect("encode should not fail")
        .into_layer_bytes()
        .expect("into_layer_bytes should not fail");

    let mut parser = Parser::default();
    let (remaining, layer) = Layer::from_bytes(&buffer, &mut parser).expect("layer must re-parse");
    assert!(
        remaining.is_empty(),
        "Re-parsing left {} trailing bytes",
        remaining.len(),
    );

    let Layer::Tag01(lazy) = layer else {
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
