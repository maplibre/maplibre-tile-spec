use mlt_core::encoder::SortStrategy::Unsorted;
use mlt_core::encoder::{EncoderConfig, StagedLayer, Tessellation, stage_tile};
use mlt_core::{MValue, MValueScope, TileLayer};

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

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum TriMValue<'a> {
    Null,
    Bool(bool),
    I8(i8),
    U8(u8),
    I32(i32),
    U32(u32),
    I64(i64),
    U64(u64),
    F32(u32),
    F64(u64),
    Str(&'a str),
}

fn tri_m_value_at(value: &MValue, tri_idx: usize) -> TriMValue<'_> {
    let f32_bits = |v: f32| {
        if v.to_bits() == (-0.0_f32).to_bits() {
            0
        } else {
            v.to_bits()
        }
    };
    let f64_bits = |v: f64| {
        if v.to_bits() == (-0.0_f64).to_bits() {
            0
        } else {
            v.to_bits()
        }
    };
    match value {
        MValue::Bool(v) => v
            .as_ref()
            .map_or(TriMValue::Null, |xs| TriMValue::Bool(xs[tri_idx])),
        MValue::I8(v) => v
            .as_ref()
            .map_or(TriMValue::Null, |xs| TriMValue::I8(xs[tri_idx])),
        MValue::U8(v) => v
            .as_ref()
            .map_or(TriMValue::Null, |xs| TriMValue::U8(xs[tri_idx])),
        MValue::I32(v) => v
            .as_ref()
            .map_or(TriMValue::Null, |xs| TriMValue::I32(xs[tri_idx])),
        MValue::U32(v) => v
            .as_ref()
            .map_or(TriMValue::Null, |xs| TriMValue::U32(xs[tri_idx])),
        MValue::I64(v) => v
            .as_ref()
            .map_or(TriMValue::Null, |xs| TriMValue::I64(xs[tri_idx])),
        MValue::U64(v) => v
            .as_ref()
            .map_or(TriMValue::Null, |xs| TriMValue::U64(xs[tri_idx])),
        MValue::F32(v) => v
            .as_ref()
            .map_or(TriMValue::Null, |xs| TriMValue::F32(f32_bits(xs[tri_idx]))),
        MValue::F64(v) => v
            .as_ref()
            .map_or(TriMValue::Null, |xs| TriMValue::F64(f64_bits(xs[tri_idx]))),
        MValue::Str(v) => v
            .as_ref()
            .map_or(TriMValue::Null, |xs| TriMValue::Str(xs[tri_idx].as_str())),
    }
}

fn canonical_triangles<'a>(
    tris: &[[u32; 3]],
    m_values: &'a [MValue],
    scopes: &[MValueScope],
) -> Vec<([u32; 3], Vec<TriMValue<'a>>)> {
    for (mv, scope) in m_values.iter().zip(scopes) {
        if *scope == MValueScope::Triangle
            && let Some(count) = mv.count()
        {
            assert_eq!(
                count,
                tris.len(),
                "triangle m-value count must match triangles"
            );
        }
    }
    let mut out: Vec<_> = tris
        .iter()
        .enumerate()
        .map(|(tri_idx, &[x, y, z])| {
            let corners = [[x, y, z], [y, z, x], [z, x, y]]
                .into_iter()
                .min()
                .expect("three rotations is non-empty");
            let values = m_values
                .iter()
                .zip(scopes)
                .filter(|&(_, scope)| *scope == MValueScope::Triangle)
                .map(|(mv, _)| tri_m_value_at(mv, tri_idx))
                .collect();
            (corners, values)
        })
        .collect();
    out.sort_unstable();
    out
}

fn assert_same_decoded_layer(a: &TileLayer, b: &TileLayer) {
    if a.features().iter().all(|f| f.triangles().is_none())
        && b.features().iter().all(|f| f.triangles().is_none())
    {
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
        assert_eq!(fa.m_values().len(), a.m_value_scopes().len());
        assert_eq!(fb.m_values().len(), b.m_value_scopes().len());
        for ((va, vb), scope) in fa
            .m_values()
            .iter()
            .zip(fb.m_values())
            .zip(a.m_value_scopes())
        {
            if *scope == MValueScope::Vertex {
                assert_eq!(va, vb);
            } else {
                assert_eq!(va.kind(), vb.kind());
                assert_eq!(va.count(), vb.count());
            }
        }
        assert_eq!(
            fa.triangles()
                .map(|tris| canonical_triangles(tris, fa.m_values(), a.m_value_scopes())),
            fb.triangles()
                .map(|tris| canonical_triangles(tris, fb.m_values(), b.m_value_scopes())),
            "mesh triangles or triangle-scoped m-values changed across restage"
        );
    }
}

