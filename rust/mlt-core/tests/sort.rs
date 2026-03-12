use std::collections::HashMap;

use insta::assert_debug_snapshot;
use mlt_core::frames::LayerEncoder;
use mlt_core::optimizer::{
    AutomaticOptimisation as _, ManualOptimisation as _, ProfileOptimisation as _,
};
use mlt_core::utils::{hilbert_curve_params, hilbert_sort_key, morton_sort_key};
use mlt_core::v01::{
    DecodedGeometry, DecodedId, DecodedProperty, GeometryProfile, GeometryType, IdProfile,
    IntEncoder, OwnedGeometry, OwnedId, OwnedLayer01, OwnedProperty, PropertyProfile, SortStrategy,
    SpaceFillingCurve, Tag01Profile, reorder_features,
};
use rstest::*;

// ── Non-Point geometry builders ──────────────────────────────────────────────

fn linestrings_layer(lines: &[&[(i32, i32)]]) -> OwnedLayer01 {
    let mut part_offsets = vec![0u32];
    let mut vertices = Vec::new();
    for &line in lines {
        for &(x, y) in line {
            vertices.push(x);
            vertices.push(y);
        }
        part_offsets.push(*part_offsets.last().unwrap() + line.len() as u32);
    }
    OwnedLayer01 {
        name: String::new(),
        extent: 4096,
        geometry: OwnedGeometry::Decoded(DecodedGeometry {
            vector_types: vec![GeometryType::LineString; lines.len()],
            part_offsets: Some(part_offsets),
            vertices: Some(vertices),
            ..Default::default()
        }),
        id: OwnedId::Decoded(Some(DecodedId((0..lines.len() as u64).map(Some).collect()))),
        properties: vec![],
    }
}

fn polygons_layer(polygons: &[&[&[(i32, i32)]]]) -> OwnedLayer01 {
    let mut part_offsets = vec![0u32];
    let mut ring_offsets = vec![0u32];
    let mut vertices = Vec::new();
    for &poly in polygons {
        part_offsets.push(*part_offsets.last().unwrap() + poly.len() as u32);
        for &ring in poly {
            for &(x, y) in ring {
                vertices.push(x);
                vertices.push(y);
            }
            ring_offsets.push(*ring_offsets.last().unwrap() + ring.len() as u32);
        }
    }
    OwnedLayer01 {
        name: String::new(),
        extent: 4096,
        geometry: OwnedGeometry::Decoded(DecodedGeometry {
            vector_types: vec![GeometryType::Polygon; polygons.len()],
            part_offsets: Some(part_offsets),
            ring_offsets: Some(ring_offsets),
            vertices: Some(vertices),
            ..Default::default()
        }),
        id: OwnedId::Decoded(Some(DecodedId(
            (0..polygons.len() as u64).map(Some).collect(),
        ))),
        properties: vec![],
    }
}

fn multipoints_layer(features: &[&[(i32, i32)]]) -> OwnedLayer01 {
    let mut geom_offsets = vec![0u32];
    let mut vertices = Vec::new();
    for &pts in features {
        for &(x, y) in pts {
            vertices.push(x);
            vertices.push(y);
        }
        geom_offsets.push(*geom_offsets.last().unwrap() + pts.len() as u32);
    }
    OwnedLayer01 {
        name: String::new(),
        extent: 4096,
        geometry: OwnedGeometry::Decoded(DecodedGeometry {
            vector_types: vec![GeometryType::MultiPoint; features.len()],
            geometry_offsets: Some(geom_offsets),
            vertices: Some(vertices),
            ..Default::default()
        }),
        id: OwnedId::Decoded(Some(DecodedId(
            (0..features.len() as u64).map(Some).collect(),
        ))),
        properties: vec![],
    }
}

