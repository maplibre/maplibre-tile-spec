use std::f64::consts::TAU;

use mlt_core::TileLayer;
use mlt_core::encoder::{EncoderConfig, Network, Profile, Tessellation, WireVersion};
use mlt_core::geo_types::{
    Coord, Geometry, LineString, MultiLineString, MultiPoint, MultiPolygon, Point, Polygon,
};
use rstest::rstest;

fn layer(name: &str, geoms: impl IntoIterator<Item = Geometry<i32>>) -> TileLayer {
    let mut builder = TileLayer::builder(name, 4096).unwrap();
    for geom in geoms {
        builder.feature(geom).finish().unwrap();
    }
    builder.finish()
}

fn building(i: i32) -> Polygon<i32> {
    let (x, y) = ((i % 32) * 100, (i / 32) * 100);
    let ring = LineString::from(vec![
        (x, y),
        (x + 40, y),
        (x + 40, y + 30),
        (x, y + 30),
        (x, y),
    ]);
    Polygon::new(ring, vec![])
}

fn buildings(count: i32) -> Vec<Geometry<i32>> {
    (0..count).map(|i| Geometry::Polygon(building(i))).collect()
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

fn lake_with_islands() -> Geometry<i32> {
    let islands = (0..4)
        .map(|i| circle(1200.0 + 600.0 * f64::from(i), 2048.0, 200.0, 120, true))
        .collect();
    let shore = circle(2048.0, 2048.0, 1900.0, 600, false);
    Geometry::Polygon(Polygon::new(shore, islands))
}

fn roads() -> Vec<Geometry<i32>> {
    (0..50)
        .map(|i| Geometry::LineString(LineString::from(vec![(i, 0), (i * 3, 400), (i * 7, 900)])))
        .collect()
}

fn one_of_each_geometry_type() -> Vec<Geometry<i32>> {
    vec![
        Geometry::Point(Point::new(1, 2)),
        Geometry::MultiPoint(MultiPoint::from(vec![(1, 1), (2, 2)])),
        Geometry::LineString(LineString::from(vec![(0, 0), (5, 5), (9, 0)])),
        Geometry::MultiLineString(MultiLineString(vec![
            LineString::from(vec![(0, 0), (1, 1)]),
            LineString::from(vec![(2, 2), (3, 9)]),
        ])),
        Geometry::Polygon(building(0)),
        Geometry::MultiPolygon(MultiPolygon(vec![building(1), building(2)])),
    ]
}

#[rstest]
fn tessellating_buildings_changes_their_encoding(
    #[values(WireVersion::V01, WireVersion::V02)] wire: WireVersion,
) {
    let buildings = layer("buildings", buildings(200));
    let base =
        EncoderConfig::from_preset(Profile::Native, Network::Cellular4G).with_wire_version(wire);

    let without_triangles = base.with_tessellation(Tessellation::Off);
    let with_triangles = base.with_tessellation(Tessellation::Always);

    assert_ne!(
        buildings.encode(without_triangles).unwrap(),
        buildings.encode(with_triangles).unwrap()
    );
}

#[rstest]
fn buildings_store_no_triangles_on_4g(
    #[values(WireVersion::V01, WireVersion::V02)] wire: WireVersion,
) {
    let buildings = layer("buildings", buildings(200));
    let base =
        EncoderConfig::from_preset(Profile::Native, Network::Cellular4G).with_wire_version(wire);

    let auto_on_4g = EncoderConfig::from_preset(Profile::Native, Network::Cellular4G)
        .with_wire_version(wire)
        .with_tessellation(Tessellation::Auto);
    let without_triangles = base.with_tessellation(Tessellation::Off);

    assert_eq!(
        buildings.encode(auto_on_4g).unwrap(),
        buildings.encode(without_triangles).unwrap()
    );
}

#[rstest]
fn buildings_store_triangles_offline(
    #[values(WireVersion::V01, WireVersion::V02)] wire: WireVersion,
) {
    let buildings = layer("buildings", buildings(200));
    let base =
        EncoderConfig::from_preset(Profile::Native, Network::Cellular4G).with_wire_version(wire);

    let auto_offline = EncoderConfig::from_preset(Profile::Native, Network::Offline)
        .with_wire_version(wire)
        .with_tessellation(Tessellation::Auto);
    let with_triangles = base.with_tessellation(Tessellation::Always);

    assert_eq!(
        buildings.encode(auto_offline).unwrap(),
        buildings.encode(with_triangles).unwrap()
    );
}

#[test]
fn a_lake_with_islands_stores_triangles_on_4g_in_v2() {
    let lake = layer("water", [lake_with_islands()]);
    let base = EncoderConfig::from_preset(Profile::Native, Network::Cellular4G)
        .with_wire_version(WireVersion::V02);

    let auto_on_4g = EncoderConfig::from_preset(Profile::Native, Network::Cellular4G)
        .with_wire_version(WireVersion::V02)
        .with_tessellation(Tessellation::Auto);
    let with_triangles = base.with_tessellation(Tessellation::Always);

    assert_eq!(
        lake.encode(auto_on_4g).unwrap(),
        lake.encode(with_triangles).unwrap()
    );
}

#[rstest]
fn a_lake_with_islands_stores_no_triangles_on_3g(
    #[values(WireVersion::V01, WireVersion::V02)] wire: WireVersion,
) {
    let lake = layer("water", [lake_with_islands()]);
    let base =
        EncoderConfig::from_preset(Profile::Native, Network::Cellular4G).with_wire_version(wire);

    let auto_on_3g = EncoderConfig::from_preset(Profile::Native, Network::Cellular3G)
        .with_wire_version(wire)
        .with_tessellation(Tessellation::Auto);
    let without_triangles = base.with_tessellation(Tessellation::Off);

    assert_eq!(
        lake.encode(auto_on_3g).unwrap(),
        lake.encode(without_triangles).unwrap()
    );
}

#[test]
fn a_lake_among_every_geometry_type_stores_triangles_on_4g_in_v2() {
    let mixed = layer(
        "mixed",
        one_of_each_geometry_type()
            .into_iter()
            .chain([lake_with_islands()])
            .chain(one_of_each_geometry_type()),
    );
    let base = EncoderConfig::from_preset(Profile::Native, Network::Cellular4G)
        .with_wire_version(WireVersion::V02);

    let auto_on_4g = EncoderConfig::from_preset(Profile::Native, Network::Cellular4G)
        .with_wire_version(WireVersion::V02)
        .with_tessellation(Tessellation::Auto);
    let with_triangles = base.with_tessellation(Tessellation::Always);

    assert_eq!(
        mixed.encode(auto_on_4g).unwrap(),
        mixed.encode(with_triangles).unwrap()
    );
}

#[test]
fn a_lake_among_every_geometry_type_stores_no_triangles_on_3g() {
    let mixed = layer(
        "mixed",
        one_of_each_geometry_type()
            .into_iter()
            .chain([lake_with_islands()])
            .chain(one_of_each_geometry_type()),
    );
    let base = EncoderConfig::from_preset(Profile::Native, Network::Cellular4G)
        .with_wire_version(WireVersion::V02);

    let auto_on_3g = EncoderConfig::from_preset(Profile::Native, Network::Cellular3G)
        .with_wire_version(WireVersion::V02)
        .with_tessellation(Tessellation::Auto);
    let without_triangles = base.with_tessellation(Tessellation::Off);

    assert_eq!(
        mixed.encode(auto_on_3g).unwrap(),
        mixed.encode(without_triangles).unwrap()
    );
}

#[rstest]
fn roads_store_no_triangles_even_offline(
    #[values(WireVersion::V01, WireVersion::V02)] wire: WireVersion,
) {
    let roads = layer("roads", roads());
    let base =
        EncoderConfig::from_preset(Profile::Native, Network::Cellular4G).with_wire_version(wire);

    let auto_offline = EncoderConfig::from_preset(Profile::Native, Network::Offline)
        .with_wire_version(wire)
        .with_tessellation(Tessellation::Auto);
    let without_triangles = base.with_tessellation(Tessellation::Off);

    assert_eq!(
        roads.encode(auto_offline).unwrap(),
        roads.encode(without_triangles).unwrap()
    );
}

#[test]
fn sorted_trials_store_no_triangles_when_the_source_order_on_3g_does_not() {
    let lake_and_buildings = layer(
        "landcover",
        [lake_with_islands()].into_iter().chain(buildings(50)),
    );
    let sorted = EncoderConfig::from_preset(Profile::Native, Network::Cellular3G)
        .with_wire_version(WireVersion::V02)
        .with_spatial_hilbert_sort(true)
        .with_id_sort(true);

    let auto = sorted.with_tessellation(Tessellation::Auto);
    let without_triangles = sorted.with_tessellation(Tessellation::Off);

    assert_eq!(
        lake_and_buildings.encode(auto).unwrap(),
        lake_and_buildings.encode(without_triangles).unwrap()
    );
}

#[test]
fn sorted_trials_store_triangles_when_the_source_order_offline_does() {
    let lake_and_buildings = layer(
        "landcover",
        [lake_with_islands()].into_iter().chain(buildings(50)),
    );
    let sorted = EncoderConfig::from_preset(Profile::Native, Network::Offline)
        .with_wire_version(WireVersion::V02)
        .with_spatial_hilbert_sort(true)
        .with_id_sort(true);

    let auto = sorted.with_tessellation(Tessellation::Auto);
    let with_triangles = sorted.with_tessellation(Tessellation::Always);

    assert_eq!(
        lake_and_buildings.encode(auto).unwrap(),
        lake_and_buildings.encode(with_triangles).unwrap()
    );
}

#[rstest]
fn the_network_changes_nothing_without_tessellation(
    #[values(Network::Offline, Network::Cellular3G, Network::Cellular5G)] network: Network,
) {
    let lake_and_buildings = layer(
        "landcover",
        [lake_with_islands()].into_iter().chain(buildings(50)),
    );
    let on_network = EncoderConfig::from_preset(Profile::Native, network)
        .with_wire_version(WireVersion::V02)
        .with_tessellation(Tessellation::Off)
        .with_spatial_hilbert_sort(true)
        .with_id_sort(true);
    let on_4g = EncoderConfig::from_preset(Profile::Native, Network::Cellular4G)
        .with_wire_version(WireVersion::V02)
        .with_tessellation(Tessellation::Off)
        .with_spatial_hilbert_sort(true)
        .with_id_sort(true);

    assert_eq!(
        lake_and_buildings.encode(on_network).unwrap(),
        lake_and_buildings.encode(on_4g).unwrap()
    );
}
