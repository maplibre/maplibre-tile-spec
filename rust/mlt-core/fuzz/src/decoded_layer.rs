use mlt_core::encoder::SortStrategy::Unsorted;
use mlt_core::encoder::{EncoderConfig, StagedLayer, Tessellation, stage_tile};

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
        let tile2 = restage_roundtrip(&tile1, cfg);

        // Same roundtrip again - must be a fixpoint (up to triangle order/corner rotation on meshes).
        let tile3 = restage_roundtrip(&tile2, cfg);
        assert_same_decoded_layer(&tile2, &tile3);
    }
}

fn assert_same_decoded_layer(a: &mlt_core::TileLayer, b: &mlt_core::TileLayer) {
    if a.features().iter().all(|f| f.triangles().is_none()) {
        assert_eq!(a, b, "canonical roundtrip is not idempotent");
        return;
    }
    assert_eq!(a.name(), b.name());
    assert_eq!(a.extent(), b.extent());
    assert_eq!(a.z_axis(), b.z_axis());
    assert_eq!(a.property_names(), b.property_names());
    assert_eq!(a.m_value_names(), b.m_value_names());
    assert_eq!(a.m_value_scopes(), b.m_value_scopes());
    assert_eq!(a.nested_names(), b.nested_names());
    assert_eq!(a.features().len(), b.features().len());
    for (fa, fb) in a.features().iter().zip(b.features()) {
        assert_eq!(fa.id(), fb.id());
        assert_eq!(fa.geometry(), fb.geometry());
        assert_eq!(fa.z(), fb.z());
        assert_eq!(fa.properties(), fb.properties());
        assert_eq!(fa.nested(), fb.nested());
        for ((va, vb), scope) in fa
            .m_values()
            .iter()
            .zip(fb.m_values())
            .zip(a.m_value_scopes())
        {
            if *scope == mlt_core::MValueScope::Vertex {
                assert_eq!(va, vb);
            } else {
                assert_eq!(va.count(), vb.count());
            }
        }
        let canon = |tris: &[[u32; 3]]| -> Vec<[u32; 3]> {
            let mut out: Vec<[u32; 3]> = tris
                .iter()
                .map(|&[x, y, z]| [[x, y, z], [y, z, x], [z, x, y]].into_iter().min().unwrap())
                .collect();
            out.sort_unstable();
            out
        };
        assert_eq!(
            fa.triangles().map(canon),
            fb.triangles().map(canon),
            "mesh triangles changed across restage"
        );
    }
}

/// Re-stage an already-decoded layer and run it through the same wire version again.
fn restage_roundtrip(tile: &mlt_core::TileLayer, cfg: EncoderConfig) -> mlt_core::TileLayer {
    let staged = stage_tile(
        tile,
        Unsorted,
        cfg.allow_shared_dict(),
        cfg.tessellation() != Tessellation::Off,
    );
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
