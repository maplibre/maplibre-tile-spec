use mlt_core::encoder::{Codecs, Encoder, StagedLayer};
use mlt_core::mvt::{mvt_to_tile_layers, tile_layers_to_mvt};
use mlt_core::{Decoder, Layer, Parser, PropValue, TileLayer};

/// Fuzz input that exercises the `TileLayer → MVT → TileLayer` round-trip.
///
/// The MVT wire format is lossier than MLT (e.g. all narrow integer widths
/// collapse to `sint64`/`uint64`, `F32` widens to `F64` only when shared with
/// a `F64` column, etc.), so the *first* round-trip is the normalizing one.
/// After that, a second round-trip must be a fixpoint.
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
        // Bring the fuzzed StagedLayer to canonical TileLayer form (drops
        // all-null columns etc. — same prep as `decoded_layer.rs`).
        let canonical = mlt_encode_decode(self.layer);

        // First MVT round-trip normalizes the types.
        let normalized = mvt_roundtrip(canonical);

        // Second MVT round-trip must be a fixpoint.
        let again = mvt_roundtrip(normalized.clone());

        assert_layers_equivalent(&normalized, &again);
    }
}

/// Encode a [`StagedLayer`] to MLT bytes, then parse + decode back into a
/// row-oriented [`TileLayer`].
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

/// Encode one [`TileLayer`] as MVT bytes and decode the bytes back. The result
/// has been normalized through MVT's narrower type system.
fn mvt_roundtrip(layer: TileLayer) -> TileLayer {
    let bytes = tile_layers_to_mvt(vec![layer]).expect("MVT encode should not fail");
    let mut layers = mvt_to_tile_layers(bytes).expect("MVT decode should not fail");
    assert_eq!(layers.len(), 1, "expected exactly one decoded MVT layer");
    layers.remove(0)
}

/// Compare two layers after MVT round-trips, treating property column order as
/// unstable (mvt-reader returns feature properties from a `HashMap`).
fn assert_layers_equivalent(a: &TileLayer, b: &TileLayer) {
    assert_eq!(a.name, b.name, "layer name");
    assert_eq!(a.extent, b.extent, "layer extent");
    assert_eq!(a.features.len(), b.features.len(), "feature count");

    let names_a: std::collections::BTreeSet<&str> =
        a.property_names.iter().map(String::as_str).collect();
    let names_b: std::collections::BTreeSet<&str> =
        b.property_names.iter().map(String::as_str).collect();
    assert_eq!(names_a, names_b, "property name set");

    for (i, (af, bf)) in a.features.iter().zip(b.features.iter()).enumerate() {
        assert_eq!(af.id, bf.id, "feature id (index {i})");
        assert_eq!(af.geometry, bf.geometry, "feature geometry (index {i})");
        assert_eq!(
            properties_by_name(a, i),
            properties_by_name(b, i),
            "feature properties (index {i})"
        );
    }
}

fn properties_by_name(
    layer: &TileLayer,
    feat_idx: usize,
) -> std::collections::BTreeMap<&str, &PropValue> {
    layer
        .property_names
        .iter()
        .map(String::as_str)
        .zip(layer.features[feat_idx].properties.iter())
        .collect()
}

impl std::fmt::Debug for MvtRoundtripInput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "MvtRoundtripInput {{\n\tlayer: {:#?}\n}}", self.layer)
    }
}
