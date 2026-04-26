use std::collections::HashSet;

use geo_types::{Coord, Geometry, LineString, Point, Polygon, point, wkt};
use pretty_assertions::assert_eq;
use rstest::rstest;

use crate::decoder::RawGeometry;
use crate::encoder::model::EncoderConfig;
use crate::encoder::{Encoder, ExplicitEncoder, IntEncoder, VertexBufferType};
use crate::test_helpers::{assert_empty, dec, parser};
use crate::{Decode as _, DictionaryType, GeometryValues, LengthType, OffsetType, StreamType};

#[rstest]
#[case::single_point(push_geoms(&[wkt!(POINT(10 20)).into()]))]
#[case::linestring(push_geoms(&[wkt!(LINESTRING(10 20, 30 40, 50 60)).into()]))]
#[case::polygon(push_geoms(&[wkt!(POLYGON((0 0, 100 0, 100 100, 0 100, 0 0))).into()]))]
#[case::multi_polygon(push_geoms(&[wkt!(MULTIPOLYGON(((0 0, 10 0, 10 10, 0 0),(5 5, 15 5, 15 15, 5 15)))).into()]))]
fn automatic_optimization_roundtrip(#[case] decoded: GeometryValues) {
    let mut enc = Encoder::default();
    decoded.clone().write_to(&mut enc).expect("optimize failed");
    assert_geometry_roundtrip(&enc.data, &decoded);
}

#[rstest]
#[case::distinct_points_vec2(
    push_geoms(&[(0i32..10).map(|i| point!{ x: i, y: i }.into()).collect::<Vec<_>>()].concat()),
    false
)]
#[case::repeated_points_dict(
    push_geoms(&std::iter::repeat_n(point!{ x: 5, y: 5 }.into(), 20).collect::<Vec<_>>()),
    true
)]
fn automatic_optimization_picks_correct_vertex_strategy(
    #[case] decoded: GeometryValues,
    #[case] expect_dict: bool,
) {
    let mut enc = Encoder::default();
    decoded.write_to(&mut enc).expect("encode failed");
    let types = encoded_stream_types(&enc.data);

    let has_vertex_offsets = types.contains(&StreamType::Offset(OffsetType::Vertex));
    let has_morton_dict = types.contains(&StreamType::Data(DictionaryType::Morton));
    let has_vertex_dict = types.contains(&StreamType::Data(DictionaryType::Vertex));

    if expect_dict {
        assert!(
            has_vertex_offsets,
            "auto-dict path must emit a vertex offset stream"
        );
        assert!(
            has_morton_dict || has_vertex_dict,
            "auto-dict path must emit either a Morton or Hilbert (Vertex) dictionary stream"
        );
    } else {
        assert!(
            !has_vertex_offsets,
            "Vec2 path must not emit a vertex offset stream"
        );
        assert!(
            has_vertex_dict,
            "Vec2 path must emit a Vertex (componentwise-delta) data stream"
        );
        assert!(
            !has_morton_dict,
            "Vec2 path must not emit a Morton dictionary stream"
        );
    }
}

#[test]
fn encoded_output_always_has_meta_stream() {
    let decoded = push_geoms(&[Geometry::<i32>::Point(Point(Coord::<i32> { x: 1, y: 1 }))]);
    let mut enc = Encoder::default();
    decoded.write_to(&mut enc).expect("encode failed");
    let raw = assert_empty(RawGeometry::from_bytes(&enc.data, &mut parser()));

    assert_eq!(
        raw.meta.meta.stream_type,
        StreamType::Length(LengthType::VarBinary),
        "meta (VarBinary) stream must always be present"
    );
}

#[test]
fn encoded_polygon_has_topology_streams() {
    let coords: Vec<Coord<i32>> = [(0, 0), (10, 0), (10, 10), (0, 0)]
        .into_iter()
        .map(|(x, y)| Coord::<i32> { x, y })
        .collect();
    let decoded = push_geoms(&[Geometry::<i32>::Polygon(Polygon::new(
        LineString(coords),
        vec![],
    ))]);
    let mut enc = Encoder::default();
    decoded.write_to(&mut enc).expect("encode failed");

    let stream_types = encoded_stream_types(&enc.data);
    assert!(
        stream_types.contains(&StreamType::Length(LengthType::Rings))
            || stream_types.contains(&StreamType::Length(LengthType::Parts)),
        "polygon must produce at least a Parts or Rings length stream"
    );
}

