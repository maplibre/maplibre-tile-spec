//! A [`MeshLayerWriter`] writes a `Meshes` layer that decodes back to its meshes.

use mlt_core::encoder::{EncoderConfig, Network, Profile, WireVersion};
use mlt_core::geojson::FeatureCollection;
use mlt_core::{
    Decoder, MValue, MValueScope, Mesh, MeshFault, MeshLayerWriter, MltError, Parser, PropKind,
    ZStep,
};
use rstest::rstest;
use serde_json::{Value, json};

fn v2() -> EncoderConfig {
    EncoderConfig::from_profile(Profile::Native(Network::Cellular4G))
        .with_wire_version(WireVersion::V02)
}

fn geojson(bytes: &[u8]) -> Value {
    let layers = Parser::default().parse_layers(bytes).unwrap();
    let decoded = Decoder::default().decode_all(layers).unwrap();
    serde_json::to_value(FeatureCollection::from_layers(decoded).unwrap()).unwrap()
}

#[test]
fn a_mesh_with_an_id_and_a_property_decodes_to_its_triangles() {
    let mut writer = MeshLayerWriter::new("landmarks", 4096, ZStep::new(-2).unwrap()).unwrap();
    let name = writer.add_property("name", PropKind::Str).unwrap();
    let mesh = Mesh::new(
        vec![[10, 10, 0], [10, 20, 0], [20, 10, 250]],
        vec![[0, 1, 2]],
    )
    .unwrap();
    let mut feature = writer.feature(&mesh);
    feature.id(Some(7));
    feature.property(name, "kiosk").unwrap();
    feature.finish().unwrap();

    let bytes = writer.encode(v2()).unwrap();

    assert_eq!(
        geojson(&bytes),
        json!({
            "type": "FeatureCollection",
            "features": [{
                "type": "Feature",
                "id": 7,
                "properties": {
                    "_extent": 4096,
                    "_layer": "landmarks",
                    "_z_step": -2,
                    "name": "kiosk"
                },
                "geometry": {
                    "type": "MultiPolygon",
                    "coordinates": [[[[10, 10, 0], [10, 20, 0], [20, 10, 250], [10, 10, 0]]]]
                }
            }]
        })
    );
}

#[test]
fn each_feature_keeps_its_triangle_values_in_the_order_the_encoder_picks() {
    let mut writer = MeshLayerWriter::new("landmarks", 4096, ZStep::new(0).unwrap()).unwrap();
    let part = writer
        .add_m_value("part", PropKind::U8, MValueScope::Triangle)
        .unwrap();
    let wall = Mesh::new(
        vec![[50, 30, 0], [70, 30, 0], [70, 30, 10], [50, 30, 10]],
        vec![[0, 1, 2], [0, 2, 3]],
    )
    .unwrap();
    let roof = Mesh::new(
        vec![
            [10, 10, 0],
            [30, 10, 0],
            [30, 30, 0],
            [10, 30, 0],
            [20, 20, 15],
        ],
        vec![[3, 2, 4], [2, 1, 4], [1, 0, 4], [0, 3, 4]],
    )
    .unwrap();
    let mut feature = writer.feature(&wall);
    feature.id(Some(2)).m_values(part, [5_u8, 6]).unwrap();
    feature.finish().unwrap();
    let mut feature = writer.feature(&roof);
    feature.id(Some(1)).m_values(part, [1_u8, 2, 3, 4]).unwrap();
    feature.finish().unwrap();
    let any_order = EncoderConfig::from_profile(Profile::Native(Network::Cellular4G))
        .with_wire_version(WireVersion::V02)
        .with_spatial_morton_sort(true)
        .with_spatial_hilbert_sort(true)
        .with_id_sort(true);

    let decoded = geojson(&writer.encode(any_order).unwrap());

    let mut parts: Vec<(&Value, &Value)> = decoded["features"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| (&f["id"], &f["properties"]["t:part"]))
        .collect();
    parts.sort_by_key(|(id, _)| id.as_u64());
    assert_eq!(
        parts,
        [
            (&json!(1), &json!([4, 1, 2, 3])),
            (&json!(2), &json!([5, 6]))
        ]
    );
}

#[test]
fn a_feature_rejected_at_finish_leaves_no_trace() {
    let mesh = Mesh::new(vec![[0, 0, 0], [0, 10, 0], [10, 0, 0]], vec![[0, 1, 2]]).unwrap();
    let mut with_rejected =
        MeshLayerWriter::new("landmarks", 4096, ZStep::new(0).unwrap()).unwrap();
    let part = with_rejected
        .add_m_value("part", PropKind::U8, MValueScope::Triangle)
        .unwrap();
    let mut feature = with_rejected.feature(&mesh);
    feature.id(Some(1)).m_values(part, [9_u8, 9]).unwrap();
    feature.finish().unwrap_err();
    let mut feature = with_rejected.feature(&mesh);
    feature.id(Some(2)).m_values(part, [3_u8]).unwrap();
    feature.finish().unwrap();
    let mut without = MeshLayerWriter::new("landmarks", 4096, ZStep::new(0).unwrap()).unwrap();
    let part = without
        .add_m_value("part", PropKind::U8, MValueScope::Triangle)
        .unwrap();
    let mut feature = without.feature(&mesh);
    feature.id(Some(2)).m_values(part, [3_u8]).unwrap();
    feature.finish().unwrap();

    assert_eq!(
        with_rejected.encode(v2()).unwrap(),
        without.encode(v2()).unwrap()
    );
}

