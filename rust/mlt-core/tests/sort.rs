use geo_types::{Coord, LineString, Point, Polygon};
use mlt_core::geojson::Geom32;
use mlt_core::optimizer::ManualOptimisation;
use mlt_core::v01::{
    DecodedGeometry, DecodedId, GeometryEncoder, IntEncoder, OwnedGeometry, OwnedId, OwnedLayer01,
    SortStrategy, Tag01Encoder,
};

/// Helper to build a layer from geometries and IDs.
fn build_layer(geoms: &[Geom32], ids: &[Option<u64>]) -> OwnedLayer01 {
    let mut decoded_geom = DecodedGeometry::default();
    for g in geoms {
        decoded_geom.push_geom(g);
    }

    OwnedLayer01 {
        name: "test".to_string(),
        extent: 4096,
        id: OwnedId::Decoded(Some(DecodedId(ids.to_vec()))),
        geometry: OwnedGeometry::Decoded(decoded_geom),
        properties: vec![],
    }
}

/// Helper to sort a layer manually.
fn sort_layer(layer: &mut OwnedLayer01, strategy: SortStrategy) {
    let encoder = Tag01Encoder {
        sort_strategy: Some(strategy),
        allow_id_regeneration: false,
        id: None,
        properties: vec![],
        geometry: GeometryEncoder::all(IntEncoder::varint()),
    };
    layer.manual_optimisation(encoder).expect("sort failed");
}

fn pt(x: i32, y: i32) -> Geom32 {
    Geom32::Point(Point::new(x, y))
}

fn ls(coords: &[(i32, i32)]) -> Geom32 {
    Geom32::LineString(LineString::new(
        coords.iter().map(|&(x, y)| Coord { x, y }).collect(),
    ))
}

#[test]
fn test_shared_morton_shift() {
    // P1 at (-10, 0), P2 at (0, -10).
    // If shifts were independent:
    // P1: sx = -10 + 10 = 0, sy = 0 + 0 = 0 -> key 0
    // P2: sx = 0 + 0 = 0, sy = -10 + 10 = 0 -> key 0 (COLLISION)
    //
    // If shift is shared (global min = -10):
    // P1: sx = -10 + 10 = 0, sy = 0 + 10 = 10 -> key interleave(0, 10)
    // P2: sx = 0 + 10 = 10, sy = -10 + 10 = 0 -> key interleave(10, 0)
    // No collision. P1 should come before P2 in interleave(0,10) < interleave(10,0).

    let mut layer = build_layer(&[pt(0, -10), pt(-10, 0)], &[Some(1), Some(2)]);
    sort_layer(&mut layer, SortStrategy::SpatialMorton);

    let types = match &layer.geometry {
        OwnedGeometry::Decoded(g) => g.vertices.as_ref().unwrap().clone(),
        _ => panic!("expected decoded geometry"),
    };
    // Expected order: P2 (-10, 0), then P1 (0, -10)
    // Because interleave(-10+10, 0+10) = interleave(0, 10) = 2
    //         interleave(0+10, -10+10) = interleave(10, 0) = 1
    // Wait, let's check interleave bits:
    // interleave(0, 10) -> x=0 (0000), y=10 (1010) -> 01000100 (in bits)
    // interleave(10, 0) -> x=10 (1010), y=0 (0000) -> 00010001 (in bits)
    // Actually 1010 is 10.
    // interleave(0, 10):
    // y bit 0 (0) -> bit 1 of code = 0
    // y bit 1 (1) -> bit 3 of code = 1
    // y bit 2 (0) -> bit 5 of code = 0
    // y bit 3 (1) -> bit 7 of code = 1
    // x bits all 0.
    // Result: (1 << 7) | (1 << 3) = 128 + 8 = 136.
    // interleave(10, 0):
    // x bit 0 (0) -> bit 0 of code = 0
    // x bit 1 (1) -> bit 2 of code = 1
    // x bit 2 (0) -> bit 4 of code = 0
    // x bit 3 (1) -> bit 6 of code = 1
    // y bits all 0.
    // Result: (1 << 6) | (1 << 2) = 64 + 4 = 68.
    //
    // So P2 (10, 0) [shifted] has key 68, P1 (0, 10) [shifted] has key 136.
    // P2 should come first.
    // P2 is (-10, 0) in raw coords. P1 is (0, -10).
    // So expected order: [(-10, 0), (0, -10)].
    assert_eq!(types, vec![-10, 0, 0, -10]);
}

#[test]
fn test_id_sort_nulls_first() {
    let mut layer = build_layer(&[pt(2, 2), pt(1, 1), pt(0, 0)], &[Some(10), None, Some(5)]);
    sort_layer(&mut layer, SortStrategy::Id);

    let ids = match &layer.id {
        OwnedId::Decoded(Some(d)) => d.0.clone(),
        _ => panic!("expected decoded IDs"),
    };
    // Expected order: [None, Some(5), Some(10)]
    assert_eq!(ids, vec![None, Some(5), Some(10)]);

    let verts = match &layer.geometry {
        OwnedGeometry::Decoded(g) => g.vertices.as_ref().unwrap().clone(),
        _ => panic!("expected decoded geometry"),
    };
    // Corresponding verts: [pt(1,1), pt(0,0), pt(2,2)]
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

    let mut layer = build_layer(
        &[pt(2, 0), ls(&[(0, 0), (0, 5)]), pt(1, 0)],
        &[Some(1), Some(2), Some(3)],
    );
    sort_layer(&mut layer, SortStrategy::SpatialMorton);

    let types = match &layer.geometry {
        OwnedGeometry::Decoded(g) => g.vector_types.clone(),
        _ => panic!("expected decoded geometry"),
    };
    use mlt_core::v01::GeometryType;
    assert_eq!(
        types,
        vec![
            GeometryType::LineString,
            GeometryType::Point,
            GeometryType::Point
        ]
    );

    let verts = match &layer.geometry {
        OwnedGeometry::Decoded(g) => g.vertices.as_ref().unwrap().clone(),
        _ => panic!("expected decoded geometry"),
    };
    // Expected vertices: LS(0,0,0,5), P2(1,0), P1(2,0)
    assert_eq!(verts, vec![0, 0, 0, 5, 1, 0, 2, 0]);
}
