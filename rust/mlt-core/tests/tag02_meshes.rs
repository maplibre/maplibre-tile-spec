//! Round-trip and wire-level tests for v2 meshes: the `Meshes` layout (`0xE`), the
//! `Mesh` geometry type, triangle-scoped m-values and z as a height above the ground.

use insta::assert_snapshot;
use mlt_core::dump::{DumpTree, RenderOpts, annotate_tile, render};
use mlt_core::encoder::{EncoderConfig, Network, Profile, Tessellation, WireVersion};
use mlt_core::geo_types::{Geometry, MultiPoint, Point};
use mlt_core::geojson::FeatureCollection;
use mlt_core::{
    Decoder, HeightMode, Layer, Layer02, MValue, MValueScope, Parser, PropKind, PropValue,
    TileLayer, ZAxis, ZStep,
};
use proptest::prelude::*;
use rstest::rstest;

fn annotate(bytes: &[u8]) -> DumpTree {
    let (tree, err) = annotate_tile(bytes);
    assert!(err.is_none(), "annotate_tile: {err:?}");
    tree
}

fn cfg_v2() -> EncoderConfig {
    EncoderConfig::from_profile(Profile::Native(Network::Cellular4G))
        .with_wire_version(WireVersion::V02)
}

fn try_decode(bytes: &[u8]) -> mlt_core::MltResult<TileLayer> {
    only_v2_layer(Parser::default().parse_layers(bytes)?).into_tile(&mut Decoder::default())
}

