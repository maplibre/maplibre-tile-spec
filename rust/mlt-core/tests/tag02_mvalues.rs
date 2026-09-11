//! Round-trip and wire-level tests for v2 m-values, the vertex-scoped columns.

#![cfg(feature = "unstable-v2")]

use mlt_core::dump::{RenderOpts, annotate_tile, render};
use mlt_core::encoder::{
    Codecs, Encoder, EncoderConfig, ExplicitEncoder, FloatEncoding, IntEncoder, StagedId,
    StagedLayer, StagedMValue, StagedMValues, StrEncoding, VertexBufferType, WireVersion,
};
use mlt_core::geo_types::{
    Coord, Geometry, LineString, MultiLineString, MultiPoint, MultiPolygon, Point, Polygon,
};
use mlt_core::{
    Decoder, GeometryValues, Layer, MValue, MltError, Parser, PropKind, PropValue, TileFeature,
    TileLayer,
};
use rstest::rstest;

fn cfg_v2() -> EncoderConfig {
    EncoderConfig::default()
        .with_spatial_morton_sort(false)
        .with_spatial_hilbert_sort(false)
        .with_id_sort(false)
        .with_wire_version(WireVersion::V02)
}

fn decode(bytes: &[u8]) -> TileLayer {
    let mut parser = Parser::default();
    let layers = parser.parse_layers(bytes).expect("parse");
    assert_eq!(layers.len(), 1);
    let Layer::Tag02(layer) = layers.into_iter().next().expect("a layer") else {
        panic!("expected a v2 layer")
    };
    layer.into_tile(&mut Decoder::default()).expect("into_tile")
}

fn decode_err(bytes: &[u8]) -> MltError {
    let mut parser = Parser::default();
    match parser.parse_layers(bytes) {
        Err(e) => e,
        Ok(layers) => layers
            .into_iter()
            .next()
            .expect("a layer")
            .into_layer01()
            .expect("layer01")
            .into_tile(&mut Decoder::default())
            .expect_err("expected a rejection"),
    }
}

/// Encode as v2, decode, and require the layer to come back as it went in.
fn round_trip(layer: &TileLayer) -> Vec<u8> {
    let bytes = layer.clone().encode(cfg_v2()).expect("v2 encode");
    assert_eq!(&decode(&bytes), layer);
    assert_dump_covers(&bytes);
    bytes
}

fn assert_dump_covers(bytes: &[u8]) {
    let tree = annotate_tile(bytes).expect("annotate_tile");
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
}

fn dump_text(bytes: &[u8]) -> String {
    let tree = annotate_tile(bytes).expect("annotate_tile");
    let mut out = Vec::new();
    render(&tree, bytes, &RenderOpts::default(), &mut out).expect("render");
    String::from_utf8(out).expect("dump is utf8")
}

/// How often the dump reports `field`, whose value the label is followed by.
fn dump_count(bytes: &[u8], field: &str) -> usize {
    dump_text(bytes).matches(field).count()
}

/// The offset and length of the first region labelled `label` at or after the
/// first region whose label starts with `after`.
fn region(bytes: &[u8], after: &str, label: &str) -> (usize, usize) {
    let tree = annotate_tile(bytes).expect("annotate_tile");
    let start = tree
        .regions
        .iter()
        .position(|r| r.label.starts_with(after))
        .unwrap_or_else(|| panic!("no region labelled {after}"));
    let found = tree.regions[start..]
        .iter()
        .find(|r| r.label == label)
        .unwrap_or_else(|| panic!("no region labelled {label} after {after}"));
    (found.offset, found.len)
}

fn coords(pts: &[(i32, i32)]) -> Vec<Coord<i32>> {
    pts.iter().map(|&(x, y)| Coord { x, y }).collect()
}

fn line(pts: &[(i32, i32)]) -> Geometry<i32> {
    Geometry::LineString(LineString::new(coords(pts)))
}

fn ring(pts: &[(i32, i32)]) -> LineString<i32> {
    let mut ls = LineString::new(coords(pts));
    ls.close();
    ls
}

/// A layer of `geoms`, each carrying the m-values of the same index in every column.
fn layer(geoms: Vec<Geometry<i32>>, columns: &[(&str, Vec<MValue>)]) -> TileLayer {
    let mut builder = TileLayer::builder("test_layer", 4096).unwrap();
    let keys: Vec<_> = columns
        .iter()
        .map(|(name, values)| builder.add_m_value(*name, values[0].kind()).unwrap())
        .collect();
    for (index, geom) in geoms.into_iter().enumerate() {
        let mut feature = builder.feature(geom);
        for (key, (_, values)) in keys.iter().zip(columns) {
            feature.m_value(*key, values[index].clone()).unwrap();
        }
        feature.finish().unwrap();
    }
    builder.finish()
}

