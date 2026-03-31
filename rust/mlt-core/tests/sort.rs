use geo_types::{Coord, LineString, Point};
use mlt_core::geojson::Geom32;
use mlt_core::test_helpers::{assert_empty, dec, into_layer01, parser};
use mlt_core::v01::{
    GeometryEncoder, GeometryType, GeometryValues, IntEncoder, SortStrategy, StagedLayer01Encoder,
    Tile01Encoder, TileFeature, TileLayer01,
};
use mlt_core::{EncodedLayer, Layer};

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
    let tile_encoder = Tile01Encoder {
        sort_strategy: strategy,
        ..Default::default()
    };
    let staged = tile_encoder.encode(&mut tile);
    let stream_encoder = StagedLayer01Encoder {
        properties: vec![],
        geometry: GeometryEncoder::all(IntEncoder::varint()),
        ..Default::default()
    };
    let layer_enc = staged.encode(stream_encoder).expect("encode failed");

    // Serialize to bytes and reparse to get a `Layer01`.
    let mut buf = Vec::new();
    EncodedLayer::Tag01(layer_enc)
        .write_to(&mut buf)
        .expect("write_to failed");

    let mut p = parser();
    let layer_back = assert_empty(Layer::from_bytes(&buf, &mut p));
    assert!(p.reserved() > 0, "parser should reserve bytes after parse");

    let layer01 = into_layer01(layer_back);

    let mut d = dec();
    let tile = layer01.into_tile(&mut d).expect("decode after sort failed");
    assert!(
        d.consumed() > 0,
        "decoder should consume bytes after decode"
    );
    tile
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
    geom.vertices().unwrap_or_default().to_vec()
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

    let types: Vec<_> = source
        .features
        .iter()
        .map(|f| GeometryType::try_from(&f.geometry).unwrap())
        .collect();

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
