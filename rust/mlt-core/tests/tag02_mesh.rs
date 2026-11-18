//! Wire-level tests for v2 meshes, the `Meshes` geometry layout.

use mlt_core::dump::{DumpTree, annotate_tile};
use mlt_core::encoder::{
    Codecs, Encoder, EncoderConfig, ExplicitEncoder, FloatEncoding, IntEncoder, Network, Profile,
    StagedId, StagedInterior, StagedLayer, StagedLeaf, StagedList, StagedMValue, StagedNested,
    StagedNode, StagedProperty, StagedValues, StrEncoding, VertexBufferType, WireVersion,
};
use mlt_core::geo_types::{Geometry, LineString, Point};
use mlt_core::{
    Decoder, GeometryValues, MValue, MValueScope, Mesh, MeshFault, MltError, NestedKind,
    NestedValue, Parser, PropKind, PropValue, ZAxis, ZStep,
};
use rstest::rstest;

fn annotate(bytes: &[u8]) -> DumpTree {
    let (tree, err) = annotate_tile(bytes);
    assert!(err.is_none(), "annotate_tile: {err:?}");
    tree
}

fn region_after(bytes: &[u8], after_prefix: &str, label: &str) -> (usize, usize) {
    let tree = annotate(bytes);
    let start = tree
        .regions
        .iter()
        .position(|r| r.label.starts_with(after_prefix))
        .unwrap_or_else(|| panic!("no region labelled {after_prefix}"));
    let found = tree.regions[start..]
        .iter()
        .find(|r| r.label == label)
        .unwrap_or_else(|| panic!("no region labelled {label} after {after_prefix}"));
    (found.offset, found.len)
}

fn decode_err(bytes: &[u8]) -> MltError {
    let mut parser = Parser::default();
    match parser.parse_layers(bytes) {
        Err(e) => e,
        Ok(layers) => {
            let mut dec = Decoder::default();
            dec.decode_all(layers).expect_err("expected a rejection")
        }
    }
}

fn encode_meshes(meshes: &[Mesh], m_values: Vec<StagedMValue>) -> Vec<u8> {
    let mut geometry = GeometryValues::new_meshes(ZStep::new(0).unwrap());
    for mesh in meshes {
        geometry.push_mesh(mesh);
    }
    let staged =
        StagedLayer::with_m_values("meshes", 4096, StagedId::None, geometry, vec![], m_values)
            .expect("staged layer");
    let explicit = ExplicitEncoder {
        vertex_buffer_type: VertexBufferType::Vec2,
        force_stream: Box::new(|_| false),
        get_int_encoder: Box::new(|_| IntEncoder::varint()),
        get_str_encoding: Box::new(|_| StrEncoding::Plain),
        get_float_encoding: Box::new(|_| FloatEncoding::None),
    };
    let cfg = EncoderConfig::from_profile(Profile::Native(Network::Cellular4G))
        .with_wire_version(WireVersion::V02);
    staged
        .encode_into(
            Encoder::with_explicit(cfg, explicit),
            &mut Codecs::default(),
        )
        .expect("encode")
        .into_layer_bytes()
        .expect("layer bytes")
}

fn tetrahedron() -> Mesh {
    Mesh::new(
        vec![[0, 0, 0], [10, 0, 0], [0, 10, 0], [0, 0, 10]],
        vec![[0, 1, 2], [0, 3, 1], [1, 3, 2], [0, 2, 3]],
    )
    .unwrap()
}

fn vertex_shade() -> Vec<u8> {
    let shade = StagedMValue::new("shade", None, StagedValues::U32(vec![1, 2, 3, 4]));
    encode_meshes(&[tetrahedron()], vec![shade])
}

#[test]
fn a_vertex_scoped_column_writes_scope_byte_0() {
    let bytes = vertex_shade();
    let (scope, len) = region_after(&bytes, "m_value[", "scope");
    assert_eq!((bytes[scope], len), (0, 1));
}

