//! Under vertex sort, the z and m-values of each vertex move with it.

use mlt_core::encoder::{EncoderConfig, Network, Profile, Tessellation, WireVersion};
use mlt_core::geo_types::{Geometry, LineString, MultiPoint, Point, Polygon};
use mlt_core::{Decoder, MValue, Parser, PropKind, TileLayer, ZStep};

fn line(coords: &[(i32, i32)]) -> LineString<i32> {
    LineString::from(coords.to_vec())
}

fn polygon(exterior: &[(i32, i32)]) -> Polygon<i32> {
    Polygon::new(line(exterior), vec![])
}

fn layer(features: Vec<(Geometry<i32>, Vec<i32>)>) -> TileLayer {
    let mut builder = TileLayer::builder("l", 4096).unwrap();
    builder.set_z_axis(ZStep::new(0).unwrap()).unwrap();
    let key = builder.add_m_value("m", PropKind::I32).unwrap();
    for (geometry, z) in features {
        let m = z.iter().map(|z| z * 10).collect();
        let mut feature = builder.feature(geometry);
        feature.z(z).unwrap();
        feature.m_value(key, MValue::I32(Some(m))).unwrap();
        feature.finish().unwrap();
    }
    builder.finish()
}

fn round_trip(layer: &TileLayer, cfg: EncoderConfig) -> TileLayer {
    let bytes = layer.encode(cfg).unwrap();
    let decoded = Parser::default().parse_layers(&bytes).unwrap().remove(0);
    decoded.into_tile(&mut Decoder::default()).unwrap().unwrap()
}

#[test]
fn z_and_m_values_follow_their_vertices() {
    let vertex_sorted = EncoderConfig::from_profile(Profile::Native(Network::Cellular4G))
        .with_wire_version(WireVersion::V02)
        .with_tessellation(Tessellation::Off)
        .with_vertex_sort(true);
    let source = layer(vec![
        (Point::new(90, 10).into(), vec![7]),
        (
            polygon(&[(0, 0), (0, 100), (100, 100), (100, 0), (0, 0)]).into(),
            vec![1, 2, 3, 4],
        ),
        (
            line(&[(500, 500), (250, 250), (1, 1)]).into(),
            vec![5, 6, 8],
        ),
        (
            MultiPoint::from(vec![(30, 30), (2, 2), (20, 20)]).into(),
            vec![9, 11, 12],
        ),
    ]);
    let expected = layer(vec![
        (Point::new(90, 10).into(), vec![7]),
        (
            polygon(&[(100, 0), (100, 100), (0, 100), (0, 0), (100, 0)]).into(),
            vec![4, 3, 2, 1],
        ),
        (
            line(&[(1, 1), (250, 250), (500, 500)]).into(),
            vec![8, 6, 5],
        ),
        (
            MultiPoint::from(vec![(30, 30), (20, 20), (2, 2)]).into(),
            vec![9, 12, 11],
        ),
    ]);
    assert_eq!(round_trip(&source, vertex_sorted), expected);
}