#[test]
fn encoded_repeated_points_uses_dict_streams() {
    // All vertices identical: uniqueness ratio is below the dictionary threshold
    // so the optimizer picks a dictionary path. The Hilbert vs Morton race then
    // chooses whichever serializes smaller — the test only asserts that the
    // dictionary path was taken, not which curve won.
    let mut decoded = GeometryValues::default();
    decoded.push_geom(&wkt!(MULTIPOINT(5 5, 5 5, 5 5)).into());
    let mut enc = Encoder::default();
    decoded.write_to(&mut enc).expect("encode failed");

    let stream_types = encoded_stream_types(&enc.data);
    assert!(
        stream_types.contains(&StreamType::Data(DictionaryType::Morton))
            || stream_types.contains(&StreamType::Data(DictionaryType::Vertex)),
        "repeated vertices must trigger a dictionary encoding (Morton or Hilbert/Vertex)"
    );
    assert!(
        stream_types.contains(&StreamType::Offset(OffsetType::Vertex)),
        "dictionary encoding must include a vertex offset stream"
    );

    let raw = assert_empty(RawGeometry::from_bytes(&enc.data, &mut parser()));
    assert_eq!(
        raw.meta.meta.stream_type,
        StreamType::Length(LengthType::VarBinary),
        "meta stream must always be present"
    );
}

#[rstest]
#[case::vec2(VertexBufferType::Vec2)]
#[case::morton(VertexBufferType::Morton)]
#[case::hilbert(VertexBufferType::Hilbert)]
fn forced_vertex_strategy_roundtrips(#[case] strategy: VertexBufferType) {
    // Repeated coordinates so dict paths actually have collisions to dedup.
    let mut decoded = GeometryValues::default();
    decoded.push_geom(&wkt!(MULTIPOINT(5 5, 10 10, 5 5, 10 10, 0 0, 5 5)).into());

    let explicit = ExplicitEncoder {
        vertex_buffer_type: strategy,
        ..ExplicitEncoder::all(IntEncoder::varint())
    };
    let mut enc = Encoder::with_explicit(EncoderConfig::default(), explicit);
    decoded.clone().write_to(&mut enc).expect("encode failed");

    let stream_types = encoded_stream_types(&enc.data);
    match strategy {
        VertexBufferType::Vec2 => {
            assert!(stream_types.contains(&StreamType::Data(DictionaryType::Vertex)));
            assert!(!stream_types.contains(&StreamType::Offset(OffsetType::Vertex)));
        }
        VertexBufferType::Morton => {
            assert!(stream_types.contains(&StreamType::Data(DictionaryType::Morton)));
            assert!(stream_types.contains(&StreamType::Offset(OffsetType::Vertex)));
        }
        VertexBufferType::Hilbert => {
            assert!(stream_types.contains(&StreamType::Data(DictionaryType::Vertex)));
            assert!(stream_types.contains(&StreamType::Offset(OffsetType::Vertex)));
            // The Hilbert path also writes a vertex offset stream, distinguishing
            // it from the Vec2 path which only emits the data stream.
        }
    }

    assert_geometry_roundtrip(&enc.data, &decoded);
}

#[test]
fn manual_encode_works() {
    let decoded = push_geoms(&[wkt!(POINT(10 20)).into()]);

    let mut enc = Encoder::default();
    decoded.clone().write_to(&mut enc).expect("encode failed");
    let types = encoded_stream_types(&enc.data);
    assert!(types.contains(&StreamType::Data(DictionaryType::Vertex)));

    assert_geometry_roundtrip(&enc.data, &decoded);
}

/// Round-trip geometry bytes: parse then decode and compare.
fn assert_geometry_roundtrip(data: &[u8], expected: &GeometryValues) {
    let mut p = parser();
    let mut d = dec();
    let raw = assert_empty(RawGeometry::from_bytes(data, &mut p));
    let result = raw.decode(&mut d).unwrap();
    assert!(
        d.consumed() > 0,
        "decoder should consume bytes after decode"
    );
    assert_eq!(expected, &result);
}

fn push_geoms(geoms: &[Geometry<i32>]) -> GeometryValues {
    let mut d = GeometryValues::default();
    for g in geoms {
        d.push_geom(g);
    }
    d
}

/// Collect all stream types present in the encoded geometry bytes (meta + items).
fn encoded_stream_types(data: &[u8]) -> HashSet<StreamType> {
    let raw = assert_empty(RawGeometry::from_bytes(data, &mut parser()));
    std::iter::once(raw.meta.meta.stream_type)
        .chain(raw.items.iter().map(|s| s.meta.stream_type))
        .collect()
}