fn only_v2_layer(mut layers: Vec<Layer<'_>>) -> Layer02<'_> {
    assert_eq!(layers.len(), 1);
    let Some(Layer::Tag02(layer)) = layers.pop() else {
        panic!("expected a v2 layer")
    };
    layer
}

/// Where the dump puts the byte it labels `label`.
fn byte_offset(bytes: &[u8], label: &str) -> usize {
    annotate(bytes)
        .regions
        .iter()
        .find(|r| r.label == label)
        .map(|r| r.offset)
        .expect("a labelled byte")
}

/// The dump's leaf regions cover `bytes` without gaps, and the layout byte names `Meshes`.
fn assert_dump_of_meshes(bytes: &[u8]) {
    let tree = annotate(bytes);
    let mut leaves: Vec<(usize, usize)> = tree
        .regions
        .iter()
        .filter(|r| !r.container)
        .map(|r| (r.offset, r.len))
        .collect();
    leaves.sort_unstable();
    let mut cursor = 0;
    for (offset, len) in &leaves {
        assert_eq!(*offset, cursor, "gap/overlap at offset {offset}");
        cursor += len;
    }
    assert_eq!(cursor, bytes.len());
    render(&tree, bytes, &RenderOpts::default(), &mut Vec::new()).expect("render");
    let layout = tree.regions.iter().find(|r| r.label == "layout").unwrap();
    assert_eq!(
        layout.bits.last().unwrap().meaning().to_string(),
        "geometry layout = Meshes"
    );
}

/// A triangle rotated to start at its smallest corner, which keeps its winding even when two
/// corners are equal.
fn canonical<T: Ord + Clone>([a, b, c]: [T; 3]) -> [T; 3] {
    [
        [a.clone(), b.clone(), c.clone()],
        [b.clone(), c.clone(), a.clone()],
        [c, a, b],
    ]
    .into_iter()
    .min()
    .expect("three rotations")
}

type MeshCorner = ([i32; 3], Vec<String>);

/// Everything a feature holds, canonicalized over vertex and triangle permutations:
/// the sorted `(xyz, vertex_m_values)` multiset and each triangle rotated to start at its
/// smallest corner beside its triangle-scoped `i32` value, sorted.
type FeatureRow = (
    Option<u64>,
    Vec<PropValue>,
    Vec<MeshCorner>,
    Option<Vec<([MeshCorner; 3], i32)>>,
);

/// The layer's features by id, each as a [`FeatureRow`], so the encoder may reorder features,
/// mesh vertices, and mesh triangles.
fn rows(layer: &TileLayer) -> Vec<FeatureRow> {
    let mut rows: Vec<FeatureRow> = layer
        .features()
        .iter()
        .map(|f| {
            let (mut per_vertex, mut faces) = (Vec::new(), Vec::new());
            for (value, scope) in f.m_values().iter().zip(layer.m_value_scopes()) {
                match (scope, value) {
                    (MValueScope::Triangle, MValue::I32(v)) => {
                        faces = v.clone().unwrap_or_default();
                    }
                    (MValueScope::Triangle, _) => panic!("triangle values are i32: {value:?}"),
                    (MValueScope::Vertex, MValue::Str(v)) => {
                        per_vertex.push(v.as_ref().map(|items| {
                            items.iter().map(ToString::to_string).collect::<Vec<_>>()
                        }));
                    }
                    (MValueScope::Vertex, other) => panic!("vertex values are str: {other:?}"),
                }
            }
            let Geometry::MultiPoint(mp) = f.geometry() else {
                panic!("expected MultiPoint mesh: {:?}", f.geometry());
            };
            let pts = &mp.0[..];
            let z = f.z();
            let corners: Vec<MeshCorner> = (0..pts.len())
                .map(|i| {
                    let xyz = [pts[i].x(), pts[i].y(), z.get(i).copied().unwrap_or(0)];
                    let v_vals = per_vertex
                        .iter()
                        .map(|col| {
                            col.as_ref()
                                .and_then(|items| items.get(i).cloned())
                                .unwrap_or_default()
                        })
                        .collect();
                    (xyz, v_vals)
                })
                .collect();
            let triangles = f.triangles().map(|triangles| {
                let mut paired: Vec<_> = triangles
                    .iter()
                    .enumerate()
                    .map(|(i, &[a, b, c])| {
                        let tri = canonical([
                            corners[a as usize].clone(),
                            corners[b as usize].clone(),
                            corners[c as usize].clone(),
                        ]);
                        (tri, faces.get(i).copied().unwrap_or_default())
                    })
                    .collect();
                paired.sort();
                paired
            });
            let mut sorted_corners = corners;
            sorted_corners.sort();
            (f.id(), f.properties().to_vec(), sorted_corners, triangles)
        })
        .collect();
    rows.sort_by_key(|row| row.0);
    rows
}

fn height() -> ZAxis {
    ZAxis {
        step: ZStep::new(-2).unwrap(),
        mode: HeightMode::RelativeToGround,
    }
}

/// `(x, y, z)` vertices and the triangles between them.
type Mesh = (Vec<[i32; 3]>, Vec<[u32; 3]>);

/// A layer of `meshes`, each with a `name`, a vertex-scoped `part` and a triangle-scoped `face`.
fn mesh_layer(meshes: &[Mesh]) -> TileLayer {
    let mut builder = TileLayer::builder("landmarks", 16384).unwrap();
    builder.set_z_axis(height()).unwrap();
    let name = builder.add_property("name", PropKind::Str).unwrap();
    let part = builder.add_m_value("part", PropKind::Str).unwrap();
    let face = builder
        .add_scoped_m_value("face", PropKind::I32, MValueScope::Triangle)
        .unwrap();
    for (id, (vertices, triangles)) in (0_i32..).zip(meshes) {
        let points = vertices.iter().map(|&[x, y, _]| Point::new(x, y));
        let mut feature = builder.feature(Geometry::MultiPoint(points.collect()));
        feature.id(Some(id.unsigned_abs().into()));
        feature.z(vertices.iter().map(|v| v[2]).collect()).unwrap();
        feature.triangles(triangles.clone()).unwrap();
        let label = PropValue::Str(Some(format!("mesh {id}").into()));
        feature.property(name, label).unwrap();
        let parts = vertices
            .iter()
            .map(|v| if v[2] > 0 { "roof" } else { "ground" });
        let parts = MValue::Str(Some(parts.map(Into::into).collect()));
        feature.m_value(part, parts).unwrap();
        let faces = (0..).take(triangles.len()).map(|t| id * 100 + t);
        feature
            .m_value(face, MValue::I32(Some(faces.collect())))
            .unwrap();
        feature.finish().unwrap();
    }
    builder.finish()
}

/// A box of 8 vertices and 12 triangles, each facing out in (east, north, up).
fn cube(x: i32, y: i32) -> Mesh {
    // Tile y runs south, so north is -y.
    let (s, h) = (100, 1500);
    let vertices = vec![
        [x, y, 0],
        [x + s, y, 0],
        [x + s, y - s, 0],
        [x, y - s, 0],
        [x, y, h],
        [x + s, y, h],
        [x + s, y - s, h],
        [x, y - s, h],
    ];
    let triangles = vec![
        [0, 2, 1],
        [0, 3, 2],
        [4, 5, 6],
        [4, 6, 7],
        [0, 1, 5],
        [0, 5, 4],
        [1, 2, 6],
        [1, 6, 5],
        [2, 3, 7],
        [2, 7, 6],
        [3, 0, 4],
        [3, 4, 7],
    ];
    (vertices, triangles)
}

#[test]
fn meshes_and_their_triangle_values_survive_every_encoder_setting() {
    let cubes: Vec<Mesh> = (0..30)
        .map(|i| cube((i % 6) * 700, (i / 6) * -700))
        .collect();
    let layer = mesh_layer(&cubes);
    let no_sort = cfg_v2()
        .with_spatial_morton_sort(false)
        .with_spatial_hilbert_sort(false)
        .with_id_sort(false);
    for cfg in [
        cfg_v2(),
        no_sort,
        cfg_v2().with_tessellation(Tessellation::Always),
        cfg_v2().with_tessellation(Tessellation::Off),
        cfg_v2().with_triangles_only(true),
    ] {
        let bytes = layer.encode(cfg).expect("v2 encode");
        assert_dump_of_meshes(&bytes);
        let back = try_decode(&bytes).unwrap();
        assert_eq!(back.z_axis(), layer.z_axis());
        assert_eq!(back.property_names(), layer.property_names());
        assert_eq!(back.m_value_names(), layer.m_value_names());
        assert_eq!(back.m_value_scopes(), layer.m_value_scopes());
        assert_eq!(rows(&back), rows(&layer));
    }

    let bytes = layer.encode(cfg_v2()).unwrap();
    let tree = annotate(&bytes);
    let scopes = tree.regions.iter().filter(|r| r.label == "scope");
    assert_eq!(scopes.map(|r| bytes[r.offset]).collect::<Vec<_>>(), [0, 1]);
    let header = tree.regions.iter().find(|r| r.label == "header").unwrap();
    assert!(
        header
            .bits
            .iter()
            .any(|b| b.meaning() == "every feature is a Mesh, no types stream")
    );
}

fn arb_mesh() -> impl Strategy<Value = Mesh> {
    prop::collection::vec(any::<[i32; 3]>(), 0..24).prop_flat_map(|vertices| {
        let n = u32::try_from(vertices.len()).unwrap();
        let triangles = if n == 0 {
            Just(Vec::new()).boxed()
        } else {
            prop::collection::vec(prop::array::uniform3(0..n), 0..40).boxed()
        };
        (Just(vertices), triangles)
    })
}

proptest! {
    /// Empty meshes, meshes without triangles, spare vertices and degenerate or repeated
    /// triangles all come back, and the layer stays `Meshes`.
    #[test]
    fn random_meshes_round_trip(meshes in prop::collection::vec(arb_mesh(), 1..6)) {
        let layer = mesh_layer(&meshes);
        let bytes = layer.encode(cfg_v2()).expect("v2 encode");
        assert_dump_of_meshes(&bytes);
        let back = try_decode(&bytes).unwrap();
        prop_assert_eq!(back.z_axis(), layer.z_axis());
        prop_assert_eq!(rows(&back), rows(&layer));
    }
}

#[test]
fn a_corrupted_byte_anywhere_fails_to_decode_or_decodes_but_never_panics() {
    let layer = mesh_layer(&[cube(0, 0), cube(700, 0), (vec![[5, 5, 5]], Vec::new())]);
    let bytes = layer.encode(cfg_v2()).unwrap();
    for offset in 0..bytes.len() {
        for flip in [0x01, 0x10, 0x80, 0xFF] {
            let mut corrupted = bytes.clone();
            corrupted[offset] ^= flip;
            if let Ok(layers) = Parser::default().parse_layers(&corrupted) {
                for layer in layers {
                    if let Layer::Tag02(layer) = layer {
                        let _ = layer.into_tile(&mut Decoder::default());
                    }
                }
            }
        }
    }
}

#[test]
fn a_missing_triangle_value_stays_missing() {
    let mut builder = TileLayer::builder("landmarks", 16384).unwrap();
    builder.set_z_axis(height()).unwrap();
    let face = builder
        .add_scoped_m_value("face", PropKind::I32, MValueScope::Triangle)
        .unwrap();
    for (id, faces) in [(0, Some(vec![7; 12])), (1, None)] {
        let (vertices, triangles) = cube(id * 700, 0);
        let points = vertices.iter().map(|&[x, y, _]| Point::new(x, y));
        let mut feature = builder.feature(Geometry::MultiPoint(points.collect()));
        feature.id(Some(id.unsigned_abs().into()));
        feature.z(vertices.iter().map(|v| v[2]).collect()).unwrap();
        feature.triangles(triangles).unwrap();
        feature.m_value(face, MValue::I32(faces)).unwrap();
        feature.finish().unwrap();
    }
    let bytes = builder.finish().encode(cfg_v2()).unwrap();
    let back = try_decode(&bytes).unwrap();
    let mut faces: Vec<_> = back
        .features()
        .iter()
        .map(|f| (f.id(), f.m_values()[0].clone()))
        .collect();
    faces.sort_by_key(|f| f.0);
    assert_eq!(
        faces,
        [
            (Some(0), MValue::I32(Some(vec![7; 12]))),
            (Some(1), MValue::I32(None)),
        ]
    );
}

#[test]
fn geojson_writes_a_mesh_as_a_multi_point_with_triangles() {
    let mesh = (vec![[0, 0, 0], [10, 0, 0], [0, -10, 250]], vec![[0, 1, 2]]);
    let bytes = mesh_layer(&[mesh]).encode(cfg_v2()).unwrap();
    let layers = Decoder::default()
        .decode_all(Parser::default().parse_layers(&bytes).unwrap())
        .unwrap();
    let json = serde_json::to_string(&FeatureCollection::from_layers(layers).unwrap()).unwrap();
    assert_snapshot!(json, @r#"{"type":"FeatureCollection","features":[{"type":"Feature","id":0,"properties":{"_extent":16384,"_height_mode":"relative_to_ground","_layer":"landmarks","_z_step":-2,"m:face":[0],"m:part":["ground","ground","roof"],"name":"mesh 0"},"geometry":{"type":"MultiPoint","coordinates":[[0,0,0],[10,0,0],[0,-10,250]]},"triangles":[[0,1,2]]}]}"#);
}

/// A layer of a one-vertex mesh, on a z grid when `z`, and a point after it when `point`.
fn tiny(z: bool, point: bool) -> TileLayer {
    let mut builder = TileLayer::builder("tiny", 4096).unwrap();
    if z {
        builder.set_z_axis(height()).unwrap();
    }
    let mut mesh = builder.feature(Geometry::MultiPoint(MultiPoint(vec![Point::new(0, 0)])));
    if z {
        mesh.z(vec![0]).unwrap();
    }
    mesh.triangles(vec![[0, 0, 0]]).unwrap();
    mesh.finish().unwrap();
    if point {
        let mut point = builder.feature(Geometry::Point(Point::new(1, 1)));
        point.z(vec![0]).unwrap();
        point.finish().unwrap();
    }
    builder.finish()
}

#[rstest]
#[case::as_v1(
    tiny(true, false),
    WireVersion::V01,
    "meshes are a v2 feature, so layer tiny cannot be written as v1"
)]
#[case::without_a_z_step(
    tiny(false, false),
    WireVersion::V02,
    "a mesh needs (x, y, z) vertices, so its layer needs a z step"
)]
#[case::beside_a_point(
    tiny(true, true),
    WireVersion::V02,
    "a layer cannot mix meshes with other geometries"
)]
fn a_mesh_layer_that_cannot_be_written_is_rejected(
    #[case] layer: TileLayer,
    #[case] wire: WireVersion,
    #[case] expected: &str,
) {
    let err = layer.encode(cfg_v2().with_wire_version(wire)).unwrap_err();
    assert_eq!(err.to_string(), expected);
}

