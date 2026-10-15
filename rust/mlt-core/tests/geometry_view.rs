//! Contracts of the borrowed geo-traits view over a decoded layer's geometry
//! (`.tmp/plan/20261015_geo_traits_view_spec.md`, §4, §5, §7).

use std::fs;
use std::path::Path;

use mlt_core::encoder::{EncoderConfig, Network, Profile, Tessellation, WireVersion};
use mlt_core::geo_traits::to_geo::ToGeoGeometry as _;
use mlt_core::geo_traits::{
    CoordTrait, Dimensions, GeometryTrait, GeometryType as GeoKind, LineStringTrait,
    MultiLineStringTrait, MultiPointTrait, MultiPolygonTrait, PointTrait, PolygonTrait,
};
use mlt_core::geo_types::{
    Coord, Geometry, LineString, MultiLineString, MultiPoint, MultiPolygon, Point, Polygon,
};
use mlt_core::test_helpers::{dec, parser};
use mlt_core::{
    GeometryType, GeometryValues, GeometryView, LendingIterator, MltError, ParsedLayer,
    ParsedLayer01, TileFeature, TileLayer, ZStep,
};
use test_each_file::test_each_path;

// ───────────────────────────── helpers ─────────────────────────────

fn parsed_layers(bytes: &[u8]) -> Vec<ParsedLayer01<'_>> {
    let layers = parser().parse_layers(bytes).expect("parse");
    dec()
        .decode_all(layers)
        .expect("decode")
        .into_iter()
        .filter_map(|layer| match layer {
            ParsedLayer::Tag01(layer) => Some(layer),
            ParsedLayer::Tag02(layer) => Some(layer.into_layer()),
            ParsedLayer::Unknown(_) | _ => None,
        })
        .collect()
}

fn encode_and_parse(layer: &TileLayer, cfg: EncoderConfig) -> Vec<u8> {
    layer.encode(cfg).expect("encode")
}

/// The Native profile without triangles unless a test asks for them, so each test's
/// geometry layout is the one it names.
fn cfg_v1() -> EncoderConfig {
    EncoderConfig::from_profile(Profile::Native(Network::Cellular4G))
        .with_tessellation(Tessellation::Off)
}

fn cfg_v2() -> EncoderConfig {
    cfg_v1().with_wire_version(WireVersion::V02)
}

fn coord(x: i32, y: i32) -> Coord<i32> {
    Coord { x, y }
}

fn ring(pts: &[(i32, i32)]) -> LineString<i32> {
    LineString(pts.iter().map(|&(x, y)| coord(x, y)).collect())
}

fn square(x: i32, y: i32) -> Polygon<i32> {
    Polygon::new(
        ring(&[(x, y), (x + 10, y), (x + 10, y + 10), (x, y + 10), (x, y)]),
        vec![],
    )
}

fn polygon_empty() -> Geometry<i32> {
    Geometry::Polygon(Polygon::new(LineString(vec![]), vec![]))
}

/// Whether feature `index` is stored as triangles alone, with no outline levels.
fn is_triangles_only(values: &GeometryValues) -> bool {
    values.part_offsets().is_none() && values.index_buffer().is_some()
}

/// What every view of `values` must satisfy against the layer that holds it.
fn assert_view_contracts(values: &GeometryValues) {
    let dim = if values.z_step().is_some() {
        Dimensions::Xyz
    } else {
        Dimensions::Xy
    };
    for index in 0..values.feature_count() {
        let view = values.geometry(index).unwrap();
        assert_eq!(view.index(), index);
        assert_eq!(view.geometry_type(), values.vector_types()[index]);
        assert_eq!(view.dim(), dim, "feature {index}");
        assert_eq!(
            view.to_geometry(),
            values.to_geojson(index).unwrap(),
            "feature {index} differs from to_geojson"
        );
        let vertices: Vec<_> = view.vertices().collect();
        assert_eq!(view.vertices().len(), vertices.len());
        for v in &vertices {
            assert_eq!(v.dim(), dim);
            assert_eq!(v.z.is_some(), dim == Dimensions::Xyz);
        }
        if is_triangles_only(values) {
            assert_eq!(vertices.len() % 3, 0, "feature {index}");
        } else {
            let range = values.vertex_range(index).unwrap();
            assert_eq!(view.vertex_range(), Some(range.clone()), "feature {index}");
            assert_eq!(vertices.len(), range.len(), "feature {index}");
        }
    }
}

fn assert_fixture(path: &Path) {
    let bytes = fs::read(path).unwrap();
    for layer in parsed_layers(&bytes) {
        let values = layer.geometry_values();
        assert_view_contracts(values);
        // The row iterator hands out the same views.
        let mut features = layer.iter_features();
        let mut index = 0;
        while let Some(feature) = features.next() {
            let feature = feature.unwrap();
            let view = feature.geometry();
            assert_eq!(view.index(), index);
            assert_eq!(view.to_geometry(), values.to_geojson(index).unwrap());
            index += 1;
        }
        assert_eq!(index, values.feature_count());
    }
}

