//! Round-trip tests for the vertex copies and line endpoint references of v2 predicted vertices.

use insta::assert_debug_snapshot;
use mlt_core::dump::{RenderOpts, annotate_tile, render};
use mlt_core::encoder::{
    EncoderConfig, Network, PredictedVertices, Profile, Tessellation, WireVersion,
};
use mlt_core::geo_types::{Geometry, LineString, MultiPolygon, Polygon};
use mlt_core::{Decoder, Layer, Parser, TileLayer};
use proptest::prelude::*;

fn line(coords: &[(i32, i32)]) -> Geometry<i32> {
    Geometry::LineString(LineString::from(coords.to_vec()))
}

fn polygon(exterior: &[(i32, i32)]) -> Geometry<i32> {
    Geometry::Polygon(Polygon::new(LineString::from(exterior.to_vec()), vec![]))
}

fn layer(geoms: Vec<Geometry<i32>>) -> TileLayer {
    let mut builder = TileLayer::builder("l", 4096).unwrap();
    for geom in geoms {
        builder.feature(geom).finish().unwrap();
    }
    builder.finish()
}

fn decode(bytes: &[u8]) -> TileLayer {
    let layers = Parser::default().parse_layers(bytes).unwrap();
    let Layer::Tag02(_) = &layers[0] else {
        panic!("expected a v2 layer");
    };
    layers
        .into_iter()
        .next()
        .unwrap()
        .into_tile(&mut Decoder::default())
        .unwrap()
        .unwrap()
}

fn predicted_headers(bytes: &[u8]) -> Vec<String> {
    let (tree, err) = annotate_tile(bytes);
    assert!(err.is_none(), "annotate_tile: {err:?}");
    let mut out = Vec::new();
    render(&tree, bytes, &RenderOpts::default(), &mut out).unwrap();
    String::from_utf8(out)
        .unwrap()
        .lines()
        .filter_map(|l| l.split_once("decoded: ").map(|(_, v)| v.to_string()))
        .filter(|v| v.starts_with("starts from "))
        .collect()
}

fn rotations(ring: &[(i32, i32)]) -> impl Iterator<Item = Vec<(i32, i32)>> + '_ {
    (0..ring.len()).map(|s| {
        ring.iter()
            .cycle()
            .skip(s)
            .take(ring.len())
            .copied()
            .collect()
    })
}

fn canonical_ring(ring: &[(i32, i32)]) -> Vec<(i32, i32)> {
    let stored = if ring.len() > 1 && ring.first() == ring.last() {
        &ring[..ring.len() - 1]
    } else {
        ring
    };
    let reversed: Vec<_> = stored.iter().rev().copied().collect();
    rotations(stored)
        .chain(rotations(&reversed))
        .min()
        .unwrap_or_default()
}

fn canonical_line(line: &[(i32, i32)]) -> Vec<(i32, i32)> {
    let reversed: Vec<_> = line.iter().rev().copied().collect();
    line.to_vec().min(reversed)
}

fn coords(line: &LineString<i32>) -> Vec<(i32, i32)> {
    line.coords().map(|c| (c.x, c.y)).collect()
}

#[expect(
    clippy::wildcard_enum_match_arm,
    reason = "the layers hold lines and polygons"
)]
fn canonical(geometry: &Geometry<i32>) -> Vec<Vec<(i32, i32)>> {
    let polygons = |polygons: &[Polygon<i32>]| {
        polygons
            .iter()
            .flat_map(|p| std::iter::once(p.exterior()).chain(p.interiors()))
            .map(|r| canonical_ring(&coords(r)))
            .collect()
    };
    match geometry {
        Geometry::LineString(l) => vec![canonical_line(&coords(l))],
        Geometry::Polygon(p) => polygons(std::slice::from_ref(p)),
        Geometry::MultiPolygon(MultiPolygon(ps)) => polygons(ps),
        other => panic!("unexpected geometry {other:?}"),
    }
}

fn canonical_layer(layer: &TileLayer) -> Vec<Vec<Vec<(i32, i32)>>> {
    let mut all: Vec<_> = layer
        .features()
        .iter()
        .map(|f| canonical(f.geometry()))
        .collect();
    all.sort();
    all
}