fn multilinestrings_layer(features: &[&[&[(i32, i32)]]]) -> OwnedLayer01 {
    let mut geom_offsets = vec![0u32];
    let mut part_offsets = vec![0u32];
    let mut vertices = Vec::new();
    for &lines in features {
        geom_offsets.push(*geom_offsets.last().unwrap() + lines.len() as u32);
        for &line in lines {
            for &(x, y) in line {
                vertices.push(x);
                vertices.push(y);
            }
            part_offsets.push(*part_offsets.last().unwrap() + line.len() as u32);
        }
    }
    OwnedLayer01 {
        name: String::new(),
        extent: 4096,
        geometry: OwnedGeometry::Decoded(DecodedGeometry {
            vector_types: vec![GeometryType::MultiLineString; features.len()],
            geometry_offsets: Some(geom_offsets),
            part_offsets: Some(part_offsets),
            vertices: Some(vertices),
            ..Default::default()
        }),
        id: OwnedId::Decoded(Some(DecodedId(
            (0..features.len() as u64).map(Some).collect(),
        ))),
        properties: vec![],
    }
}

#[allow(clippy::type_complexity)]
fn multipolygons_layer(features: &[&[&[&[(i32, i32)]]]]) -> OwnedLayer01 {
    let mut geom_offsets = vec![0u32];
    let mut part_offsets = vec![0u32];
    let mut ring_offsets = vec![0u32];
    let mut vertices = Vec::new();
    for &polys in features {
        geom_offsets.push(*geom_offsets.last().unwrap() + polys.len() as u32);
        for &rings in polys {
            part_offsets.push(*part_offsets.last().unwrap() + rings.len() as u32);
            for &ring in rings {
                for &(x, y) in ring {
                    vertices.push(x);
                    vertices.push(y);
                }
                ring_offsets.push(*ring_offsets.last().unwrap() + ring.len() as u32);
            }
        }
    }
    OwnedLayer01 {
        name: String::new(),
        extent: 4096,
        geometry: OwnedGeometry::Decoded(DecodedGeometry {
            vector_types: vec![GeometryType::MultiPolygon; features.len()],
            geometry_offsets: Some(geom_offsets),
            part_offsets: Some(part_offsets),
            ring_offsets: Some(ring_offsets),
            vertices: Some(vertices),
            ..Default::default()
        }),
        id: OwnedId::Decoded(Some(DecodedId(
            (0..features.len() as u64).map(Some).collect(),
        ))),
        properties: vec![],
    }
}

fn get_geom(layer: &OwnedLayer01) -> &DecodedGeometry {
    let OwnedGeometry::Decoded(g) = &layer.geometry else {
        panic!("not decoded")
    };
    g
}

#[fixture]
fn points_geom_builder() -> impl Fn(&[(i32, i32)]) -> DecodedGeometry {
    |coords| DecodedGeometry {
        vector_types: vec![GeometryType::Point; coords.len()],
        vertices: Some(coords.iter().flat_map(|&(x, y)| [x, y]).collect()),
        ..Default::default()
    }
}

/// Canonical test layer: 4 points, IDs, and "class" strings.
#[fixture]
fn four_point_layer(
    points_geom_builder: impl Fn(&[(i32, i32)]) -> DecodedGeometry,
) -> OwnedLayer01 {
    OwnedLayer01 {
        name: String::new(),
        extent: 4096,
        geometry: OwnedGeometry::Decoded(points_geom_builder(&[(3, 1), (0, 0), (2, 3), (1, 2)])),
        id: OwnedId::Decoded(Some(DecodedId(vec![
            Some(40),
            Some(10),
            Some(30),
            Some(20),
        ]))),
        properties: vec![OwnedProperty::Decoded(DecodedProperty::str(
            "class",
            vec![
                Some("delta".into()),
                Some("alpha".into()),
                Some("gamma".into()),
                Some("beta".into()),
            ],
        ))],
    }
}