#[rstest]
#[case::multi_polygon_over_meshes(
    true,
    0x6,
    "uniform geometry type Some(MultiPolygon) cannot go with geometry layout Meshes: a Mesh needs the Meshes layout, which holds nothing else"
)]
#[case::types_stream_over_meshes(
    true,
    0x0,
    "uniform geometry type None cannot go with geometry layout Meshes: a Mesh needs the Meshes layout, which holds nothing else"
)]
#[case::mesh_over_points(
    false,
    0x7,
    "uniform geometry type Some(Mesh) cannot go with geometry layout Points: a Mesh needs the Meshes layout, which holds nothing else"
)]
fn the_uniform_type_mesh_and_the_meshes_layout_need_each_other(
    #[case] meshes: bool,
    #[case] nibble: u8,
    #[case] expected: &str,
) {
    let layer = if meshes {
        mesh_layer(&[cube(0, 0)])
    } else {
        points(ZStep::new(-2).unwrap())
    };
    let mut bytes = layer.encode(cfg_v2()).unwrap();
    let header = byte_offset(&bytes, "header");
    bytes[header] = (bytes[header] & 0b1000_1111) | (nibble << 4);
    assert_eq!(try_decode(&bytes).unwrap_err().to_string(), expected);
}

fn points(z_step: ZStep) -> TileLayer {
    let mut builder = TileLayer::builder("points", 4096).unwrap();
    builder.set_z_axis(z_step).unwrap();
    let mut point = builder.feature(Geometry::Point(Point::new(1, 1)));
    point.z(vec![7]).unwrap();
    point.finish().unwrap();
    builder.finish()
}