fn i32s(values: &[&[i32]]) -> Vec<MValue> {
    values
        .iter()
        .map(|v| MValue::I32(Some(v.to_vec())))
        .collect()
}

#[test]
fn a_line_layer_holds_one_value_per_vertex() {
    let l = layer(
        vec![line(&[(0, 0), (1, 1), (2, 2)]), line(&[(5, 5), (6, 6)])],
        &[("dist", i32s(&[&[0, 10, 20], &[0, 30]]))],
    );
    let bytes = round_trip(&l);
    assert_eq!(dump_count(&bytes, "m-value section = 1"), 1);
    assert_eq!(dump_count(&bytes, "m_value[0] I32 \"dist\""), 1);
}

#[test]
fn a_polygon_ring_has_no_value_for_its_closing_vertex() {
    let hole = ring(&[(2, 2), (2, 3), (3, 3), (3, 2)]);
    let outer = ring(&[(0, 0), (0, 9), (9, 9), (9, 0)]);
    let poly = Geometry::Polygon(Polygon::new(outer, vec![hole]));
    // Four outer and four inner vertices, with the two closing ones stripped.
    let l = layer(vec![poly], &[("m", i32s(&[&[1, 2, 3, 4, 5, 6, 7, 8]]))]);
    round_trip(&l);
}

#[rstest]
#[case::multipoint(
    Geometry::MultiPoint(MultiPoint(vec![Point::new(1, 2), Point::new(3, 4)])),
    vec![7, 8],
)]
#[case::multiline(
    Geometry::MultiLineString(MultiLineString(vec![
        LineString::new(coords(&[(0, 0), (1, 1)])),
        LineString::new(coords(&[(4, 4), (5, 5), (6, 6)])),
    ])),
    vec![1, 2, 3, 4, 5],
)]
#[case::multipolygon(
    Geometry::MultiPolygon(MultiPolygon(vec![
        Polygon::new(ring(&[(0, 0), (0, 2), (2, 2)]), vec![]),
        Polygon::new(ring(&[(5, 5), (5, 7), (7, 7)]), vec![]),
    ])),
    vec![1, 2, 3, 4, 5, 6],
)]
fn every_multi_geometry_counts_all_of_its_vertices(
    #[case] geom: Geometry<i32>,
    #[case] values: Vec<i32>,
) {
    let l = layer(vec![geom], &[("m", vec![MValue::I32(Some(values))])]);
    round_trip(&l);
}

#[test]
fn columns_of_every_type_round_trip() {
    let two = |a, b| vec![a, b];
    let l = layer(
        vec![line(&[(0, 0), (1, 1)]), line(&[(2, 2), (3, 3)])],
        &[
            (
                "bool",
                two(
                    MValue::Bool(Some(vec![true, false])),
                    MValue::Bool(Some(vec![false, true])),
                ),
            ),
            (
                "i8",
                two(
                    MValue::I8(Some(vec![-1, 2])),
                    MValue::I8(Some(vec![3, i8::MIN])),
                ),
            ),
            (
                "u8",
                two(MValue::U8(Some(vec![0, 255])), MValue::U8(Some(vec![7, 8]))),
            ),
            (
                "i32",
                two(
                    MValue::I32(Some(vec![i32::MIN, 0])),
                    MValue::I32(Some(vec![5, i32::MAX])),
                ),
            ),
            (
                "u32",
                two(
                    MValue::U32(Some(vec![0, u32::MAX])),
                    MValue::U32(Some(vec![9, 10])),
                ),
            ),
            (
                "i64",
                two(
                    MValue::I64(Some(vec![i64::MIN, 1])),
                    MValue::I64(Some(vec![2, i64::MAX])),
                ),
            ),
            (
                "u64",
                two(
                    MValue::U64(Some(vec![0, u64::MAX])),
                    MValue::U64(Some(vec![3, 4])),
                ),
            ),
            (
                "f32",
                two(
                    MValue::F32(Some(vec![0.5, -1.25])),
                    MValue::F32(Some(vec![2.0, 3.5])),
                ),
            ),
            (
                "f64",
                two(
                    MValue::F64(Some(vec![0.5, -1.25])),
                    MValue::F64(Some(vec![2.0, 3.5])),
                ),
            ),
            (
                "str",
                two(
                    MValue::Str(Some(vec!["a".into(), "bb".into()])),
                    MValue::Str(Some(vec!["a".into(), "ccc".into()])),
                ),
            ),
        ],
    );
    round_trip(&l);
}

