use mlt_core::encoder::{EncoderConfig, Network, Profile, Ranking, WireVersion};
use mlt_core::geo_types::{Geometry, Point};
use mlt_core::wire::{Analyze as _, StreamMeta};
use mlt_core::{Layer, Parser, PropKind, PropValue, TileLayer};
use rstest::rstest;

fn xorshift(n: usize) -> impl Iterator<Item = u64> {
    let mut state = 0x2545_f491_4f6c_dd1d_u64;
    (0..n).map(move |_| {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    })
}

fn three_large_values_in_random_order() -> TileLayer {
    let mut builder = TileLayer::builder("layer", 4096).unwrap();
    let key = builder.add_property("v", PropKind::U32).unwrap();
    for (x, n) in (0..).zip(xorshift(2000)) {
        let value = [100_000, 200_000, 300_000][usize::try_from(n % 3).unwrap()];
        let mut feature = builder.feature(Geometry::Point(Point::new(x % 4096, 7)));
        feature.property(key, PropValue::U32(Some(value))).unwrap();
        feature.finish().unwrap();
    }
    builder.finish()
}

#[expect(clippy::wildcard_enum_match_arm, reason = "Layer is non_exhaustive")]
fn last_stream_encoding(bytes: &[u8]) -> String {
    let layers = Parser::default().parse_layers(bytes).unwrap();
    let mut last = String::new();
    let mut push = |meta: StreamMeta| {
        last = format!(
            "{} {} {}",
            meta.stream_type, meta.encoding.logical, meta.encoding.physical
        );
    };
    match &layers[0] {
        Layer::Tag01(layer) => layer.for_each_stream(&mut push),
        Layer::Tag02(layer) => layer.for_each_stream(&mut push),
        _ => panic!("expected a known layer"),
    }
    last
}

#[rstest]
#[case::v1(WireVersion::V01)]
#[case::v2(WireVersion::V02)]
fn the_web_preset_keeps_the_column_coding_that_deflates_smaller(#[case] wire: WireVersion) {
    let web =
        EncoderConfig::from_profile(Profile::Web(Network::Cellular4G)).with_wire_version(wire);
    let bytes = three_large_values_in_random_order().encode(web).unwrap();
    assert_eq!(last_stream_encoding(&bytes), "data int/none varint");
}

#[rstest]
#[case::v1(WireVersion::V01)]
#[case::v2(WireVersion::V02)]
fn the_web_preset_ranked_by_stored_size_keeps_the_column_coding_that_stores_fewer_bytes(
    #[case] wire: WireVersion,
) {
    let web = EncoderConfig::from_profile(Profile::Web(Network::Cellular4G))
        .with_wire_version(wire)
        .with_ranking(Ranking::StoredSize);
    let bytes = three_large_values_in_random_order().encode(web).unwrap();
    assert_eq!(last_stream_encoding(&bytes), "data int/delta varint");
}