#[test]
fn a_reserved_scope_byte_is_rejected() {
    let mut bytes = mesh_layer(&[cube(0, 0)]).encode(cfg_v2()).unwrap();
    let scope = byte_offset(&bytes, "scope");
    assert_eq!(bytes[scope], 0);
    bytes[scope] = 2;
    assert_snapshot!(
        try_decode(&bytes).unwrap_err(),
        @"m-value scope byte 2 is reserved: only 0 (vertex) and 1 (triangle) are defined"
    );
}

#[test]
fn a_triangle_scoped_value_needs_a_mesh_and_one_value_per_triangle() {
    let mut builder = TileLayer::builder("landmarks", 4096).unwrap();
    builder.set_z_axis(height()).unwrap();
    let key = builder
        .add_scoped_m_value("part", PropKind::U8, MValueScope::Triangle)
        .unwrap();
    let points = || Geometry::MultiPoint((0..3).map(|i| Point::new(i, 0)).collect());

    let mut mesh = builder.feature(points());
    mesh.z(vec![0, 0, 0])
        .unwrap()
        .triangles(vec![[0, 1, 2]])
        .unwrap();
    mesh.m_value(key, MValue::U8(Some(vec![1, 2, 3]))).unwrap();
    assert_snapshot!(
        mesh.finish().unwrap_err(),
        @"m-value 0 holds 3 values, but needs one per triangle: 1"
    );

    let mut not_a_mesh = builder.feature(points());
    not_a_mesh.z(vec![0, 0, 0]).unwrap();
    not_a_mesh.m_value(key, MValue::U8(Some(vec![1]))).unwrap();
    assert_snapshot!(
        not_a_mesh.finish().unwrap_err(),
        @"m-value column part holds a value per triangle, which only a mesh has"
    );
}