#[test]
fn a_written_mesh_reads_back_as_the_same_mesh_with_its_values() {
    let mut writer = MeshLayerWriter::new("landmarks", 4096, ZStep::new(-1).unwrap()).unwrap();
    let shade = writer
        .add_m_value("shade", PropKind::U32, MValueScope::Vertex)
        .unwrap();
    let part = writer
        .add_m_value("part", PropKind::U8, MValueScope::Triangle)
        .unwrap();
    let wall = Mesh::new(
        vec![[50, 30, 0], [70, 30, 0], [70, 30, 10], [50, 30, 10]],
        vec![[0, 1, 2], [0, 2, 3]],
    )
    .unwrap();
    let mut feature = writer.feature(&wall);
    feature.id(Some(3));
    feature.m_values(shade, [1_u32, 2, 3, 4]).unwrap();
    feature.m_values(part, [8_u8, 9]).unwrap();
    feature.finish().unwrap();
    let bytes = writer.encode(v2()).unwrap();

    let layer = Parser::default()
        .parse_layers(&bytes)
        .unwrap()
        .into_iter()
        .next()
        .unwrap()
        .into_meshes(&mut Decoder::default())
        .unwrap()
        .unwrap();

    assert_eq!(layer.name(), "landmarks");
    assert_eq!(layer.z_axis(), ZStep::new(-1).unwrap().into());
    assert_eq!(
        layer.m_value_scopes(),
        [MValueScope::Vertex, MValueScope::Triangle]
    );
    let [feature] = layer.features() else {
        panic!("one feature")
    };
    assert_eq!(feature.id(), Some(3));
    assert_eq!(feature.mesh(), &wall);
    assert_eq!(
        feature.m_values(),
        [
            MValue::U32(Some(vec![1, 2, 3, 4])),
            MValue::U8(Some(vec![8, 9]))
        ]
    );
}

#[rstest]
#[case::empty(Mesh::new(vec![], vec![]).unwrap())]
#[case::closed_and_wound_outward(
    Mesh::new(
        vec![[0, 0, 0], [10, 0, 0], [0, 10, 0], [0, 0, 10]],
        vec![[0, 1, 2], [0, 3, 1], [0, 2, 3], [1, 3, 2]],
    )
    .unwrap()
)]
#[case::one_position_with_two_shades(
    Mesh::new(
        vec![[0, 0, 0], [10, 0, 0], [0, 10, 0], [0, 0, 0]],
        vec![[0, 1, 2], [3, 2, 1]],
    )
    .unwrap()
)]
fn a_mesh_the_encoder_accepts_reads_back_unchanged(#[case] mesh: Mesh) {
    let mut writer = MeshLayerWriter::new("landmarks", 4096, ZStep::new(0).unwrap()).unwrap();
    let shade = writer
        .add_m_value("shade", PropKind::U32, MValueScope::Vertex)
        .unwrap();
    let shades: Vec<u32> = (0..u32::try_from(mesh.vertices().len()).unwrap()).collect();
    let mut feature = writer.feature(&mesh);
    feature.m_values(shade, shades).unwrap();
    feature.finish().unwrap();
    let bytes = writer.encode(v2()).unwrap();

    let layer = Parser::default()
        .parse_layers(&bytes)
        .unwrap()
        .into_iter()
        .next()
        .unwrap()
        .into_meshes(&mut Decoder::default())
        .unwrap()
        .unwrap();

    let [feature] = layer.features() else {
        panic!("one feature")
    };
    assert_eq!(feature.mesh().vertices(), mesh.vertices());
    assert_eq!(feature.mesh().triangles().len(), mesh.triangles().len());
}