// ───────────────────────────── fixtures ─────────────────────────────

test_each_path! { for ["mlt"] in "../test/expected/0x01" as v1 => every_v1_feature_views_like_to_geojson }
test_each_path! { for ["mlt"] in "../test/expected/0x02" as v2 => every_v2_feature_views_like_to_geojson }

fn every_v1_feature_views_like_to_geojson([mlt]: [&Path; 1]) {
    assert_fixture(mlt);
}

fn every_v2_feature_views_like_to_geojson([mlt]: [&Path; 1]) {
    assert_fixture(mlt);
}

// ───────────────────────────── shapes and levels ─────────────────────────────

fn single(geom: &Geometry<i32>) -> GeometryValues {
    GeometryValues::default().with_geom(geom)
}

#[test]
fn a_ring_is_closed_in_the_view_but_not_in_storage() {
    let values = single(&Geometry::Polygon(square(0, 0)));
    let view = values.geometry(0).unwrap();
    let GeoKind::Polygon(polygon) = view.as_type() else {
        panic!("not a polygon")
    };
    let exterior = polygon.exterior().unwrap();
    assert!(exterior.is_ring());
    assert_eq!(exterior.num_coords(), 5);
    assert_eq!(
        exterior.coord(4).unwrap().x_y(),
        exterior.coord(0).unwrap().x_y()
    );
    assert!(exterior.coord(5).is_none());
    assert_eq!(exterior.vertex_range(), Some(0..4));
    assert_eq!(polygon.num_interiors(), 0);
    assert_eq!(view.vertex_range(), Some(0..4));
    assert_eq!(view.vertices().len(), 4);
    assert_eq!(values.vertices().unwrap().len(), 8);
}

#[test]
fn a_line_string_is_not_closed() {
    let values = single(&Geometry::LineString(ring(&[(0, 0), (5, 5)])));
    let view = values.geometry(0).unwrap();
    let GeoKind::LineString(line) = view.as_type() else {
        panic!("not a line string")
    };
    assert!(!line.is_ring());
    assert_eq!(line.num_coords(), 2);
    assert_eq!(line.vertex_range(), Some(0..2));
    let xs: Vec<i32> = line.coords().map(|c| c.x()).collect();
    assert_eq!(xs, [0, 5]);
}

#[test]
fn an_open_input_ring_views_the_same_as_a_closed_one() {
    let open = Polygon::new(ring(&[(0, 0), (10, 0), (10, 10), (0, 10)]), vec![]);
    let a = single(&Geometry::Polygon(open));
    let b = single(&Geometry::Polygon(square(0, 0)));
    assert_eq!(
        a.geometry(0).unwrap().to_geometry(),
        b.geometry(0).unwrap().to_geometry()
    );
}

#[test]
fn a_single_vertex_ring_closes_to_two_coords() {
    let values = single(&Geometry::Polygon(Polygon::new(ring(&[(3, 3)]), vec![])));
    let view = values.geometry(0).unwrap();
    let GeoKind::Polygon(polygon) = view.as_type() else {
        panic!("not a polygon")
    };
    let exterior = polygon.exterior().unwrap();
    assert_eq!(exterior.num_coords(), 2);
    assert_eq!(exterior.vertex_range().unwrap().len(), 1);
}

#[test]
fn polygon_empty_has_no_exterior() {
    let values = single(&polygon_empty());
    let view = values.geometry(0).unwrap();
    let GeoKind::Polygon(polygon) = view.as_type() else {
        panic!("not a polygon")
    };
    assert!(polygon.exterior().is_none());
    assert_eq!(polygon.num_interiors(), 0);
    assert_eq!(view.vertices().len(), 0);
    assert_eq!(view.to_geometry(), values.to_geojson(0).unwrap());
}

#[test]
fn empty_multis_report_zero_parts() {
    let values = single(&Geometry::MultiPoint(MultiPoint(vec![])));
    let view = values.geometry(0).unwrap();
    let GeoKind::MultiPoint(mp) = view.as_type() else {
        panic!()
    };
    assert_eq!(mp.num_points(), 0);
    assert!(mp.point(0).is_none());

    let values = single(&Geometry::MultiLineString(MultiLineString(vec![])));
    let view = values.geometry(0).unwrap();
    let GeoKind::MultiLineString(mls) = view.as_type() else {
        panic!()
    };
    assert_eq!(mls.num_line_strings(), 0);

    let values = single(&Geometry::MultiPolygon(MultiPolygon(vec![])));
    let view = values.geometry(0).unwrap();
    let GeoKind::MultiPolygon(mpoly) = view.as_type() else {
        panic!()
    };
    assert_eq!(mpoly.num_polygons(), 0);
}