/// Larger layer with negative coordinates.
#[fixture]
fn negative_coords_layer(
    points_geom_builder: impl Fn(&[(i32, i32)]) -> DecodedGeometry,
) -> OwnedLayer01 {
    let coords = [
        (-3, -3),
        (-1, -1),
        (0, 0),
        (1, 1),
        (-2, 2),
        (2, -2),
        (3, 3),
        (-3, 3),
    ];
    let ids = (1u64..=8).rev().map(|v| Some(v * 10)).collect();
    OwnedLayer01 {
        name: String::new(),
        extent: 4096,
        geometry: OwnedGeometry::Decoded(points_geom_builder(&coords)),
        id: OwnedId::Decoded(Some(DecodedId(ids))),
        properties: vec![],
    }
}

fn get_vertices(layer: &OwnedLayer01) -> Vec<i32> {
    let OwnedGeometry::Decoded(g) = &layer.geometry else {
        panic!("geometry not in decoded form")
    };
    g.vertices.clone().unwrap_or_default()
}

fn get_ids(layer: &OwnedLayer01) -> Vec<Option<u64>> {
    match &layer.id {
        OwnedId::Decoded(Some(d)) => d.0.clone(),
        OwnedId::Decoded(None) => vec![],
        OwnedId::Encoded(_) => panic!("id not in decoded form"),
    }
}

fn get_string_col(layer: &OwnedLayer01, col: usize) -> Vec<Option<String>> {
    let OwnedProperty::Decoded(DecodedProperty::Str(s)) = &layer.properties[col] else {
        panic!("expected Str property at column {col}")
    };
    s.materialize()
}