#[test]
fn a_feature_with_no_values_stores_an_inline_bitfield_and_no_values() {
    let l = layer(
        vec![
            line(&[(0, 0), (1, 1)]),
            line(&[(2, 2), (3, 3)]),
            line(&[(4, 4), (5, 5)]),
        ],
        &[(
            "m",
            vec![
                MValue::I32(Some(vec![1, 2])),
                MValue::I32(None),
                MValue::I32(Some(vec![3, 4])),
            ],
        )],
    );
    let bytes = round_trip(&l);
    assert_eq!(dump_count(&bytes, "presence = Inline"), 1);
    // Four values for the two features that have them, not six.
    let (count, _) = region(&bytes, "m_values", "num_values");
    assert_eq!(bytes[count], 4);
    assert_eq!(
        decode(&bytes).features()[1].m_values()[0],
        MValue::I32(None)
    );
}

#[test]
fn two_m_value_columns_with_the_same_nulls_share_one_bitfield() {
    let geoms = vec![
        line(&[(0, 0), (1, 1)]),
        line(&[(2, 2), (3, 3)]),
        line(&[(4, 4), (5, 5)]),
    ];
    let column = || {
        vec![
            MValue::I32(Some(vec![1, 2])),
            MValue::I32(None),
            MValue::I32(Some(vec![3, 4])),
        ]
    };
    let l = layer(geoms, &[("a", column()), ("b", column())]);
    let bytes = round_trip(&l);
    assert_eq!(dump_count(&bytes, "shared presence bitfields = 1"), 1);
    assert_eq!(dump_count(&bytes, "presence = Shared(0)"), 2);
    assert_eq!(dump_count(&bytes, "presence = Inline"), 0);
}

#[test]
fn an_m_value_column_shares_a_bitfield_with_a_property_column() {
    let mut builder = TileLayer::builder("test_layer", 4096).unwrap();
    let prop = builder.add_property("p", PropKind::U32).unwrap();
    let m = builder.add_m_value("m", PropKind::I32).unwrap();
    for (index, present) in [true, false, true].into_iter().enumerate() {
        let geom = line(&[(0, i32::try_from(index).unwrap()), (1, 1)]);
        let mut feature = builder.feature(geom);
        feature
            .property(prop, PropValue::U32(present.then_some(1)))
            .unwrap();
        feature
            .m_value(m, MValue::I32(present.then(|| vec![1, 2])))
            .unwrap();
        feature.finish().unwrap();
    }
    let l = builder.finish();
    let bytes = round_trip(&l);
    assert_eq!(dump_count(&bytes, "shared presence bitfields = 1"), 1);
    assert_eq!(dump_count(&bytes, "presence = Shared(0)"), 2);
}

#[test]
fn m_values_on_a_point_layer_are_rejected() {
    let mut builder = TileLayer::builder("test_layer", 4096).unwrap();
    let m = builder.add_m_value("m", PropKind::I32).unwrap();
    let mut feature = builder.feature(Geometry::Point(Point::new(1, 2)));
    feature.m_value(m, MValue::I32(Some(vec![5]))).unwrap();
    feature.finish().unwrap();

    let err = builder.finish().encode(cfg_v2()).unwrap_err();
    assert!(
        matches!(err, MltError::MValuesNeedVertexCounts("Points")),
        "{err:?}"
    );
}

#[test]
fn a_layer_with_m_values_cannot_be_written_as_v1() {
    let l = layer(vec![line(&[(0, 0), (1, 1)])], &[("m", i32s(&[&[1, 2]]))]);
    let err = l
        .encode(cfg_v2().with_wire_version(WireVersion::V01))
        .unwrap_err();
    assert!(
        matches!(err, MltError::MValuesNeedV2(ref name) if name == "test_layer"),
        "expected the v1 writer to refuse the layer"
    );
}