#[test]
fn an_empty_line_string_has_no_coords() {
    let values = single(&Geometry::LineString(LineString(vec![])));
    let view = values.geometry(0).unwrap();
    let GeoKind::LineString(line) = view.as_type() else {
        panic!()
    };
    assert_eq!(line.num_coords(), 0);
    assert_eq!(line.coords().count(), 0);
}

/// Every shape in one layer: lines before polygons, so the encoder moves the part level
/// to the ring level, and points after both, so a point resolves through every level.
fn mixed_geoms() -> Vec<Geometry<i32>> {
    vec![
        Geometry::LineString(ring(&[(0, 0), (1, 1), (2, 0)])),
        Geometry::MultiLineString(MultiLineString(vec![
            ring(&[(5, 5), (6, 6)]),
            ring(&[(7, 7), (8, 8), (9, 9)]),
        ])),
        Geometry::Polygon(Polygon::new(
            ring(&[(0, 0), (20, 0), (20, 20), (0, 20), (0, 0)]),
            vec![ring(&[(5, 5), (5, 10), (10, 10), (5, 5)])],
        )),
        Geometry::MultiPolygon(MultiPolygon(vec![square(30, 30), square(50, 50)])),
        Geometry::Point(Point::new(1, 2)),
        Geometry::MultiPoint(MultiPoint(vec![Point::new(3, 4), Point::new(5, 6)])),
        Geometry::MultiPolygon(MultiPolygon(vec![])),
        polygon_empty(),
    ]
}

/// `geoms` as one v1 layer on the wire. `push_geom` alone leaves the encoder's compact
/// offsets, which only the wire round trip expands into the decoder's per-feature levels.
fn encoded(geoms: &[Geometry<i32>]) -> Vec<u8> {
    let mut builder = TileLayer::builder("mixed", 4096).unwrap();
    for g in geoms {
        builder.feature(g.clone()).finish().unwrap();
    }
    encode_and_parse(&builder.finish(), cfg_v1())
}

#[test]
fn a_mixed_layer_resolves_every_level() {
    let geoms = mixed_geoms();
    let bytes = encoded(&geoms);
    let layers = parsed_layers(&bytes);
    let values = layers[0].geometry_values();
    assert!(values.geometry_offsets().is_some());
    assert!(values.part_offsets().is_some());
    assert!(values.ring_offsets().is_some());
    assert_view_contracts(values);
    for (index, geom) in geoms.iter().enumerate() {
        let view = values.geometry(index).unwrap();
        assert_eq!(view.geometry_type(), GeometryType::try_from(geom).unwrap());
    }
    // Spot checks through the typed sub-views.
    let view = values.geometry(1).unwrap();
    let GeoKind::MultiLineString(mls) = view.as_type() else {
        panic!()
    };
    assert_eq!(mls.num_line_strings(), 2);
    assert_eq!(mls.line_string(1).unwrap().num_coords(), 3);
    let view = values.geometry(2).unwrap();
    let GeoKind::Polygon(polygon) = view.as_type() else {
        panic!()
    };
    assert_eq!(polygon.num_interiors(), 1);
    assert_eq!(polygon.interior(0).unwrap().num_coords(), 4);
    let view = values.geometry(3).unwrap();
    let GeoKind::MultiPolygon(mpoly) = view.as_type() else {
        panic!()
    };
    assert_eq!(mpoly.num_polygons(), 2);
    assert_eq!(
        mpoly
            .polygon(1)
            .unwrap()
            .exterior()
            .unwrap()
            .coord(0)
            .unwrap()
            .x_y(),
        (50, 50)
    );
    let view = values.geometry(4).unwrap();
    let GeoKind::Point(point) = view.as_type() else {
        panic!()
    };
    assert_eq!(point.coord().unwrap().x_y(), (1, 2));
    let view = values.geometry(5).unwrap();
    let GeoKind::MultiPoint(mp) = view.as_type() else {
        panic!()
    };
    assert_eq!(mp.point(1).unwrap().coord().unwrap().x_y(), (5, 6));
}

#[test]
fn views_are_copy_and_outlive_the_iterator_step() {
    let geoms = mixed_geoms();
    let bytes = encoded(&geoms);
    let layers = parsed_layers(&bytes);
    let layer = &layers[0];
    let mut kept: Vec<GeometryView<'_>> = Vec::new();
    let mut features = layer.iter_features();
    while let Some(feature) = features.next() {
        let view = feature.unwrap().geometry();
        let copy = view;
        kept.push(view);
        kept.push(copy);
    }
    drop(features);
    assert_eq!(kept.len(), 2 * geoms.len());
    for (i, view) in kept.iter().enumerate() {
        assert_eq!(view.index(), i / 2);
        assert_eq!(
            view.to_geometry(),
            layer.geometry_values().to_geojson(i / 2).unwrap()
        );
    }
}