fn wall(i: i32) -> Vec<(i32, i32)> {
    (0..7)
        .map(|k| (100 * i + (k * 37 + i * 11) % 13, 20 * k + (k * i) % 5))
        .collect()
}

fn strip_cell(i: i32, rotated_by: usize) -> Vec<(i32, i32)> {
    let mut ring = wall(i);
    ring.extend(wall(i + 1).into_iter().rev());
    ring.rotate_left(rotated_by);
    ring
}

fn strip(rotated_by: usize) -> Vec<Geometry<i32>> {
    (0..6)
        .map(|i| polygon(&strip_cell(i, rotated_by)))
        .collect()
}

fn star() -> Vec<Geometry<i32>> {
    const DIRECTIONS: [(i32, i32); 6] =
        [(30, 4), (21, 22), (-3, 31), (-27, 12), (-24, -19), (9, -28)];
    (0..6)
        .map(|arm| {
            let (dx, dy) = DIRECTIONS[arm];
            let mut spoke: Vec<(i32, i32)> = (1..=9)
                .rev()
                .map(|k| {
                    (
                        2000 + k * dx * 8 + (k * k * 53) % 97 * 3,
                        2000 + k * dy * 8 - (k * 31) % 89 * 3,
                    )
                })
                .collect();
            spoke.push((2000, 2000));
            if arm % 2 == 0 {
                spoke.reverse();
            }
            line(&spoke)
        })
        .collect()
}