#[test]
fn a_column_holding_the_wrong_number_of_values_is_rejected() {
    let mut builder = TileLayer::builder("test_layer", 4096).unwrap();
    let m = builder.add_m_value("m", PropKind::I32).unwrap();
    let mut feature = builder.feature(line(&[(0, 0), (1, 1), (2, 2)]));
    let err = feature
        .m_value(m, MValue::I32(Some(vec![1, 2])))
        .err()
        .expect("a rejection");
    assert!(
        matches!(
            err,
            MltError::MValueVertexCountMismatch {
                index: 0,
                expected: 3,
                actual: 2,
                ..
            }
        ),
        "{err:?}"
    );
}

#[test]
fn a_column_of_the_wrong_kind_is_rejected() {
    let mut builder = TileLayer::builder("test_layer", 4096).unwrap();
    let m = builder.add_m_value("m", PropKind::I32).unwrap();
    let mut feature = builder.feature(line(&[(0, 0), (1, 1)]));
    let err = feature
        .m_value(m, MValue::U32(Some(vec![1, 2])))
        .err()
        .expect("a rejection");
    assert!(
        matches!(
            err,
            MltError::MValueKindMismatch {
                index: 0,
                expected: PropKind::I32,
                actual: PropKind::U32,
            }
        ),
        "{err:?}"
    );
}

#[test]
fn a_duplicate_m_value_name_is_rejected() {
    let mut l = TileLayer::new("test_layer", 4096).unwrap();
    l.add_m_value("m", PropKind::I32).unwrap();
    let err = l.add_m_value("m", PropKind::U32).unwrap_err();
    assert!(
        matches!(err, MltError::DuplicateMValueName(ref name) if name == "m"),
        "{err:?}"
    );
}

/// A layer whose one m-value column holds four single-byte values.
fn patchable() -> Vec<u8> {
    let l = layer(
        vec![line(&[(0, 0), (1, 1)]), line(&[(2, 2), (3, 3)])],
        &[("m", i32s(&[&[3, 9], &[4, 20]]))],
    );
    l.encode(cfg_v2()).expect("v2 encode")
}

#[test]
fn a_leading_stream_without_its_own_count_is_rejected() {
    let mut bytes = patchable();
    let (encoding, _) = region(&bytes, "m_values", "encoding");
    let (count, count_len) = region(&bytes, "m_values", "num_values");
    assert_eq!(count_len, 1, "the count has to be a single byte to drop it");
    bytes[encoding] &= 0b0111_1111;
    bytes.remove(count);
    // The layer's length prefix counts the byte that was dropped.
    bytes[0] -= 1;

    let err = decode_err(&bytes);
    assert!(
        matches!(err, MltError::MValueImplicitCount { ref name, .. } if name == "m"),
        "{err:?}"
    );
}

#[test]
fn a_count_the_geometry_disagrees_with_is_rejected() {
    let mut bytes = patchable();
    let (count, _) = region(&bytes, "m_values", "num_values");
    let (byte_length, _) = region(&bytes, "m_values", "byte_length");
    assert_eq!(bytes[count], 4);
    assert_eq!(bytes[byte_length], 4);
    bytes[count] = 3;
    bytes[byte_length] = 3;
    bytes.remove(byte_length + 4);
    bytes[0] -= 1;

    let err = decode_err(&bytes);
    assert!(
        matches!(
            err,
            MltError::MValueCountMismatch {
                ref name,
                expected: 4,
                actual: 3,
            } if name == "m"
        ),
        "{err:?}"
    );
}

#[test]
fn an_empty_m_value_section_is_rejected() {
    let mut bytes = patchable();
    let (count, _) = region(&bytes, "m_values", "m_value_count");
    bytes[count] = 0;

    assert!(
        matches!(decode_err(&bytes), MltError::EmptyMValueSection),
        "expected an empty section to be rejected"
    );
}

#[rstest]
#[case::id(0x00)]
#[case::long_id(0x01)]
#[case::shared_dict(0x0F)]
#[case::unassigned(0x0C)]
fn a_type_byte_no_vertex_can_hold_is_rejected(#[case] data_type: u8) {
    let mut bytes = patchable();
    let (typ, _) = region(&bytes, "m_values", "type");
    bytes[typ] = (bytes[typ] & 0b1111_0000) | data_type;

    let err = decode_err(&bytes);
    assert!(
        matches!(err, MltError::ParsingColumnType(b) if b == bytes[typ]),
        "{err:?}"
    );
}

