use geo_types::{Coord, LineString, Point};
use mlt_core::geojson::Geom32;
use mlt_core::v01::sort::reorder_features;
use mlt_core::v01::tile::TileLayer01;
use mlt_core::v01::{
    GeometryEncoder, GeometryType, IntEncoder, ParsedGeometry, ParsedId, RawGeometry, SortStrategy,
    StagedLayer01,
};

/// Roundtrip a `ParsedGeometry` through encode/decode to produce a properly-indexed geometry.
fn roundtrip_geom(decoded: ParsedGeometry) -> ParsedGeometry {
    use mlt_core::v01::Geometry;
    let encoded = decoded
        .encode(GeometryEncoder::all(IntEncoder::varint()))
        .expect("encode failed");
    let mut buf = Vec::new();
    encoded.write_to(&mut buf).expect("serialize failed");
    let (_, raw) = RawGeometry::parse(&buf).expect("parse failed");
    Geometry::Encoded(raw).decode().expect("decode failed")
}

/// Helper to build a `TileLayer01` from geometries and IDs.
fn build_layer(geoms: &[Geom32], ids: &[Option<u64>]) -> TileLayer01 {
    let mut decoded_geom = ParsedGeometry::default();
    for g in geoms {
        decoded_geom.push_geom(g);
    }
    // Roundtrip through encode/decode to produce properly-indexed offset arrays
    // needed for per-feature access via `to_geojson`.
    let decoded_geom = roundtrip_geom(decoded_geom);

    let staged = StagedLayer01 {
        name: "test".to_string(),
        extent: 4096,
        id: Some(ParsedId(ids.to_vec())),
        geometry: decoded_geom,
        properties: vec![],
    };
    TileLayer01::from(staged)
}

/// Helper to sort a `TileLayer01` and return it for inspection.
fn sort_and_decode(layer: TileLayer01, strategy: SortStrategy) -> TileLayer01 {
    let mut layer = layer;
    reorder_features(&mut layer, Some(strategy));
    layer
}

fn pt(x: i32, y: i32) -> Geom32 {
    Geom32::Point(Point::new(x, y))
}

fn ls(coords: &[(i32, i32)]) -> Geom32 {
    Geom32::LineString(LineString::new(
        coords.iter().map(|&(x, y)| Coord { x, y }).collect(),
    ))
}

/// Rebuild a flat vertex buffer from the feature geometries in source order.
fn vertices_from_source(source: &TileLayer01) -> Vec<i32> {
    let mut geom = ParsedGeometry::default();
    for f in &source.features {
        geom.push_geom(&f.geometry);
    }
    geom.vertices.unwrap_or_default()
}

/// Rebuild the `GeometryType` list from source order.
fn geom_types_from_source(source: &TileLayer01) -> Vec<GeometryType> {
    source
        .features
        .iter()
        .map(|f| match &f.geometry {
            geo_types::Geometry::LineString(_) => GeometryType::LineString,
            geo_types::Geometry::Polygon(_) => GeometryType::Polygon,
            geo_types::Geometry::MultiPoint(_) => GeometryType::MultiPoint,
            geo_types::Geometry::MultiLineString(_) => GeometryType::MultiLineString,
            geo_types::Geometry::MultiPolygon(_) => GeometryType::MultiPolygon,
            _ => GeometryType::Point, // fallback
        })
        .collect()
}

#[test]
fn test_shared_morton_shift() {
    // P1 at (0, -10), P2 at (-10, 0).
    // With shared shift = 10:
    // P1 shifted: (10, 0) -> interleave(10, 0) = 68
    // P2 shifted: (0, 10) -> interleave(0, 10) = 136
    // P1 (key 68) < P2 (key 136), so expected order: [P1(0,-10), P2(-10,0)].

    let layer = build_layer(&[pt(0, -10), pt(-10, 0)], &[Some(1), Some(2)]);
    let source = sort_and_decode(layer, SortStrategy::SpatialMorton);

    let verts = vertices_from_source(&source);
    assert_eq!(verts, vec![0, -10, -10, 0]);
}

#[test]
fn test_id_sort_nulls_first() {
    let layer = build_layer(&[pt(2, 2), pt(1, 1), pt(0, 0)], &[Some(10), None, Some(5)]);
    let source = sort_and_decode(layer, SortStrategy::Id);

    let ids: Vec<Option<u64>> = source.features.iter().map(|f| f.id).collect();
    // Expected order: [None, Some(5), Some(10)]
    assert_eq!(ids, vec![None, Some(5), Some(10)]);

    let verts = vertices_from_source(&source);
    // Corresponding verts: [pt(1,1), pt(0,0), pt(2,2)] -> [1,1, 0,0, 2,2]
    assert_eq!(verts, vec![1, 1, 0, 0, 2, 2]);
}

#[test]
fn test_mixed_geometry_morton_sort() {
    // [Point(2,0), LineString(0,0 -> 0,5), Point(1,0)]
    // Morton keys (assuming shift 0):
    // P1(2,0) -> 4
    // LS(0,0) -> 0
    // P2(1,0) -> 1
    // Expected order: [LS, P2, P1]

    let layer = build_layer(
        &[pt(2, 0), ls(&[(0, 0), (0, 5)]), pt(1, 0)],
        &[Some(1), Some(2), Some(3)],
    );
    let source = sort_and_decode(layer, SortStrategy::SpatialMorton);

    let types = geom_types_from_source(&source);
    assert_eq!(
        types,
        vec![
            GeometryType::LineString,
            GeometryType::Point,
            GeometryType::Point
        ]
    );

    let verts = vertices_from_source(&source);
    // Expected vertices: LS(0,0,0,5), P2(1,0), P1(2,0)
    assert_eq!(verts, vec![0, 0, 0, 5, 1, 0, 2, 0]);
}