#[test]
fn an_index_past_the_last_feature_is_an_error() {
    let values = single(&Geometry::Point(Point::new(0, 0)));
    let err = values.geometry(1).unwrap_err();
    assert!(
        matches!(err, MltError::GeometryIndexOutOfBounds(1)),
        "{err}"
    );
}

// ───────────────────────────── z ─────────────────────────────

fn step() -> ZStep {
    ZStep::new(-1).unwrap()
}

/// A v2 layer on `step` whose z count up by 7 from 1, one per stored vertex.
fn z_layer(geoms: Vec<Geometry<i32>>) -> TileLayer {
    let mut builder = TileLayer::builder("z", 4096).unwrap();
    builder.set_z_step(step()).unwrap();
    let mut next = 0;
    for geom in geoms {
        let n = TileFeature::new(geom.clone()).vertex_count();
        let z = (next..next + n)
            .map(|i| 1 + i32::try_from(i).unwrap() * 7)
            .collect();
        next += n;
        let mut feature = builder.feature(geom);
        feature.z(z).unwrap();
        feature.finish().unwrap();
    }
    builder.finish()
}

fn z_geoms() -> Vec<Geometry<i32>> {
    vec![
        Geometry::Point(Point::new(1, 1)),
        Geometry::LineString(ring(&[(0, 0), (5, 5), (9, 2)])),
        Geometry::Polygon(Polygon::new(
            ring(&[(0, 0), (20, 0), (20, 20), (0, 20), (0, 0)]),
            vec![ring(&[(5, 5), (5, 10), (10, 10), (5, 5)])],
        )),
        Geometry::MultiPolygon(MultiPolygon(vec![square(30, 30), square(50, 50)])),
    ]
}

#[test]
fn a_z_layer_views_every_vertex_with_its_z() {
    let bytes = encode_and_parse(&z_layer(z_geoms()), cfg_v2());
    let layers = parsed_layers(&bytes);
    let values = layers[0].geometry_values();
    assert!(values.z_step().is_some());
    assert_view_contracts(values);
    for index in 0..values.feature_count() {
        let view = values.geometry(index).unwrap();
        let z: Vec<i32> = view.vertices().map(|v| v.z.unwrap()).collect();
        assert_eq!(z, values.z(index).unwrap(), "feature {index}");
        let nth: Vec<i32> = view.vertices().map(|v| v.nth(2).unwrap()).collect();
        assert_eq!(nth, z);
    }
    // The closing coordinate of a ring repeats the first vertex's z.
    let view = values.geometry(2).unwrap();
    let GeoKind::Polygon(polygon) = view.as_type() else {
        panic!()
    };
    let exterior = polygon.exterior().unwrap();
    assert_eq!(exterior.dim(), Dimensions::Xyz);
    assert_eq!(
        exterior.coord(4).unwrap().nth(2),
        exterior.coord(0).unwrap().nth(2)
    );
}

#[test]
fn a_flat_layer_has_no_z() {
    let bytes = encoded(&mixed_geoms());
    let layers = parsed_layers(&bytes);
    let values = layers[0].geometry_values();
    let view = values.geometry(2).unwrap();
    assert_eq!(view.dim(), Dimensions::Xy);
    for v in view.vertices() {
        assert_eq!(v.z, None);
        assert_eq!(v.nth(2), None);
        assert_eq!(v.dim(), Dimensions::Xy);
    }
}

#[test]
fn a_triangles_only_feature_is_a_multipolygon_of_its_triangles() {
    let layer = z_layer(vec![Geometry::Polygon(square(0, 0))]);
    let bytes = encode_and_parse(
        &layer,
        cfg_v2()
            .with_tessellation(Tessellation::Always)
            .with_triangles_only(true),
    );
    let layers = parsed_layers(&bytes);
    let values = layers[0].geometry_values();
    assert!(is_triangles_only(values), "fixture is not triangles-only");
    assert_view_contracts(values);
    let view = values.geometry(0).unwrap();
    let GeoKind::MultiPolygon(mpoly) = view.as_type() else {
        panic!("not a multipolygon")
    };
    assert_eq!(mpoly.num_polygons(), 2);
    for polygon in mpoly.polygons() {
        assert_eq!(polygon.num_interiors(), 0);
        assert_eq!(polygon.exterior().unwrap().num_coords(), 4);
    }
    assert_eq!(view.vertices().len(), 6);
    let z: Vec<i32> = view.vertices().map(|v| v.z.unwrap()).collect();
    assert_eq!(z, values.z(0).unwrap());
}
