use std::collections::HashSet;

use geo_types::{Coord, Geometry, LineString, Point, Polygon, point, wkt};
use pretty_assertions::assert_eq;
use rstest::rstest;

use crate::decoder::RawGeometry;
use crate::encoder::model::EncoderConfig;
use crate::encoder::{Codecs, Encoder, ExplicitEncoder, IntEncoder, VertexBufferType};
use crate::test_helpers::{assert_empty, dec, parser};
use crate::{Decode as _, DictionaryType, GeometryValues, LengthType, StreamType};

#[rstest]
#[case::single_point(push_geoms(&[wkt!(POINT(10 20)).into()]))]
#[case::linestring(push_geoms(&[wkt!(LINESTRING(10 20, 30 40, 50 60)).into()]))]
#[case::polygon(push_geoms(&[wkt!(POLYGON((0 0, 100 0, 100 100, 0 100, 0 0))).into()]))]
#[case::multi_polygon(push_geoms(&[wkt!(MULTIPOLYGON(((0 0, 10 0, 10 10, 0 0),(5 5, 15 5, 15 15, 5 15)))).into()]))]
fn automatic_optimization_roundtrip(#[case] decoded: GeometryValues) {
    let mut enc = Encoder::default();
    let mut codecs = Codecs::default();
    decoded
        .clone()
        .write_to(&mut enc, &mut codecs)
        .expect("optimize failed");
    assert_geometry_roundtrip(&enc.data, &decoded);
}

fn auto_mode_streams(decoded: &GeometryValues) -> Vec<StreamType> {
    let mut enc = Encoder::default();
    let mut codecs = Codecs::default();
    decoded
        .clone()
        .write_to(&mut enc, &mut codecs)
        .expect("encode failed");

    let mut stream_types: Vec<StreamType> = encoded_stream_types(&enc.data).into_iter().collect();
    stream_types.sort();

    assert_geometry_roundtrip(&enc.data, decoded);
    stream_types
}

#[test]
fn automatic_optimization_distinct_points_picks_vec2() {
    let decoded = push_geoms(
        &(0i32..10)
            .map(|i| point! { x: i, y: i }.into())
            .collect::<Vec<_>>(),
    );
    insta::assert_debug_snapshot!(auto_mode_streams(&decoded), @r"
    [
        Data(
            Vertex,
        ),
        Length(
            VarBinary,
        ),
    ]
    ");
}

#[test]
fn automatic_optimization_repeated_points_picks_dict() {
    // The Hilbert vs. Morton race resolves deterministically for this input —
    // Hilbert wins, so the encoded streams use `Data(Vertex)` + a vertex
    // offset stream. The snapshot pins that outcome; if the race tie-break
    // or the heuristic ever changes it should fail loudly.
    let decoded =
        push_geoms(&std::iter::repeat_n(point! { x: 5, y: 5 }.into(), 20).collect::<Vec<_>>());
    insta::assert_debug_snapshot!(auto_mode_streams(&decoded), @r"
    [
        Data(
            Vertex,
        ),
        Offset(
            Vertex,
        ),
        Length(
            VarBinary,
        ),
    ]
    ");
}

#[test]
fn encoded_output_always_has_meta_stream() {
    let decoded = push_geoms(&[Geometry::<i32>::Point(Point(Coord::<i32> { x: 1, y: 1 }))]);
    let mut enc = Encoder::default();
    let mut codecs = Codecs::default();
    decoded
        .write_to(&mut enc, &mut codecs)
        .expect("encode failed");
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
    let mut codecs = Codecs::default();
    decoded
        .write_to(&mut enc, &mut codecs)
        .expect("encode failed");

    let stream_types = encoded_stream_types(&enc.data);
    assert!(
        stream_types.contains(&StreamType::Length(LengthType::Rings))
            || stream_types.contains(&StreamType::Length(LengthType::Parts)),
        "polygon must produce at least a Parts or Rings length stream"
    );
}

/// Encode `decoded` with the vertex layout pinned to `strategy`, return the
/// (sorted) stream types in the wire output, and assert the bytes round-trip
/// back to the same `GeometryValues`.
fn forced_vertex_strategy_streams(
    decoded: &GeometryValues,
    strategy: VertexBufferType,
) -> Vec<StreamType> {
    let explicit = ExplicitEncoder {
        vertex_buffer_type: strategy,
        ..ExplicitEncoder::all(IntEncoder::varint())
    };
    let mut enc = Encoder::with_explicit(EncoderConfig::default(), explicit);
    let mut codecs = Codecs::default();
    decoded
        .clone()
        .write_to(&mut enc, &mut codecs)
        .expect("encode failed");

    let mut stream_types: Vec<StreamType> = encoded_stream_types(&enc.data).into_iter().collect();
    stream_types.sort();

    assert_geometry_roundtrip(&enc.data, decoded);
    stream_types
}

/// Multipoint with repeated coordinates so dict paths actually dedup.
fn repeated_multipoint() -> GeometryValues {
    let mut g = GeometryValues::default();
    g.push_geom(&wkt!(MULTIPOINT(5 5, 10 10, 5 5, 10 10, 0 0, 5 5)).into());
    g
}

#[test]
fn forced_vec2_streams() {
    let streams = forced_vertex_strategy_streams(&repeated_multipoint(), VertexBufferType::Vec2);
    insta::assert_debug_snapshot!(streams, @r"
    [
        Data(
            Vertex,
        ),
        Length(
            VarBinary,
        ),
        Length(
            Geometries,
        ),
    ]
    ");
}

#[test]
fn forced_morton_streams() {
    let streams = forced_vertex_strategy_streams(&repeated_multipoint(), VertexBufferType::Morton);
    insta::assert_debug_snapshot!(streams, @r"
    [
        Data(
            Morton,
        ),
        Offset(
            Vertex,
        ),
        Length(
            VarBinary,
        ),
        Length(
            Geometries,
        ),
    ]
    ");
}

#[test]
fn forced_hilbert_streams() {
    let streams = forced_vertex_strategy_streams(&repeated_multipoint(), VertexBufferType::Hilbert);
    insta::assert_debug_snapshot!(streams, @r"
    [
        Data(
            Vertex,
        ),
        Offset(
            Vertex,
        ),
        Length(
            VarBinary,
        ),
        Length(
            Geometries,
        ),
    ]
    ");
}

#[test]
fn manual_encode_works() {
    let decoded = push_geoms(&[wkt!(POINT(10 20)).into()]);

    let mut enc = Encoder::default();
    let mut codecs = Codecs::default();
    decoded
        .clone()
        .write_to(&mut enc, &mut codecs)
        .expect("encode failed");
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