#[rstest]
#[case::first_reserved(2)]
#[case::last_reserved(255)]
fn a_reserved_scope_byte_is_rejected(#[case] byte: u8) {
    let mut bytes = vertex_shade();
    let (scope, _) = region_after(&bytes, "m_value[", "scope");
    bytes[scope] = byte;

    let err = decode_err(&bytes);
    assert!(
        matches!(err, MltError::ParsingMValueScope(b) if b == byte),
        "{err:?}"
    );
}

#[rstest]
#[case::points(0x00)]
#[case::polygons(0x08)]
#[case::tess_polygons(0x0C)]
#[case::tess_polygons_with_outlines(0x0D)]
fn a_mesh_uniform_type_under_another_layout_is_rejected(#[case] layout: u8) {
    let mut bytes = encode_meshes(&[tetrahedron()], vec![]);
    let (layout_byte, _) = region_after(&bytes, "layer[", "layout");
    bytes[layout_byte] = layout;

    let err = decode_err(&bytes);
    assert!(
        matches!(err, MltError::MeshTypeOutsideMeshes(name) if name != "Meshes"),
        "{err:?}"
    );
}

#[rstest]
#[case::types_stream(0x0)]
#[case::uniform_point(0x1)]
#[case::uniform_polygon(0x3)]
#[case::uniform_multi_polygon(0x6)]
fn a_meshes_layout_without_the_mesh_uniform_type_is_rejected(#[case] nibble: u8) {
    let mut bytes = encode_meshes(&[tetrahedron()], vec![]);
    let (header, _) = region_after(&bytes, "layer[", "header");
    let patched = (bytes[header] & 0x8F) | (nibble << 4);
    bytes[header] = patched;

    let err = decode_err(&bytes);
    assert!(
        matches!(err, MltError::MeshesWithoutMeshType(b) if b == patched),
        "{err:?}"
    );
}

#[rstest]
#[case::v1(WireVersion::V01, "meta")]
#[case::v2(WireVersion::V02, "types")]
fn a_types_stream_naming_a_mesh_is_rejected(#[case] wire: WireVersion, #[case] types: &str) {
    let mut geometry = GeometryValues::default();
    geometry.push_geom(&Geometry::Point(Point::new(1, 2)));
    geometry.push_geom(&Geometry::LineString(LineString::from(vec![
        (0, 0),
        (5, 5),
    ])));
    let staged = StagedLayer::new("mixed", 4096, StagedId::None, geometry, vec![]).unwrap();
    let explicit = ExplicitEncoder {
        vertex_buffer_type: VertexBufferType::Vec2,
        force_stream: Box::new(|_| false),
        get_int_encoder: Box::new(|_| IntEncoder::varint()),
        get_str_encoding: Box::new(|_| StrEncoding::Plain),
        get_float_encoding: Box::new(|_| FloatEncoding::None),
    };
    let cfg =
        EncoderConfig::from_profile(Profile::Native(Network::Cellular4G)).with_wire_version(wire);
    let mut bytes = staged
        .encode_into(
            Encoder::with_explicit(cfg, explicit),
            &mut Codecs::default(),
        )
        .expect("encode")
        .into_layer_bytes()
        .expect("layer bytes");
    let (data, len) = region_after(&bytes, types, "data");
    assert_eq!(bytes[data..data + len], [0, 1]);
    bytes[data + 1] = 6;

    let err = decode_err(&bytes);
    assert!(
        matches!(&err, MltError::TryFromPrimitive(e) if e.number == 6),
        "{err:?}"
    );
}

#[test]
fn a_meshes_vertex_stream_of_xy_pairs_is_rejected() {
    let mut bytes = encode_meshes(&[tetrahedron()], vec![]);
    let (encoding, _) = region_after(&bytes, "vertices", "encoding");
    let (num_values, _) = region_after(&bytes, "vertices", "num_values");
    let (z_axis, _) = region_after(&bytes, "vertices", "z_axis");
    assert_eq!(bytes[num_values], 4);
    bytes[encoding] &= !1;
    bytes[num_values] = 6;
    bytes.remove(z_axis);
    bytes[0] -= 1;

    let err = decode_err(&bytes);
    assert!(matches!(err, MltError::MeshVerticesWithoutZ), "{err:?}");
}

