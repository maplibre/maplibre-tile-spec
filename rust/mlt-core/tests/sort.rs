use geo_types::{Coord, LineString, Point};
use mlt_core::Decoder;
use mlt_core::geojson::Geom32;
use mlt_core::v01::{
    GeometryEncoder, GeometryType, GeometryValues, IntEncoder, SortStrategy, StagedLayer01Encoder,
    Tile01Encoder, TileFeature, TileLayer01,
};

/// Build row-oriented tile layer from geometries and IDs (one feature per geometry).
fn build_tile_layer(geoms: &[Geom32], ids: &[Option<u64>]) -> TileLayer01 {
    assert_eq!(geoms.len(), ids.len());
    TileLayer01 {
        name: "test".to_string(),
        extent: 4096,
        property_names: vec![],
        features: geoms
            .iter()
            .zip(ids.iter())
            .map(|(g, &id)| TileFeature {
                id,
                geometry: g.clone(),
                properties: vec![],
            })
            .collect(),
    }
}

/// Encode the layer with a given sort strategy, decode it back, and return the `TileLayer01`.
/// This tests the full encode→decode roundtrip, verifying that sorting was applied.
fn sort_encode_decode(mut tile: TileLayer01, strategy: SortStrategy) -> TileLayer01 {
    use mlt_core::{EncodedLayer, Layer};

    let tile_encoder = Tile01Encoder {
        sort_strategy: Some(strategy),
    };
    let staged = tile_encoder.encode(&mut tile);
    let stream_encoder = StagedLayer01Encoder {
        id: None, // auto-encode IDs
        properties: vec![],
        geometry: GeometryEncoder::all(IntEncoder::varint()),
    };
    let layer_enc = staged.encode(stream_encoder).expect("encode failed");

    // Serialize to bytes and reparse to get a `Layer01`.
    let mut buf = Vec::new();
    EncodedLayer::Tag01(layer_enc)
        .write_to(&mut buf)
        .expect("write_to failed");

    let (remaining, layer_back) = Layer::from_bytes(&buf).expect("parse failed");
    assert!(remaining.is_empty());

    let Layer::Tag01(layer01) = layer_back else {
        panic!("expected Tag01 layer");
    };

    layer01
        .into_tile(&mut Decoder::default())
        .expect("decode after sort failed")
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
    let mut geom = GeometryValues::default();
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

    let tile = build_tile_layer(&[pt(0, -10), pt(-10, 0)], &[Some(1), Some(2)]);
    let source = sort_encode_decode(tile, SortStrategy::SpatialMorton);

    let verts = vertices_from_source(&source);
    assert_eq!(verts, vec![0, -10, -10, 0]);
}

#[test]
fn test_id_sort_nulls_first() {
    let tile = build_tile_layer(&[pt(2, 2), pt(1, 1), pt(0, 0)], &[Some(10), None, Some(5)]);
    let source = sort_encode_decode(tile, SortStrategy::Id);

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

    let tile = build_tile_layer(
        &[pt(2, 0), ls(&[(0, 0), (0, 5)]), pt(1, 0)],
        &[Some(1), Some(2), Some(3)],
    );
    let source = sort_encode_decode(tile, SortStrategy::SpatialMorton);

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