#[test]
fn a_duplicate_name_on_the_wire_is_rejected() {
    let l = layer(
        vec![line(&[(0, 0), (1, 1)])],
        &[("aa", i32s(&[&[1, 2]])), ("bb", i32s(&[&[3, 4]]))],
    );
    let mut bytes = l.encode(cfg_v2()).expect("v2 encode");
    let (second, len) = region(&bytes, "m_value[1]", "name");
    assert_eq!(len, 3, "a one-byte length prefix and a two-byte name");
    bytes[second + 1..second + 3].copy_from_slice(b"aa");

    let err = decode_err(&bytes);
    assert!(
        matches!(err, MltError::DuplicateMValueName(ref name) if name == "aa"),
        "{err:?}"
    );
}

#[test]
fn the_layer_reports_its_m_value_columns() {
    let l = layer(
        vec![line(&[(0, 0), (1, 1)])],
        &[("a", i32s(&[&[1, 2]])), ("b", i32s(&[&[3, 4]]))],
    );
    let decoded = decode(&l.clone().encode(cfg_v2()).unwrap());
    assert_eq!(decoded.m_value_names(), ["a", "b"]);
    assert_eq!(decoded.m_value_kinds(), [PropKind::I32, PropKind::I32]);
    assert_eq!(
        decoded.features()[0].m_values(),
        [MValue::I32(Some(vec![1, 2])), MValue::I32(Some(vec![3, 4]))]
    );
}

#[test]
fn a_feature_iterator_sees_the_same_geometry_as_the_m_values_describe() {
    let l = layer(
        vec![line(&[(0, 0), (1, 1), (2, 2)])],
        &[("m", i32s(&[&[1, 2, 3]]))],
    );
    let decoded = decode(&l.clone().encode(cfg_v2()).unwrap());
    let feature: &TileFeature = &decoded.features()[0];
    assert_eq!(feature.vertex_count(), 3);
    assert_eq!(feature.m_values()[0].count(), Some(3));
}

/// A layer whose vertices are stored as a dictionary, which m-values run over
/// through the offsets rather than the distinct vertices.
fn dictionary_layer() -> Vec<u8> {
    let mut geometry = GeometryValues::default();
    // Two lines over the same three vertices, so the dictionary holds three of six.
    for _ in 0..2 {
        geometry.push_geom(&line(&[(0, 0), (8, 8), (16, 16)]));
    }
    let m_value = StagedMValue::new("m", None, StagedMValues::I32(vec![1, 2, 3, 4, 5, 6]));
    let staged = StagedLayer::with_m_values(
        "test_layer",
        4096,
        StagedId::None,
        geometry,
        vec![],
        vec![m_value],
    )
    .expect("staged layer");
    let explicit = ExplicitEncoder {
        vertex_buffer_type: VertexBufferType::Hilbert,
        force_stream: Box::new(|_| false),
        get_int_encoder: Box::new(|_| IntEncoder::varint()),
        get_str_encoding: Box::new(|_| StrEncoding::Plain),
        get_float_encoding: Box::new(|_| FloatEncoding::None),
    };
    staged
        .encode_into(
            Encoder::with_explicit(cfg_v2(), explicit),
            &mut Codecs::default(),
        )
        .expect("encode")
        .into_layer_bytes()
        .expect("layer bytes")
}

#[test]
fn a_dictionary_vertex_layout_holds_one_value_per_offset() {
    let bytes = dictionary_layer();
    assert_eq!(dump_count(&bytes, "geometry layout = LinesDict"), 1);
    let (count, _) = region(&bytes, "m_values", "num_values");
    // Six vertices over a dictionary of three.
    assert_eq!(bytes[count], 6);

    let decoded = decode(&bytes);
    assert_eq!(
        decoded.features()[0].m_values()[0],
        MValue::I32(Some(vec![1, 2, 3]))
    );
    assert_eq!(
        decoded.features()[1].m_values()[0],
        MValue::I32(Some(vec![4, 5, 6]))
    );
}

#[rstest]
#[case::points(0x0, "Points")]
#[case::points_dict(0x1, "PointsDict")]
#[case::tess_polygons(0xC, "TessPolygons")]
fn a_geometry_layout_without_vertex_counts_is_rejected(#[case] geo_layout: u8, #[case] name: &str) {
    let mut bytes = patchable();
    let (layout, _) = region(&bytes, "layout", "layout");
    bytes[layout] = (bytes[layout] & 0b1111_0000) | geo_layout;

    let err = decode_err(&bytes);
    assert!(
        matches!(err, MltError::MValuesNeedVertexCounts(n) if n == name),
        "{err:?}"
    );
}