#[rstest]
#[case::short([4, 3], 7)]
#[case::long([4, 5], 9)]
fn geo_lengths_not_summing_to_the_vertex_count_are_rejected(
    #[case] lengths: [u8; 2],
    #[case] sum: usize,
) {
    let mut bytes = encode_meshes(&[tetrahedron(), tetrahedron()], vec![]);
    let (data, len) = region_after(&bytes, "geo_lengths", "data");
    assert_eq!(bytes[data..data + len], [4, 4]);
    bytes[data..data + len].copy_from_slice(&lengths);

    let err = decode_err(&bytes);
    assert!(
        matches!(err, MltError::MeshVertexCountMismatch { expected, actual: 8 } if expected == sum),
        "{err:?}"
    );
}

#[test]
fn a_triangle_corner_in_another_meshes_vertex_run_is_rejected() {
    let mut bytes = encode_meshes(&[tetrahedron(), tetrahedron()], vec![]);
    let (data, len) = region_after(&bytes, "tri_cycles", "data");
    assert_eq!(
        bytes[data..data + len],
        [0, 1, 3, 2, 3, 1, 4, 5, 7, 6, 7, 5]
    );
    bytes[data + 6] = 2;

    let err = decode_err(&bytes);
    assert!(
        matches!(
            err,
            MltError::MeshCornerOutsideRun {
                feature: 1,
                corner: 2,
                start: 4,
                end: 8
            }
        ),
        "{err:?}"
    );
}

#[test]
fn geo_lengths_moving_a_corner_out_of_its_meshes_run_are_rejected() {
    let mut bytes = encode_meshes(&[tetrahedron(), tetrahedron()], vec![]);
    let (data, len) = region_after(&bytes, "geo_lengths", "data");
    assert_eq!(bytes[data..data + len], [4, 4]);
    bytes[data..data + len].copy_from_slice(&[3, 5]);

    let err = decode_err(&bytes);
    assert!(
        matches!(
            err,
            MltError::MeshCornerOutsideRun {
                feature: 0,
                corner: 3,
                start: 0,
                end: 3
            }
        ),
        "{err:?}"
    );
}

#[test]
fn triangle_lengths_counting_more_meshes_than_the_layer_holds_are_rejected() {
    let mut bytes = encode_meshes(&[tetrahedron()], vec![]);
    let (stream, len) = region_after(&bytes, "layer[", "tri_lengths");
    assert_eq!(bytes[stream..stream + len], [8, 1, 4]);
    bytes.splice(stream..stream + len, [0x88, 2, 2, 2, 2]);
    bytes[0] += 2;

    let err = decode_err(&bytes);
    assert!(
        matches!(
            err,
            MltError::MeshTriangleLengthsMismatch {
                expected: 1,
                actual: 2
            }
        ),
        "{err:?}"
    );
}

#[test]
fn geo_lengths_counting_more_meshes_than_the_layer_holds_are_rejected() {
    let mut bytes = encode_meshes(&[tetrahedron()], vec![]);
    let (stream, len) = region_after(&bytes, "layer[", "geo_lengths");
    assert_eq!(bytes[stream..stream + len], [8, 1, 4]);
    bytes.splice(stream..stream + len, [0x88, 2, 2, 4, 9]);
    bytes[0] += 2;

    let err = decode_err(&bytes);
    assert!(
        matches!(
            err,
            MltError::MeshGeoLengthsMismatch {
                expected: 1,
                actual: 2
            }
        ),
        "{err:?}"
    );
}

