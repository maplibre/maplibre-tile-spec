//! What a v2 m-value section can make the decoder allocate, which its budget has to bound.

use mlt_core::encoder::{
    Codecs, Encoder, EncoderConfig, StagedId, StagedLayer, StagedMValue, StagedValues, WireVersion,
};
use mlt_core::geo_types::{Coord, Geometry, LineString};
use mlt_core::{Decoder, GeometryValues, Layer, MltError, Parser};

/// The budget the decode below is given, which is the default one.
const BUDGET: usize = 20 * 1024 * 1024;
const FEATURES: usize = 20_000;
const COLUMNS: usize = 128;

/// A tile whose m-value columns no feature carries values for.
///
/// Every column declares one presence bit per feature and no values at all, so a
/// column costs a handful of wire bytes however many features it claims.
fn absent_columns_tile() -> Vec<u8> {
    let mut geometry = GeometryValues::default();
    for i in 0..FEATURES {
        let i = i32::try_from(i).expect("a feature index fits in i32");
        let coords = vec![Coord { x: i, y: i }, Coord { x: i + 1, y: i + 1 }];
        geometry.push_geom(&Geometry::LineString(LineString::new(coords)));
    }
    let m_values = (0..COLUMNS)
        .map(|i| {
            StagedMValue::new(
                format!("m{i}"),
                Some(vec![false; FEATURES]),
                StagedValues::I32(Vec::new()),
            )
        })
        .collect();
    let cfg = EncoderConfig::default()
        .with_spatial_morton_sort(false)
        .with_spatial_hilbert_sort(false)
        .with_id_sort(false)
        .with_wire_version(WireVersion::V02);
    StagedLayer::with_m_values(
        "test_layer",
        4096,
        StagedId::None,
        geometry,
        vec![],
        m_values,
    )
    .expect("staged layer")
    .encode_into(Encoder::new(cfg), &mut Codecs::default())
    .expect("v2 encode")
    .into_layer_bytes()
    .expect("layer bytes")
}

/// The process's peak resident memory in bytes, or [`None`] where the OS does not report it.
fn peak_rss() -> Option<usize> {
    let status = std::fs::read_to_string("/proc/self/status").ok()?;
    let field = status.lines().find(|line| line.starts_with("VmHWM:"))?;
    let kib: usize = field.split_whitespace().nth(1)?.parse().ok()?;
    Some(kib * 1024)
}

#[test]
fn a_column_per_feature_cannot_allocate_past_the_decoder_budget() {
    let bytes = absent_columns_tile();
    assert!(
        bytes.len() < 64 * 1024,
        "a tile claiming {FEATURES} features and {COLUMNS} columns is {} bytes",
        bytes.len()
    );

    let mut parser = Parser::default();
    let layers = parser.parse_layers(&bytes).expect("parse");
    let Layer::Tag02(layer) = layers.into_iter().next().expect("a layer") else {
        panic!("expected a v2 layer")
    };

    let before = peak_rss();
    let budget = u32::try_from(BUDGET).expect("the budget fits in u32");
    let err = layer
        .into_tile(&mut Decoder::with_max_size(budget))
        .expect_err("the budget has to stop this");
    assert!(
        matches!(err, MltError::MemoryLimitExceeded { .. }),
        "{err:?}"
    );

    // Holding one span per feature per column would add FEATURES * COLUMNS * 24 bytes
    // on top of this, none of which the budget would ever see.
    let (Some(before), Some(after)) = (before, peak_rss()) else {
        return;
    };
    let grew = after.saturating_sub(before);
    assert!(
        grew < BUDGET * 3 / 2,
        "decoding grew the peak by {grew} bytes on a budget of {BUDGET}"
    );
}
