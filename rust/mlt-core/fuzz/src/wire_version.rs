use mlt_core::encoder::{EncoderConfig, StagedLayer, WireVersion};

use crate::roundtrip::encode_decode;

/// An arbitrary tile encoded to both wire versions, whose decoded layers must match.
///
/// v1 is the reference: whatever it decodes to is what v2 has to reproduce, so
/// this catches v2 envelope, presence, and stream-header bugs that a v2-only
/// roundtrip would round-trip consistently wrong.
pub struct WireVersionInput {
    pub layer: StagedLayer,
    pub config: EncoderConfig,
}

impl arbitrary::Arbitrary<'_> for WireVersionInput {
    fn arbitrary(u: &mut arbitrary::Unstructured<'_>) -> arbitrary::Result<Self> {
        Ok(Self {
            layer: u.arbitrary()?,
            // The version is what this target varies, so it is not left to chance.
            config: EncoderConfig::arbitrary(u)?.with_wire_version(WireVersion::V01),
        })
    }
}

impl WireVersionInput {
    pub fn fuzz(self) {
        let cfg_v1 = self.config;
        let cfg_v2 = cfg_v1.with_wire_version(WireVersion::V02);

        let v1 =
            encode_decode(self.layer.clone(), cfg_v1).expect("v1 represents every staged layer");
        let Some(v2) = encode_decode(self.layer, cfg_v2) else {
            return; // v2 cannot represent this layer yet
        };
        assert_eq!(v1, v2, "v1 and v2 decoded layers must be identical");
    }
}

impl std::fmt::Debug for WireVersionInput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "WireVersionInput {{\n\tconfig: {:#?}\n\tlayer: {:#?}\n}}",
            self.config, self.layer
        )
    }
}