#[test]
fn a_meshes_layer_reads_back_as_meshes_with_values_kept_on_their_triangles() {
    let tetrahedron = Mesh::new(
        vec![[0, 0, 0], [10, 0, 0], [0, 10, 0], [0, 0, 10]],
        vec![[0, 1, 2], [0, 3, 1], [1, 3, 2], [0, 2, 3]],
    )
    .unwrap();
    let square = Mesh::new(
        vec![[0, 0, 5], [20, 0, 5], [20, 20, 5], [0, 20, 5]],
        vec![[0, 1, 2], [0, 2, 3]],
    )
    .unwrap();
    let mut geometry = GeometryValues::new_meshes(ZStep::new(0).unwrap());
    geometry.push_mesh(&tetrahedron);
    geometry.push_mesh(&square);
    let staged = StagedLayer::with_nested(
        "buildings",
        4096,
        StagedId::from_optional(vec![Some(7), Some(9)]),
        geometry,
        vec![StagedProperty::str("name", ["tower", "roof"])],
        vec![
            StagedMValue::new(
                "shade",
                None,
                StagedValues::U32(vec![1, 2, 3, 4, 5, 6, 7, 8]),
            ),
            StagedMValue::new(
                "material",
                Some(vec![true, false]),
                StagedValues::U8(vec![10, 11, 12, 13]),
            )
            .with_scope(MValueScope::Triangle),
        ],
        vec![StagedNested::new(
            "floors",
            StagedInterior::List(StagedList::new(
                None,
                vec![2, 1],
                StagedNode::Leaf(StagedLeaf::new(None, StagedValues::I32(vec![0, 1, 2]))),
            )),
        )],
    )
    .unwrap();
    let explicit = ExplicitEncoder {
        vertex_buffer_type: VertexBufferType::Vec2,
        force_stream: Box::new(|_| false),
        get_int_encoder: Box::new(|_| IntEncoder::varint()),
        get_str_encoding: Box::new(|_| StrEncoding::Plain),
        get_float_encoding: Box::new(|_| FloatEncoding::None),
    };
    let cfg = EncoderConfig::from_profile(Profile::Native(Network::Cellular4G))
        .with_wire_version(WireVersion::V02);
    let bytes = staged
        .encode_into(
            Encoder::with_explicit(cfg, explicit),
            &mut Codecs::default(),
        )
        .unwrap()
        .into_layer_bytes()
        .unwrap();

    let mut layers = Parser::default().parse_layers(&bytes).unwrap();
    let layer = layers
        .remove(0)
        .into_meshes(&mut Decoder::default())
        .unwrap()
        .unwrap();

    assert_eq!(
        (
            layer.name(),
            layer.extent().get(),
            layer.z_axis(),
            layer.property_names(),
            layer.property_kinds(),
            layer.m_value_names(),
            layer.m_value_kinds(),
            layer.m_value_scopes(),
            layer.nested_names(),
            layer.nested_kinds(),
        ),
        (
            "buildings",
            4096,
            ZAxis::from(ZStep::new(0).unwrap()),
            &["name".to_string()][..],
            &[PropKind::Str][..],
            &["shade".to_string(), "material".to_string()][..],
            &[PropKind::U32, PropKind::U8][..],
            &[MValueScope::Vertex, MValueScope::Triangle][..],
            &["floors".to_string()][..],
            &[NestedKind::list(NestedKind::Leaf(PropKind::I32))][..],
        )
    );
    let features: Vec<_> = layer
        .features()
        .iter()
        .map(|f| {
            (
                f.id(),
                f.properties().to_vec(),
                f.mesh().clone(),
                f.m_values().to_vec(),
                f.nested().to_vec(),
            )
        })
        .collect();
    assert_eq!(
        features,
        vec![
            (
                Some(7),
                vec![PropValue::Str(Some("tower".into()))],
                Mesh::new(
                    vec![[0, 0, 0], [10, 0, 0], [0, 10, 0], [0, 0, 10]],
                    vec![[0, 1, 2], [2, 1, 3], [0, 2, 3], [0, 3, 1]],
                )
                .unwrap(),
                vec![
                    MValue::U32(Some(vec![1, 2, 3, 4])),
                    MValue::U8(Some(vec![10, 12, 13, 11])),
                ],
                vec![NestedValue::list([
                    NestedValue::Leaf(PropValue::I32(Some(0))),
                    NestedValue::Leaf(PropValue::I32(Some(1))),
                ])],
            ),
            (
                Some(9),
                vec![PropValue::Str(Some("roof".into()))],
                Mesh::new(
                    vec![[0, 0, 5], [20, 0, 5], [20, 20, 5], [0, 20, 5]],
                    vec![[0, 1, 2], [0, 2, 3]],
                )
                .unwrap(),
                vec![MValue::U32(Some(vec![5, 6, 7, 8])), MValue::U8(None)],
                vec![NestedValue::list([NestedValue::Leaf(PropValue::I32(
                    Some(2)
                ))])],
            ),
        ]
    );
}