#[cfg(feature = "__private")]
#[test]
fn an_empty_meshes_layer_round_trips_and_staged_triangle_m_values_validate() {
    use mlt_core::GeometryValues;
    use mlt_core::encoder::{Codecs, Encoder, StagedId, StagedLayer, StagedMValue, StagedValues};

    let empty = StagedLayer::with_m_values(
        "landmarks",
        4096,
        StagedId::None,
        GeometryValues::new_meshes(height()),
        Vec::new(),
        vec![
            StagedMValue::new("face", None, StagedValues::I32(Vec::new()))
                .with_scope(MValueScope::Triangle),
        ],
    )
    .unwrap();
    let mut codecs = Codecs::default();
    let bytes = empty
        .encode_into(Encoder::new(cfg_v2()), &mut codecs)
        .unwrap()
        .into_layer_bytes()
        .unwrap();
    assert_dump_of_meshes(&bytes);
    let raw = only_v2_layer(Parser::default().parse_layers(&bytes).unwrap());
    assert_eq!(
        raw.m_values()[0].as_raw().unwrap().scope(),
        MValueScope::Triangle
    );
    let parsed = raw.decode_all(&mut Decoder::default()).unwrap();
    assert!(parsed.layer().geometry_values().is_meshes());
    let back = parsed.into_tile(&mut Decoder::default()).unwrap();
    assert_eq!(back.feature_count(), 0);
    assert_eq!(back.z_axis(), Some(height()));
    assert_eq!(back.m_value_scopes(), &[MValueScope::Triangle]);
}

