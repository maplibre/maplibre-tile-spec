use mlt_core::encoder::{Codecs, Encoder, EncoderConfig, StagedLayer, WireVersion};
use mlt_core::{Decoder, Layer, MltError, Parser, TileLayer};

/// Encode `staged` with `cfg`, then parse and decode it back to a row-oriented [`TileLayer`].
///
/// Returns `None` when the wire version cannot represent the layer, e.g. a v2 layer
/// tessellated without its full outline topology.
/// Every other encode failure is a bug and panics.
pub fn encode_decode(staged: StagedLayer, cfg: EncoderConfig) -> Option<TileLayer> {
    let bytes = encode(staged, cfg)?;
    Some(decode(&bytes, expected_tag(cfg)))
}

/// Encode `staged` to a complete layer record, or `None` if `cfg`'s wire version lacks the feature.
pub fn encode(staged: StagedLayer, cfg: EncoderConfig) -> Option<Vec<u8>> {
    let mut codecs = Codecs::default();
    match staged.encode_into(Encoder::new(cfg), &mut codecs) {
        Ok(enc) => Some(
            enc.into_layer_bytes()
                .expect("into_layer_bytes should not fail"),
        ),
        Err(MltError::NotImplemented(_)) => None,
        Err(e) => panic!("encode should not fail: {e}"),
    }
}

/// Parse and decode a single-layer record, asserting it was written with `tag`.
pub fn decode(bytes: &[u8], tag: u8) -> TileLayer {
    let mut layers = Parser::default()
        .parse_layers(bytes)
        .expect("layer must re-parse");
    assert_eq!(layers.len(), 1, "expected exactly one layer");
    let layer = layers.remove(0);
    assert_eq!(actual_tag(&layer), tag, "encoder wrote the wrong layer tag");
    layer
        .into_layer01()
        .expect("layer01 representation")
        .into_tile(&mut Decoder::default())
        .expect("into_tile should not fail")
}

pub fn expected_tag(cfg: EncoderConfig) -> u8 {
    match cfg.wire_version() {
        WireVersion::V01 => 1,
        WireVersion::V02 => 2,
    }
}

fn actual_tag(layer: &Layer<'_>) -> u8 {
    match layer {
        Layer::Tag01(_) => 1,
        Layer::Tag02(_) => 2,
        Layer::Unknown(u) => panic!("expected a Tag01/Tag02 layer, got tag {:#04x}", u.tag()),
        other => panic!("expected a Tag01/Tag02 layer, got {other:?}"),
    }
}