#[test]
fn into_tile_rejects_a_meshes_layer_naming_it() {
    let bytes = encode_meshes(&[tetrahedron()], vec![]);
    let mut layers = Parser::default().parse_layers(&bytes).unwrap();

    let err = layers
        .remove(0)
        .into_tile(&mut Decoder::default())
        .unwrap_err();
    assert!(
        matches!(&err, MltError::MeshLayerIntoTile(name) if name == "meshes"),
        "{err:?}"
    );
}

#[rstest]
#[case::v1(WireVersion::V01)]
#[case::v2(WireVersion::V02)]
fn into_meshes_rejects_a_layer_of_points(#[case] wire: WireVersion) {
    let mut geometry = GeometryValues::default();
    geometry.push_geom(&Geometry::Point(Point::new(1, 2)));
    let staged = StagedLayer::new("points", 4096, StagedId::None, geometry, vec![]).unwrap();
    let explicit = ExplicitEncoder {
        vertex_buffer_type: VertexBufferType::Vec2,
        force_stream: Box::new(|_| false),
        get_int_encoder: Box::new(|_| IntEncoder::varint()),
        get_str_encoding: Box::new(|_| StrEncoding::Plain),
        get_float_encoding: Box::new(|_| FloatEncoding::None),
    };
    let cfg =
        EncoderConfig::from_profile(Profile::Native(Network::Cellular4G)).with_wire_version(wire);
    let bytes = staged
        .encode_into(
            Encoder::with_explicit(cfg, explicit),
            &mut Codecs::default(),
        )
        .unwrap()
        .into_layer_bytes()
        .unwrap();
    let mut layers = Parser::default().parse_layers(&bytes).unwrap();

    let err = layers
        .remove(0)
        .into_meshes(&mut Decoder::default())
        .unwrap_err();
    assert!(
        matches!(&err, MltError::NotAMeshLayer(name) if name == "points"),
        "{err:?}"
    );
}

#[test]
fn a_staged_mesh_with_a_vertex_no_triangle_names_is_rejected() {
    let mut geometry = GeometryValues::new_meshes(ZStep::new(0).unwrap());
    geometry.push_mesh(&tetrahedron());
    geometry.push_mesh(
        &Mesh::new(
            vec![[0, 0, 0], [10, 0, 0], [0, 10, 0], [5, 5, 5]],
            vec![[0, 1, 2]],
        )
        .unwrap(),
    );

    let err = StagedLayer::new("meshes", 4096, StagedId::None, geometry, vec![]).unwrap_err();

    assert!(
        matches!(
            err,
            MltError::InvalidMesh {
                feature: 1,
                fault: MeshFault::UnusedVertex { vertex: 3 }
            }
        ),
        "{err:?}"
    );
}

#[test]
fn a_mesh_pushed_without_a_z_axis_is_rejected_at_encode() {
    let mut geometry = GeometryValues::default();
    geometry.push_mesh(&tetrahedron());
    let staged = StagedLayer::new("meshes", 4096, StagedId::None, geometry, vec![]).unwrap();
    let explicit = ExplicitEncoder {
        vertex_buffer_type: VertexBufferType::Vec2,
        force_stream: Box::new(|_| false),
        get_int_encoder: Box::new(|_| IntEncoder::varint()),
        get_str_encoding: Box::new(|_| StrEncoding::Plain),
        get_float_encoding: Box::new(|_| FloatEncoding::None),
    };
    let cfg = EncoderConfig::from_profile(Profile::Native(Network::Cellular4G))
        .with_wire_version(WireVersion::V02);

    let err = staged
        .encode_into(
            Encoder::with_explicit(cfg, explicit),
            &mut Codecs::default(),
        )
        .err()
        .expect("a rejection");

    assert!(
        matches!(&err, MltError::MeshesNeedZ(name) if name == "meshes"),
        "{err:?}"
    );
}