#[test]
fn walker_and_parser_reject_meshes_without_z() {
    let mut no_z = mesh_layer(&[cube(0, 0)]).encode(cfg_v2()).unwrap();
    let vertices_enc = annotate(&no_z)
        .regions
        .iter()
        .find(|r| r.label == "vertices")
        .map(|r| r.offset)
        .unwrap();
    // Clear extension bit 0 (XYZ) on the vertex stream's encoding byte.
    no_z[vertices_enc] &= !0x01;
    assert_eq!(
        Parser::default()
            .parse_layers(&no_z)
            .unwrap_err()
            .to_string(),
        "a mesh needs (x, y, z) vertices, so its layer needs a z step"
    );
    let (_, walk_err) = annotate_tile(&no_z);
    assert!(walk_err.is_some());
}

#[test]
fn scrambled_mesh_vertices_reorder_along_tree_cycles_and_keep_m_values_attached() {
    let (verts, tris) = cube(100, -100);
    // Reverse the vertex order so original triangle corners jump backwards from 7..0.
    let perm = [7_u32, 3, 5, 0, 6, 1, 4, 2];
    let mut inv = [0_u32; 8];
    for (new_idx, &old_idx) in perm.iter().enumerate() {
        inv[old_idx as usize] = u32::try_from(new_idx).unwrap();
    }
    let scrambled_verts: Vec<[i32; 3]> = perm.iter().map(|&old| verts[old as usize]).collect();
    let scrambled_tris: Vec<[u32; 3]> = tris.iter().map(|t| t.map(|v| inv[v as usize])).collect();

    let layer = mesh_layer(&[(scrambled_verts, scrambled_tris)]);
    let bytes = layer.encode(cfg_v2()).unwrap();
    let back = try_decode(&bytes).unwrap();
    assert_eq!(rows(&back), rows(&layer));

    // The first-seen vertices in `tri_cycles` now start at 0, 1, 2 in ascending order.
    let tree = annotate(&bytes);
    let cycles_idx = tree
        .regions
        .iter()
        .position(|r| r.label == "tri_cycles")
        .unwrap();
    let data_region = &tree.regions[(cycles_idx + 1..tree.regions.len())
        .find(|&i| tree.regions[i].blob.is_some())
        .unwrap()];
    let slice = &bytes[data_region.offset..data_region.offset + data_region.len];
    let mlt_core::dump::DecodedBlob::Numbers { values: cycles, .. } = mlt_core::dump::decode_blob(
        data_region.blob.unwrap(),
        slice,
        64,
        &mut Decoder::default(),
    ) else {
        panic!("expected numeric cycle blob");
    };
    let mut max_seen = 0.0_f64;
    for &c in &cycles {
        assert!(
            c <= max_seen + 1.0,
            "cycle jumped past next consecutive vertex: {cycles:?}"
        );
        max_seen = max_seen.max(c);
    }
}