fn assert_columns_in_sync(layer: &OwnedLayer01) {
    let lookup: HashMap<(i32, i32), (u64, &str)> = [
        ((3, 1), (40, "delta")),
        ((0, 0), (10, "alpha")),
        ((2, 3), (30, "gamma")),
        ((1, 2), (20, "beta")),
    ]
    .into_iter()
    .collect();

    let verts = get_vertices(layer);
    let ids = get_ids(layer);
    let classes = get_string_col(layer, 0);

    for i in 0..(verts.len() / 2) {
        let coord = (verts[2 * i], verts[2 * i + 1]);
        let (exp_id, exp_class) = lookup[&coord];
        assert_eq!(ids[i], Some(exp_id), "id mismatch at {i}");
        assert_eq!(
            classes[i].as_deref(),
            Some(exp_class),
            "class mismatch at {i}"
        );
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[rstest]
fn none_strategy_preserves_order(#[from(four_point_layer)] mut layer: OwnedLayer01) {
    reorder_features(&mut layer, SortStrategy::None).unwrap();
    assert_eq!(get_vertices(&layer), vec![3, 1, 0, 0, 2, 3, 1, 2]);
    assert_eq!(
        get_ids(&layer),
        vec![Some(40), Some(10), Some(30), Some(20)]
    );
}

// ── Non-Point geometry type coverage ─────────────────────────────────────────
//
// tests/sort.rs fixtures are all Points. These tests verify that reorder_features
// correctly rebuilds the offset arrays for every other geometry kind.

#[test]
fn linestrings_swap_preserves_offsets_and_vertices() {
    // Line 0: (0,0)->(1,1)   Line 1: (2,2)->(3,3)->(4,4)
    // Morton keys: first vertex of line 0 = (0,0) -> 0; line 1 = (2,2) -> key > 0
    // Both curves will keep them in the same order (line 0 already has the smaller key),
    // so use Id strategy on the attached IDs to force a known swap instead.
    let mut layer = linestrings_layer(&[&[(2, 2), (3, 3), (4, 4)], &[(0, 0), (1, 1)]]);
    // IDs are [Some(0), Some(1)]; Id sort keeps that order — swap by reversing the IDs.
    if let OwnedId::Decoded(Some(ref mut d)) = layer.id {
        d.0 = vec![Some(1), Some(0)];
    }
    reorder_features(&mut layer, SortStrategy::Id).unwrap();

    let g = get_geom(&layer);
    assert_eq!(
        g.vertices.as_deref().unwrap(),
        &[0, 0, 1, 1, 2, 2, 3, 3, 4, 4],
    );
    assert_eq!(g.part_offsets.as_deref(), Some(&[0u32, 2, 5][..]));
}

#[test]
fn polygons_swap_preserves_ring_offsets() {
    // Poly 0: one ring [(10,10),(11,10),(10,11)]  – further from origin, sorts last
    // Poly 1: one ring [(0,0),(1,0),(0,1)]        – closer to origin, sorts first
    let mut layer = polygons_layer(&[
        &[&[(10, 10), (11, 10), (10, 11)]],
        &[&[(0, 0), (1, 0), (0, 1)]],
    ]);
    reorder_features(&mut layer, SortStrategy::Spatial(SpaceFillingCurve::Morton)).unwrap();

    let g = get_geom(&layer);
    assert_eq!(
        g.vertices.as_deref().unwrap(),
        &[0, 0, 1, 0, 0, 1, 10, 10, 11, 10, 10, 11],
    );
    assert_eq!(g.part_offsets.as_deref(), Some(&[0u32, 1, 2][..]));
    assert_eq!(g.ring_offsets.as_deref(), Some(&[0u32, 3, 6][..]));
}

#[test]
fn polygons_multi_ring_preserves_ring_offsets() {
    // Poly 0: two rings (exterior + hole)   Poly 1: one ring
    // Give Poly 1 a smaller Morton key so it sorts first.
    let mut layer = polygons_layer(&[
        &[
            &[(10, 10), (11, 10), (10, 11)],
            &[(10, 10), (10, 11), (11, 11)],
        ],
        &[&[(0, 0), (1, 0), (0, 1)]],
    ]);
    reorder_features(&mut layer, SortStrategy::Spatial(SpaceFillingCurve::Morton)).unwrap();

    let g = get_geom(&layer);
    // Poly 1 (1 ring, 3 verts) comes first; Poly 0 (2 rings, 6 verts) second.
    assert_eq!(g.part_offsets.as_deref(), Some(&[0u32, 1, 3][..]));
    assert_eq!(g.ring_offsets.as_deref(), Some(&[0u32, 3, 6, 9][..]));
    assert_eq!(&g.vertices.as_deref().unwrap()[..6], &[0, 0, 1, 0, 0, 1],);
}

#[test]
fn multipoints_swap_preserves_geometry_offsets() {
    // Feature 0: single point (9,9)   Feature 1: two points (0,0),(1,1)
    let mut layer = multipoints_layer(&[&[(9, 9)], &[(0, 0), (1, 1)]]);
    reorder_features(&mut layer, SortStrategy::Spatial(SpaceFillingCurve::Morton)).unwrap();

    let g = get_geom(&layer);
    assert_eq!(g.vertices.as_deref().unwrap(), &[0, 0, 1, 1, 9, 9],);
    assert_eq!(g.geometry_offsets.as_deref(), Some(&[0u32, 2, 3][..]));
}

#[test]
fn multilinestrings_swap_preserves_all_offsets() {
    // Feature 0: two lines   Feature 1: one line
    // Give Feature 1 a smaller Morton key so it sorts first.
    let mut layer = multilinestrings_layer(&[
        &[&[(10, 10), (11, 11)], &[(12, 12), (13, 13), (14, 14)]],
        &[&[(0, 0), (1, 1)]],
    ]);
    reorder_features(&mut layer, SortStrategy::Spatial(SpaceFillingCurve::Morton)).unwrap();

    let g = get_geom(&layer);
    // Feature 1 (1 line, 2 verts) first; Feature 0 (2 lines, 5 verts) second.
    assert_eq!(g.geometry_offsets.as_deref(), Some(&[0u32, 1, 3][..]));
    assert_eq!(g.part_offsets.as_deref(), Some(&[0u32, 2, 4, 7][..]));
    assert_eq!(&g.vertices.as_deref().unwrap()[..4], &[0, 0, 1, 1],);
}

#[test]
fn multipolygons_swap_preserves_all_offsets() {
    // Feature 0: 1 polygon, 1 ring at (10,10)   Feature 1: 1 polygon, 1 ring at (0,0)
    let mut layer = multipolygons_layer(&[
        &[&[&[(10, 10), (11, 10), (10, 11)]]],
        &[&[&[(0, 0), (1, 0), (0, 1)]]],
    ]);
    reorder_features(&mut layer, SortStrategy::Spatial(SpaceFillingCurve::Morton)).unwrap();

    let g = get_geom(&layer);
    assert_eq!(
        g.vertices.as_deref().unwrap(),
        &[0, 0, 1, 0, 0, 1, 10, 10, 11, 10, 10, 11],
    );
    assert_eq!(g.geometry_offsets.as_deref(), Some(&[0u32, 1, 2][..]));
    assert_eq!(g.part_offsets.as_deref(), Some(&[0u32, 1, 2][..]));
    assert_eq!(g.ring_offsets.as_deref(), Some(&[0u32, 3, 6][..]));
}

// ── Edge cases ────────────────────────────────────────────────────────────────

#[test]
fn single_feature_is_noop() {
    let mut layer = linestrings_layer(&[&[(3, 7), (4, 8)]]);
    reorder_features(&mut layer, SortStrategy::Spatial(SpaceFillingCurve::Morton)).unwrap();
    assert_eq!(get_geom(&layer).vertices.as_deref().unwrap(), &[3, 7, 4, 8],);
}

#[test]
fn tessellated_layer_is_skipped() {
    let mut layer = linestrings_layer(&[&[(0, 0), (1, 1)], &[(5, 5), (6, 6)]]);
    if let OwnedGeometry::Decoded(ref mut g) = layer.geometry {
        g.index_buffer = Some(vec![0, 1, 2]);
    }
    let before = get_geom(&layer).vertices.clone();
    reorder_features(&mut layer, SortStrategy::Spatial(SpaceFillingCurve::Morton)).unwrap();
    assert_eq!(
        get_geom(&layer).vertices,
        before,
        "tessellated layer must not be reordered"
    );
}

#[test]
fn id_sort_without_id_column_preserves_order() {
    let mut layer = linestrings_layer(&[&[(9, 0), (8, 0)], &[(3, 3), (4, 4)], &[(0, 9), (0, 8)]]);
    layer.id = OwnedId::Decoded(None);
    reorder_features(&mut layer, SortStrategy::Id).unwrap();
    assert_eq!(
        get_geom(&layer).vertices.as_deref().unwrap(),
        &[9, 0, 8, 0, 3, 3, 4, 4, 0, 9, 0, 8],
    );
}

#[test]
fn morton_sort_known_order() {
    // Morton keys: (0,0)=0  (3,1)=7  (1,2)=9  (2,3)=14
    let mut layer = multipoints_layer(&[&[(3, 1)], &[(0, 0)], &[(2, 3)], &[(1, 2)]]);
    reorder_features(&mut layer, SortStrategy::Spatial(SpaceFillingCurve::Morton)).unwrap();
    assert_eq!(
        get_geom(&layer).vertices.as_deref().unwrap(),
        &[0, 0, 3, 1, 1, 2, 2, 3],
    );
}

mod id_strategy {
    use super::*;

    #[rstest]
    fn sorts_ascending_ids(#[from(four_point_layer)] mut layer: OwnedLayer01) {
        reorder_features(&mut layer, SortStrategy::Id).unwrap();
        assert_debug_snapshot!(get_ids(&layer), @"
        [
            Some(
                10,
            ),
            Some(
                20,
            ),
            Some(
                30,
            ),
            Some(
                40,
            ),
        ]
        ");
        assert_eq!(get_vertices(&layer), vec![0, 0, 1, 2, 2, 3, 3, 1]);
        assert_columns_in_sync(&layer);
    }

    #[rstest]
    fn null_ids_land_at_end(points_geom_builder: impl Fn(&[(i32, i32)]) -> DecodedGeometry) {
        let mut layer = OwnedLayer01 {
            name: String::new(),
            extent: 4096,
            geometry: OwnedGeometry::Decoded(points_geom_builder(&[(0, 0), (9, 9), (5, 5)])),
            id: OwnedId::Decoded(Some(DecodedId(vec![None, Some(2), Some(1)]))),
            properties: vec![],
        };
        reorder_features(&mut layer, SortStrategy::Id).unwrap();
        assert_eq!(get_ids(&layer), vec![Some(1), Some(2), None]);
        assert_eq!(get_vertices(&layer), vec![5, 5, 9, 9, 0, 0]);
    }
}

mod spatial_strategy {
    use super::*;

    #[rstest]
    fn keeps_columns_in_sync(
        #[from(four_point_layer)] mut layer: OwnedLayer01,
        #[values(SpaceFillingCurve::Morton, SpaceFillingCurve::Hilbert)] curve: SpaceFillingCurve,
    ) {
        reorder_features(&mut layer, SortStrategy::Spatial(curve)).unwrap();
        assert_columns_in_sync(&layer);
    }

    #[rstest]
    fn produces_monotone_keys(
        #[from(four_point_layer)] mut layer: OwnedLayer01,
        #[values(SpaceFillingCurve::Morton, SpaceFillingCurve::Hilbert)] curve: SpaceFillingCurve,
    ) {
        reorder_features(&mut layer, SortStrategy::Spatial(curve)).unwrap();
        let verts = get_vertices(&layer);
        let n = verts.len() / 2;

        let keys: Vec<u32> = match curve {
            SpaceFillingCurve::Morton => (0..n)
                .map(|i| morton_sort_key(verts[2 * i], verts[2 * i + 1], 0, 0))
                .collect(),
            SpaceFillingCurve::Hilbert => {
                let (shift, num_bits) = hilbert_curve_params(&verts);
                (0..n)
                    .map(|i| hilbert_sort_key(verts[2 * i], verts[2 * i + 1], shift, num_bits))
                    .collect()
            }
        };

        for w in keys.windows(2) {
            assert!(w[0] <= w[1], "{curve:?}: keys not monotone: {keys:?}");
        }
    }

    #[rstest]
    fn handles_negative_coordinates(
        #[from(negative_coords_layer)] mut layer: OwnedLayer01,
        #[values(SpaceFillingCurve::Morton, SpaceFillingCurve::Hilbert)] curve: SpaceFillingCurve,
    ) {
        reorder_features(&mut layer, SortStrategy::Spatial(curve)).unwrap();
        let verts = get_vertices(&layer);
        let ids = get_ids(&layer);
        let original: HashMap<(i32, i32), u64> = [
            ((-3, -3), 80),
            ((-1, -1), 70),
            ((0, 0), 60),
            ((1, 1), 50),
            ((-2, 2), 40),
            ((2, -2), 30),
            ((3, 3), 20),
            ((-3, 3), 10),
        ]
        .into_iter()
        .collect();

        for i in 0..(verts.len() / 2) {
            let coord = (verts[2 * i], verts[2 * i + 1]);
            assert_eq!(
                ids[i],
                Some(original[&coord]),
                "{curve:?}: id mismatch at {i}"
            );
        }
    }

    #[rstest]
    fn stable_on_identical_coords(points_geom_builder: impl Fn(&[(i32, i32)]) -> DecodedGeometry) {
        let mut layer = OwnedLayer01 {
            name: String::new(),
            extent: 4096,
            geometry: OwnedGeometry::Decoded(points_geom_builder(&[
                (1, 1),
                (1, 1),
                (1, 1),
                (1, 1),
            ])),
            id: OwnedId::Decoded(Some(DecodedId(vec![Some(1), Some(2), Some(3), Some(4)]))),
            properties: vec![],
        };
        reorder_features(&mut layer, SortStrategy::Spatial(SpaceFillingCurve::Morton)).unwrap();
        assert_eq!(get_ids(&layer), vec![Some(1), Some(2), Some(3), Some(4)]);
    }
}

mod optimization_integration {
    use super::*;

    #[rstest]
    fn manual_with_morton(#[from(four_point_layer)] mut layer: OwnedLayer01) {
        let enc = layer.automatic_encoding_optimisation().unwrap();
        let layer_enc =
            LayerEncoder::Tag01(enc).with_sort(SortStrategy::Spatial(SpaceFillingCurve::Morton));

        let mut layer2 = layer.clone();
        let LayerEncoder::Tag01(final_enc) = layer_enc else {
            panic!()
        };
        layer2.manual_optimisation(final_enc).unwrap();
    }

    #[rstest]
    fn profile_driven_with_hilbert(#[from(four_point_layer)] mut layer: OwnedLayer01) {
        let OwnedGeometry::Decoded(geom) = &layer.geometry else {
            panic!()
        };
        let OwnedId::Decoded(Some(id)) = &layer.id else {
            panic!()
        };

        let profile = Tag01Profile::new(
            SortStrategy::Spatial(SpaceFillingCurve::Hilbert),
            IdProfile::from_sample(id),
            PropertyProfile::new(vec![]),
            GeometryProfile::from_sample(geom).unwrap(),
        );

        let enc = layer.profile_driven_optimisation(&profile).unwrap();
        assert_eq!(
            enc.sort_strategy,
            SortStrategy::Spatial(SpaceFillingCurve::Hilbert)
        );
    }

    #[rstest]
    fn automatic_is_deterministic(#[from(four_point_layer)] mut layer: OwnedLayer01) {
        let mut layer_b = layer.clone();
        let enc_a = layer.automatic_encoding_optimisation().unwrap();
        let enc_b = layer_b.automatic_encoding_optimisation().unwrap();

        let mut buf_a = Vec::new();
        let mut buf_b = Vec::new();
        layer.write_to(&mut buf_a).unwrap();
        layer_b.write_to(&mut buf_b).unwrap();

        assert_eq!(enc_a.sort_strategy, enc_b.sort_strategy);
        assert_eq!(buf_a, buf_b);
    }
}

mod profile_merge {
    use super::*;

    fn empty_profile(strategy: SortStrategy) -> Tag01Profile {
        Tag01Profile::new(
            strategy,
            IdProfile::new(vec![]),
            PropertyProfile::new(vec![]),
            GeometryProfile::from_sample(&DecodedGeometry::default()).unwrap(),
        )
    }

    #[rstest]
    fn majority_wins() {
        let merged = empty_profile(SortStrategy::Id)
            .merge(&empty_profile(SortStrategy::Id))
            .merge(&empty_profile(SortStrategy::None));
        assert_eq!(merged.preferred_sort_strategy, SortStrategy::Id);
    }

    #[rstest]
    fn tie_breaks_to_simpler() {
        let merged = empty_profile(SortStrategy::None).merge(&empty_profile(SortStrategy::Id));
        assert_eq!(merged.preferred_sort_strategy, SortStrategy::None);
    }

    #[rstest]
    fn combines_sub_profiles(#[from(four_point_layer)] mut layer: OwnedLayer01) {
        let a = Tag01Profile::new(
            SortStrategy::None,
            IdProfile::new(vec![IntEncoder::varint()]),
            PropertyProfile::new(vec![]),
            GeometryProfile::from_sample(&DecodedGeometry::default()).unwrap(),
        );
        let b = Tag01Profile::new(
            SortStrategy::None,
            IdProfile::new(vec![IntEncoder::delta_rle_varint()]),
            PropertyProfile::new(vec![]),
            GeometryProfile::from_sample(&DecodedGeometry::default()).unwrap(),
        );

        layer
            .profile_driven_optimisation(&a.merge(&b))
            .expect("merge failed");
    }
}