/// Re-stage an already-decoded layer and run it through the same wire version again.
fn restage_roundtrip(tile: &TileLayer, cfg: EncoderConfig) -> TileLayer {
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

#[cfg(test)]
mod tests {
    use mlt_core::PropKind;
    use mlt_core::encoder::{Network, Profile, WireVersion};
    use mlt_core::geo_types::{Coord, Geometry, MultiPoint, Point};

    use super::*;

    fn mesh_layer(tris: Vec<[u32; 3]>, materials: Vec<u32>, labels: Vec<&str>) -> TileLayer {
        let mut builder = TileLayer::builder("mesh", 4096).expect("valid layer");
        builder
            .set_meshes(mlt_core::ZAxis {
                step: mlt_core::ZStep::new(-2).expect("centimeter step"),
                mode: mlt_core::HeightMode::Absolute,
            })
            .expect("valid z axis");
        let v_col = builder
            .add_scoped_m_value("uv", PropKind::I32, MValueScope::Vertex)
            .expect("vertex m-value");
        let mat_col = builder
            .add_scoped_m_value("material", PropKind::U32, MValueScope::Triangle)
            .expect("triangle m-value");
        let tag_col = builder
            .add_scoped_m_value("surface", PropKind::Str, MValueScope::Triangle)
            .expect("triangle str m-value");
        let pts = MultiPoint(vec![
            Point(Coord { x: 0, y: 0 }),
            Point(Coord { x: 10, y: 0 }),
            Point(Coord { x: 10, y: 10 }),
            Point(Coord { x: 0, y: 10 }),
        ]);
        let mut row = builder.feature(Geometry::MultiPoint(pts));
        row.z(vec![0, 0, 5, 5]).expect("four z values");
        row.triangles(tris).expect("valid triangles");
        row.m_value(v_col, MValue::I32(Some(vec![1, 2, 3, 4])))
            .expect("four vertex m-values");
        row.m_value(mat_col, MValue::U32(Some(materials)))
            .expect("triangle u32 m-values");
        row.m_value(
            tag_col,
            MValue::Str(Some(labels.into_iter().map(Into::into).collect())),
        )
        .expect("triangle str m-values");
        row.finish().expect("valid mesh feature");
        builder.finish()
    }

    #[test]
    fn accepts_reordered_and_rotated_triangles_with_matching_values() {
        let a = mesh_layer(
            vec![[0, 1, 2], [0, 2, 3]],
            vec![10, 20],
            vec!["wall", "roof"],
        );
        let b = mesh_layer(
            vec![[2, 3, 0], [1, 2, 0]],
            vec![20, 10],
            vec!["roof", "wall"],
        );
        assert_same_decoded_layer(&a, &b);

        let cfg = EncoderConfig::from_profile(Profile::Native(Network::default()))
            .with_wire_version(WireVersion::V02);
        let t2 = restage_roundtrip(&a, cfg);
        let t3 = restage_roundtrip(&t2, cfg);
        assert_same_decoded_layer(&t2, &t3);
    }

    #[test]
    #[should_panic(expected = "mesh triangles or triangle-scoped m-values changed across restage")]
    fn rejects_values_attached_to_wrong_reordered_triangles() {
        let a = mesh_layer(
            vec![[0, 1, 2], [0, 2, 3]],
            vec![10, 20],
            vec!["wall", "roof"],
        );
        let b = mesh_layer(
            vec![[2, 3, 0], [1, 2, 0]],
            vec![10, 20],
            vec!["wall", "roof"],
        );
        assert_same_decoded_layer(&a, &b);
    }

    #[test]
    #[should_panic(expected = "mesh triangles or triangle-scoped m-values changed across restage")]
    fn rejects_replaced_triangle_values_with_same_count() {
        let a = mesh_layer(
            vec![[0, 1, 2], [0, 2, 3]],
            vec![10, 20],
            vec!["wall", "roof"],
        );
        let b = mesh_layer(
            vec![[0, 1, 2], [0, 2, 3]],
            vec![99, 88],
            vec!["wall", "roof"],
        );
        assert_same_decoded_layer(&a, &b);
    }
}