#[rstest]
#[case::repeated_corner(
    Mesh::new(vec![[0, 0, 0], [10, 0, 0], [0, 10, 0]], vec![[0, 1, 2], [0, 0, 1]]).unwrap(),
    MeshFault::RepeatedCorner { triangle: 1 }
)]
#[case::unused_vertex(
    Mesh::new(vec![[0, 0, 0], [10, 0, 0], [0, 10, 0], [5, 5, 5]], vec![[0, 1, 2]]).unwrap(),
    MeshFault::UnusedVertex { vertex: 3 }
)]
#[case::duplicate_vertex(
    Mesh::new(
        vec![[0, 0, 0], [10, 0, 0], [0, 10, 0], [0, 0, 0]],
        vec![[0, 1, 2], [3, 2, 1]],
    )
    .unwrap(),
    MeshFault::DuplicateVertex { first: 0, second: 3 }
)]
#[case::closed_and_wound_inward(
    Mesh::new(
        vec![[0, 0, 0], [10, 0, 0], [0, 10, 0], [0, 0, 10]],
        vec![[0, 2, 1], [0, 1, 3], [0, 3, 2], [1, 2, 3]],
    )
    .unwrap(),
    MeshFault::InsideOut
)]
fn a_mesh_the_spec_advises_against_is_rejected_at_finish(
    #[case] mesh: Mesh,
    #[case] expected: MeshFault,
) {
    let mut writer = MeshLayerWriter::new("landmarks", 4096, ZStep::new(0).unwrap()).unwrap();
    let ok = Mesh::new(vec![[0, 0, 0], [10, 0, 0], [0, 10, 0]], vec![[0, 1, 2]]).unwrap();
    writer.feature(&ok).finish().unwrap();

    let err = writer.feature(&mesh).finish().unwrap_err();

    assert!(
        matches!(err, MltError::InvalidMesh { feature: 1, fault } if fault == expected),
        "{err:?}"
    );
    assert_eq!(writer.feature_count(), 1);
}

#[test]
fn a_mesh_layer_cannot_be_written_as_v1() {
    let mut writer = MeshLayerWriter::new("landmarks", 4096, ZStep::new(0).unwrap()).unwrap();
    let mesh = Mesh::new(vec![[0, 0, 0], [0, 10, 0], [10, 0, 0]], vec![[0, 1, 2]]).unwrap();
    writer.feature(&mesh).finish().unwrap();
    let v1 = EncoderConfig::from_profile(Profile::Native(Network::Cellular4G))
        .with_wire_version(WireVersion::V01);

    let err = writer.encode(v1).unwrap_err();

    assert!(
        matches!(&err, MltError::MeshesNeedV2(name) if name == "landmarks"),
        "{err:?}"
    );
}

#[test]
fn a_triangle_scoped_run_of_the_vertex_count_is_rejected() {
    let mut writer = MeshLayerWriter::new("landmarks", 4096, ZStep::new(0).unwrap()).unwrap();
    let part = writer
        .add_m_value("part", PropKind::U8, MValueScope::Triangle)
        .unwrap();
    let mesh = Mesh::new(vec![[0, 0, 0], [0, 10, 0], [10, 0, 0]], vec![[0, 1, 2]]).unwrap();
    let mut feature = writer.feature(&mesh);
    feature.m_values(part, [1_u8, 2, 3]).unwrap();

    let err = feature.finish().unwrap_err();

    assert!(
        matches!(
            err,
            MltError::MValueTriangleCountMismatch {
                index: 0,
                expected: 1,
                actual: 3
            }
        ),
        "{err:?}"
    );
    assert_eq!(writer.feature_count(), 0);
}

#[test]
fn triangle_values_stay_with_their_triangles_and_vertex_values_with_their_vertices() {
    let mut writer = MeshLayerWriter::new("landmarks", 4096, ZStep::new(0).unwrap()).unwrap();
    let shade = writer
        .add_m_value("shade", PropKind::U32, MValueScope::Vertex)
        .unwrap();
    let part = writer
        .add_m_value("part", PropKind::Str, MValueScope::Triangle)
        .unwrap();
    let roof = Mesh::new(
        vec![
            [10, 10, 0],
            [30, 10, 0],
            [30, 30, 0],
            [10, 30, 0],
            [20, 20, 15],
        ],
        vec![[3, 2, 4], [2, 1, 4], [1, 0, 4], [0, 3, 4]],
    )
    .unwrap();
    let mut feature = writer.feature(&roof);
    feature.m_values(shade, [1_u32, 2, 3, 4, 5]).unwrap();
    feature
        .m_values(part, ["south", "east", "north", "west"])
        .unwrap();
    feature.finish().unwrap();

    let bytes = writer.encode(v2()).unwrap();

    assert_eq!(
        geojson(&bytes),
        json!({
            "type": "FeatureCollection",
            "features": [{
                "type": "Feature",
                "properties": {
                    "_extent": 4096,
                    "_layer": "landmarks",
                    "_z_step": 0,
                    "t:part": ["west", "south", "east", "north"],
                    "m:shade": [1, 2, 3, 4, 5]
                },
                "geometry": {
                    "type": "MultiPolygon",
                    "coordinates": [
                        [[[10, 10, 0], [10, 30, 0], [20, 20, 15], [10, 10, 0]]],
                        [[[20, 20, 15], [10, 30, 0], [30, 30, 0], [20, 20, 15]]],
                        [[[20, 20, 15], [30, 30, 0], [30, 10, 0], [20, 20, 15]]],
                        [[[10, 10, 0], [20, 20, 15], [30, 10, 0], [10, 10, 0]]]
                    ]
                }
            }]
        })
    );
}
