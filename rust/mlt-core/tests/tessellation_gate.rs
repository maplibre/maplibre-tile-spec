//! `Tessellation::Auto` writes either the plain or the tessellated encoding of a layer.

use std::f64::consts::TAU;

use mlt_core::TileLayer;
use mlt_core::encoder::{EncoderConfig, Network, Tessellation, WireVersion};
use mlt_core::geo_types::{Coord, Geometry, LineString, Polygon};
use rstest::rstest;

fn layer(geoms: Vec<Geometry<i32>>) -> TileLayer {
    let mut builder = TileLayer::builder("gate", 4096).unwrap();
    for geom in geoms {
        builder.feature(geom).finish().unwrap();
    }
    builder.finish()
}

fn squares(count: i32) -> Vec<Geometry<i32>> {
    (0..count)
        .map(|i| {
            let (x, y) = ((i % 32) * 100, (i / 32) * 100);
            let ring = LineString::from(vec![
                (x, y),
                (x + 40, y),
                (x + 40, y + 30),
                (x, y + 30),
                (x, y),
            ]);
            Geometry::Polygon(Polygon::new(ring, vec![]))
        })
        .collect()
}

fn circle(cx: f64, cy: f64, r: f64, vertices: u32, clockwise: bool) -> LineString<i32> {
    let mut coords: Vec<Coord<i32>> = (0..vertices)
        .map(|i| {
            let a = TAU * f64::from(i) / f64::from(vertices);
            let a = if clockwise { -a } else { a };
            #[expect(clippy::cast_possible_truncation, reason = "tile coordinates")]
            Coord {
                x: (cx + r * a.cos()).round() as i32,
                y: (cy + r * a.sin()).round() as i32,
            }
        })
        .collect();
    coords.dedup();
    let mut ring = LineString::new(coords);
    ring.close();
    ring
}

fn lake_with_islands() -> Vec<Geometry<i32>> {
    let holes = (0..4)
        .map(|i| circle(1200.0 + 600.0 * f64::from(i), 2048.0, 200.0, 120, true))
        .collect();
    vec![Geometry::Polygon(Polygon::new(
        circle(2048.0, 2048.0, 1900.0, 600, false),
        holes,
    ))]
}

fn lines() -> Vec<Geometry<i32>> {
    (0..50)
        .map(|i| Geometry::LineString(LineString::from(vec![(i, 0), (i * 3, 400), (i * 7, 900)])))
        .collect()
}

fn cfg(wire: WireVersion, tessellation: Tessellation) -> EncoderConfig {
    EncoderConfig::default()
        .with_wire_version(wire)
        .with_tessellation(tessellation)
}

fn auto(l: &TileLayer, wire: WireVersion, network: Network) -> Vec<u8> {
    l.encode(cfg(wire, Tessellation::Auto).with_network(network))
        .unwrap()
}

fn off(l: &TileLayer, wire: WireVersion) -> Vec<u8> {
    l.encode(cfg(wire, Tessellation::Off)).unwrap()
}

fn always(l: &TileLayer, wire: WireVersion) -> Vec<u8> {
    l.encode(cfg(wire, Tessellation::Always)).unwrap()
}

#[rstest]
fn buildings_store_no_triangles_on_4g(
    #[values(WireVersion::V01, WireVersion::V02)] wire: WireVersion,
) {
    let l = layer(squares(200));
    assert_ne!(off(&l, wire), always(&l, wire));
    assert_eq!(auto(&l, wire, Network::Cellular4G), off(&l, wire));
}

#[rstest]
fn buildings_store_triangles_offline(
    #[values(WireVersion::V01, WireVersion::V02)] wire: WireVersion,
) {
    let l = layer(squares(200));
    assert_eq!(auto(&l, wire, Network::Offline), always(&l, wire));
}

#[test]
fn a_lake_with_islands_stores_triangles_on_4g_in_v2() {
    let l = layer(lake_with_islands());
    assert_eq!(
        auto(&l, WireVersion::V02, Network::Cellular4G),
        always(&l, WireVersion::V02)
    );
}

#[rstest]
fn a_lake_with_islands_stores_no_triangles_on_3g(
    #[values(WireVersion::V01, WireVersion::V02)] wire: WireVersion,
) {
    let l = layer(lake_with_islands());
    assert_eq!(auto(&l, wire, Network::Cellular3G), off(&l, wire));
}

#[rstest]
fn a_layer_without_polygons_encodes_as_without_tessellation(
    #[values(WireVersion::V01, WireVersion::V02)] wire: WireVersion,
) {
    let l = layer(lines());
    assert_eq!(auto(&l, wire, Network::Offline), off(&l, wire));
}

#[rstest]
fn sort_trials_keep_the_choice_of_the_source_order(
    #[values((Network::Cellular3G, Tessellation::Off), (Network::Offline, Tessellation::Always))]
    case: (Network, Tessellation),
) {
    let (network, expected) = case;
    let l = layer(lake_with_islands().into_iter().chain(squares(50)).collect());
    let sorted = |t| {
        l.encode(
            cfg(WireVersion::V02, t)
                .with_network(network)
                .with_spatial_hilbert_sort(true)
                .with_id_sort(true),
        )
        .unwrap()
    };
    assert_eq!(sorted(Tessellation::Auto), sorted(expected));
}