#[test]
fn polygons_that_share_walls_are_stored_as_copies() {
    let predicted = EncoderConfig::from_profile(Profile::Native(Network::Cellular4G))
        .with_wire_version(WireVersion::V02)
        .with_tessellation(Tessellation::Off)
        .with_predicted_vertices(PredictedVertices::Smallest);
    let bytes = layer(strip(0)).encode(predicted).unwrap();
    assert_debug_snapshot!(predicted_headers(&bytes), @r#"
    [
        "starts from the previous run, lines from the previous vertex, copies, 111 B",
    ]
    "#);
}

#[test]
fn the_fast_encoding_profile_stores_shared_walls_as_residuals() {
    let fast =
        EncoderConfig::from_profile(Profile::FastEncoding).with_wire_version(WireVersion::V02);
    let bytes = layer(strip(0)).encode(fast).unwrap();
    assert_debug_snapshot!(predicted_headers(&bytes), @"");
}

#[test]
fn copies_make_a_strip_of_polygons_smaller() {
    let source = layer(strip(0));
    let unpredicted = EncoderConfig::from_profile(Profile::Native(Network::Cellular4G))
        .with_wire_version(WireVersion::V02)
        .with_tessellation(Tessellation::Off)
        .with_predicted_vertices(PredictedVertices::Off);
    let predicted = EncoderConfig::from_profile(Profile::Native(Network::Cellular4G))
        .with_wire_version(WireVersion::V02)
        .with_tessellation(Tessellation::Off)
        .with_predicted_vertices(PredictedVertices::Smallest);
    let without = source.encode(unpredicted).unwrap();
    let with = source.encode(predicted).unwrap();
    assert_eq!((without.len(), with.len()), (201, 131));
}

#[test]
fn polygons_that_share_walls_come_back_unchanged_without_vertex_sort() {
    let source = layer(strip(3));
    let unpredicted = EncoderConfig::from_profile(Profile::Native(Network::Cellular4G))
        .with_wire_version(WireVersion::V02)
        .with_tessellation(Tessellation::Off)
        .with_predicted_vertices(PredictedVertices::Off);
    let predicted = EncoderConfig::from_profile(Profile::Native(Network::Cellular4G))
        .with_wire_version(WireVersion::V02)
        .with_tessellation(Tessellation::Off)
        .with_predicted_vertices(PredictedVertices::Smallest);
    assert_eq!(
        decode(&source.encode(predicted).unwrap()),
        decode(&source.encode(unpredicted).unwrap())
    );
}

#[test]
fn polygons_that_share_walls_decode_to_the_same_rings_under_vertex_sort() {
    let source = layer(strip(3));
    let predicted = EncoderConfig::from_profile(Profile::Native(Network::Cellular4G))
        .with_wire_version(WireVersion::V02)
        .with_tessellation(Tessellation::Off)
        .with_predicted_vertices(PredictedVertices::Smallest);
    let vertex_sorted = EncoderConfig::from_profile(Profile::Native(Network::Cellular4G))
        .with_wire_version(WireVersion::V02)
        .with_tessellation(Tessellation::Off)
        .with_predicted_vertices(PredictedVertices::Smallest)
        .with_vertex_sort(true);
    let unsorted = source.encode(predicted).unwrap();
    let sorted = source.encode(vertex_sorted).unwrap();
    assert_eq!((unsorted.len(), sorted.len()), (140, 138));
    assert_eq!(canonical_layer(&decode(&sorted)), canonical_layer(&source));
}

#[test]
fn lines_that_meet_at_a_junction_are_stored_as_line_endpoint_references() {
    let predicted = EncoderConfig::from_profile(Profile::Native(Network::Cellular4G))
        .with_wire_version(WireVersion::V02)
        .with_tessellation(Tessellation::Off)
        .with_predicted_vertices(PredictedVertices::Smallest);
    let bytes = layer(star()).encode(predicted).unwrap();
    assert_debug_snapshot!(predicted_headers(&bytes), @r#"
    [
        "starts from the previous vertex, lines from the previous vertex, line endpoint references, 154 B",
    ]
    "#);
}

#[test]
fn lines_that_meet_at_a_junction_come_back_unchanged_without_vertex_sort() {
    let source = layer(star());
    let unpredicted = EncoderConfig::from_profile(Profile::Native(Network::Cellular4G))
        .with_wire_version(WireVersion::V02)
        .with_tessellation(Tessellation::Off)
        .with_predicted_vertices(PredictedVertices::Off);
    let predicted = EncoderConfig::from_profile(Profile::Native(Network::Cellular4G))
        .with_wire_version(WireVersion::V02)
        .with_tessellation(Tessellation::Off)
        .with_predicted_vertices(PredictedVertices::Smallest);
    assert_eq!(
        decode(&source.encode(predicted).unwrap()),
        decode(&source.encode(unpredicted).unwrap())
    );
}

#[test]
fn lines_that_meet_at_a_junction_decode_to_the_same_lines_under_vertex_sort() {
    let source = layer(star());
    let vertex_sorted = EncoderConfig::from_profile(Profile::Native(Network::Cellular4G))
        .with_wire_version(WireVersion::V02)
        .with_tessellation(Tessellation::Off)
        .with_predicted_vertices(PredictedVertices::Smallest)
        .with_vertex_sort(true);
    let sorted = source.encode(vertex_sorted).unwrap();
    assert_eq!(canonical_layer(&decode(&sorted)), canonical_layer(&source));
}

fn arb_geometry() -> impl Strategy<Value = Geometry<i32>> {
    prop_oneof![
        prop::collection::vec((0..5, 0..5), 2..9).prop_map(|c| line(&c)),
        prop::collection::vec((0..5, 0..5), 3..9).prop_map(|c| polygon(&c)),
    ]
}

proptest! {
    #[test]
    fn any_layer_over_few_positions_decodes_to_the_same_geometries(
        geoms in prop::collection::vec(arb_geometry(), 1..12),
    ) {
        let predicted = EncoderConfig::from_profile(Profile::Native(Network::Cellular4G))
            .with_wire_version(WireVersion::V02)
            .with_tessellation(Tessellation::Off)
            .with_predicted_vertices(PredictedVertices::Smallest);
        let vertex_sorted = EncoderConfig::from_profile(Profile::Native(Network::Cellular4G))
            .with_wire_version(WireVersion::V02)
            .with_tessellation(Tessellation::Off)
            .with_predicted_vertices(PredictedVertices::Smallest)
            .with_vertex_sort(true);
        let source = layer(geoms);
        for cfg in [predicted, vertex_sorted] {
            let decoded = decode(&source.encode(cfg).unwrap());
            prop_assert_eq!(canonical_layer(&decoded), canonical_layer(&source));
        }
    }
}
